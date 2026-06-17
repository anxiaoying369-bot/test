# AI 助理可调用功能清单（Function-Calling Catalog）

> 目标：把项目里所有 Tauri 命令整理成「应用内 AI 助理（`chat.rs` + `tools.rs`）」可通过对话调用的工具。
> 这是一份**蓝图**——先理后做，逐项决定是否写进 `tool_definitions()` / `dispatch_tool()`。
>
> 现状：`tools.rs` 已暴露 **9 个工具**（6 只读 + 3 分析生成）+ 4 个动作工具。项目实际有 **100+ 个命令**。

## 图例

- **状态**：✅ 已在 `tools.rs` 暴露 ｜ ➕ 建议新增 ｜ ⛔ 不建议暴露（UI/底层/递归）
- **类型**：`只读` 自动执行 ｜ `分析` 自动执行但消耗 API ｜ `动作` 需前端确认（human-in-the-loop）
- **风险**：🟢 安全 ｜ 🟡 写入/耗时/费用 ｜ 🔴 破坏性/外发/控制设备

---

## 1. 企业知识库（Knowledge Base）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `search_knowledge_base` | ✅ | 只读 | 🟢 | `query` | 语义检索知识库片段 |
| `list_kb_documents` | ✅ | 只读 | 🟢 | — | 列出已索引文档 |
| `get_kb_file_details` | ➕ | 只读 | 🟢 | `filename` | 查看单个文档的切片/元数据 |
| `add_document_to_kb` | ✅ | 动作 | 🟡 | `file_path` | 添加本地文件并向量化入库 |
| `delete_kb_file` | ✅ | 动作 | 🔴 | `filename` | 从知识库删除文档 |

后端：`knowledge_base.rs`（`kb_search` 与 `search_kb_internal` 已被 `search_knowledge_base` 复用）。

---

## 2. 账号管理（Accounts）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `list_accounts` | ✅ | 只读 | 🟢 | — | 列出已管理平台账号及状态 |
| `verify_account` | ➕ | 动作 | 🟡 | `platform`,`name` | 校验某账号 Cookie 是否有效 |
| `sync_local_accounts` | ➕ | 动作 | 🟡 | — | 扫描本地 cookie 目录回写账号库 |
| `delete_account` | ➕ | 动作 | 🔴 | `platform`,`name` | 删除账号 |
| 登录流程 `init/get_status/finish/cleanup_login_session` | ⛔ | — | — | — | 交互式扫码登录，需 UI，不适合助理调用 |

后端：`account.rs`。

---

## 3. 数据采集（抖音博主作品 / 评论）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `list_scraped_users` | ✅ | 只读 | 🟢 | — | 列出已采集博主（拿 sec_uid） |
| `query_videos` | ✅ | 只读 | 🟢 | `sec_uid`,`limit?` | 查某博主作品列表 |
| `query_comments` | ✅ | 只读 | 🟢 | `sec_uid`,`aweme_id?`,`limit?` | 查已采集评论 |
| `resolve_user_sec_uid` | ➕ | 只读 | 🟢 | `input`(链接/号) | 把分享链接/抖音号解析成 sec_uid |
| `fetch_douyin_user_info` | ➕ | 分析 | 🟡 | `account_name`,`user_id` | 在线拉取博主主页资料 |
| `start_scrape` | ✅ | 动作 | 🟡 | `account_name`,`platform`,`sec_uid`,`scrape_type`,`limit`,`skip_existing?`,`incremental?` | 启动后台采集任务 |
| `get_scrape_progress` | ➕ | 只读 | 🟢 | `task_id` | 查询采集进度 |
| `cancel_scrape` | ➕ | 动作 | 🟡 | `task_id` | 取消采集任务 |
| `delete_scraped_user` | ➕ | 动作 | 🔴 | `sec_uid` | 删除某博主已采数据 |
| `open_video_in_browser` | ⛔ | — | — | — | 打开浏览器，本地 UI 动作 |

后端：`scraper.rs`。

---

