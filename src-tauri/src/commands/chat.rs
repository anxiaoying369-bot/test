use std::fs;
use tauri::State;
use uuid::Uuid;
use crate::models::{ChatMessage, ChatSession};
use crate::state::{AppState, pending_confirmations, PendingToolCall, PENDING_TOOL_TTL_SECS};
use crate::utils::{get_data_dir};
use crate::commands::common::get_config;
use crate::commands::tools::{dispatch_tool, is_action_tool, summarize_action_result, tool_definitions_all, MAX_TOOL_RESULT_CHARS};

fn get_chats_dir() -> std::path::PathBuf {
    get_data_dir().join("chats")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 把 Phase 3 动作工具翻译成人类可读的描述，用于确认弹窗。
fn describe_action_tool(name: &str, args: &serde_json::Value) -> String {
    match name {
        "start_scrape" => {
            let account = args.get("account_name").and_then(|v| v.as_str()).unwrap_or("?");
            let typ = args.get("scrape_type").and_then(|v| v.as_str()).unwrap_or("?");
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);
            format!("启动对博主「{}」的 {} 采集（最多 {} 条）", account, typ, limit)
        }
        "add_document_to_kb" => {
            let fp = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("添加文档到企业知识库：{}", fp)
        }
        "delete_kb_file" => {
            let fn_ = args.get("filename").and_then(|v| v.as_str()).unwrap_or("?");
            format!("删除企业知识库中的文档：{}", fn_)
        }
        "synthesize_speech" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let preview = if text.chars().count() > 30 {
                format!("{}…", text.chars().take(30).collect::<String>())
            } else {
                text.to_string()
            };
            format!("合成语音：「{}」", preview)
        }
        _ => format!("执行工具 {}", name),
    }
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

    // system 提示：告知助理可调用工具获取真实业务数据，需求时必须调用、不得编造。
    let system_content = "你是 AutoCast AI 助手，一个专业、友好的中文 AI 创作与运营助理。\n\
        你可以调用工具获取真实业务数据。当用户的问题涉及：已采集的博主/作品/评论数据、\
        企业知识库内容、平台账号信息时，必须先调用相应工具获取真实数据，再依据结果回答，严禁编造。\n\
        若工具返回 error 或空数据，请如实告知，并给出下一步建议（例如先去采集数据或上传知识库文档）。\n\
        请用简洁清晰的中文回答。";

    // 组装发给 LLM 的消息：system + 历史对话（只取 user/assistant 文本）
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    // ===== Function Calling 循环 =====
    // 每轮：带 tools 请求 LLM → 若返回 tool_calls 则执行工具并回填结果，继续下一轮；
    // 否则取最终文本回复结束。至多 MAX_TOOL_ROUNDS 轮，防止无限调用。
    //
    // Phase 3 特殊处理：遇到动作类工具（start_scrape/add_document_to_kb/delete_kb_file/
    // synthesize_speech）时，不立即执行，而是把 tool_call 暂存到 PENDING_TOOL_CONFIRMATIONS，
    // 返回一条"待确认"的 assistant 消息给前端，由用户在 UI 弹窗中确认/取消。
    // 用户确认后调用 confirm_tool_execution 真正执行；取消则回填"已取消"给 LLM。
    const MAX_TOOL_ROUNDS: usize = 5;
    let tools = tool_definitions_all();
    // 记录本次对话用到的工具调用轨迹，写入 assistant 消息的 tool_data 供前端展示。
    let mut tool_trace: Vec<serde_json::Value> = Vec::new();

    let reply: String;
    let mut round = 0usize;
    let mut forced_finalize = false;
    loop {
        let allow_tools = round < MAX_TOOL_ROUNDS;
        // 达到轮数上限：追加一次收尾提示，要求模型停止调用工具、直接用文字作答，
        // 避免它继续只返回 tool_calls / null content 而触发"空内容"错误
        // （常见于知识库无相关资料、模型反复检索直到轮数耗尽）。
        if !allow_tools && !forced_finalize {
            api_messages.push(serde_json::json!({
                "role": "system",
                "content": "工具调用次数已达上限。请不要再调用任何工具，直接基于以上已获取的信息用简洁中文给出最终回答；若信息不足，请如实说明并给出下一步建议。"
            }));
            forced_finalize = true;
        }
        let mut payload = serde_json::json!({
            "model": config.llm.model,
            "messages": api_messages,
            "temperature": 0.7
        });
        if allow_tools {
            payload["tools"] = tools.clone();
            payload["tool_choice"] = serde_json::json!("auto");
        }

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

        if let Some(err) = res_data.get("error") {
            return Err(format!("LLM 返回错误：{}", err));
        }

        let message = &res_data["choices"][0]["message"];
        let tool_calls = message.get("tool_calls").and_then(|v| v.as_array()).cloned();

        // 有工具调用 → 执行并回填，进入下一轮
        if allow_tools {
            if let Some(calls) = tool_calls.filter(|c| !c.is_empty()) {
                // 0) 检查本轮是否含 Phase 3 动作工具 → 触发 human-in-the-loop。
                // 所有动作工具都需用户确认；下面会统一收集并暂存。
                let has_action = calls.iter().any(|call| {
                    is_action_tool(call["function"]["name"].as_str().unwrap_or(""))
                });

                if has_action {
                    // 找出本轮所有动作工具：第一个为"主暂停项"，其余一起存为 pending
                    // 改进：所有动作工具都暂存，全部需要用户确认；只有只读工具照常执行
                    let mut action_indices: Vec<usize> = Vec::new();
                    let mut all_actions: Vec<(usize, String, String, serde_json::Value)> = Vec::new();
                    for (i, call) in calls.iter().enumerate() {
                        let fname2 = call["function"]["name"].as_str().unwrap_or("").to_string();
                        if is_action_tool(&fname2) {
                            let cid2 = call["id"].as_str().unwrap_or("").to_string();
                            let raw2 = call["function"]["arguments"].as_str().unwrap_or("{}");
                            let args2: serde_json::Value = serde_json::from_str(raw2)
                                .unwrap_or(serde_json::json!({}));
                            action_indices.push(i);
                            all_actions.push((i, cid2, fname2, args2));
                        }
                    }

                    // 全部暂存到 PENDING_TOOL_CONFIRMATIONS
                    let now = now_secs();
                    let mut confirmation_ids: Vec<(usize, String, String, String)> = Vec::new();
                    for (i, cid, fname_a, args_a) in &all_actions {
                        let human_desc = describe_action_tool(fname_a, args_a);
                        let cid_uuid = Uuid::new_v4().to_string();
                        let cancel_msg = format!("用户取消了 AI 想执行的动作: {}", human_desc);
                        {
                            let mut map = pending_confirmations().lock().unwrap();
                            map.insert(cid_uuid.clone(), PendingToolCall {
                                tool_call_id: cid.clone(),
                                tool_name: fname_a.clone(),
                                args: args_a.clone(),
                                session_id: session_id.clone(),
                                cancel_message: cancel_msg,
                                created_at: now,
                            });
                        }
                        confirmation_ids.push((i.clone(), cid_uuid, fname_a.clone(), human_desc));
                    }

                    // 把整条 assistant 消息（含全部 tool_calls）回填给 LLM，
                    // 对每个 call：动作工具回填"待确认"占位 result；只读工具照常执行。
                    api_messages.push(message.clone());
                    for (i, call) in calls.iter().enumerate() {
                        let cid = call["id"].as_str().unwrap_or("").to_string();
                        let fn_name = call["function"]["name"].as_str().unwrap_or("").to_string();
                        let raw = call["function"]["arguments"].as_str().unwrap_or("{}");
                        let cargs: serde_json::Value = serde_json::from_str(raw)
                            .unwrap_or(serde_json::json!({}));

                        let (result_str, status, my_conf_id) = if action_indices.contains(&i) {
                            // 动作工具：返回"待确认"占位
                            let my_id = confirmation_ids.iter()
                                .find(|(idx, _, _, _)| *idx == i)
                                .map(|(_, id, _, _)| id.clone())
                                .unwrap_or_default();
                            let human_desc = confirmation_ids.iter()
                                .find(|(idx, _, _, _)| *idx == i)
                                .map(|(_, _, _, desc)| desc.clone())
                                .unwrap_or_default();
                            let s = serde_json::json!({
                                "status": "pending_confirmation",
                                "confirmation_id": my_id,
                                "message": format!("AI 想执行『{}』，需用户确认后才执行。", human_desc),
                            }).to_string();
                            (s, "pending_confirmation", my_id)
                        } else {
                            // 只读工具：照常执行
                            let r = dispatch_tool(&fn_name, &cargs).await;
                            let mut s = serde_json::to_string(&r).unwrap_or_else(|_| "null".to_string());
                            if s.chars().count() > MAX_TOOL_RESULT_CHARS {
                                s = s.chars().take(MAX_TOOL_RESULT_CHARS).collect::<String>()
                                    + "\n...(结果过长已截断)";
                            }
                            (s, "executed", String::new())
                        };

                        tool_trace.push(serde_json::json!({
                            "name": fn_name,
                            "args": cargs,
                            "status": status,
                            "confirmation_id": if my_conf_id.is_empty() { serde_json::Value::Null } else { serde_json::json!(my_conf_id) },
                        }));

                        api_messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": cid,
                            "content": result_str
                        }));
                    }

                    // 返回"待确认"消息给前端。组装人类可读摘要。
                    let summary: Vec<String> = confirmation_ids.iter()
                        .map(|(_, _, name, desc)| format!("• `{}` → {}", name, desc))
                        .collect();
                    let summary_text = summary.join("\n");
                    let first_cid = confirmation_ids.first().map(|(_, id, _, _)| id.clone()).unwrap_or_default();
                    let first_name = confirmation_ids.first().map(|(_, _, n, _)| n.clone()).unwrap_or_default();

                    let assistant_msg = ChatMessage {
                        role: "assistant".to_string(),
                        content: format!(
                            "⚠️ 我想执行以下 **{}** 个动作，但都需要你确认：\n\n{}\n\n请在弹窗中选择「允许」或「拒绝」（会一次性处理所有动作）。",
                            confirmation_ids.len(),
                            summary_text
                        ),
                        timestamp: now,
                        tool_used: Some(first_name.clone()),
                        tool_data: Some(serde_json::json!({
                            "calls": tool_trace,
                            "pending_confirmation_ids": confirmation_ids.iter().map(|(_, id, _, _)| id.clone()).collect::<Vec<_>>(),
                            "primary_confirmation_id": first_cid,
                        })),
                    };
                    session.messages.push(assistant_msg.clone());
                    session.updated_at = now;
                    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
                    fs::write(&path, content).map_err(|e| e.to_string())?;
                    return Ok(assistant_msg);
                }

                // 1) 原样回填 assistant 的 tool_calls 消息（OpenAI 要求）
                api_messages.push(message.clone());

                // 2) 逐个执行工具，回填 role=tool 结果
                for call in &calls {
                    let call_id = call["id"].as_str().unwrap_or("").to_string();
                    let fname = call["function"]["name"].as_str().unwrap_or("").to_string();
                    let raw_args = call["function"]["arguments"].as_str().unwrap_or("{}");
                    let args: serde_json::Value = serde_json::from_str(raw_args).unwrap_or(serde_json::json!({}));

                    let result = dispatch_tool(&fname, &args).await;
                    let mut result_str = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
                    if result_str.chars().count() > MAX_TOOL_RESULT_CHARS {
                        result_str = result_str.chars().take(MAX_TOOL_RESULT_CHARS).collect::<String>()
                            + "\n...(结果过长已截断)";
                    }

                    tool_trace.push(serde_json::json!({
                        "name": fname,
                        "args": args,
                    }));

                    api_messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": result_str
                    }));
                }

                round += 1;
                continue;
            }
        }

        // 无工具调用 → 取最终文本回复
        let content_opt = message["content"].as_str()
            .or_else(|| res_data["choices"][0]["text"].as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());

        reply = match content_opt {
            Some(s) => s,
            None => {
                // 兜底：模型调用过工具却没给出文本回答（常见于知识库无相关资料、
                // 模型反复检索直到轮数耗尽）。返回友好降级提示，而非抛原始错误。
                if !tool_trace.is_empty() {
                    "抱歉，我查询了相关信息但未能整理出最终回答。可能是知识库中暂无相关资料，或问题需要更具体。你可以换个问法，或先在「企业知识库」上传相关文档后重试。".to_string()
                } else {
                    return Err(format!(
                        "LLM 返回空内容。原始响应：{}",
                        body_text.chars().take(400).collect::<String>()
                    ));
                }
            }
        };
        break;
    }

    let tool_used = if tool_trace.is_empty() {
        None
    } else {
        Some(tool_trace.iter()
            .filter_map(|t| t["name"].as_str())
            .collect::<Vec<_>>()
            .join(", "))
    };
    let tool_data = if tool_trace.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "calls": tool_trace }))
    };

    let assistant_msg = ChatMessage {
        role: "assistant".to_string(),
        content: reply,
        timestamp: now_secs(),
        tool_used,
        tool_data,
    };
    session.messages.push(assistant_msg.clone());
    session.updated_at = now_secs();

    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(assistant_msg)
}

