use std::fs;
use tauri::State;
use uuid::Uuid;
use crate::models::{ChatMessage, ChatSession};
use crate::state::AppState;
use crate::utils::{get_data_dir};
use crate::commands::common::get_config;
use crate::commands::knowledge_base::search_kb_internal;

fn get_chats_dir() -> std::path::PathBuf {
    get_data_dir().join("chats")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tauri::command]
pub async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
    let dir = get_chats_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = vec![];
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            let content = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                sessions.push(session);
            }
        }
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

#[tauri::command]
pub async fn create_chat_session(title: String) -> Result<ChatSession, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_secs();
    let session = ChatSession {
        id: id.clone(),
        title,
        messages: vec![],
        created_at: now,
        updated_at: now,
    };

    let dir = get_chats_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", id));
    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(session)
}

#[tauri::command]
pub async fn delete_chat_session(session_id: String) -> Result<(), String> {
    let path = get_chats_dir().join(format!("{}.json", session_id));
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_chat_messages(session_id: String) -> Result<Vec<ChatMessage>, String> {
    let path = get_chats_dir().join(format!("{}.json", session_id));
    if !path.exists() {
        return Ok(vec![]);
    }
    let s = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let session: ChatSession = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    Ok(session.messages)
}

#[tauri::command]
pub async fn send_chat_message(
    session_id: String,
    content: String,
    _state: State<'_, AppState>,
    _app: tauri::AppHandle,
) -> Result<ChatMessage, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    let dir = get_chats_dir();
    let path = dir.join(format!("{}.json", session_id));
    let mut session: ChatSession = if path.exists() {
        let s = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| e.to_string())?
    } else {
        return Err("会话不存在".to_string());
    };

    let user_msg = ChatMessage {
        role: "user".to_string(),
        content: content.clone(),
        timestamp: now_secs(),
        tool_used: None,
        tool_data: None,
    };
    session.messages.push(user_msg.clone());

    // 用当前这条用户消息检索企业知识库，命中内容注入 system 提示
    let kb_context = match search_kb_internal(content.clone()).await {
        Ok(res_str) => {
            let v: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut buf = String::new();
            if let Some(arr) = v.as_array() {
                for item in arr.iter().take(6) {
                    if let Some(t) = item["text"].as_str() {
                        if !t.trim().is_empty() {
                            buf.push_str(&format!("- {}\n", t.trim()));
                        }
                    }
                }
            }
            buf
        }
        Err(_) => String::new(),
    };

    let system_content = if kb_context.is_empty() {
        "你是 AutoCast AI 助手，一个专业、友好的中文 AI 创作与运营助理。请用简洁清晰的中文回答用户的问题。".to_string()
    } else {
        format!(
            "你是 AutoCast AI 助手，一个专业、友好的中文 AI 创作与运营助理。请用简洁清晰的中文回答用户的问题。\n\n\
            以下是企业知识库中与用户问题相关的资料，回答时请优先依据这些事实，不要编造：\n{}",
            kb_context
        )
    };

    // 组装发给 LLM 的消息：system 提示（含知识库） + 历史对话（只取 user/assistant）
    let mut api_messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": system_content
    })];
    for m in &session.messages {
        if m.role == "user" || m.role == "assistant" {
            api_messages.push(serde_json::json!({ "role": m.role, "content": m.content }));
        }
    }

    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": api_messages,
        "temperature": 0.7
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await
        .map_err(|e| format!("AI 对话请求失败: {}", e))?;

    let status = response.status();
    let body_text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("LLM API 错误 {}: {}", status, body_text.chars().take(300).collect::<String>()));
    }

    let res_data: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("LLM 响应解析失败（{}）：{}", e, body_text.chars().take(300).collect::<String>()))?;

    let reply = res_data["choices"][0]["message"]["content"].as_str()
        .or_else(|| res_data["choices"][0]["text"].as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());

    let reply = match reply {
        Some(s) => s,
        None => {
            if let Some(err) = res_data.get("error") {
                return Err(format!("LLM 返回错误：{}", err));
            }
            return Err(format!("LLM 返回空内容。原始响应：{}", body_text.chars().take(400).collect::<String>()));
        }
    };

    let assistant_msg = ChatMessage {
        role: "assistant".to_string(),
        content: reply,
        timestamp: now_secs(),
        tool_used: None,
        tool_data: None,
    };
    session.messages.push(assistant_msg.clone());
    session.updated_at = now_secs();

    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(assistant_msg)
}
