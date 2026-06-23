pub struct DeviceView {
    pub device_id: String,
    pub model: String,
    pub remark: Option<String>,
    pub online: bool,
    pub width: u32,
    pub height: u32,
    pub last_seen: i64,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub port: u16,
    pub ips: Vec<String>,
}

/// 返回服务端口和本机局域网 IP，供手机端填写连接地址。
#[tauri::command]
pub fn mobile_get_server_info() -> ServerInfo {
    let mut ips: Vec<String> = Vec::new();
    if let Ok(ifas) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in ifas {
            if let std::net::IpAddr::V4(v4) = ip {
                if !v4.is_loopback() && !v4.is_link_local() {
                    ips.push(v4.to_string());
                }
            }
        }
    }
    ServerInfo { port: MOBILE_WS_PORT, ips }
}

/// 列出所有已知设备（数据库记录 + 在线状态合并）。
#[tauri::command]
pub async fn mobile_list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceView>, String> {
    let online: HashMap<String, OnlineDevice> = {
        let map = state.mobile.devices.read().await;
        map.values().map(|c| (c.info.device_id.clone(), c.info.clone())).collect()
    };

    let records = {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        crate::db::mobile_list_devices(&db).map_err(|e| e.to_string())?
    };

    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<DeviceView> = Vec::new();
    for rec in records {
        seen.insert(rec.device_id.clone());
        let on = online.get(&rec.device_id);
        out.push(DeviceView {
            device_id: rec.device_id,
            model: rec.model,
            remark: rec.remark,
            online: on.is_some(),
            width: on.map(|d| d.width).unwrap_or(0),
            height: on.map(|d| d.height).unwrap_or(0),
            last_seen: on.map(|d| d.last_seen).unwrap_or(rec.last_seen),
        });
    }
    // 在线但还没写入数据库的设备（理论上不会发生，兜底）
    for (id, d) in online {
        if !seen.contains(&id) {
            out.push(DeviceView {
                device_id: id,
                model: d.model,
                remark: None,
                online: true,
                width: d.width,
                height: d.height,
                last_seen: d.last_seen,
            });
        }
    }
    out.sort_by(|a, b| b.online.cmp(&a.online).then(b.last_seen.cmp(&a.last_seen)));
    Ok(out)
}

#[tauri::command]
pub fn mobile_set_device_remark(
    state: State<'_, AppState>,
    device_id: String,
    remark: String,
) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    crate::db::mobile_set_remark(&db, &device_id, &remark).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mobile_delete_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    {
        let map = state.mobile.devices.read().await;
        if map.contains_key(&device_id) {
            return Err("设备在线，无法删除记录".into());
        }
    }
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    crate::db::mobile_delete_device(&db, &device_id).map_err(|e| e.to_string())
}

async fn send_to_device(
    state: &State<'_, AppState>,
    device_id: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let log_msg = format!("[mobile] Sending to {}: {}\n", device_id, payload);
    println!("{}", log_msg);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_data_dir().join("mobile_debug.log"))
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(log_msg.as_bytes())
        });

    let map = state.mobile.devices.read().await;
    let conn = map.get(device_id).ok_or_else(|| {
        let err = format!("设备 {} 不在线", device_id);
        println!("[mobile] Error: {}", err);
        err
    })?;
    conn.tx
        .send(Message::Text(payload.to_string()))
        .map_err(|_| {
            let err = "发送失败：连接已断开".to_string();
            println!("[mobile] Error: {}", err);
            err
        })
}

#[tauri::command]
pub async fn mobile_tap(
    state: State<'_, AppState>,
    device_id: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    send_to_device(&state, &device_id, serde_json::json!({ "type": "click", "x": x, "y": y })).await
}

#[tauri::command]
pub async fn mobile_swipe(
    state: State<'_, AppState>,
    device_id: String,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    duration: Option<u64>,
) -> Result<(), String> {
    send_to_device(
        &state,
        &device_id,
        serde_json::json!({
            "type": "swipe",
            "x1": x1, "y1": y1, "x2": x2, "y2": y2,
            "duration": duration.unwrap_or(300),
        }),
    )
    .await
}

/// 全局按键：back / home / recents / notifications
#[tauri::command]
pub async fn mobile_key(
    state: State<'_, AppState>,
    device_id: String,
    name: String,
) -> Result<(), String> {
    send_to_device(&state, &device_id, serde_json::json!({ "type": "key", "name": name })).await
}

/// 请求一帧截图；结果通过 `mobile-screenshot` 事件异步返回。
#[tauri::command]
pub async fn mobile_request_screenshot(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    send_to_device(&state, &device_id, serde_json::json!({ "type": "screenshot" })).await
}

/// 请求手机同步过去 24 小时的录音。
#[tauri::command]
pub async fn mobile_sync_recordings(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    send_to_device(&state, &device_id, serde_json::json!({ "type": "sync_recordings" })).await
}

