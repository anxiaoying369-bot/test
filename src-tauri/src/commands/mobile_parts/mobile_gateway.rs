// 手机无线控制：WebSocket 服务端 + 设备管理。
//
// 配套 Android 端为 autocast-mobile（AccessibilityService 模拟触控 + 录音回传）。
// 协议见 autocast-mobile/DESIGN_PLAN.md：
//   - 手机连接 ws://PC_IP:1422/ws?device_id=xxx&model=yyy
//   - 下行 JSON 指令：click / swipe / key / screenshot
//   - 上行 JSON：ping / device_info / file_start / file_end，文件内容走二进制分片
// 截图（file_type=image）在内存中重组后以 base64 通过事件推给前端；
// 录音等其他文件落盘到 data_dir/mobile/recordings/<device_id>/。

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::state::AppState;
use crate::utils::get_data_dir;

pub const MOBILE_WS_PORT: u16 = 1422;

/// 心跳间隔为 15s（手机端写死），超过 60s 没有任何消息视为掉线。
const READ_TIMEOUT_SECS: u64 = 60;

#[derive(Clone, Serialize)]
pub struct OnlineDevice {
    pub device_id: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub connected_at: i64,
    pub last_seen: i64,
}

pub struct DeviceConn {
    /// 同一 device_id 重连时用于识别旧连接，避免旧任务清理掉新连接。
    conn_id: u64,
    info: OnlineDevice,
    tx: mpsc::UnboundedSender<Message>,
}

pub type Devices = Arc<RwLock<HashMap<String, DeviceConn>>>;

pub struct MobileState {
    pub devices: Devices,
}

impl Default for MobileState {
    fn default() -> Self {
        Self { devices: Arc::new(RwLock::new(HashMap::new())) }
    }
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

// ============ WebSocket Server ============

pub async fn run_ws_server(app: AppHandle, devices: Devices) {
    let listener = match TcpListener::bind(("0.0.0.0", MOBILE_WS_PORT)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[mobile] 无法监听端口 {}: {}", MOBILE_WS_PORT, e);
            return;
        }
    };
    println!("[mobile] WebSocket server listening on 0.0.0.0:{}", MOBILE_WS_PORT);

    let mut next_conn_id: u64 = 0;
    while let Ok((stream, addr)) = listener.accept().await {
        next_conn_id += 1;
        let conn_id = next_conn_id;
        let app = app.clone();
        let devices = devices.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(app, devices, stream, conn_id).await {
                eprintln!("[mobile] connection {} ({}) closed with error: {}", conn_id, addr, e);
            }
        });
    }
}

/// 从 ws 握手 URL query 中取参数（手机端的 device_id 不含需要转义的字符，不做 url-decode）。
fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

struct IncomingFile {
    name: String,
    size: u64,
    file_type: String,
    buf: Vec<u8>,
}

