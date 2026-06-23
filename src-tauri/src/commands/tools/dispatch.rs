//! 工具分发：把 LLM 的 tool_call 分发到项目已有业务函数，薄包装结果（见 [`super`] 说明）。

use serde_json::{json, Value};

use crate::commands::account::{delete_account, list_accounts, sync_local_accounts, verify_account};
use crate::commands::diagnostics::autocast_diagnostics;
use crate::commands::geo::geo_monitor_query;
use crate::commands::knowledge_base::{get_kb_file_details, list_kb_files, search_kb_internal};
use crate::commands::live_monitor::{generate_live_reply, get_live_history, resolve_live_url};
use crate::commands::scraper::{
    delete_scraped_user, fetch_douyin_user_info, get_scrape_progress, get_scraped_comments,
    get_scraped_videos, list_scraped_users, resolve_user_sec_uid,
};
use crate::commands::studio::{studio_analyze_video_comments, studio_generate_content};
use crate::commands::user_cards::{
    delete_user_card, list_user_cards, query_and_save_user, refresh_user_card,
};
use crate::commands::video_studio::generation::video_generate_script;
use crate::commands::video_studio::mpt::video_mpt_generate_terms;

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn arg_i32(args: &Value, key: &str, default: i32) -> i32 {
    args.get(key)
        .and_then(|v| v.as_i64())
        .map(|n| n as i32)
        .unwrap_or(default)
}