/// 通过本机 ADB 直接拉取录音文件（作为底层兜底通道）。
#[tauri::command]
pub async fn mobile_adb_sync_recordings(device_id: String) -> Result<String, String> {
    let target_dir = get_data_dir().join("mobile").join("recordings").join(&device_id);
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let expected_android_id = device_id.strip_prefix("phone_").unwrap_or(&device_id);

    // 1. 获取所有通过 ADB 连接的设备 Serial
    let adb_devices_out = crate::utils::std_command("adb")
        .arg("devices")
        .output()
        .map_err(|e| format!("adb devices 失败: {}", e))?;
    let devices_str = String::from_utf8_lossy(&adb_devices_out.stdout);
    
    let mut target_serial = None;
    let mut available_serials = Vec::new();

    for line in devices_str.lines().skip(1) { // Skip "List of devices attached"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && (parts[1] == "device" || parts[1] == "emulator") {
            let serial = parts[0].to_string();
            available_serials.push(serial.clone());
            // 2. 查询这个 Serial 对应的 android_id
            let id_out = crate::utils::std_command("adb")
                .arg("-s")
                .arg(&serial)
                .arg("shell")
                .arg("settings")
                .arg("get")
                .arg("secure")
                .arg("android_id")
                .output();
            if let Ok(out) = id_out {
                let current_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if current_id == expected_android_id {
                    target_serial = Some(serial);
                    break;
                }
            }
        }
    }

    let serial_to_use = match target_serial {
        Some(s) => s,
        None => {
            if available_serials.len() == 1 {
                available_serials.pop().unwrap()
            } else {
                return Err(format!("未找到 ID 为 {} 的设备。已连接设备数量: {}。请确 保拔掉其他手机并已授权调试。", expected_android_id, available_serials.len()));
            }
        }
    };


    let dirs_to_pull = [
        "/sdcard/Sounds/CallRecord",
        "/sdcard/Music/Recordings/Call Recordings",
        "/sdcard/Recordings/Call Recordings",
        "/sdcard/MIUI/sound_recorder/call_rec",
        "/sdcard/Recordings/Call",
        "/sdcard/Recordings/CallRecord",
    ];

    let mut log = String::new();
    let target_path = target_dir.to_string_lossy().to_string();

    for dir in dirs_to_pull {
        let output = crate::utils::std_command("adb")
            .arg("-s")
            .arg(&serial_to_use)
            .arg("pull")
            .arg("-a") // 保留时间戳
            .arg(format!("{}/.", dir))
            .arg(&target_path)
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stdout.is_empty() || (!stderr.is_empty() && !stderr.contains("does not exist")) {
                log.push_str(&format!(" pulled {}: {}\n", dir, stdout));
                if !stderr.is_empty() && !stderr.contains("not found") && !stderr.contains("No such file") {
                    log.push_str(&format!(" err: {}\n", stderr));
                }
            }
        }
    }
    
    if log.is_empty() {
        Ok(format!("ADB 同步完成 (设备: {})，未发现新文件或拉取失败", serial_to_use))
    } else {
        Ok(format!("ADB 同步完成 (设备: {})：\n{}", serial_to_use, log))
    }
}

// ============ 通话录音记录 ============

#[derive(Serialize)]
pub struct RecordingItem {
    pub device_id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    /// 文件修改时间（Unix 秒）
    pub modified: i64,
}

/// 列出已回传的通话录音。`device_id` 为空时返回所有设备的录音，按时间倒序。
#[tauri::command]
pub fn mobile_list_recordings(device_id: Option<String>) -> Result<Vec<RecordingItem>, String> {
    let base = get_data_dir().join("mobile").join("recordings");
    let mut out: Vec<RecordingItem> = Vec::new();
    if !base.exists() {
        return Ok(out);
    }

    let dirs: Vec<std::path::PathBuf> = match &device_id {
        Some(id) if !id.is_empty() => vec![base.join(id)],
        _ => std::fs::read_dir(&base)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect(),
    };

    for dir in dirs {
        let dev = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            out.push(RecordingItem {
                device_id: dev.clone(),
                name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                path: path.to_string_lossy().to_string(),
                size: meta.len(),
                modified,
            });
        }
    }

    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(out)
}

/// 删除一条录音文件。`path` 必须位于录音目录内（防越权删除）。
#[tauri::command]
pub fn mobile_delete_recording(path: String) -> Result<(), String> {
    let base = get_data_dir().join("mobile").join("recordings");
    let target = std::path::Path::new(&path);
    let canon_base = base.canonicalize().map_err(|e| e.to_string())?;
    let canon_target = target.canonicalize().map_err(|e| e.to_string())?;
    if !canon_target.starts_with(&canon_base) {
        return Err("非法路径".into());
    }
    std::fs::remove_file(&canon_target).map_err(|e| e.to_string())
}
