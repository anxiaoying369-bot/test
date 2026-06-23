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
