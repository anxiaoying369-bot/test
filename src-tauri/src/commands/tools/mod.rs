//! AI 助理 Function Calling 工具层。
//!
//! 设计见 TOOL_CALLING_CATALOG.md。本层只做两件事：
//!   1. definitions::tool_definitions* —— 暴露给 LLM 的 OpenAI tools[] schema
//!   2. dispatch::dispatch_tool —— 把 tool_call 分发到项目已有业务函数，薄包装结果
//!
//! 原则：复用现有命令的内部逻辑，工具层不重写业务。
//! 阶段：Phase 1 只读 / Phase 2 分析生成 / Phase 3-4 动作（需前端确认）。

pub mod audit;
pub mod definitions;
pub mod dispatch;

pub use audit::{log_action_execution, summarize_action_result};
pub use definitions::{
    is_action_tool, tool_definitions, tool_definitions_action, tool_definitions_all,
};
pub use dispatch::dispatch_tool;

/// 单个工具结果序列化后允许的最大字符数，超出则截断，防止撑爆 LLM 上下文。
pub const MAX_TOOL_RESULT_CHARS: usize = 6000;
