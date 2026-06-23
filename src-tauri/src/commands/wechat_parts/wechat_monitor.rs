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

/// 全局自动监控：不再按指定会话，而是轮询 `get_sessions`（最近活跃会话），
/// 监控启动后**任意**会话出现新消息（lastTimestamp 前移）时，拉取该会话最近 100 条历史，
/// 并 emit `wechat-monitor-hit`（携带 category=friend|group，由前端按勾选类型过滤显示）。
///
/// 与旧的 `wechat_start_monitor`（按 targets）互斥：都通过 `monitor_running` 控制，
/// 启动新监控会自动停掉旧监控；`wechat_stop_monitor` 同样可停止本监控。
#[tauri::command]
pub async fn wechat_start_monitor_auto(
    interval_secs: Option<u64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let interval = interval_secs.unwrap_or(5).max(2);

    let running = Arc::new(AtomicBool::new(true));
    {
        let mut guard = state.wechat.lock().await;
        let sc = guard.as_mut().ok_or("尚未连接微信数据库")?;
        if !sc.connected {
            return Err("尚未连接微信数据库".to_string());
        }
        if let Some(old) = sc.monitor_running.take() {
            old.store(false, Ordering::SeqCst);
        }
        sc.monitor_running = Some(running.clone());
    }

    tokio::spawn(async move {
        let app_state = app.state::<AppState>();
        // 基线：首轮只记录各会话当前 lastTimestamp，不触发；只监控启动后新增的消息。
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let mut baselined = false;

        loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if !running.load(Ordering::SeqCst) {
                break;
            }

            // 一次调用拉取所有最近会话，规避逐会话轮询的开销。
            let sessions = {
                let mut guard = app_state.wechat.lock().await;
                let sc = match guard.as_mut() {
                    Some(sc) if sc.connected => sc,
                    _ => continue,
                };
                match sc
                    .call("get_sessions", json!({}), Duration::from_secs(30))
                    .await
                {
                    Ok(v) => v
                        .get("sessions")
                        .and_then(|s| s.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    Err(e) => {
                        eprintln!("[wechat-monitor] get_sessions 失败: {e}");
                        continue;
                    }
                }
            };

            if !baselined {
                for s in &sessions {
                    if let Some(u) = s.get("username").and_then(|v| v.as_str()) {
                        let ts = s.get("lastTimestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                        last_seen.insert(u.to_string(), ts);
                    }
                }
                baselined = true;
                continue;
            }

            for s in &sessions {
                let username = match s.get("username").and_then(|v| v.as_str()) {
                    Some(u) => u.to_string(),
                    None => continue,
                };
                let ts = s.get("lastTimestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                let is_new = match last_seen.get(&username) {
                    Some(prev) => ts > *prev,
                    None => true,
                };
                if !is_new {
                    continue;
                }
                last_seen.insert(username.clone(), ts);

                let display_name = s
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_group = s
                    .get("isGroup")
                    .and_then(|v| v.as_bool())
                    .unwrap_or_else(|| username.ends_with("@chatroom"));
                let summary = s
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // 监控到聊天：拉取该会话最近 100 条历史对话。
                let messages = {
                    let mut guard = app_state.wechat.lock().await;
                    let sc = match guard.as_mut() {
                        Some(sc) if sc.connected => sc,
                        _ => continue,
                    };
                    match sc
                        .call(
                            "get_messages",
                            json!({ "sessionId": username, "limit": 100, "offset": 0 }),
                            Duration::from_secs(30),
                        )
                        .await
                    {
                        Ok(v) => v
                            .get("messages")
                            .and_then(|m| m.as_array())
                            .cloned()
                            .unwrap_or_default(),
                        Err(e) => {
                            eprintln!("[wechat-monitor] get_messages 失败 ({username}): {e}");
                            Vec::new()
                        }
                    }
                };

                let _ = app.emit(
                    "wechat-monitor-hit",
                    json!({
                        "sessionId": username,
                        "displayName": display_name,
                        "isGroup": is_group,
                        "category": if is_group { "group" } else { "friend" },
                        "summary": summary,
                        "lastTimestamp": ts,
                        "messages": messages,
                    }),
                );
            }
        }
        eprintln!("[wechat-monitor] 自动监控轮询已退出");
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
