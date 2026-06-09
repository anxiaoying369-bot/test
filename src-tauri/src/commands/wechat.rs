// 微信聊天监控接入层。
//
// 完全独立实现：拉起常驻 Python 引擎 `scripts/wechat/wechat_engine.py`，它用公开的
// SQLCipher-v4 参数自行解密微信 4.0+ 的加密库（不依赖 WeFlow 的任何原生库，规避其
// 反盗版保护）。通信走行分隔 JSON（{id,type,...} → {id,result|error}），日志走 stderr。
//
// 对前端暴露的命令：
//   wechat_get_key / wechat_open / wechat_list_sessions / wechat_get_messages /
//   wechat_resolve_session / wechat_start_monitor / wechat_stop_monitor /
//   wechat_get_status / wechat_save_credentials / wechat_load_credentials
//
// 新消息监控走 Rust 侧 tokio 轮询 get_new_messages → emit("wechat-new-message")。

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::oneshot;

use crate::state::AppState;
use crate::utils::{get_data_dir, get_scripts_dir, python_cmd};

type PendingMap = Arc<StdMutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// 常驻引擎句柄。`call` 写一行请求到 stdin，并通过 pending 表等待对应响应。
pub struct WeChatSidecar {
    _child: tokio::process::Child,
    stdin: ChildStdin,
    pending: PendingMap,
    counter: u64,
    pub connected: bool,
    pub account_dir: Option<String>,
    /// 监控运行标志（被 stop_monitor 置 false，轮询任务自行退出）。
    pub monitor_running: Option<Arc<AtomicBool>>,
}

impl WeChatSidecar {
    /// 发一个请求：协议把 `type` 与 payload 的字段一起**平铺**进顶层 JSON。
    async fn call(&mut self, typ: &str, payload: Value, timeout: Duration) -> Result<Value, String> {
        self.counter += 1;
        let id = self.counter;
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id, tx);

        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), json!(id));
        obj.insert("type".to_string(), json!(typ));
        if let Some(p) = payload.as_object() {
            for (k, v) in p {
                obj.insert(k.clone(), v.clone());
            }
        }
        let line = format!("{}\n", Value::Object(obj));
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("写入引擎失败: {e}"))?;
        self.stdin.flush().await.map_err(|e| e.to_string())?;

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("引擎通道已关闭".to_string()),
            Err(_) => {
                self.pending.lock().ok().and_then(|mut m| m.remove(&id));
                Err("引擎调用超时".to_string())
            }
        }
    }
}

/// 定位打包进 bundle 的微信资源目录（resources/wechat，含密钥提取 helper）。
fn resolve_wechat_resources_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(debug_assertions)]
    {
        candidates.push(PathBuf::from("resources").join("wechat"));
        candidates.push(PathBuf::from("src-tauri").join("resources").join("wechat"));
        candidates.push(PathBuf::from("..").join("src-tauri").join("resources").join("wechat"));
        if let Ok(exe) = std::env::current_exe() {
            let mut dir = exe.parent().map(|p| p.to_path_buf());
            while let Some(d) = dir {
                let cand = d.join("src-tauri").join("resources").join("wechat");
                if cand.exists() {
                    return Some(cand);
                }
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    if let Some(res) = crate::state::RESOURCE_DIR.get() {
        candidates.push(res.join("resources").join("wechat"));
        candidates.push(res.join("wechat"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("resources").join("wechat"));
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("resources").join("wechat"));
            }
        }
    }

    candidates.into_iter().find(|c| c.exists())
}

/// 确保引擎已启动；首次调用时拉起 python wechat_engine.py 并挂上 stdout/stderr 读取任务。
async fn ensure_sidecar(state: &State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.wechat.lock().await;
    if guard.is_some() {
        return Ok(());
    }

    let script = get_scripts_dir().join("wechat").join("wechat_engine.py");
    if !script.exists() {
        return Err(format!("微信引擎脚本缺失: {}", script.display()));
    }

    let mut cmd = python_cmd();
    cmd.arg(&script)
        .env("WECHAT_USER_DATA", get_data_dir().join("wechat"))
        .env("WECHAT_FFMPEG", crate::ffmpeg::get_ffmpeg_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(resources) = resolve_wechat_resources_dir() {
        cmd.env("WCDB_RESOURCES_PATH", resources);
    }

    let mut child = cmd.spawn().map_err(|e| format!("启动微信引擎失败: {e}"))?;
    let stdin = child.stdin.take().ok_or("无法获取引擎 stdin")?;
    let stdout = child.stdout.take().ok_or("无法获取引擎 stdout")?;
    let stderr = child.stderr.take().ok_or("无法获取引擎 stderr")?;

    let pending: PendingMap = Arc::new(StdMutex::new(HashMap::new()));

    // stdout：解析响应并唤醒对应 pending
    let pending_reader = pending.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                    let tx = pending_reader.lock().ok().and_then(|mut m| m.remove(&id));
                    if let Some(tx) = tx {
                        if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                            let _ = tx.send(Err(err.to_string()));
                        } else {
                            let _ = tx.send(Ok(v.get("result").cloned().unwrap_or(Value::Null)));
                        }
                    }
                }
            }
        }
    });

    // stderr：日志透传
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("{line}");
        }
    });

    *guard = Some(WeChatSidecar {
        _child: child,
        stdin,
        pending,
        counter: 0,
        connected: false,
        account_dir: None,
        monitor_running: None,
    });
    Ok(())
}

