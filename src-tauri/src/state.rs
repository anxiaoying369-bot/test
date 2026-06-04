use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use crate::models::LoginFlow;

pub static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static SCRIPTS_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static BUNDLED_PYTHON: OnceLock<String> = OnceLock::new();

/// Phase 3 human-in-the-loop：保存待用户确认的 Phase 3 工具调用。
/// key = confirmation_id（UUID），value = 完整 tool_call 信息。
/// 用户在前端确认/取消后，调用 `confirm_tool_execution` 或 `cancel_tool_execution` 消费。
pub static PENDING_TOOL_CONFIRMATIONS: OnceLock<Mutex<std::collections::HashMap<String, PendingToolCall>>> = OnceLock::new();

/// 暂存的 Phase 3 工具调用信息。
/// 包含：原 tool_call_id、工具名、参数、所在会话 id、回填 messages 所需上下文。
#[derive(Clone, Debug)]
pub struct PendingToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    /// 这个 tool_call 来自哪一次 LLM 响应（用于在用户确认后回填结果）
    pub session_id: String,
    /// 用户拒绝时回填给 LLM 的 "已取消" 文案
    pub cancel_message: String,
    /// 暂存时间（Unix 秒）。超过 `PENDING_TOOL_TTL_SECS` 视为超时，自动清理。
    pub created_at: u64,
}

/// 待确认工具的最大存活时间。超时后 confirm/cancel 调用会返回错误。
/// 设为 5 分钟：足够用户思考、避免长时间挂起导致内存泄漏。
pub const PENDING_TOOL_TTL_SECS: u64 = 5 * 60;

pub fn pending_confirmations() -> &'static Mutex<std::collections::HashMap<String, PendingToolCall>> {
    PENDING_TOOL_CONFIRMATIONS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
    pub process_handles: Mutex<std::collections::HashMap<String, tokio::process::Child>>,
    pub current_task_id: Mutex<Option<String>>,
    pub video_db: Mutex<rusqlite::Connection>,
}