## 4. 用户画像卡片（User Cards）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `list_user_cards` | ➕ | 只读 | 🟢 | — | 列出已保存的用户画像卡 |
| `query_and_save_user` | ➕ | 分析 | 🟡 | `account_name`,`user_id` | 查询并生成/保存画像卡 |
| `refresh_user_card` | ➕ | 动作 | 🟡 | (按实现) | 刷新画像 |
| `delete_user_card` | ➕ | 动作 | 🔴 | (按实现) | 删除画像卡 |

后端：`user_cards.rs`。

---

## 5. 内容创作 / 舆情分析（Studio）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `analyze_comments` | ✅ | 分析 | 🟡 | `comments[]` | AI 舆情分析（情绪/话题/建议） |
| `generate_content` | ➕ | 分析 | 🟡 | `topic`,`material`,`mode`,`platform`,`platform_prompt?` | 通用内容生成（多模式/多平台） |

后端：`studio.rs`（`studio_generate_content` / `studio_analyze_video_comments`）。

---

## 6. 视频工作室（Video Studio）

### 6.1 脚本 / 文案
| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `generate_script` | ✅ | 分析 | 🟡 | `product`,`video_ratio?`,`platform?` | 生成口播/表演脚本（注入知识库） |
| `mpt_generate_terms` | ➕ | 分析 | 🟡 | `video_subject`,`video_script`,`amount?` | 生成素材检索关键词 |

### 6.2 项目 / 素材（管理）
| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `video_list_projects` | ➕ | 只读 | 🟢 | — | 列出视频项目 |
| `video_list_materials` | ➕ | 只读 | 🟢 | `project_id` | 列出项目素材 |
| `video_list_tasks` | ➕ | 只读 | 🟢 | `project_id?` | 列出渲染/生成任务 |
| `video_upsert_project` / `clone` / `delete_project` | ➕ | 动作 | 🟡/🔴 | … | 项目增改删 |
| `video_upload_material` / `delete_material` | ➕ | 动作 | 🟡 | … | 素材增删 |

### 6.3 生成 / 合成（耗费 API/算力）
| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `synthesize_speech` | ✅ | 动作 | 🟡 | `project_id`,`text`,`voice_id`,`speed?` | TTS 合成入项目 |
| `tts_list_voices` | ➕ | 只读 | 🟢 | (provider) | 列出可用音色 |
| `video_mpt_preview_voice` | ➕ | 分析 | 🟡 | `voice_name` | 试听音色 |
| `video_mpt_generate` | ➕ | 动作 | 🟡 | `project_id`,`params` | MoneyPrinterTurbo 全流程出片 |
| `video_start_generation` | ➕ | 动作 | 🟡 | `project_id`,`prompt`,`provider`,`mode`,`ratio`,… | AI 视频生成（含 `api_key`，建议后端注入而非让 LLM 传） |
| `video_poll_task_status` | ➕ | 只读 | 🟢 | `task_id` | 查询生成任务状态 |

### 6.4 底层渲染（不建议暴露给助理）
`video_run_ffmpeg` / `video_concat_materials` / `video_render_advanced` / `video_get_metadata` / `video_test_ffmpeg` —— ⛔ 属底层 ffmpeg 管线，参数复杂且易误用，由 UI/上层工具编排，不直接给 LLM。

后端：`video_studio/{generation,mpt,project,material,rendering,tasks}.rs`。

---

## 7. 直播监控（Live Monitor）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `resolve_live_url` | ➕ | 只读 | 🟢 | `url` | 直播链接 → room_id |
| `get_active_monitors` | ➕ | 只读 | 🟢 | — | 当前监控中的直播间 |
| `get_live_history` | ➕ | 只读 | 🟢 | `room_id` | 某直播间历史弹幕/记录 |
| `generate_live_reply` | ➕ | 分析 | 🟡 | `user_name`,`content` | 为弹幕生成回复话术 |
| `start_live_monitor` | ➕ | 动作 | 🟡 | `room_id`,`account_name` | 启动直播间监控 |
| `stop_live_monitor` | ➕ | 动作 | 🟡 | `room_id` | 停止监控 |

后端：`live_monitor.rs`。

---

