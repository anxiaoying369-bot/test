# AI 助理工具调用（Function Calling）设计文档

> 版本：v2 · 状态：Phase 0–3 已落地，待充分测试 · 适用模块：AI 助理（ChatView / chat.rs）

## 1. 背景与目标

当前 AI 助理（`ChatView` + `chat.rs::send_chat_message`）是**纯对话**模式：每条用户消息都会盲目检索一次企业知识库，把命中内容塞进 system prompt，再请求 LLM。它"看不见"业务数据——无法主动查询采集到的博主、评论、直播历史，也无法触发分析、脚本生成等动作。

**目标**：让 AI 助理具备 **OpenAI 标准 Function Calling 能力**，把项目已有的业务命令包装成工具，使助理能：

- 按需查询业务数据（采集结果、评论、知识库、账号、直播历史）
- 按需触发分析与生成（评论分析、脚本生成）
- 在严格授权下执行有副作用的动作（采集、删除、合成等）

## 2. 现状分析

| 现状 | 说明 |
| --- | --- |
| 已有 `scripts/funcall.py` / `funcall_tools.py` | 含 `Tool` 类与 OpenAI 格式转换，但是**孤立的 CLI**，未接入 Rust，且默认工具是通用开发工具（`read_file`/`run_command` 等），与业务无关且不安全 |
| `chat.rs::send_chat_message` | 纯对话 + 每轮强制 KB 注入，无工具调用 |
| `models.rs::ChatMessage` | **已预留** `tool_used` / `tool_data` 字段，设计之初即考虑工具调用，但未实现 |

**结论**：不复用 Python CLI 工具。改为 **Rust 作为工具执行层**，把已验证的 Tauri 命令薄包装成工具——业务逻辑零重写。

## 3. 架构设计

```
用户提问
  → chat.rs 组装 messages + tools[] 请求 LLM
  → LLM 返回 finish_reason=tool_calls
  → Rust 解析 tool_calls，按白名单分发到对应业务函数
      ├─ 只读工具：直接执行
      └─ 动作工具：需前端确认（human-in-the-loop）
  → 工具结果作为 role=tool 消息回填
  → 再次请求 LLM（循环，至多 N 轮）
  → finish_reason=stop → 返回最终回复
  → tool_used / tool_data 记录调用轨迹，前端可展示
```

设计原则：

1. **复用现有内部函数**（`search_kb_internal`、`list_scraped_users` 等），工具层只做参数映射与 JSON 包装。
2. **分级授权**：只读自动执行；写入/动作类需用户确认。
3. **轮数上限**：单次对话最多 `MAX_TOOL_ROUNDS = 5` 轮工具调用，防止 LLM 无限循环。
4. **优雅降级**：未配置工具或模型不支持时，回退到现有纯对话逻辑。

## 4. 工具清单（分级）

### 🟢 Phase 1 — 只读查询（自动执行，零副作用）✅ 已实现

| 工具名 | 复用命令/函数 | 参数 | 用途 |
| --- | --- | --- | --- |
| `search_knowledge_base` | `search_kb_internal(query)` | `query: string` | 检索企业知识库 |
| `list_kb_documents` | `list_kb_files()` | 无 | 列出已索引文档 |
| `list_scraped_users` | `list_scraped_users()` | 无 | 列出已采集博主 |
| `query_videos` | `get_scraped_videos(secUid,limit,offset)` | `sec_uid: string, limit?: int` | 查某博主作品 |
| `query_comments` | `get_scraped_comments(secUid,awemeId?,limit,offset)` | `sec_uid: string, aweme_id?: string, limit?: int` | 查评论 |
| `list_accounts` | `list_accounts(None)` | 无 | 列出登录账号 |

> 改进：现有 chat 每轮盲目检索 KB，改为 `search_knowledge_base` 工具后变为**按需检索**，省 token 且更精准。

### 🟡 Phase 2 — 分析/生成（自动执行，消耗 API，需进度提示）✅ 已实现

| 工具名 | 复用命令 | 参数 | 用途 |
| --- | --- | --- | --- |
| `analyze_comments` | `get_scraped_comments` + `studio_analyze_video_comments` | `sec_uid, aweme_id` | 取评论并做舆情分析（闭环） |
| `generate_script` | `video_generate_script` | `product, platform?` | 生成口播/表演脚本 |
| `geo_query` | `geo_monitor_query` | `query` | GEO 监控查询 |

### 🔴 Phase 3 — 动作/写入（必须二次确认，human-in-the-loop）✅ 已实现

| 工具名 | 复用命令 | 风险 |
| --- | --- | --- |
| `start_scrape` | `start_scrape` | 长耗时、消耗配额 |
| `add_document_to_kb` | `add_to_kb` | 写入向量库 |
| `delete_kb_file` | `delete_kb_file` | **破坏性删除** |
| `synthesize_speech` | `tts_synthesize` | 花费、生成文件 |

### ⛔ 不暴露

- funcall 默认的 `write_file` / `run_command` / 任意路径 `read_file` —— 桌面应用安全隐患，不接入。

## 5. 工具 Schema（OpenAI 格式示例）

