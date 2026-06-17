//! AI 助理 Function Calling 工具层。
//!
//! 设计见 TOOL_CALLING_PLAN.md。本模块只做两件事：
//!   1. tool_definitions(): 返回 OpenAI tools[] schema（暴露给 LLM 的工具清单）
//!   2. dispatch_tool(): 把 LLM 的 tool_call 分发到项目已有的业务函数，薄包装结果
//!
//! 原则：复用现有命令的内部逻辑，工具层不重写业务。
//!
//! 阶段：
//!   Phase 1（只读查询）   — 已完成
//!   Phase 2（分析/生成）   — 已完成
//!   Phase 3（动作/写入）   — 部分完成（add/delete_kb_file/start_scrape/tts_synthesize 标记为确认型）

use serde_json::{json, Value};

use chrono::TimeZone;

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

/// 单个工具结果序列化后允许的最大字符数，超出则截断，防止撑爆 LLM 上下文。
pub const MAX_TOOL_RESULT_CHARS: usize = 6000;

/// 暴露给 LLM 的工具定义（OpenAI `tools` 数组）。
/// 阶段：Phase 1（只读）+ Phase 2（分析/生成）。Phase 3 动作工具在 `tool_definitions_action()` 中。
pub fn tool_definitions() -> Value {
    json!([
        // ===== Phase 1：只读查询（自动执行）=====
        {
            "type": "function",
            "function": {
                "name": "search_knowledge_base",
                "description": "检索企业知识库，返回与查询语义最相关的文档片段。当用户的问题可能涉及公司/产品/政策等专业背景知识时调用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "检索关键词或自然语言问题" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_kb_documents",
                "description": "列出企业知识库中已索引的所有文档名称。当用户想知道知识库里有哪些资料时调用。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_scraped_users",
                "description": "列出本地已采集过的所有博主（含 sec_uid、昵称等）。需要进一步查询某博主作品或评论前，先用它拿到 sec_uid。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "query_videos",
                "description": "查询某个已采集博主的作品列表。需要先有该博主的 sec_uid（可用 list_scraped_users 获取）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sec_uid": { "type": "string", "description": "博主唯一 ID" },
                        "limit":   { "type": "integer", "description": "返回作品条数，默认 20" }
                    },
                    "required": ["sec_uid"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "query_comments",
                "description": "查询已采集的评论。可按博主 sec_uid 查其全部评论，或附带 aweme_id 只查某条作品的评论。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sec_uid":  { "type": "string", "description": "博主唯一 ID" },
                        "aweme_id": { "type": "string", "description": "作品 ID，省略则返回该博主全部评论" },
                        "limit":    { "type": "integer", "description": "返回评论条数，默认 50" }
                    },
                    "required": ["sec_uid"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_accounts",
                "description": "列出当前已登录/管理的平台账号及其状态。当用户询问账号情况时调用。",
                "parameters": { "type": "object", "properties": {} }
            }
        },

        // ===== Phase 2：分析/生成（自动执行，消耗 API）=====
        {
            "type": "function",
            "function": {
                "name": "analyze_comments",
                "description": "对一批已采集的评论做 AI 舆情分析，输出情绪倾向、热点话题、用户意图、互动建议等。需先有评论数据（可用 query_comments 拿到）。会自动取前 50 条做分析。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "comments": {
                            "type": "array",
                            "description": "评论列表，每条形如 {text: string, ...}。如未传，会自动尝试用 sec_uid+aweme_id 拉取。",
                            "items": { "type": "object" }
                        }
                    },
                    "required": ["comments"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "generate_script",
                "description": "根据产品/主题生成短视频口播或表演脚本。会自动注入知识库上下文。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "product":      { "type": "string", "description": "要卖的产品或主题描述" },
                        "video_ratio":  { "type": "string", "description": "视频比例，可选 9:16 / 16:9 / 1:1，默认 9:16" },
                        "platform":     { "type": "string", "description": "目标平台 ID（douyin/kuaishou/xiaohongshu等），可选" }
                    },
                    "required": ["product"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "geo_query",
                "description": "对企业的 GEO（生成式引擎优化）监控做查询，会结合知识库返回当前监控结果摘要。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "查询关键词或问题" }
                    },
                    "required": ["query"]
                }
            }
        },

        // ===== Phase 4：扩展只读 / 分析工具（自动执行）=====
        {
            "type": "function",
            "function": {
                "name": "get_kb_document_details",
                "description": "查看企业知识库中某个文档的详情（切片数、元数据等）。文档名可用 list_kb_documents 获取。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "filename": { "type": "string", "description": "知识库中的文档名" }
                    },
                    "required": ["filename"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "resolve_user_sec_uid",
                "description": "把抖音分享链接 / 抖音号 / 主页 URL 解析成博主唯一 sec_uid。需要 sec_uid 但用户只给了链接或抖音号时调用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input": { "type": "string", "description": "抖音分享链接、抖音号或主页地址" }
                    },
                    "required": ["input"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_scrape_progress",
                "description": "查询某个采集任务的进度（已采条数、状态等）。task_id 由 start_scrape 返回。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "string", "description": "采集任务 ID" }
                    },
                    "required": ["task_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "fetch_douyin_user_info",
                "description": "在线拉取某个抖音博主的主页资料（昵称、粉丝数、简介等）。会发起网络请求。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "博主昵称/账号名（用于展示）" },
                        "user_id":      { "type": "string", "description": "博主 sec_uid 或抖音号" }
                    },
                    "required": ["account_name", "user_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_user_cards",
                "description": "列出已保存的抖音用户画像卡片（含昵称、sec_uid、粉丝数等画像信息）。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "query_and_save_user",
                "description": "在线查询某抖音博主资料并生成/保存为画像卡片。会写入本地画像库。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "博主昵称/账号名" },
                        "user_id":      { "type": "string", "description": "博主 sec_uid 或抖音号" }
                    },
                    "required": ["account_name", "user_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "generate_content",
                "description": "通用内容生成：根据主题+素材，按指定模式与目标平台生成文案/脚本。比 generate_script 更通用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "topic":           { "type": "string", "description": "内容主题" },
                        "material":        { "type": "string", "description": "参考素材/背景信息" },
                        "mode":            { "type": "string", "description": "生成模式（如 口播/图文/标题 等，按业务约定）" },
                        "platform":        { "type": "string", "description": "目标平台 ID（douyin/xiaohongshu 等）" },
                        "platform_prompt": { "type": "string", "description": "平台风格补充提示，可选" }
                    },
                    "required": ["topic", "material", "mode", "platform"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "generate_video_search_terms",
                "description": "为短视频脚本生成一组素材检索关键词（用于到素材库/网络找配图配片）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "video_subject": { "type": "string", "description": "视频主题" },
                        "video_script":  { "type": "string", "description": "视频脚本全文" },
                        "amount":        { "type": "integer", "description": "关键词数量，默认 5" }
                    },
                    "required": ["video_subject", "video_script"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "resolve_live_url",
                "description": "把抖音直播间链接解析成 room_id。需要监控直播间但只有链接时调用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "抖音直播间分享链接或地址" }
                    },
                    "required": ["url"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_live_history",
                "description": "查询某个直播间已记录的历史弹幕/互动数据。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "room_id": { "type": "string", "description": "直播间 ID" }
                    },
                    "required": ["room_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "generate_live_reply",
                "description": "为直播间某条观众弹幕生成一条合适的回复话术。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "user_name": { "type": "string", "description": "发言观众昵称" },
                        "content":   { "type": "string", "description": "观众弹幕内容" }
                    },
                    "required": ["user_name", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_diagnostics",
                "description": "运行 AutoCast 一键自检，返回依赖/网关/配置等环境健康状况。当用户报告功能异常、想排查环境时调用。",
                "parameters": { "type": "object", "properties": {} }
            }
        }
    ])
}