fn creds_path() -> PathBuf {
    get_data_dir().join("wechat_state.json")
}

// ---------------------------------------------------------------------------
// 命令
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn wechat_get_key(state: State<'_, AppState>) -> Result<Value, String> {
    ensure_sidecar(&state).await?;
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("引擎未初始化")?;
    // 密钥提取含 sudo 授权 + hook，给足超时
    sc.call("get_key", json!({ "timeoutMs": 180000 }), Duration::from_secs(240))
        .await
}

#[tauri::command]
pub async fn wechat_open(
    account_dir: String,
    hex_key: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    ensure_sidecar(&state).await?;
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("引擎未初始化")?;
    let res = sc
        .call(
            "open",
            json!({ "accountDir": account_dir, "hexKey": hex_key }),
            Duration::from_secs(60),
        )
        .await?;
    let ok = res.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    if ok {
        sc.connected = true;
        sc.account_dir = Some(account_dir.clone());
        Ok(true)
    } else {
        let err = res
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("打开数据库失败")
            .to_string();
        Err(err)
    }
}

#[tauri::command]
pub async fn wechat_list_sessions(state: State<'_, AppState>) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call("get_sessions", json!({}), Duration::from_secs(60)).await
}

#[tauri::command]
pub async fn wechat_list_contacts(state: State<'_, AppState>) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call("get_contacts", json!({}), Duration::from_secs(60)).await
}

#[tauri::command]
pub async fn wechat_get_messages(
    session_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call(
        "get_messages",
        json!({ "sessionId": session_id, "limit": limit.unwrap_or(100), "offset": offset.unwrap_or(0) }),
        Duration::from_secs(60),
    )
    .await
}

/// 取某条语音消息的可播放音频（SILK→WAV，base64）。
#[tauri::command]
pub async fn wechat_get_voice(
    session_id: String,
    svr_id: Option<i64>,
    local_id: Option<i64>,
    create_time: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call(
        "get_voice",
        json!({
            "sessionId": session_id,
            "svrId": svr_id.unwrap_or(0),
            "localId": local_id.unwrap_or(0),
            "createTime": create_time.unwrap_or(0),
        }),
        Duration::from_secs(60),
    )
    .await
}

/// 取图片消息的解密图片（base64）。want_full=false 取缩略图，true 取大图（wxgf 会转码）。
#[tauri::command]
pub async fn wechat_get_image(
    session_id: String,
    local_id: i64,
    want_full: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call(
        "get_image",
        json!({ "sessionId": session_id, "localId": local_id, "wantFull": want_full.unwrap_or(false) }),
        Duration::from_secs(60),
    )
    .await
}

/// 解析视频消息的明文 mp4 路径并用系统默认播放器打开。
#[tauri::command]
pub async fn wechat_open_video(
    session_id: String,
    local_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let res = {
        let mut guard = state.wechat.lock().await;
        let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
        sc.call(
            "get_video",
            json!({ "sessionId": session_id, "localId": local_id }),
            Duration::from_secs(30),
        )
        .await?
    };
    if !res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(res
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("未找到视频文件")
            .to_string());
    }
    let path = res
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or("视频路径为空")?
        .to_string();

    #[cfg(target_os = "macos")]
    let spawn = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let spawn = std::process::Command::new("cmd").args(["/C", "start", "", &path]).spawn();
    #[cfg(target_os = "linux")]
    let spawn = std::process::Command::new("xdg-open").arg(&path).spawn();
    spawn.map_err(|e| format!("打开视频失败: {e}"))?;
    Ok(())
}

