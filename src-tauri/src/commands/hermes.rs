use std::fs;
use std::path::PathBuf;
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::commands::knowledge_base::search_kb_internal;

pub fn which_hermes() -> String {
    let exe_name = if cfg!(windows) { "hermes.exe" } else { "hermes" };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local").join("bin").join(exe_name));
        if cfg!(windows) {
            candidates.push(home.join("AppData").join("Local").join("Programs").join("hermes").join(exe_name));
            candidates.push(home.join("AppData").join("Local").join("hermes").join("bin").join(exe_name));
            candidates.push(home.join("AppData").join("Roaming").join("Python").join("Scripts").join(exe_name));
            candidates.push(home.join("scoop").join("shims").join(exe_name));
        }
    }
    if cfg!(windows) {
        candidates.push(PathBuf::from(r"C:\Program Files\hermes").join(exe_name));
        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin").join(exe_name));
    } else {
        candidates.push(PathBuf::from("/usr/local/bin/hermes"));
        candidates.push(PathBuf::from("/opt/homebrew/bin/hermes"));
        candidates.push(PathBuf::from("/usr/bin/hermes"));
    }
    for c in candidates { if c.exists() { return c.to_string_lossy().to_string(); } }
    if cfg!(windows) { "hermes.exe".to_string() } else { "hermes".to_string() }
}

#[tauri::command]
pub async fn hermes_enable_api_server() -> Result<String, String> {
    let env_path = dirs::home_dir().ok_or("无法获取 home 目录")?.join(".hermes").join(".env");
    let content = if env_path.exists() { fs::read_to_string(&env_path).map_err(|e| e.to_string())? } else { String::new() };
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut api_key_value = String::new();
    if !lines.iter().any(|l| l.trim() == "API_SERVER_ENABLED=true") { lines.push("API_SERVER_ENABLED=true".to_string()); }
    for line in &lines { if let Some(val) = line.strip_prefix("API_SERVER_KEY=") { api_key_value = val.trim().to_string(); } }
    if api_key_value.is_empty() {
        api_key_value = uuid::Uuid::new_v4().to_string().replace("-", "");
        lines.push(format!("API_SERVER_KEY={}", api_key_value));
    }
    let new_content = lines.join("\n") + "\n";
    fs::write(&env_path, new_content).map_err(|e| e.to_string())?;
    Ok(api_key_value)
}