/// Phase 3：动作/写入类工具（需前端确认）。单独返回以便在 chat.rs 中按风险等级分开处理。
/// LLM 调这些工具时，Rust 不立即执行，而是先返回一个 pending_confirmation 给前端，
/// 由用户确认后才走真实命令。
pub fn tool_definitions_action() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "start_scrape",
                "description": "启动对一个博主的作品/评论采集任务（后台长耗时任务）。需用户提供 sec_uid 与博主昵称。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "博主昵称/账号名（用于展示）" },
                        "platform":     { "type": "string", "description": "平台名，固定 douyin" },
                        "sec_uid":      { "type": "string", "description": "博主唯一 sec_uid" },
                        "scrape_type":  { "type": "string", "description": "采集类型：videos / comments / videos_comments" },
                        "limit":        { "type": "integer", "description": "采集上限，默认 50" },
                        "skip_existing":{ "type": "boolean", "description": "是否跳过已存在作品" },
                        "incremental":  { "type": "boolean", "description": "是否增量" }
                    },
                    "required": ["account_name", "platform", "sec_uid", "scrape_type"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "add_document_to_kb",
                "description": "把本地文件（PDF/DOCX/XLSX/TXT/JSON）添加到企业知识库。会向量化并写入 LanceDB。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "本地文件的绝对路径" }
                    },
                    "required": ["file_path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_kb_file",
                "description": "从企业知识库中删除指定文档（破坏性操作）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "filename": { "type": "string", "description": "知识库中的文档名（从 list_kb_documents 获取）" }
                    },
                    "required": ["filename"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "synthesize_speech",
                "description": "用 TTS 合成语音到当前激活的视频项目。会写入项目目录并产生费用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": { "type": "string", "description": "视频项目 ID" },
                        "text":       { "type": "string", "description": "要合成的文本" },
                        "voice_id":   { "type": "string", "description": "声纹 ID（可用 tts_list_voices 查）" },
                        "speed":      { "type": "number",  "description": "语速倍数，默认 1.0" }
                    },
                    "required": ["project_id", "text", "voice_id"]
                }
            }
        },

        // ===== Phase 4：扩展动作工具（需前端确认）=====
        {
            "type": "function",
            "function": {
                "name": "verify_account",
                "description": "校验某个平台账号的登录状态（Cookie 是否仍有效）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "platform": { "type": "string", "description": "平台 ID（douyin/kuaishou 等）" },
                        "name":     { "type": "string", "description": "账号名" }
                    },
                    "required": ["platform", "name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "sync_local_accounts",
                "description": "扫描本地 cookie 目录，把发现的账号回写到账号库。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_account",
                "description": "删除某个平台账号（破坏性操作）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "platform": { "type": "string", "description": "平台 ID" },
                        "name":     { "type": "string", "description": "账号名" }
                    },
                    "required": ["platform", "name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_scraped_user",
                "description": "删除某个已采集博主的全部本地作品/评论数据（破坏性操作）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sec_uid": { "type": "string", "description": "博主唯一 sec_uid" }
                    },
                    "required": ["sec_uid"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "refresh_user_card",
                "description": "重新在线拉取并刷新某个用户画像卡片。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "博主昵称/账号名" },
                        "sec_uid":      { "type": "string", "description": "博主唯一 sec_uid" }
                    },
                    "required": ["account_name", "sec_uid"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_user_card",
                "description": "删除某个用户画像卡片（破坏性操作）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sec_uid": { "type": "string", "description": "博主唯一 sec_uid" }
                    },
                    "required": ["sec_uid"]
                }
            }
        }
    ])
}

