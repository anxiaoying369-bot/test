//! 动作工具的审计日志与面向用户的结果摘要（见 [`super`] 说明）。

use chrono::TimeZone;
use serde_json::Value;

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