#[tauri::command]
pub async fn hermes_restart_service() -> Result<(), String> {
    let hermes_bin = which_hermes();
    tokio::process::Command::new(&hermes_bin).args(["gateway", "restart"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("重启网关失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn start_hermes_gateway(_app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let key = "hermes_gateway".to_string();
    let old_child = { let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?; handles.remove(&key) };
    if let Some(mut child) = old_child { let _ = child.start_kill(); let _ = child.wait().await; tokio::time::sleep(std::time::Duration::from_millis(500)).await; }
    let hermes_bin = which_hermes();
    let spawned = tokio::process::Command::new(&hermes_bin).args(["gateway", "restart"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    match spawned {
        Ok(_) => Ok(()),
        Err(_) => {
            let child = tokio::process::Command::new(&hermes_bin).args(["gateway", "run", "--replace"]).env("API_SERVER_ENABLED", "true").stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit()).kill_on_drop(true).spawn().map_err(|e| format!("启动 Hermes 失败 ({}): {}", hermes_bin, e))?;
            let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
            handles.insert(key, child);
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn stop_hermes_gateway(state: State<'_, AppState>) -> Result<(), String> {
    { let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?; if let Some(mut child) = handles.remove("hermes_gateway") { let _ = child.start_kill(); return Ok(()); } }
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin).args(["gateway", "stop"]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn check_hermes_status(state: State<'_, AppState>) -> Result<bool, String> {
    { let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?; if let Some(child) = handles.get_mut("hermes_gateway") { if let Ok(Some(_)) = child.try_wait() { handles.remove("hermes_gateway"); } } }
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(3)).build().map_err(|e| e.to_string())?;
    match client.get("http://127.0.0.1:8642/health").send().await { Ok(r) => Ok(r.status().is_success()), Err(_) => Ok(false) }
}

#[tauri::command]
pub async fn check_hermes_gateway_health(gateway_url: String, api_key: String) -> Result<serde_json::Value, String> {
    let url = format!("{}/health", gateway_url.trim_end_matches('/'));
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().map_err(|e| e.to_string())?;
    let mut req = client.get(&url);
    if !api_key.is_empty() { req = req.bearer_auth(&api_key); }
    let response = req.send().await.map_err(|e| format!("连接失败: {}", e))?;
    if response.status().is_success() { Ok(response.json::<serde_json::Value>().await.unwrap_or(serde_json::json!({"status": "ok"}))) } else { Err(format!("HTTP {}", response.status().as_u16())) }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HermesSession {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub last_active: String,
}

#[tauri::command]
pub async fn list_hermes_sessions() -> Result<Vec<HermesSession>, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin).args(["sessions", "list", "--limit", "30"]).output().await.map_err(|e| e.to_string())?;
    if !output.status.success() { return Ok(vec![]); }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sessions = Vec::new();
    for line in stdout.lines().skip(2) {
        let trimmed = line.trim(); if trimmed.is_empty() || trimmed.starts_with('─') { continue; }
        let parts: Vec<&str> = line.splitn(4, "  ").map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            let title = parts.first().unwrap_or(&"").to_string(); let preview = parts.get(1).unwrap_or(&"").to_string(); let last_active = parts.get(2).unwrap_or(&"").to_string(); let id = parts.get(3).unwrap_or(&"").to_string();
            if !id.is_empty() || !last_active.is_empty() {
                let (real_last_active, real_id) = if parts.len() == 3 { (last_active.clone(), preview.clone()) } else { (last_active, id) };
                if !real_id.is_empty() { sessions.push(HermesSession { id: real_id, title: if title == "—" || title.is_empty() { "未命名会话".to_string() } else { title }, preview: preview.chars().take(80).collect(), last_active: real_last_active }); }
            }
        }
    }
    Ok(sessions)
}

#[tauri::command]
pub async fn hermes_read_api_key() -> Result<String, String> {
    let env_path = dirs::home_dir().ok_or("无法获取 home 目录")?.join(".hermes").join(".env");
    if !env_path.exists() { return Ok(String::new()); }
    let content = fs::read_to_string(&env_path).map_err(|e| e.to_string())?;
    for line in content.lines() { if let Some(val) = line.strip_prefix("API_SERVER_KEY=") { return Ok(val.trim().to_string()); } }
    Ok(String::new())
}

#[tauri::command]
pub async fn hermes_set_api_key(key: String) -> Result<(), String> {
    let hermes_dir = dirs::home_dir().ok_or("无法获取 home 目录")?.join(".hermes");
    fs::create_dir_all(&hermes_dir).map_err(|e| e.to_string())?;
    let env_path = hermes_dir.join(".env");
    let content = if env_path.exists() { fs::read_to_string(&env_path).map_err(|e| e.to_string())? } else { String::new() };
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let existing_pos = lines.iter().position(|l| l.starts_with("API_SERVER_KEY="));
    if key.is_empty() { if let Some(pos) = existing_pos { lines.remove(pos); } } else {
        let new_line = format!("API_SERVER_KEY={}", key);
        if let Some(pos) = existing_pos { lines[pos] = new_line; } else { lines.push(new_line); }
    }
    let new_content = if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" };
    fs::write(&env_path, new_content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn hermes_list_skills() -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin).arg("skills").arg("list").output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut skills = vec![];
    for line in stdout.lines() { if line.starts_with('│') && !line.contains(" Name ") {
        let parts: Vec<&str> = line.split('│').collect();
        if parts.len() >= 6 { skills.push(serde_json::json!({ "name": parts[1].trim(), "category": parts[2].trim(), "source": parts[3].trim(), "trust": parts[4].trim(), "status": parts[5].trim() })); }
    } }
    Ok(skills)
}

#[tauri::command]
pub async fn hermes_install_skill(name: String) -> Result<String, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin).arg("skills").arg("install").arg(&name).arg("--yes").output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn hermes_uninstall_skill(name: String) -> Result<String, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin).arg("skills").arg("uninstall").arg(&name).output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn hermes_list_tools() -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin).arg("tools").arg("--summary").arg("list").output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tools = vec![];
    for line in stdout.lines() {
        let line = line.trim(); if line.is_empty() || line.ends_with(':') { continue; }
        let enabled = line.contains("✓ enabled");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 { tools.push(serde_json::json!({ "name": parts[2], "enabled": enabled, "description": parts[3..].join(" "), "keyword": format!("!{}", parts[2]) })); }
    }
    Ok(tools)
}

#[tauri::command]
pub async fn hermes_get_session_messages(session_id: String) -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin).arg("sessions").arg("export").arg("-").arg("--session-id").arg(&session_id).output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).to_string()); }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().next() { if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) { if let Some(messages) = val.get("messages").and_then(|m| m.as_array()) { return Ok(messages.clone()); } } }
    Ok(vec![])
}

#[tauri::command]
pub async fn hermes_toggle_skill_status(name: String, enable: bool) -> Result<(), String> {
    let hermes_bin = which_hermes();
    let action = if enable { "enable" } else { "disable" };
    let output = tokio::process::Command::new(&hermes_bin).arg("skills").arg(action).arg(&name).output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { let stderr = String::from_utf8_lossy(&output.stderr).to_string(); let stdout = String::from_utf8_lossy(&output.stdout).to_string(); Err(if !stderr.is_empty() { stderr } else { stdout }) }
}