async fn handle_connection(
    app: AppHandle,
    devices: Devices,
    stream: TcpStream,
    conn_id: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut device_id = String::new();
    let mut model = String::new();
    let ws = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, res: Response| {
        if let Some(q) = req.uri().query() {
            device_id = query_param(q, "device_id").unwrap_or_default();
            model = query_param(q, "model").unwrap_or_default();
        }
        Ok(res)
    })
    .await?;

    if device_id.is_empty() {
        device_id = format!("unknown_{}", conn_id);
    }
    if model.is_empty() {
        model = device_id.clone();
    }

    let (mut sink, mut source) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 注册设备（同 id 重连直接顶掉旧连接）
    {
        let mut map = devices.write().await;
        map.insert(
            device_id.clone(),
            DeviceConn {
                conn_id,
                info: OnlineDevice {
                    device_id: device_id.clone(),
                    model: model.clone(),
                    width: 0,
                    height: 0,
                    connected_at: now_ts(),
                    last_seen: now_ts(),
                },
                tx,
            },
        );
    }
    upsert_device_record(&app, &device_id, &model);
    emit_devices_changed(&app, &devices).await;
    println!("[mobile] device connected: {} ({})", device_id, model);

    // 下行转发任务：channel -> websocket
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // 上行读取循环
    let mut incoming_file: Option<IncomingFile> = None;
    loop {
        let next = tokio::time::timeout(
            std::time::Duration::from_secs(READ_TIMEOUT_SECS),
            source.next(),
        )
        .await;
        let msg = match next {
            Err(_) => break,            // 超时未收到心跳，判定掉线
            Ok(None) => break,          // 流结束
            Ok(Some(Err(_))) => break,  // 协议/网络错误
            Ok(Some(Ok(m))) => m,
        };

        match msg {
            Message::Text(text) => {
                handle_text_message(&app, &devices, &device_id, &text, &mut incoming_file).await;
            }
            Message::Binary(data) => {
                if let Some(f) = incoming_file.as_mut() {
                    f.buf.extend_from_slice(&data);
                }
            }
            Message::Ping(_) | Message::Pong(_) => {
                touch_device(&devices, &device_id).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    writer.abort();

    // 清理：仅当 map 里还是本连接时才移除（避免误删重连后的新连接）
    let removed = {
        let mut map = devices.write().await;
        match map.get(&device_id) {
            Some(c) if c.conn_id == conn_id => {
                map.remove(&device_id);
                true
            }
            _ => false,
        }
    };
    if removed {
        touch_device_record(&app, &device_id);
        emit_devices_changed(&app, &devices).await;
        println!("[mobile] device disconnected: {}", device_id);
    }
    Ok(())
}

async fn handle_text_message(
    app: &AppHandle,
    devices: &Devices,
    device_id: &str,
    text: &str,
    incoming_file: &mut Option<IncomingFile>,
) {
    let json: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };
    touch_device(devices, device_id).await;

    match json.get("type").and_then(|v| v.as_str()).unwrap_or("") {
        "ping" => {
            let map = devices.read().await;
            if let Some(c) = map.get(device_id) {
                let _ = c.tx.send(Message::Text("{\"type\":\"pong\"}".into()));
            }
        }
        "device_info" => {
            {
                let mut map = devices.write().await;
                if let Some(c) = map.get_mut(device_id) {
                    c.info.width = json.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    c.info.height = json.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                }
            }
            emit_devices_changed(app, devices).await;
        }
        "adb_info" => {
            let target = json.get("target").and_then(|v| v.as_str()).unwrap_or("");
            let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if !target.is_empty() {
                println!("[mobile] Received ADB info from {}: target={}, code={}", device_id, target, code);
                
                // 1. 如果有配对码，先执行配对
                if !code.is_empty() {
                    let mut child = std::process::Command::new("adb")
                        .arg("pair")
                        .arg(target)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                        .expect("Failed to start adb pair");
                    
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(format!("{}\n", code).as_bytes());
                    }
                    let _ = child.wait();
                }

                // 2. 对于配对和连接，通常端口是不同的（Android 11+ 中配对端口和连接端口随机分配且不同）
                // 但是某些设备直接 connect 也能行。如果是 Android 11+，配对完成后，开发者选项界面会显示另一个“连接端口”。
                // 这里我们尝试执行 connect
                let _ = crate::utils::std_command("adb")
                    .arg("connect")
                    .arg(target)
                    .output();
                
                // 触发前端提醒
                let _ = app.emit(
                    "mobile-adb-connected",
                    serde_json::json!({ "device_id": device_id, "target": target }),
                );
            }
        }
        "file_start" => {

            *incoming_file = Some(IncomingFile {
                name: json.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed").to_string(),
                size: json.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                file_type: json.get("file_type").and_then(|v| v.as_str()).unwrap_or("file").to_string(),
                buf: Vec::new(),
            });
        }
        "file_end" => {
            if let Some(file) = incoming_file.take() {
                finalize_file(app, device_id, file);
            }
        }
        _ => {}
    }
}

fn finalize_file(app: &AppHandle, device_id: &str, file: IncomingFile) {
    if file.size > 0 && (file.buf.len() as u64) < file.size {
        eprintln!(
            "[mobile] file {} from {} incomplete: {}/{} bytes, discarded",
            file.name, device_id, file.buf.len(), file.size
        );
        return;
    }
    if file.file_type == "image" {
        // 截图：不落盘，直接推给前端实时显示
        let b64 = base64::engine::general_purpose::STANDARD.encode(&file.buf);
        let _ = app.emit(
            "mobile-screenshot",
            serde_json::json!({ "device_id": device_id, "data": b64 }),
        );
        return;
    }
    // 录音等文件：落盘并通知前端
    let dir = get_data_dir().join("mobile").join("recordings").join(device_id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[mobile] create dir failed: {}", e);
        return;
    }
    let path = dir.join(&file.name);
    match std::fs::write(&path, &file.buf) {
        Ok(_) => {
            let _ = app.emit(
                "mobile-file-received",
                serde_json::json!({
                    "device_id": device_id,
                    "name": file.name,
                    "path": path.to_string_lossy(),
                    "size": file.buf.len(),
                    "file_type": file.file_type,
                }),
            );
        }
        Err(e) => eprintln!("[mobile] write file failed: {}", e),
    }
}

async fn touch_device(devices: &Devices, device_id: &str) {
    let mut map = devices.write().await;
    if let Some(c) = map.get_mut(device_id) {
        c.info.last_seen = now_ts();
    }
}

async fn emit_devices_changed(app: &AppHandle, devices: &Devices) {
    let list: Vec<OnlineDevice> = {
        let map = devices.read().await;
        map.values().map(|c| c.info.clone()).collect()
    };
    let _ = app.emit("mobile-devices-changed", list);
}

/// 设备首次连接时把 device_id/model 记入 sqlite（备注表），便于离线后仍可见。
fn upsert_device_record(app: &AppHandle, device_id: &str, model: &str) {
    let state = app.state::<AppState>();
    let guard = state.video_db.lock();
    if let Ok(db) = guard {
        let _ = crate::db::mobile_upsert_device(&db, device_id, model);
    }
}

fn touch_device_record(app: &AppHandle, device_id: &str) {
    let state = app.state::<AppState>();
    let guard = state.video_db.lock();
    if let Ok(db) = guard {
        let _ = crate::db::mobile_touch_device(&db, device_id);
    }
}

// ============ Tauri Commands ============

#[derive(Serialize)]