/// 返回所有工具（Phase 1+2+3 合并）。chat.rs 调用时按"是否在动作白名单"判断是否走确认。
pub fn tool_definitions_all() -> Value {
    let mut all = tool_definitions();
    let action = tool_definitions_action();
    if let (Some(arr1), Some(arr2)) = (all.as_array_mut(), action.as_array()) {
        for t in arr2 {
            arr1.push(t.clone());
        }
    }
    all
}

/// Phase 3 动作工具名集合（用于 chat.rs 判断是否需走 human-in-the-loop）。
pub fn is_action_tool(name: &str) -> bool {
    matches!(
        name,
        "start_scrape"
            | "add_document_to_kb"
            | "delete_kb_file"
            | "synthesize_speech"
            | "verify_account"
            | "sync_local_accounts"
            | "delete_account"
            | "delete_scraped_user"
            | "refresh_user_card"
            | "delete_user_card"
    )
}

/// Phase 3 审计日志目录
const AUDIT_LOG_DIR: &str = "tool_audit";

/// Phase 3 动作执行的审计日志。每条记录一行 JSON，包含时间戳、工具名、参数、结果摘要。
/// 设计原则：失败也记录，便于事后追溯。
pub async fn log_action_execution(
    tool_name: &str,
    args: &Value,
    result: &Value,
) {
    use std::io::Write;

    // 获取数据目录（与项目其它模块保持一致）
    let dir = crate::utils::get_data_dir().join(AUDIT_LOG_DIR);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[tool_audit] 创建目录失败 {}: {}", dir.display(), e);
        return;
    }

    // 文件名按日期切分（每天一个文件）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let date = chrono::Local.timestamp_opt(now as i64, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let path = dir.join(format!("actions-{}.jsonl", date));

    let record = serde_json::json!({
        "ts": now,
        "tool": tool_name,
        "args": args,
        "result": summarize_result(result),
    });

    // 追加写（用 Mutex 避免多线程冲突）
    if let Ok(line) = serde_json::to_string(&record) {
        let line_with_nl = format!("{}\n", line);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = f.write_all(line_with_nl.as_bytes());
        } else {
            eprintln!("[tool_audit] 打开日志文件失败: {}", path.display());
        }
    }
}