#[tauri::command]
pub async fn hermes_toggle_tool_status(name: String, enable: bool) -> Result<(), String> {
    let hermes_bin = which_hermes();
    let action = if enable { "enable" } else { "disable" };
    let output = tokio::process::Command::new(&hermes_bin).arg("tools").arg(action).arg(&name).output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn hermes_search_kb(query: String) -> Result<serde_json::Value, String> {
    let res = search_kb_internal(query).await?;
    let v: serde_json::Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
    Ok(v)
}

#[tauri::command]
pub async fn hermes_list_runs(gateway_url: String, api_key: String, run_id: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let Some(rid) = run_id else { return Ok(vec![]); };
    let url = format!("{}/v1/runs/{}", gateway_url.trim_end_matches('/'), rid);
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().map_err(|e| e.to_string())?;
    let mut req = client.get(&url);
    if !api_key.is_empty() { req = req.bearer_auth(&api_key); }
    match req.send().await { Ok(resp) if resp.status().is_success() => { match resp.json::<serde_json::Value>().await { Ok(body) => Ok(vec![body]), Err(_) => Ok(vec![]) } } _ => Ok(vec![]) }
}

#[tauri::command]
pub async fn hermes_stop_run(gateway_url: String, api_key: String, run_id: String) -> Result<(), String> {
    let url = format!("{}/v1/runs/{}/stop", gateway_url.trim_end_matches('/'), run_id);
    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&serde_json::json!({}));
    if !api_key.is_empty() { req = req.bearer_auth(&api_key); }
    req.send().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn hermes_approve_run(gateway_url: String, api_key: String, run_id: String, approved: bool) -> Result<(), String> {
    let url = format!("{}/v1/runs/{}/approval", gateway_url.trim_end_matches('/'), run_id);
    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&serde_json::json!({"approved": approved}));
    if !api_key.is_empty() { req = req.bearer_auth(&api_key); }
    req.send().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn hermes_send_message(app: tauri::AppHandle, gateway_url: String, api_key: String, messages: Vec<serde_json::Value>, session_id: Option<String>) -> Result<(), String> {
    use futures_util::StreamExt;
    use tauri::Emitter;
    let url = format!("{}/v1/chat/completions", gateway_url.trim_end_matches('/'));
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(300)).build().map_err(|e| e.to_string())?;
    let mut req = client.post(&url).header("Content-Type", "application/json").header("Accept", "text/event-stream");
    if !api_key.is_empty() { req = req.bearer_auth(&api_key); if let Some(ref sid) = session_id { if !sid.is_empty() { req = req.header("X-Hermes-Session-Id", sid.as_str()); } } }
    let body = serde_json::json!({ "messages": messages, "stream": true, "model": "hermes-agent" });
    let response = req.json(&body).send().await.map_err(|e| { let msg = format!("连接失败: {}", e); let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()})); msg })?;
    if !response.status().is_success() { let status = response.status().as_u16(); let text = response.text().await.unwrap_or_default(); let msg = format!("HTTP {}: {}", status, text); let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()})); return Err(msg); }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut current_event = String::new();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| { let msg = e.to_string(); let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()})); msg })?;
        let text = String::from_utf8_lossy(&chunk); buffer.push_str(&text);
        loop { match buffer.find('\n') {
            None => break,
            Some(pos) => {
                let line = buffer[..pos].trim_end_matches('\r').to_string(); buffer = buffer[pos + 1..].to_string();
                if line.starts_with("event: ") { current_event = line[7..].to_string(); continue; }
                if !line.starts_with("data: ") { if line.is_empty() { current_event = String::new(); } continue; }
                let data = &line[6..]; if data == "[DONE]" { let _ = app.emit("hermes-done", serde_json::json!({})); return Ok(()); }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(err) = val.get("error") { let msg = err["message"].as_str().or_else(|| err.as_str()).unwrap_or("未知流错误"); let _ = app.emit("hermes-error", serde_json::json!({"message": msg})); return Ok(()); }
                    if !current_event.is_empty() { if current_event == "hermes.tool.progress" { let _ = app.emit("hermes-tool-progress", val.clone()); } else { let _ = app.emit("hermes-event", serde_json::json!({ "event": current_event, "data": val })); } }
                    if let Some(run_id) = val.get("id").and_then(|v| v.as_str()) { let _ = app.emit("hermes-run-id", serde_json::json!({"run_id": run_id})); }
                    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) { if let Some(choice) = choices.get(0) {
                        if let Some(delta) = choice.get("delta") {
                            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) { if !content.is_empty() { let _ = app.emit("hermes-chunk", serde_json::json!({"content": content})); } }
                            if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) { if !reasoning.is_empty() { let _ = app.emit("hermes-thinking", serde_json::json!({"content": reasoning})); } }
                            if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) { if !tool_calls.is_empty() { let _ = app.emit("hermes-tool-calls", serde_json::json!({"tool_calls": tool_calls})); } }
                        }
                        let finish = choice.get("finish_reason").and_then(|v| v.as_str()).unwrap_or(""); if !finish.is_empty() && finish != "null" { let _ = app.emit("hermes-done", serde_json::json!({"finish_reason": finish})); return Ok(()); }
                    } }
                }
            }
        } }
    }
    let _ = app.emit("hermes-done", serde_json::json!({}));
    Ok(())
}
