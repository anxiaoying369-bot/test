//! 工具定义：暴露给 LLM 的 OpenAI tools[] schema，以及动作工具判定（见 [`super`] 说明）。

use serde_json::{json, Value};

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