/// 把一条消息追加到会话文件并持久化。会话不存在则静默跳过。
fn append_message_to_session(session_id: &str, msg: &ChatMessage) -> Result<(), String> {
    let path = get_chats_dir().join(format!("{}.json", session_id));
    if !path.exists() {
        return Ok(());
    }
    let s = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut session: ChatSession = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    session.messages.push(msg.clone());
    session.updated_at = now_secs();
    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 不调 LLM：把多个动作执行结果拼成结构化中文说明（默认方式，省配额）。
fn fallback_structured_summary(
    executions: &[(String, serde_json::Value, serde_json::Value)],
) -> String {
    executions.iter()
        .map(|(name, args, result)| summarize_action_result(name, args, result))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 开关 ai_summarize_actions 开启时：调一次 LLM，把动作执行结果总结成自然语言。
async fn summarize_executions_with_llm(
    config: &crate::models::AppConfig,
    executions: &[(String, serde_json::Value, serde_json::Value)],
) -> Result<String, String> {
    let mut detail = String::new();
    for (name, args, result) in executions {
        detail.push_str(&format!(
            "- 动作：{}\n  参数：{}\n  结果：{}\n",
            name,
            serde_json::to_string(args).unwrap_or_default(),
            serde_json::to_string(result).unwrap_or_default().chars().take(800).collect::<String>(),
        ));
    }

    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };
    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": "你是 AutoCast AI 助手。系统刚替用户执行了一些动作，请用简洁、友好的中文向用户说明每个动作的执行结果（每个动作 1-2 句话），不要编造未提供的信息，不要输出 JSON。" },
            { "role": "user", "content": format!("已执行以下动作，请总结结果：\n{}", detail) }
        ],
        "temperature": 0.4
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build().map_err(|e| e.to_string())?;
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload).send().await
        .map_err(|e| format!("总结请求失败: {}", e))?;
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("LLM API 错误 {}", status));
    }
    let data: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    data["choices"][0]["message"]["content"].as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "LLM 返回空内容".to_string())
}