## 8. 微信监控（WeChat）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `wechat_list_sessions` | ➕ | 只读 | 🟢 | — | 列出会话 |
| `wechat_list_contacts` | ➕ | 只读 | 🟢 | — | 列出联系人 |
| `wechat_get_messages` | ➕ | 只读 | 🟡 | `session_id`,`limit?`,`offset?` | 读取聊天记录（隐私） |
| `wechat_get_status` | ➕ | 只读 | 🟢 | — | 监控运行状态 |
| `wechat_transcribe_voice` | ➕ | 分析 | 🟡 | (voice) | 语音转写 |
| `wechat_start_monitor` | ➕ | 动作 | 🟡 | `targets[]`,`interval_secs?` | 启动新消息监控 |
| `wechat_stop_monitor` | ➕ | 动作 | 🟡 | — | 停止监控 |
| `wechat_get_key/save/load_credentials`,`open`,`download_stt_model`,`get_media/image/voice` | ⛔ | — | 🔴 | — | 密钥/媒体/底层桥接，涉敏感数据，不暴露 |

> ⚠️ 微信聊天涉及强隐私，建议即便暴露只读工具也加一道确认或在系统提示里限定用途。
后端：`wechat.rs`（详见记忆 `wechat-monitor-feature`）。

---

## 9. 手机无线控制（Mobile）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `mobile_list_devices` | ➕ | 只读 | 🟢 | — | 列出在线设备 |
| `mobile_get_server_info` | ➕ | 只读 | 🟢 | — | WS 服务地址/端口 |
| `mobile_list_recordings` | ➕ | 只读 | 🟢 | `device_id?` | 列出录音/通话记录 |
| `mobile_request_screenshot` | ➕ | 动作 | 🟡 | `device_id` | 请求实时截图 |
| `mobile_set_device_remark` | ➕ | 动作 | 🟢 | `device_id`,`remark` | 设备备注 |
| `mobile_tap` / `mobile_swipe` / `mobile_key` | ➕ | 动作 | 🔴 | `device_id`,坐标/键 | 远程触控/按键，**强烈建议每次确认** |
| `mobile_delete_device` / `mobile_delete_recording` | ➕ | 动作 | 🔴 | … | 删除 |

后端：`mobile.rs`（详见记忆 `mobile-control-feature`）。

---

## 10. GEO 监控

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `geo_query` | ✅ | 只读 | 🟢 | `query` | GEO 监控查询（结合知识库） |

后端：`geo.rs`。

---

## 11. 系统 / 诊断（可选暴露）

| 工具名 | 状态 | 类型 | 风险 | 参数 | 说明 |
|---|---|---|---|---|---|
| `autocast_diagnostics` | ➕ | 只读 | 🟢 | — | 一键自检（依赖/网关/配置） |
| Hermes 网关管理（`start/stop/status/health/skills/tools…`） | ⛔ | — | — | — | 基础设施 meta，由 UI 管理，不递归给助理 |
| `get_config/save_config`,`open_file_in_finder` | ⛔ | — | — | — | 配置/UI 管道 |
| 聊天会话 CRUD（`chat.rs`） | ⛔ | — | — | — | 助理自身载体，递归无意义 |

---

## 汇总：建议纳入助理的工具规模

- **已暴露**：9（+4 动作）
- **建议新增（只读/分析，自动执行）**：约 22 个 —— 见各表 ➕ 且类型为 `只读`/`分析`
- **建议新增（动作，需确认）**：约 20 个 —— 各表 ➕ 且类型为 `动作`
- **明确不暴露**：登录流程、底层 ffmpeg、微信密钥/媒体、Hermes meta、配置管道、聊天 CRUD

## 落地建议（待你确认范围后实施）

1. **分批扩充**，按风险递增：先把 §1–§5、§10 的只读/分析工具补进 `tool_definitions()` 与 `dispatch_tool()`（低风险、高频）。
2. **动作工具**统一走现有 human-in-the-loop：新增的写入/控制类工具加入 `is_action_tool()` 白名单，并在 `summarize_action_result()` 补中文反馈。
3. **敏感参数**（如 `api_key`、provider key、设备坐标）不要让 LLM 自由生成——由 Rust 侧从配置注入，工具 schema 里不暴露。
4. **隐私域**（微信、手机控制）建议默认关闭，按系统提示/开关位决定是否启用。