/// 把一个 tool_call 分发到对应业务函数。返回结构化 JSON 结果（成功或 {"error": ...}）。
///
/// 注意：永远返回 Ok——工具自身的失败以 {"error"} 形式回传给 LLM，由模型决定如何应对，
/// 而不是中断整个对话。
///
/// Phase 3 动作工具也可以通过此函数调用，但 chat.rs 应在调用前先走 human-in-the-loop 确认。
pub async fn dispatch_tool(name: &str, args: &Value) -> Value {
    let result: Result<Value, String> = match name {
        // ===== Phase 1：只读 =====
        "search_knowledge_base" => match arg_str(args, "query") {
            Some(q) => match search_kb_internal(q).await {
                Ok(s) => Ok(serde_json::from_str(&s).unwrap_or(json!([]))),
                Err(e) => Err(e),
            },
            None => Err("缺少必填参数 query".to_string()),
        },

        "list_kb_documents" => list_kb_files().await,

        "list_scraped_users" => list_scraped_users().await,

        "query_videos" => match arg_str(args, "sec_uid") {
            Some(sec_uid) => {
                let limit = arg_i32(args, "limit", 20);
                get_scraped_videos(sec_uid, limit, 0).await
            }
            None => Err("缺少必填参数 sec_uid".to_string()),
        },

        "query_comments" => match arg_str(args, "sec_uid") {
            Some(sec_uid) => {
                let aweme_id = arg_str(args, "aweme_id");
                let limit = arg_i32(args, "limit", 50);
                get_scraped_comments(sec_uid, aweme_id, limit, 0).await
            }
            None => Err("缺少必填参数 sec_uid".to_string()),
        },

        "list_accounts" => list_accounts(None)
            .await
            .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),

        // ===== Phase 2：分析/生成 =====
        "analyze_comments" => {
            let comments = args.get("comments")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if comments.is_empty() {
                return json!({"error": "缺少必填参数 comments（评论数组）"});
            }
            match studio_analyze_video_comments(comments).await {
                Ok(report) => Ok(json!({"report": report})),
                Err(e) => Err(e),
            }
        }

        "generate_script" => match arg_str(args, "product") {
            Some(product) => {
                let video_ratio = arg_str(args, "video_ratio").unwrap_or_else(|| "9:16".to_string());
                let platform = arg_str(args, "platform");
                match video_generate_script(
                    product,
                    None,            // reference_script
                    video_ratio,
                    platform,
                    None,            // script_type
                    None,            // previous_script
                    None,            // feedback
                ).await {
                    Ok(script) => Ok(json!({"script": script})),
                    Err(e) => Err(e),
                }
            }
            None => Err("缺少必填参数 product".to_string()),
        },

        "geo_query" => match arg_str(args, "query") {
            Some(q) => geo_monitor_query(None, Some(q)).await,
            None => Err("缺少必填参数 query".to_string()),
        },

        // ===== Phase 4：扩展只读 / 分析 =====
        "get_kb_document_details" => match arg_str(args, "filename") {
            Some(f) => get_kb_file_details(f).await,
            None => Err("缺少必填参数 filename".to_string()),
        },

        "resolve_user_sec_uid" => match arg_str(args, "input") {
            Some(input) => resolve_user_sec_uid(input).await.map(|s| json!({ "sec_uid": s })),
            None => Err("缺少必填参数 input".to_string()),
        },

        "get_scrape_progress" => match arg_str(args, "task_id") {
            Some(tid) => get_scrape_progress(tid)
                .await
                .and_then(|p| serde_json::to_value(p).map_err(|e| e.to_string())),
            None => Err("缺少必填参数 task_id".to_string()),
        },

        "fetch_douyin_user_info" => match (arg_str(args, "account_name"), arg_str(args, "user_id")) {
            (Some(name), Some(uid)) => fetch_douyin_user_info(name, uid).await,
            _ => Err("缺少必填参数 account_name / user_id".to_string()),
        },

        "list_user_cards" => list_user_cards()
            .await
            .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),

        "query_and_save_user" => match (arg_str(args, "account_name"), arg_str(args, "user_id")) {
            (Some(name), Some(uid)) => query_and_save_user(name, uid)
                .await
                .and_then(|c| serde_json::to_value(c).map_err(|e| e.to_string())),
            _ => Err("缺少必填参数 account_name / user_id".to_string()),
        },

        "generate_content" => {
            match (arg_str(args, "topic"), arg_str(args, "material"), arg_str(args, "mode"), arg_str(args, "platform")) {
                (Some(topic), Some(material), Some(mode), Some(platform)) => {
                    let platform_prompt = arg_str(args, "platform_prompt");
                    studio_generate_content(topic, material, mode, platform, platform_prompt).await
                }
                _ => Err("缺少必填参数 topic / material / mode / platform".to_string()),
            }
        }

        "generate_video_search_terms" => {
            match (arg_str(args, "video_subject"), arg_str(args, "video_script")) {
                (Some(subject), Some(script)) => {
                    let amount = args.get("amount").and_then(|v| v.as_u64()).map(|n| n as u32);
                    video_mpt_generate_terms(subject, script, amount)
                        .await
                        .map(|terms| json!({ "terms": terms }))
                }
                _ => Err("缺少必填参数 video_subject / video_script".to_string()),
            }
        }

        "resolve_live_url" => match arg_str(args, "url") {
            Some(url) => resolve_live_url(url).await.map(|s| json!({ "room_id": s })),
            None => Err("缺少必填参数 url".to_string()),
        },

        "get_live_history" => match arg_str(args, "room_id") {
            Some(rid) => get_live_history(rid).await.map(|v| json!({ "history": v })),
            None => Err("缺少必填参数 room_id".to_string()),
        },

        "generate_live_reply" => match (arg_str(args, "user_name"), arg_str(args, "content")) {
            (Some(name), Some(content)) => {
                generate_live_reply(name, content).await.map(|s| json!({ "reply": s }))
            }
            _ => Err("缺少必填参数 user_name / content".to_string()),
        },

        "run_diagnostics" => autocast_diagnostics().await,

        // ===== Phase 3：动作/写入（需先经前端确认）=====
        // 详见 chat.rs 中的 confirm_tool_call 流程
        "add_document_to_kb" => match arg_str(args, "file_path") {
            Some(p) => crate::commands::knowledge_base::add_to_kb(p).await,
            None => Err("缺少必填参数 file_path".to_string()),
        },
        "delete_kb_file" => match arg_str(args, "filename") {
            Some(n) => crate::commands::knowledge_base::delete_kb_file(n).await,
            None => Err("缺少必填参数 filename".to_string()),
        },

        // ===== Phase 4：扩展动作（无 State 依赖，经前端确认后走本函数）=====
        "verify_account" => match (arg_str(args, "platform"), arg_str(args, "name")) {
            (Some(p), Some(n)) => verify_account(p, n)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(|e| e.to_string())),
            _ => Err("缺少必填参数 platform / name".to_string()),
        },

        "sync_local_accounts" => sync_local_accounts().await.map(|n| json!({ "synced": n })),

        "delete_account" => match (arg_str(args, "platform"), arg_str(args, "name")) {
            (Some(p), Some(n)) => delete_account(p, n).await.map(|_| json!({ "status": "ok" })),
            _ => Err("缺少必填参数 platform / name".to_string()),
        },

        "delete_scraped_user" => match arg_str(args, "sec_uid") {
            Some(uid) => delete_scraped_user(uid).await.map(|_| json!({ "status": "ok" })),
            None => Err("缺少必填参数 sec_uid".to_string()),
        },

        "refresh_user_card" => match (arg_str(args, "account_name"), arg_str(args, "sec_uid")) {
            (Some(name), Some(uid)) => refresh_user_card(name, uid)
                .await
                .and_then(|c| serde_json::to_value(c).map_err(|e| e.to_string())),
            _ => Err("缺少必填参数 account_name / sec_uid".to_string()),
        },

        "delete_user_card" => match arg_str(args, "sec_uid") {
            Some(uid) => delete_user_card(uid).await.map(|_| json!({ "status": "ok" })),
            None => Err("缺少必填参数 sec_uid".to_string()),
        },

        // start_scrape / synthesize_speech 因依赖 State<'_, AppState>，在 chat.rs 中
        // 通过专用 confirm-and-execute 路径直接调用，不走本函数。

        other => Err(format!("未知工具: {}", other)),
    };

    match result {
        Ok(v) => v,
        Err(e) => json!({ "error": e }),
    }
}
