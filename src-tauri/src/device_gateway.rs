use std::sync::{Arc, Mutex};
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::device_manager::{DeviceManager, DeviceInfo};
use crate::utils::get_data_dir;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct GatewayState {
    pub manager: DeviceManager,
}

#[derive(serde::Deserialize)]
pub struct WsParams {
    pub device_id: String,
    pub model: String,
}

pub async fn start_gateway_server(manager: DeviceManager) {
    let state = Arc::new(GatewayState { manager });
    
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1421").await.unwrap();
    println!("[Device Gateway] Listening on 0.0.0.0:1421");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, state))
}

async fn handle_socket(socket: WebSocket, params: WsParams, state: Arc<GatewayState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let device_id = params.device_id.clone();
    
    // 注册设备
    state.manager.register(DeviceInfo {
        id: device_id.clone(),
        model: params.model.clone(),
        last_seen: now_secs(),
        status: "online".to_string(),
    }, tx);

    // 任务1：将 tx 发送的指令通过 WebSocket 发给手机
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 录音状态暂存
    let mut current_file: Option<(String, File)> = None;
    let recordings_dir = get_data_dir().join("recordings");
    let _ = std::fs::create_dir_all(&recordings_dir);

    // 任务2：处理来自手机的消息
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    match json["type"].as_str() {
                        Some("ping") => {
                            // 自动回复心跳或仅更新最后在线时间
                        }
                        Some("file_start") => {
                            let name = json["name"].as_str().unwrap_or("unnamed.m4a");
                            let path = recordings_dir.join(format!("{}_{}", device_id, name));
                            if let Ok(f) = File::create(&path) {
                                current_file = Some((name.to_string(), f));
                            }
                        }
                        Some("file_end") => {
                            current_file = None;
                            println!("[Gateway] Recording received from {}", device_id);
                        }
                        _ => {}
                    }
                }
            }
            Message::Binary(bin) => {
                if let Some((_, ref mut f)) = current_file {
                    let _ = f.write_all(&bin);
                }
            }
            _ => {}
        }
    }

    // 断连处理
    send_task.abort();
    state.manager.unregister(&device_id);
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}