/// Phase 3：用户点「允许」后调用，**批量**确认并真正执行所有暂存动作。
/// 执行结果会持久化为一条 assistant 消息（按 ai_summarize_actions 开关决定是否额外 LLM 总结），
/// 并把该消息返回给前端展示。
///
/// start_scrape / synthesize_speech 因依赖 State<'_, AppState>，在此专门调用，
/// 其余动作走 dispatch_tool 的通用路径。
#[tauri::command]
pub async fn confirm_tool_execution(
    confirmation_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ChatMessage, String> {
    if confirmation_ids.is_empty() {
        return Err("没有待确认的动作".to_string());
    }
    let config = get_config().await?;

    let mut session_id = String::new();
    // 每个动作收集 (tool_name, args, result)
    let mut executions: Vec<(String, serde_json::Value, serde_json::Value)> = Vec::new();

    for cid in &confirmation_ids {
        let pending = {
            let mut map = pending_confirmations().lock().unwrap();
            map.remove(cid)
        };
        let pending = match pending {
            Some(p) => p,
            None => continue, // 找不到（已超时/已处理）则跳过
        };
        if session_id.is_empty() {
            session_id = pending.session_id.clone();
        }

        // TTL 检查：超时的不执行，记一条 error 结果
        if now_secs().saturating_sub(pending.created_at) > PENDING_TOOL_TTL_SECS {
            executions.push((
                pending.tool_name.clone(),
                pending.args.clone(),
                serde_json::json!({ "error": "确认已超时，未执行" }),
            ));
            continue;
        }

        // 真正执行
        let result: serde_json::Value = match pending.tool_name.as_str() {
            "start_scrape" => {
                let account_name = pending.args.get("account_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let platform = pending.args.get("platform").and_then(|v| v.as_str()).unwrap_or("douyin").to_string();
                let sec_uid = pending.args.get("sec_uid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let scrape_type = pending.args.get("scrape_type").and_then(|v| v.as_str()).unwrap_or("videos_comments").to_string();
                let limit = pending.args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32).unwrap_or(50);
                let skip_existing = pending.args.get("skip_existing").and_then(|v| v.as_bool()).unwrap_or(true);
                let incremental = pending.args.get("incremental").and_then(|v| v.as_bool()).unwrap_or(false);
                match crate::commands::scraper::start_scrape(
                    account_name, platform, sec_uid, scrape_type,
                    limit, skip_existing, incremental, state.clone(),
                ).await {
                    Ok(task) => serde_json::to_value(task).unwrap_or(serde_json::json!(null)),
                    Err(e) => serde_json::json!({ "error": e }),
                }
            }
            "synthesize_speech" => {
                let project_id = pending.args.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let text = pending.args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let voice_id = pending.args.get("voice_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let speed = pending.args.get("speed").and_then(|v| v.as_f64()).map(|n| n as f32).unwrap_or(1.0);
                match crate::commands::video_studio::generation::tts_synthesize(
                    state.clone(), project_id, text, voice_id, speed,
                    None, None, None, None,
                ).await {
                    Ok(p) => serde_json::json!({ "audio_path": p }),
                    Err(e) => serde_json::json!({ "error": e }),
                }
            }
            _ => dispatch_tool(&pending.tool_name, &pending.args).await,
        };

        crate::commands::tools::log_action_execution(&pending.tool_name, &pending.args, &result).await;
        executions.push((pending.tool_name.clone(), pending.args.clone(), result));
    }

    if executions.is_empty() {
        return Err("所有待确认动作都已超时或失效，请重新发起。".to_string());
    }

    // 生成展示内容：开关开启且配置了 key → LLM 总结（失败回退结构化）；否则结构化
    let content = if config.llm.ai_summarize_actions && !config.llm.api_key.is_empty() {
        match summarize_executions_with_llm(&config, &executions).await {
            Ok(s) => s,
            Err(_) => fallback_structured_summary(&executions),
        }
    } else {
        fallback_structured_summary(&executions)
    };

    let calls_trace: Vec<serde_json::Value> = executions.iter()
        .map(|(name, args, _)| serde_json::json!({ "name": name, "args": args, "status": "executed" }))
        .collect();
    let tool_used = executions.iter().map(|(n, _, _)| n.clone()).collect::<Vec<_>>().join(", ");

    let msg = ChatMessage {
        role: "assistant".to_string(),
        content,
        timestamp: now_secs(),
        tool_used: Some(tool_used),
        tool_data: Some(serde_json::json!({ "calls": calls_trace })),
    };
    if !session_id.is_empty() {
        append_message_to_session(&session_id, &msg)?;
    }
    Ok(msg)
}

/// Phase 3：用户点「拒绝」后调用，**批量**取消所有暂存动作，并持久化一条取消消息。
#[tauri::command]
pub async fn cancel_tool_execution(
    confirmation_ids: Vec<String>,
) -> Result<ChatMessage, String> {
    let mut session_id = String::new();
    let mut names: Vec<String> = Vec::new();
    for cid in &confirmation_ids {
        let pending = {
            let mut map = pending_confirmations().lock().unwrap();
            map.remove(cid)
        };
        if let Some(p) = pending {
            if session_id.is_empty() {
                session_id = p.session_id.clone();
            }
            names.push(p.tool_name);
        }
    }

    let content = if names.len() > 1 {
        format!("🚫 已取消 {} 个动作（{}）。", names.len(), names.join("、"))
    } else if names.len() == 1 {
        format!("🚫 已取消动作 `{}`。", names[0])
    } else {
        "🚫 没有可取消的动作（可能已超时失效）。".to_string()
    };

    let msg = ChatMessage {
        role: "system".to_string(),
        content,
        timestamp: now_secs(),
        tool_used: None,
        tool_data: None,
    };
    if !session_id.is_empty() {
        append_message_to_session(&session_id, &msg)?;
    }
    Ok(msg)
}