```jsonc
{
  "type": "function",
  "function": {
    "name": "query_comments",
    "description": "查询某条作品已采集的评论列表。需先有该博主的 sec_uid（可用 list_scraped_users 获取）。",
    "parameters": {
      "type": "object",
      "properties": {
        "sec_uid":  { "type": "string", "description": "博主唯一 ID" },
        "aweme_id": { "type": "string", "description": "作品 ID，省略则返回该博主全部评论" },
        "limit":    { "type": "integer", "description": "返回条数，默认 50", "default": 50 }
      },
      "required": ["sec_uid"]
    }
  }
}
```

## 6. 数据结构与前端展示

复用 `ChatMessage` 现有字段：

```rust
pub tool_used: Option<String>,            // 本轮调用的工具名（多个用逗号或取最后）
pub tool_data: Option<serde_json::Value>, // { calls: [{name, args, result_summary}] }
```

前端 `ChatView` 在 assistant 气泡上方渲染一个可折叠的"🔧 调用了工具：query_comments"提示，点击展开看参数与结果摘要（`expandedAudits` 折叠机制已存在，可复用）。

## 7. 安全与授权

- **只读工具**：直接执行，无需确认。
- **动作工具（Phase 3）**：LLM 返回 tool_call 后，Rust **不立即执行**，而是返回一个 `pending_confirmation` 给前端，前端弹窗"AI 想执行 X，是否允许？"，用户确认后再走第二个命令真正执行。
- **轮数上限**：`MAX_TOOL_ROUNDS = 5`。
- **错误隔离**：单个工具失败，把错误信息作为 tool 结果回填，让 LLM 决定是否换方式，而非整体中断。

## 8. 实现现状

> Phase 0–3 均已落地，等待充分测试。

| 阶段 | 状态 | 交付物 |
| --- | --- | --- |
| Phase 0 基础设施 | ✅ | `commands/tools.rs`（schema + `dispatch_tool`）；`chat.rs::send_chat_message` function-calling 循环；`ChatView` 工具调用轨迹展示 |
| Phase 1 只读查询 | ✅ | `search_knowledge_base` / `list_kb_documents` / `list_scraped_users` / `query_videos` / `query_comments` / `list_accounts` |
| Phase 2 分析/生成 | ✅ | `analyze_comments` / `generate_script` / `geo_query` |
| Phase 3 动作/写入 | ✅ | `start_scrape` / `add_document_to_kb` / `delete_kb_file` / `synthesize_speech` + 前端确认弹窗 |

**关键实现点：**

- `tools.rs`：
  - `tool_definitions()` — 只读 + 分析/生成工具（自动执行）
  - `tool_definitions_action()` — 动作工具（需确认）
  - `tool_definitions_all()` — 合并清单，发给 LLM
  - `is_action_tool(name)` — 判定是否动作工具
  - `dispatch_tool(name, args)` — 只读/分析工具的执行分发
  - `log_action_execution(...)` — 动作执行日志记录
- `chat.rs`：
  - `send_chat_message` — function-calling 循环；命中动作工具时**不执行**，返回 `pending_confirmation` 交前端确认
  - `confirm_tool_execution` / `cancel_tool_execution` — 用户确认/取消后真正执行或放弃动作工具
- 前端 `ChatView` + `useChat.ts`：
  - 工具调用轨迹（`tool_data.calls`）可折叠展示
  - 动作确认弹窗（`pendingConfirmation` / `confirmTool` / `cancelTool`），支持单次多动作确认

## 9. Phase 3 增强（已实现）

确认执行后的闭环与持久化：

- **结果持久化**：`confirm_tool_execution` / `cancel_tool_execution` 改为**批量**（接受 `confirmation_ids: Vec<String>`），执行后通过 `append_message_to_session` 把结果消息**写回会话文件**，不再只在前端临时显示（修复了刷新会话即丢失记录的 bug）。
- **结果反馈双模式（带开关）**：新增配置 `LLMConfig.ai_summarize_actions`（默认 `false`）。
  - 关闭（默认）：`summarize_action_result()` 生成简洁中文要点，**不额外消耗配额**。
  - 开启：`summarize_executions_with_llm()` 调一次 LLM 把结果总结成自然语言（失败自动回退结构化）。
  - 开关 UI 在「系统设置 → AI 模型设置 → AI 助理行为」。
- **轮次收尾兜底**：function-calling 循环达到 `MAX_TOOL_ROUNDS` 时追加收尾提示强制模型用文字作答；模型仍返回空内容则给友好降级提示，不再抛原始错误。

## 10. 后续优化建议（TODO）

- **工具结果摘要**：评论/作品等大列表目前按 `MAX_TOOL_RESULT_CHARS` 截断，可改为结构化摘要，进一步省 token。
- **模型降级**：当 LLM 不支持 function calling 时自动回退到纯对话（含 KB 注入）模式。
- **更多只读工具**：如 `get_live_history`（直播历史）、`get_kb_file_details`（文档切片）等。
- **流式输出**：工具执行期间向前端推送进度，改善长任务（分析/生成）的等待体验。