/// 取某条媒体消息（视频缩略图等）的图片数据（base64）。
#[tauri::command]
pub async fn wechat_get_media(
    session_id: String,
    local_id: i64,
    local_type: i64,
    svr_id: Option<i64>,
    create_time: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call(
        "get_media",
        json!({
            "sessionId": session_id,
            "localId": local_id,
            "localType": local_type,
            "svrId": svr_id.unwrap_or(0),
            "createTime": create_time.unwrap_or(0),
        }),
        Duration::from_secs(60),
    )
    .await
}

#[tauri::command]
pub async fn wechat_resolve_session(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut guard = state.wechat.lock().await;
    let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
    sc.call("resolve_session", json!({ "keyword": keyword }), Duration::from_secs(60))
        .await
}

/// 监控时携带的会话信息。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTarget {
    pub session_id: String,
    pub display_name: String,
}

#[tauri::command]
pub async fn wechat_start_monitor(
    targets: Vec<MonitorTarget>,
    interval_secs: Option<u64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if targets.is_empty() {
        return Err("请至少选择一个要监控的会话".to_string());
    }
    let interval = interval_secs.unwrap_or(5).max(2);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let running = Arc::new(AtomicBool::new(true));
    {
        let mut guard = state.wechat.lock().await;
        let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
        if let Some(old) = sc.monitor_running.take() {
            old.store(false, Ordering::SeqCst);
        }
        sc.monitor_running = Some(running.clone());
    }

    let mut last_times: HashMap<String, i64> =
        targets.iter().map(|t| (t.session_id.clone(), now)).collect();
    let names: HashMap<String, String> = targets
        .iter()
        .map(|t| (t.session_id.clone(), t.display_name.clone()))
        .collect();
    let session_ids: Vec<String> = targets.iter().map(|t| t.session_id.clone()).collect();

    tokio::spawn(async move {
        let app_state = app.state::<AppState>();
        loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if !running.load(Ordering::SeqCst) {
                break;
            }

            for sid in &session_ids {
                let min_time = *last_times.get(sid).unwrap_or(&now);
                let res = {
                    let mut guard = app_state.wechat.lock().await;
                    let sc = match guard.as_mut() {
                        Some(sc) if sc.connected => sc,
                        _ => continue,
                    };
                    sc.call(
                        "get_new_messages",
                        json!({ "sessionId": sid, "minTime": min_time, "limit": 50 }),
                        Duration::from_secs(30),
                    )
                    .await
                };

                let messages = match res {
                    Ok(v) => v
                        .get("messages")
                        .and_then(|m| m.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    Err(e) => {
                        eprintln!("[wechat-monitor] get_new_messages 失败 ({sid}): {e}");
                        continue;
                    }
                };

                let fresh: Vec<Value> = messages
                    .into_iter()
                    .filter(|m| m.get("createTime").and_then(|c| c.as_i64()).unwrap_or(0) > min_time)
                    .collect();
                if fresh.is_empty() {
                    continue;
                }
                let max_time = fresh
                    .iter()
                    .filter_map(|m| m.get("createTime").and_then(|c| c.as_i64()))
                    .max()
                    .unwrap_or(min_time);
                last_times.insert(sid.clone(), max_time);

                let _ = app.emit(
                    "wechat-new-message",
                    json!({
                        "sessionId": sid,
                        "displayName": names.get(sid).cloned().unwrap_or_default(),
                        "messages": fresh,
                    }),
                );
            }
        }
        eprintln!("[wechat-monitor] 监控轮询已退出");
    });

    Ok(())
}

#[tauri::command]
pub async fn wechat_stop_monitor(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.wechat.lock().await;
    if let Some(sc) = guard.as_mut() {
        if let Some(running) = sc.monitor_running.take() {
            running.store(false, Ordering::SeqCst);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn wechat_get_status(state: State<'_, AppState>) -> Result<Value, String> {
    let guard = state.wechat.lock().await;
    let (connected, monitoring, account_dir) = match guard.as_ref() {
        Some(sc) => (
            sc.connected,
            sc.monitor_running
                .as_ref()
                .map(|r| r.load(Ordering::SeqCst))
                .unwrap_or(false),
            sc.account_dir.clone(),
        ),
        None => (false, false, None),
    };
    Ok(json!({
        "connected": connected,
        "monitoring": monitoring,
        "accountDir": account_dir,
    }))
}

#[tauri::command]
pub async fn wechat_save_credentials(account_dir: String, hex_key: String) -> Result<(), String> {
    let path = creds_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&json!({
        "accountDir": account_dir,
        "hexKey": hex_key,
    }))
    .map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wechat_load_credentials() -> Result<Value, String> {
    let path = creds_path();
    if !path.exists() {
        return Ok(json!({ "accountDir": null, "hexKey": null }));
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(v)
}
