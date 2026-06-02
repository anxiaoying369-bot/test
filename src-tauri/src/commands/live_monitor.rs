use std::fs;
use tauri::{State, Emitter, Manager};
use crate::state::AppState;
use crate::utils::{get_data_dir, get_scripts_dir, python_cmd};
use crate::commands::common::get_config;
use crate::commands::knowledge_base::search_kb_internal;

#[tauri::command]
pub async fn resolve_live_url(url: String) -> Result<String, String> {
    let url = url.trim();
    if !url.is_empty() && url.chars().all(|c| c.is_ascii_digit()) {
        return Ok(url.to_string());
    }
    if url.contains("live.douyin.com/") {
        let parts: Vec<&str> = url.split("live.douyin.com/").collect();
        if parts.len() > 1 {
            let id_part = parts[1].split('?').next().unwrap_or("").split('/').next().unwrap_or("");
            if !id_part.is_empty() {
                return Ok(id_part.to_string());
            }
        }
    }
    if url.starts_with("http") {
        return Err("目前仅支持直播间 ID 或以 live.douyin.com/ 开头的直播间链接".to_string());
    }
    Err("请输入有效的直播间 ID 或直播间链接".to_string())
}

#[tauri::command]
pub async fn start_live_monitor(
    room_id: String,
    account_name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    
    // 清理已退出的进程句柄，并统计真正运行中的直播监控数量
    let mut keys_to_remove = Vec::new();
    let mut active_count = 0;
    for (k, child) in handles.iter_mut() {
        if k.starts_with("live_") {
            if let Ok(Some(_status)) = child.try_wait() {
                keys_to_remove.push(k.clone());
            } else {
                active_count += 1;
            }
        }
    }
    for k in keys_to_remove {
        handles.remove(&k);
    }

    let key = format!("live_{}", room_id);
    if handles.contains_key(&key) {
        return Err("该直播间已在监控中".to_string());
    }

    if active_count >= 10 {
        return Err("最多只能同时开启 10 路实时监控，请先停止其他直播间".to_string());
    }

    let script_path = get_scripts_dir().join("douyin_live_monitor.py");
    
    let mut child = python_cmd()
        .arg(&script_path)
        .arg("--room-id").arg(&room_id)
        .arg("--account-name").arg(&account_name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn().map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().ok_or("无法打开 Python stdout")?;
    let room_id_clone = room_id.clone();
    let app_handle = app.clone();
    
    tauri::async_runtime::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let _ = app_handle.emit("live-event", val);
            }
        }

        {
            if let Ok(mut h) = app_handle.state::<AppState>().process_handles.lock() {
                h.remove(&format!("live_{}", room_id_clone));
            }
        }

        let _ = app_handle.emit("live-event", serde_json::json!({
            "type": "status",
            "status": "stopped",
            "live_id": room_id_clone
        }));
    });

    let pid = child.id();
    crate::utils::register_task(&state, key.clone(), format!("监控: {}", room_id), "live_monitor".to_string(), pid);
    handles.insert(key, child);
    Ok(())
}

#[tauri::command]
pub async fn stop_live_monitor(room_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let key = format!("live_{}", room_id);
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(child) = handles.remove(&key) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
            }
        }
        #[cfg(not(unix))]
        {
            let mut child = child;
            let _ = child.start_kill();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_active_monitors(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    let ids: Vec<String> = handles.keys()
        .filter(|k| k.starts_with("live_"))
        .map(|k| k.replace("live_", ""))
        .collect();
    Ok(ids)
}

#[tauri::command]
pub async fn get_live_history(room_id: String) -> Result<Vec<serde_json::Value>, String> {
    let data_dir = get_data_dir().join("live_data").join(&room_id);
    let history_path = data_dir.join("history.jsonl");
    
    if !history_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    let mut history = vec![];
    
    let lines: Vec<&str> = content.lines().collect();
    for line in lines.iter().rev().take(200).rev() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            history.push(val);
        }
    }
    Ok(history)
}

#[tauri::command]
pub async fn generate_live_reply(user_name: String, content: String) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    let system_prompt = if config.llm.live_reply_prompt.is_empty() {
        "你是一位正在直播的主播。请根据直播主题和直播内容，简短地回复用户的弹幕。回复必须非常简短（20字以内），语气亲切自然，像真人在直播间说话一样。".to_string()
    } else {
        config.llm.live_reply_prompt.clone()
    };

    let kb_context = match search_kb_internal(content.clone()).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut context_text = String::from("\n相关背景知识：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(3) {
                    if let Some(text) = item["text"].as_str() {
                        context_text.push_str(&format!("- {}\n", text));
                    }
                }
            }
            if context_text.len() < 20 { String::new() } else { context_text }
        },
        Err(_) => String::new(),
    };

    let user_context = format!(
        "直播主题：{}\n直播内容：{}\n{}\n\n请回复用户 {} 的弹幕：{}",
        config.llm.live_theme,
        config.llm.live_content,
        kb_context,
        user_name,
        content
    );

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_context }
        ],
        "temperature": 0.8,
        "max_tokens": 50
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("LLM API 错误: {}", response.status()));
    }

    let resp_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let reply = resp_data["choices"][0]["message"]["content"]
        .as_str().ok_or("LLM 返回格式错误")?.trim().to_string();

    Ok(reply)
}