/// 提取 result 的简短摘要，避免日志过大。
fn summarize_result(result: &Value) -> Value {
    if let Some(err) = result.get("error") {
        return serde_json::json!({ "error": err });
    }
    if result.is_object() && result.as_object().map(|o| o.len() <= 3).unwrap_or(false) {
        return result.clone();
    }
    // 对大结果只保留关键字段
    let mut summary = serde_json::Map::new();
    if let Some(s) = result.get("status") {
        summary.insert("status".into(), s.clone());
    }
    if let Some(s) = result.get("chunks_added") {
        summary.insert("chunks_added".into(), s.clone());
    }
    if let Some(s) = result.get("task_id") {
        summary.insert("task_id".into(), s.clone());
    }
    if let Some(s) = result.get("audio_path") {
        summary.insert("audio_path".into(), s.clone());
    }
    if summary.is_empty() {
        // fallback: 截断到 200 字符
        let s = result.to_string();
        if s.len() > 200 {
            serde_json::json!({ "_truncated": format!("{}…", &s[..200]) })
        } else {
            result.clone()
        }
    } else {
        Value::Object(summary)
    }
}

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
            Some(q) => geo_monitor_query(q).await,
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

/// 把动作工具的执行结果整理成面向用户的简洁中文说明（不调 LLM，省配额）。
/// 用于 Phase 3 动作工具确认执行后的默认反馈。
pub fn summarize_action_result(tool_name: &str, args: &Value, result: &Value) -> String {
    if let Some(err) = result.get("error") {
        return format!("❌ 执行 `{}` 失败：{}", tool_name, err);
    }
    match tool_name {
        "start_scrape" => {
            let account = args.get("account_name").and_then(|v| v.as_str()).unwrap_or("该博主");
            let stype = args.get("scrape_type").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(0);
            let task_id = result.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
            let type_cn = match stype {
                "videos" => "作品",
                "comments" => "评论",
                _ => "作品+评论",
            };
            let tail = if task_id.is_empty() {
                String::new()
            } else {
                format!(" 任务 ID：`{}`，可在「评论采集」页查看进度。", task_id)
            };
            format!(
                "✅ 已启动对 **{}** 的采集任务（类型：{}，上限 {} 条），正在后台运行。{}",
                account, type_cn, limit, tail
            )
        }
        "add_document_to_kb" => {
            let n = result.get("chunks_added").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("✅ 已将文档加入企业知识库，新增 **{}** 个知识切片。", n)
        }
        "delete_kb_file" => {
            let fname = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            format!("✅ 已从知识库删除文件 **{}**。", fname)
        }
        "synthesize_speech" => {
            let path = result.get("audio_path").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                "✅ 语音合成完成。".to_string()
            } else {
                format!("✅ 语音合成完成。音频文件：`{}`", path)
            }
        }
        "verify_account" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let message = result.get("message").and_then(|v| v.as_str()).unwrap_or("");
            format!("账号 **{}** 校验结果：{} {}", name, status, message)
        }
        "sync_local_accounts" => {
            let n = result.get("synced").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("✅ 已同步本地账号，共 **{}** 个。", n)
        }
        "delete_account" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            format!("✅ 已删除账号 **{}**。", name)
        }
        "delete_scraped_user" => {
            let uid = args.get("sec_uid").and_then(|v| v.as_str()).unwrap_or("");
            format!("✅ 已删除博主 `{}` 的本地采集数据。", uid)
        }
        "refresh_user_card" => {
            let name = args.get("account_name").and_then(|v| v.as_str()).unwrap_or("该用户");
            format!("✅ 已刷新 **{}** 的画像卡片。", name)
        }
        "delete_user_card" => {
            let uid = args.get("sec_uid").and_then(|v| v.as_str()).unwrap_or("");
            format!("✅ 已删除画像卡片 `{}`。", uid)
        }
        _ => {
            let s = serde_json::to_string(result).unwrap_or_default();
            let s = if s.chars().count() > 500 {
                s.chars().take(500).collect::<String>() + "..."
            } else {
                s
            };
            format!("✅ 已执行 `{}`。结果：{}", tool_name, s)
        }
    }
}
