use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

// ============ 状态管理 ============
pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
    pub process_handles: Mutex<std::collections::HashMap<String, tokio::process::Child>>,
    pub current_task_id: Mutex<Option<String>>,
}

pub struct LoginFlow {
    pub port: u16,
    pub platform: String,
    pub status: String,
    pub qrcode_base64: Option<String>,
    pub user_name: Option<String>,
    pub user_id: Option<String>,
    pub cookie_data: Option<CookieData>,
    pub error_msg: Option<String>,
}

// ============ 数据结构 ============

#[derive(Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CookieData {
    pub cookies: Vec<CookieEntry>,
    pub origins: Option<Vec<CookieOrigin>>,
    pub user_info: Option<UserInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: Option<String>,
    pub expires: Option<f64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CookieOrigin {
    pub origin: String,
    pub local_storage: Option<Vec<LocalStorageEntry>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalStorageEntry {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct GeoModelConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct GeoPublishPlatform {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
}

fn default_true() -> bool { true }

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LLMConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub kb_api_key: String,
    #[serde(default)]
    pub kb_base_url: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    #[serde(default = "default_analysis_prompt")]
    pub analysis_prompt: String,
    #[serde(default = "default_live_reply_prompt")]
    pub live_reply_prompt: String,
    #[serde(default = "default_im_reply_prompt")]
    pub im_reply_prompt: String,
    #[serde(default)]
    pub live_theme: String,
    #[serde(default)]
    pub live_content: String,
    #[serde(default)]
    pub geo_models: Vec<GeoModelConfig>,
    #[serde(default)]
    pub geo_publish_platforms: Vec<GeoPublishPlatform>,
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_im_reply_prompt() -> String {
    "你是一位专业的客户经理。请根据用户的私信内容和提供的企业背景知识，给出一个专业、礼貌且简洁的回复建议。回复应直接面向用户，语气真诚。".to_string()
}

fn default_live_reply_prompt() -> String {
    "你是一位正在直播的主播。请根据直播主题和直播内容，简短地回复用户的弹幕。回复必须非常简短（20字以内），语气亲切自然，像真人在直播间说话一样。".to_string()
}

fn default_analysis_prompt() -> String {
    "你是一位资深的社交媒体数据分析师。我会为你提供一组短视频评论数据，请从以下几个维度进行深度分析：\n1. 舆情氛围：整体情绪倾向（积极、消极、中立）及其占比。\n2. 核心热点：用户最关心的前3个话题或痛点。\n3. 用户意图：是否存在高潜力的咨询、购买意向或反馈建议。\n4. 互动建议：针对当前评论区，建议运营人员如何进行回复或引导。\n请用专业且简洁的 Markdown 格式输出分析报告。".to_string()
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub llm: LLMConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    /// 本条消息中 AI 调用的创作工具名称（如 "generate_content"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_used: Option<String>,
    /// 工具返回的结构化数据，用于前端富文本渲染
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_data: Option<serde_json::Value>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

// 持久化存储结构
#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub platform: String,
    pub user_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub nickname: Option<String>,
    pub status: String,
    pub cookie_file: String,
    pub avatar: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// 前端返回结构（含 meta 嵌套）
#[derive(Clone, Serialize)]
pub struct AccountMeta {
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct AccountView {
    pub id: String,
    pub platform: String,
    pub name: String,
    pub status: String,
    pub meta: AccountMeta,
}

fn account_to_view(a: &Account) -> AccountView {
    AccountView {
        id: a.id.clone(),
        platform: a.platform.clone(),
        name: a.name.clone(),
        status: a.status.clone(),
        meta: AccountMeta {
            user_id: a.user_id.clone(),
            nickname: a.nickname.clone(),
            avatar: a.avatar.clone(),
        },
    }
}

#[derive(Serialize)]
pub struct VerifyResult {
    pub status: String,
    pub method: String,
    pub message: String,
}

// ============ 评论采集相关结构 ============

#[derive(Serialize, Deserialize, Clone)]
pub struct ScraperTask {
    pub task_id: String,
    pub account_name: String,
    pub platform: String,
    pub sec_uid: String,
    pub scrape_type: String,
    pub limit: i32,
    pub skip_existing: bool,
    pub status: String,       // running | completed | error | cookie_expired | cancelled
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScraperProgress {
    pub task_id: String,
    pub status: String,
    pub progress: i32,
    pub total: i32,
    pub current_type: String,
    pub current_user: String,
    pub stats: serde_json::Value,
    pub log_lines: Vec<String>,
    pub started_at: f64,
    pub finished_at: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct AccountsStoreFile {
    accounts: Vec<Account>,
}

impl Default for AccountsStoreFile {
    fn default() -> Self {
        Self { accounts: Vec::new() }
    }
}

// ============ Python HTTP Server 响应结构 ============

#[derive(Deserialize)]
struct PyLoginStatus {
    status: String,
    qrcode_base64: Option<String>,
    user_name: Option<String>,
    user_id: Option<String>,
}

#[derive(Serialize)]
pub struct LoginSession {
    pub session_id: String,
    pub platform: String,
    pub status: String,
    pub user_info: Option<UserInfo>,
    pub cookie_data: Option<CookieData>,
    pub qrcode_base64: Option<String>,
}

// ============ 数据存储路径 ============

fn get_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("AutoCastAI")
}

/// 跨平台 Python 可执行文件路径。
/// 优先使用项目根目录下的 `.venv`，如果不存在则回退到系统 Python。
fn python_executable() -> String {
    let venv_path = if cfg!(windows) {
        PathBuf::from("..").join(".venv").join("Scripts").join("python.exe")
    } else {
        PathBuf::from("..").join(".venv").join("bin").join("python3")
    };

    if venv_path.exists() {
        venv_path.to_string_lossy().to_string()
    } else if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}

/// 创建已预置 AUTOCAST_DATA_DIR 环境变量的 tokio Python 子进程 Command。
/// Python 脚本通过 compat.get_data_dir() 优先读取该变量，确保路径与 Rust 端严格一致。
fn python_cmd() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(python_executable());
    cmd.env("AUTOCAST_DATA_DIR", get_data_dir().to_string_lossy().to_string());
    cmd
}

/// 同步版（std::process）Python Command，用于需要阻塞等待输出的场景。
fn python_cmd_sync() -> std::process::Command {
    let mut cmd = std::process::Command::new(python_executable());
    cmd.env("AUTOCAST_DATA_DIR", get_data_dir().to_string_lossy().to_string());
    cmd
}

fn get_accounts_db_path() -> PathBuf {
    get_data_dir().join("accounts.json")
}

fn get_cookies_dir() -> PathBuf {
    get_data_dir().join("cookies")
}

fn get_account_dir(platform: &str, account_name: &str) -> PathBuf {
    get_cookies_dir().join(platform).join(account_name)
}

fn load_accounts() -> AccountsStoreFile {
    let path = get_accounts_db_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AccountsStoreFile::default()
    }
}

fn save_accounts(store: &AccountsStoreFile) -> Result<(), String> {
    let path = get_accounts_db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

// 从磁盘的 meta.json 读用户信息
fn read_meta_json(platform: &str, name: &str) -> Option<UserInfo> {
    let path = get_account_dir(platform, name).join("meta.json");
    let content = fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let ui = v.get("user_info")?;
    Some(UserInfo {
        user_id: ui.get("user_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        name: ui.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        avatar: ui.get("avatar").and_then(|x| x.as_str()).map(|s| s.to_string()),
    })
}

// 注册账号到 accounts.json（cookie 文件已由 Python 写好）
fn register_account_on_disk(platform: &str, name: &str) -> Result<Account, String> {
    let user_info = read_meta_json(platform, name);
    let user_id = user_info.as_ref().map(|u| u.user_id.clone()).filter(|s| !s.is_empty());
    let nickname = user_info.as_ref().map(|u| u.name.clone()).filter(|s| !s.is_empty());
    let avatar = user_info.as_ref().and_then(|u| u.avatar.clone());

    let now = chrono_now();
    let account = Account {
        id: Uuid::new_v4().to_string(),
        platform: platform.to_string(),
        user_id,
        name: name.to_string(),
        nickname,
        status: "active".to_string(),
        cookie_file: name.to_string(),  // 现在存的是目录名（=account_name）
        avatar,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut store = load_accounts();
    store.accounts.retain(|a| !(a.platform == platform && a.name == name));
    store.accounts.push(account.clone());
    save_accounts(&store)?;
    Ok(account)
}

fn get_chats_dir() -> PathBuf {
    get_data_dir().join("chats")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
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
async fn create_chat_session(title: String) -> Result<ChatSession, String> {
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
async fn delete_chat_session(id: String) -> Result<(), String> {
    let path = get_chats_dir().join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn generate_im_reply(messages: Vec<serde_json::Value>) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    if messages.is_empty() {
        return Err("对话记录为空".to_string());
    }

    let system_prompt = if config.llm.im_reply_prompt.is_empty() {
        default_im_reply_prompt()
    } else {
        config.llm.im_reply_prompt.clone()
    };

    // 获取最后一条用户的消息内容用于知识库搜索
    let last_user_content = messages.iter().rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap_or("");

    // 1. 从知识库检索相关背景
    let kb_context = if !last_user_content.is_empty() {
        match search_kb_internal(last_user_content.to_string()).await {
            Ok(res_str) => {
                let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
                let mut context_text = String::from("\n相关背景知识参考：\n");
                if let Some(arr) = res.as_array() {
                    for item in arr.iter().take(5) {
                        if let Some(text) = item["text"].as_str() {
                            context_text.push_str(&format!("- {}\n", text));
                        }
                    }
                }
                if context_text.len() < 20 { String::new() } else { context_text }
            },
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    // 构建完整消息列表：System (Prompt + KB) + History
    let mut api_messages = vec![
        serde_json::json!({ 
            "role": "system", 
            "content": format!("{}\n\n{}", system_prompt, kb_context) 
        })
    ];
    
    // 添加历史记录
    for m in messages {
        api_messages.push(serde_json::json!({
            "role": m["role"],
            "content": m["content"]
        }));
    }

    // 显式要求生成回复
    api_messages.push(serde_json::json!({
        "role": "user",
        "content": "请根据以上对话历史和参考知识，为我（assistant）生成一段专业、得体的回复。只需要输出回复内容本身。"
    }));

    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": api_messages,
        "temperature": 0.7
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API 错误: {}", err_text));
    }

    let resp_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let reply = resp_data["choices"][0]["message"]["content"]
        .as_str().ok_or("LLM 返回格式错误")?.trim().to_string();

    Ok(reply)
}

#[tauri::command]
async fn generate_live_reply(user_name: String, content: String) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    let system_prompt = if config.llm.live_reply_prompt.is_empty() {
        default_live_reply_prompt()
    } else {
        config.llm.live_reply_prompt.clone()
    };

    // 1. 从知识库搜索相关背景
    let kb_context = match search_kb_internal(content.clone()).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut context_text = String::from("\n相关背景知识：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(3) {
                    if let Some(text) = item["text"].as_str() {
                        context_text.push_str(&format!("- {}\n", text));
                    }
                }
            }
            if context_text.len() < 20 { String::new() } else { context_text }
        },
        Err(_) => String::new(),
    };

    let user_context = format!(
        "直播主题：{}\n直播内容：{}\n{}\n\n请回复用户 {} 的弹幕：{}",
        config.llm.live_theme,
        config.llm.live_content,
        kb_context,
        user_name,
        content
    );

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_context }
        ],
        "temperature": 0.8,
        "max_tokens": 50
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API 错误: {}", err_text));
    }

    let resp_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let reply = resp_data["choices"][0]["message"]["content"]
        .as_str().ok_or("LLM 返回格式错误")?.trim().to_string();

    Ok(reply)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn delete_scraped_user(secUid: String) -> Result<(), String> {
    println!("[Backend] Deleting user data for sec_uid: {}", secUid);
    let path = get_data_dir().join("scraper_data").join(&secUid);
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        println!("[Backend] User data deleted successfully");
    } else {
        println!("[Backend] User data path not found: {:?}", path);
    }
    Ok(())
}

#[tauri::command]
async fn analyze_comments(comments: Vec<serde_json::Value>) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    if comments.is_empty() {
        return Err("没有可分析的评论内容".to_string());
    }

    // 提取评论文本并格式化，限制长度防止超长
    let mut text_to_analyze = String::new();
    for (idx, c) in comments.iter().take(50).enumerate() {
        let content = c["text"].as_str().unwrap_or("");
        if !content.is_empty() {
            text_to_analyze.push_str(&format!("{}. {}\n", idx + 1, content));
        }
    }

    let system_prompt = if config.llm.analysis_prompt.is_empty() {
        default_analysis_prompt()
    } else {
        config.llm.analysis_prompt.clone()
    };

    // 1. 从知识库搜索企业相关背景（基于前几条评论或关键词）
    let query_for_kb = comments.get(0).and_then(|c| c["text"].as_str()).unwrap_or("产品评价").to_string();
    let kb_context = match search_kb_internal(query_for_kb).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut context_text = String::from("\n相关背景/产品手册知识：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(5) {
                    if let Some(text) = item["text"].as_str() {
                        context_text.push_str(&format!("- {}\n", text));
                    }
                }
            }
            if context_text.len() < 20 { String::new() } else { context_text }
        },
        Err(_) => String::new(),
    };

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": format!("{}\n\n以下是与当前内容相关的企业知识库信息作为参考：\n{}", system_prompt, kb_context) },
            { "role": "user", "content": format!("请分析以下评论：\n\n{}", text_to_analyze) }
        ],
        "temperature": 0.7
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API 错误: {}", err_text));
    }

    let resp_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let report = resp_data["choices"][0]["message"]["content"]
        .as_str().ok_or("LLM 返回格式错误")?.to_string();

    Ok(report)
}

#[tauri::command]
async fn send_chat_message(
    session_id: String,
    content: String,
    state: State<'_, AppState>,
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
        return Err("回话不存在".to_string());
    };

    // 添加用户消息
    let user_msg = ChatMessage {
        role: "user".to_string(),
        content: content.clone(),
        timestamp: now_secs(),
        tool_used: None,
        tool_data: None,
    };
    session.messages.push(user_msg);

    // 定义工具
    let tools = serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "list_accounts",
                "description": "获取当前已登录的账号列表",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "platform": { "type": "string", "description": "平台名称，如 douyin" }
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_scrape",
                "description": "启动评论或视频采集任务。调用前必须先调用 get_scrape_status 确认当前无任务在运行。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "执行任务的账号名称" },
                        "platform": { "type": "string", "description": "平台，目前仅支持 douyin" },
                        "sec_uid": { "type": "string", "description": "目标用户的 sec_uid" },
                        "scrape_type": { "type": "string", "enum": ["video", "comment", "reply", "all"], "description": "采集类型" },
                        "limit": { "type": "integer", "description": "采集数量限制" }
                    },
                    "required": ["account_name", "platform", "sec_uid", "scrape_type"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_scrape_status",
                "description": "查询当前是否有采集任务在运行，以及任务的实时进度。在调用 start_scrape 之前应先调用此工具确认无任务在运行。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_scraped_users",
                "description": "列出已采集数据的用户信息",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "send_im_message",
                "description": "向指定用户发送私信",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "account_name": { "type": "string", "description": "发送者的账号名称" },
                        "to_user_id": { "type": "string", "description": "接收者的 UID" },
                        "content": { "type": "string", "description": "私信内容" }
                    },
                    "required": ["account_name", "to_user_id", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_knowledge_base",
                "description": "从本地知识库中搜索相关背景知识、产品说明、企业规则等。在回答用户专业问题或背景知识时应优先使用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "搜索关键词或问题短语" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "generate_content",
                "description": "调用 AI 创作中心为指定平台（抖音/微信/知乎）生成 GEO 优化的内容文章，或对已有内容进行 GEO 深度改造重写。完成后同步输出舆情及 GEO 评分报告。当用户要求写文案、创作文章、内容改写时必须调用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "创作主题（全新创作时填写）" },
                        "material": { "type": "string", "description": "参考素材或待改造的原始内容（改造模式时填写）" },
                        "mode": { "type": "string", "enum": ["new", "rewrite"], "description": "new=全新创作，rewrite=改造已有内容" },
                        "platform": { "type": "string", "enum": ["douyin", "wechat", "zhihu"], "description": "目标发布平台" }
                    },
                    "required": ["mode", "platform"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "audit_content",
                "description": "对已有内容进行 GEO 评分和舆情分析，返回改进建议报告。当用户要求审核、分析、评估、优化已有内容时调用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "待审核的内容文本" }
                    },
                    "required": ["content"]
                }
            }
        }
    ]);

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    // 系统提示：约束 AI 助理在工具调用场景下的回复风格
    let system_prompt = serde_json::json!({
        "role": "system",
        "content": "你是 AutoCast AI 助理，帮助用户管理抖音账号、采集数据、创作内容。\n\
        \n\
        【采集任务行为规范】\n\
        - 调用 start_scrape 之前，必须先调用 get_scrape_status 确认当前没有任务在运行。\n\
        - 如果 get_scrape_status 返回有任务在运行，告知用户等待或去「评论采集」页面取消，不要再次启动。\n\
        - start_scrape 启动成功后，工具会返回以 BACKGROUND_TASK_STARTED: 开头的消息。\n\
          此时立即用简短友好的语言告知用户：「已在后台开始采集，请切换到「评论采集」页面查看实时进度和结果」。\n\
          不要等待、不要追问，直接给出这个提示即可。\n\
        \n\
        【账号与 sec_uid 规范】\n\
        - 调用 start_scrape 前必须先调用 list_accounts 确认可用账号名称（account_name 必须完全一致）。\n\
        - 如果用户没有提供 sec_uid，主动询问并说明可从抖音主页链接获取。\n\
        \n\
        【回复风格】简洁、直接，不要重复罗列参数和技术细节。"
    });

    let mut current_messages: Vec<serde_json::Value> = vec![system_prompt];
    current_messages.extend(session.messages.iter().map(|m| {
        serde_json::json!({
            "role": m.role,
            "content": m.content
        })
    }));

    // 循环处理工具调用（支持多轮调用）
    let mut max_iterations = 5;
    let mut final_assistant_msg: Option<ChatMessage> = None;
    // 追踪创作工具调用结果，用于前端富文本渲染
    let mut studio_tool_used: Option<String> = None;
    let mut studio_tool_data: Option<serde_json::Value> = None;

    while max_iterations > 0 {
        max_iterations -= 1;

        let payload = serde_json::json!({
            "model": config.llm.model,
            "messages": current_messages,
            "tools": tools,
            "tool_choice": "auto"
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", config.llm.api_key))
            .json(&payload)
            .send().await.map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(format!("LLM API 错误: {}", err_text));
        }

        let resp_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let choice = &resp_data["choices"][0];
        let message = &choice["message"];

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            // 将助手的工具调用消息加入上下文
            current_messages.push(message.clone());

            for tool_call in tool_calls {
                let call_id = tool_call["id"].as_str().unwrap_or_default();
                let func_name = tool_call["function"]["name"].as_str().unwrap_or_default();
                let func_args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let func_args: serde_json::Value = serde_json::from_str(func_args_str).unwrap_or_default();

                println!("[Tool Call] {} with args: {}", func_name, func_args_str);

                // 执行工具逻辑
                let tool_result = match func_name {
                    "list_accounts" => {
                        let platform = func_args["platform"].as_str().map(|s| s.to_string());
                        let res = list_accounts(platform).await?;
                        serde_json::to_string(&res).unwrap_or_default()
                    },
                    "get_scrape_status" => {
                        let current_task = {
                            let current = state.current_task_id.lock().unwrap();
                            current.clone()
                        };
                        match current_task {
                            None => "当前没有采集任务在运行，可以安全地启动新任务。".to_string(),
                            Some(task_id) => {
                                let progress_path = get_scraper_dir().join(format!("{}.json", &task_id));
                                if progress_path.exists() {
                                    if let Ok(content) = fs::read_to_string(&progress_path) {
                                        if let Ok(p) = serde_json::from_str::<serde_json::Value>(&content) {
                                            let status = p["status"].as_str().unwrap_or("unknown");
                                            let progress_pct = p["progress"].as_i64().unwrap_or(0);
                                            let current_type = p["current_type"].as_str().unwrap_or("");
                                            format!("当前有采集任务正在运行中。状态：{}，进度：{}%，当前阶段：{}。请等待任务完成，或提示用户去「评论采集」页面手动取消后再尝试。", status, progress_pct, current_type)
                                        } else {
                                            format!("有任务正在运行（ID：{}），暂时无法读取进度。", &task_id[..8])
                                        }
                                    } else {
                                        format!("有任务正在运行（ID：{}）。", &task_id[..8])
                                    }
                                } else {
                                    "有采集任务 ID 记录但进度文件尚未生成，任务可能刚刚启动。".to_string()
                                }
                            }
                        }
                    },
                    "start_scrape" => {
                        let acc = func_args["account_name"].as_str().unwrap_or_default().to_string();
                        let plat = func_args["platform"].as_str().unwrap_or("douyin").to_string();
                        let uid = func_args["sec_uid"].as_str().unwrap_or_default().to_string();
                        let stype = func_args["scrape_type"].as_str().unwrap_or("comment").to_string();
                        let lim = func_args["limit"].as_i64().unwrap_or(0) as i32;

                        // 启动采集任务后立即返回，任务在后台运行
                        match start_scrape(acc, plat, uid, stype.clone(), lim, true, true, true, state.clone()).await {
                            Ok(task) => {
                                let limit_desc = if lim > 0 { format!("限制 {} 条", lim) } else { "不限数量".to_string() };
                                format!("BACKGROUND_TASK_STARTED:采集任务已在后台启动。类型：{}，{}。任务 ID：{}。请告知用户去「评论采集」页面查看实时进度和最终数据。",
                                    stype, limit_desc, &task.task_id[..8])
                            }
                            Err(e) => format!("采集任务启动失败：{}", e)
                        }
                    },
                    "list_scraped_users" => {
                        let res = list_scraped_users().await?;
                        serde_json::to_string(&res).unwrap_or_default()
                    },
                    "send_im_message" => {
                        let acc = func_args["account_name"].as_str().unwrap_or_default().to_string();
                        let to = func_args["to_user_id"].as_str().unwrap_or_default().to_string();
                        let msg = func_args["content"].as_str().unwrap_or_default().to_string();
                        
                        let res = douyin_im_send(acc, msg, Some(to), None, None, None, None, None, None).await?;
                        serde_json::to_string(&res).unwrap_or_default()
                    },
                    "search_knowledge_base" => {
                        let query = func_args["query"].as_str().unwrap_or_default().to_string();
                        search_kb_internal(query).await?
                    },
                    "generate_content" => {
                        let topic    = func_args["topic"].as_str().unwrap_or_default().to_string();
                        let material = func_args["material"].as_str().unwrap_or_default().to_string();
                        let mode     = func_args["mode"].as_str().unwrap_or("new").to_string();
                        let platform = func_args["platform"].as_str().unwrap_or("douyin").to_string();
                        match studio_generate_internal(topic.clone(), material, mode, platform.clone()).await {
                            Ok(val) => {
                                let char_count = val["content"].as_str().unwrap_or("").chars().count();
                                studio_tool_used = Some("generate_content".to_string());
                                studio_tool_data = Some(serde_json::json!({
                                    "content":  val["content"],
                                    "audit":    val["audit"],
                                    "platform": platform,
                                    "topic":    topic,
                                }));
                                format!("✅ 内容已生成完毕。平台：{}，主题：「{}」，共约 {} 字。内容和 GEO 评估报告已在对话气泡中以卡片形式展示，用户可直接复制使用。", platform, topic, char_count)
                            }
                            Err(e) => format!("❌ 内容生成失败: {}", e),
                        }
                    },
                    "audit_content" => {
                        let content = func_args["content"].as_str().unwrap_or_default().to_string();
                        match audit_content_internal(content).await {
                            Ok(audit) => {
                                studio_tool_used = Some("audit_content".to_string());
                                studio_tool_data = Some(serde_json::json!({ "audit": audit }));
                                "✅ 内容审计完成，GEO 评估报告已在对话气泡中展示。".to_string()
                            }
                            Err(e) => format!("❌ 审计失败: {}", e),
                        }
                    },
                    _ => format!("未知工具: {}", func_name)
                };

                // 将工具结果加入上下文
                current_messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "name": func_name,
                    "content": tool_result
                }));
            }
            // 继续循环，让 LLM 根据工具结果生成下一条消息
            continue;
        } else {
            // 没有工具调用，获取最终内容
            let assistant_content = message["content"].as_str().unwrap_or_default().to_string();
            let assistant_msg = ChatMessage {
                role: "assistant".to_string(),
                content: assistant_content,
                timestamp: now_secs(),
                tool_used: studio_tool_used.take(),
                tool_data: studio_tool_data.take(),
            };
            final_assistant_msg = Some(assistant_msg);
            break;
        }
    }

    let assistant_msg = final_assistant_msg.ok_or("LLM 未返回有效回复")?;
    session.messages.push(assistant_msg.clone());
    session.updated_at = now_secs();

    // 自动更新标题
    if session.title == "新对话" && session.messages.len() >= 2 {
        let first_user_content = session.messages.iter()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "新对话".to_string());
        session.title = first_user_content.chars().take(20).collect::<String>();
        if first_user_content.len() > 20 {
            session.title.push_str("...");
        }
    }

    // 保存回话
    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(assistant_msg)
}

#[tauri::command]
async fn get_chat_messages(session_id: String) -> Result<Vec<ChatMessage>, String> {
    let path = get_chats_dir().join(format!("{}.json", session_id));
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let session: ChatSession = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(session.messages)
}

#[tauri::command]
async fn get_default_config() -> Result<AppConfig, String> {
    Ok(AppConfig {
        llm: LLMConfig {
            api_key: "".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            kb_api_key: "".to_string(),
            kb_base_url: "https://api.openai.com/v1".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            analysis_prompt: default_analysis_prompt(),
            live_reply_prompt: default_live_reply_prompt(),
            im_reply_prompt: default_im_reply_prompt(),
            live_theme: "".to_string(),
            live_content: "".to_string(),
            geo_models: vec![],
            geo_publish_platforms: vec![],
        }
    })
}

#[tauri::command]
async fn get_config() -> Result<AppConfig, String> {
    let path = get_data_dir().join("config.json");
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig::default())
    }
}

#[tauri::command]
async fn save_config(config: AppConfig) -> Result<(), String> {
    let path = get_data_dir().join("config.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

// ============ Tauri 命令 ============

#[tauri::command]
async fn list_accounts(platform: Option<String>) -> Result<Vec<AccountView>, String> {
    let _ = sync_local_accounts().await;
    let store = load_accounts();
    let views = match platform {
        Some(p) => store.accounts.iter().filter(|a| a.platform == p).map(account_to_view).collect(),
        None => store.accounts.iter().map(account_to_view).collect(),
    };
    Ok(views)
}

// 扫描 cookies/{platform}/*/cookie.txt，把磁盘上存在但 accounts.json 没记录的账号补回
#[tauri::command]
async fn sync_local_accounts() -> Result<usize, String> {
    let platforms = vec!["douyin"];
    let mut sync_count = 0;

    for platform in platforms {
        let dir = get_cookies_dir().join(platform);
        if !dir.exists() { continue; }

        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let store = load_accounts();
        let known: std::collections::HashSet<String> = store.accounts.iter()
            .filter(|a| a.platform == platform)
            .map(|a| a.name.clone())
            .collect();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let cookie_file = path.join("cookie.txt");
            if !cookie_file.exists() { continue; }
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if known.contains(&name) { continue; }
            register_account_on_disk(platform, &name)?;
            sync_count += 1;
        }
    }
    Ok(sync_count)
}

#[tauri::command]
async fn init_login_session(platform: String, state: State<'_, AppState>) -> Result<LoginSession, String> {
    let session_id = Uuid::new_v4().to_string();
    let port: u16 = 18000 + (Uuid::new_v4().as_u128() as u16 % 10000);

    let script_name = match platform.as_str() {
        "douyin" => "douyin_login.py",
        _ => return Err("不支持的平台".to_string()),
    };

    let script_path = PathBuf::from("..").join("scripts").join(script_name);

    // 把 Python 子进程的 stdout/stderr 重定向到日志文件，方便排查
    let log_dir = get_data_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log_path = log_dir.join(format!("login_{}_{}.log", platform, &session_id[..8]));
    let log_file = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let stderr_file = log_file.try_clone().map_err(|e| e.to_string())?;

    let child = python_cmd()
        .arg(&script_path).arg("--port").arg(port.to_string()).arg("--session-id").arg(&session_id)
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(stderr_file))
        .kill_on_drop(true)
        .spawn().map_err(|e| e.to_string())?;

    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.insert(session_id.clone(), child);
    }

    let flow = LoginFlow {
        port, platform: platform.clone(), status: "pending".to_string(),
        qrcode_base64: None, user_name: None, user_id: None, cookie_data: None, error_msg: None,
    };

    {
        let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        flows.insert(session_id.clone(), flow);
    }

    Ok(LoginSession { session_id, platform, status: "pending".to_string(), user_info: None, cookie_data: None, qrcode_base64: None })
}

#[tauri::command]
async fn get_login_status(session_id: String, state: State<'_, AppState>) -> Result<LoginSession, String> {
    let (port, platform) = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        let flow = flows.get(&session_id).ok_or("Session not found")?;
        (flow.port, flow.platform.clone())
    };

    let client = reqwest::Client::new();
    let py_status: PyLoginStatus = client.get(format!("http://127.0.0.1:{}/status", port))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    Ok(LoginSession {
        session_id, platform, status: py_status.status,
        user_info: py_status.user_name.map(|name| UserInfo { user_id: py_status.user_id.unwrap_or_default(), name, avatar: None }),
        cookie_data: None, qrcode_base64: py_status.qrcode_base64,
    })
}

#[derive(Deserialize)]
struct PyFinishResult {
    ok: bool,
    error: Option<String>,
    user_info: Option<UserInfo>,
}

// 用户点"登录完成"时调用：通知 Python 抓 cookie 写文件，然后注册账号
#[tauri::command]
async fn finish_login(
    session_id: String,
    account_name: String,
    state: State<'_, AppState>,
) -> Result<AccountView, String> {
    if account_name.trim().is_empty() {
        return Err("账号名称不能为空".to_string());
    }
    // 路径里的非法字符（防 traversal）
    if account_name.contains('/') || account_name.contains('\\') || account_name.contains("..") {
        return Err("账号名称不能包含 / \\ ..".to_string());
    }

    let (port, platform) = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        let flow = flows.get(&session_id).ok_or("Session not found")?;
        (flow.port, flow.platform.clone())
    };

    let save_dir = get_account_dir(&platform, &account_name);
    fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;
    let save_dir_str = save_dir.to_string_lossy().to_string();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{}/finish", port))
        .query(&[("save_dir", &save_dir_str)])
        .send().await.map_err(|e| e.to_string())?;

    let body: PyFinishResult = resp.json().await.map_err(|e| e.to_string())?;
    if !body.ok {
        return Err(body.error.unwrap_or_else(|| "finish 失败".to_string()));
    }

    // 此时 Python 已写好 cookie.txt/cookie.json/meta.json，直接注册
    let account = register_account_on_disk(&platform, &account_name)?;
    let _ = body.user_info;  // 已存进 meta.json，这里只是文档化
    Ok(account_to_view(&account))
}

#[tauri::command]
async fn verify_account(platform: String, name: String) -> Result<VerifyResult, String> {
    let store = load_accounts();
    let _account = store.accounts.iter()
        .find(|a| a.platform == platform && a.name == name)
        .ok_or_else(|| "账号不存在".to_string())?;

    let cookie_json = get_account_dir(&platform, &name).join("cookie.json");
    let script_path = PathBuf::from("..").join("scripts").join("verify_account.py");

    let output = python_cmd()
        .arg(&script_path)
        .arg(&platform)
        .arg(&cookie_json)
        .output().await.map_err(|e| e.to_string())?;

    // 把 Python stderr 日志打印到 Rust 控制台（npm run tauri dev 可见）
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    if !stderr_str.is_empty() {
        eprintln!("[verify_account] Python stderr:\n{}", stderr_str);
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("验证结果解析失败: {}", result_str))?;

    Ok(VerifyResult {
        status: result["status"].as_str().unwrap_or("unknown").to_string(),
        method: result["method"].as_str().unwrap_or("unknown").to_string(),
        message: result["message"].as_str().unwrap_or("").to_string(),
    })
}

#[tauri::command]
async fn delete_account(platform: String, name: String) -> Result<(), String> {
    println!("[账号管理] 尝试删除账号: {}/{}", platform, name);
    let mut store = load_accounts();
    if let Some(pos) = store.accounts.iter().position(|a| a.platform == platform && a.name == name) {
        store.accounts.remove(pos);
        let dir = get_account_dir(&platform, &name);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        save_accounts(&store)?;
    }
    Ok(())
}

#[tauri::command]
async fn cleanup_login_session(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = handles.remove(&session_id) { let _ = child.start_kill(); }
    let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
    flows.remove(&session_id);
    Ok(())
}

// ============ 评论采集命令 ============

fn get_scraper_dir() -> PathBuf {
    get_data_dir().join("scraper")
}

#[tauri::command]
async fn resolve_user_sec_uid(input: String) -> Result<String, String> {
    let input = input.trim();

    // 1. 如果包含 www.douyin.com/user/，提取之后的部分
    if input.contains("www.douyin.com/user/") {
        let parts: Vec<&str> = input.split("www.douyin.com/user/").collect();
        if parts.len() > 1 {
            let id_part = parts[1].split('?').next().unwrap_or("").split('/').next().unwrap_or("");
            if !id_part.is_empty() {
                return Ok(id_part.to_string());
            }
        }
    }

    // 2. 如果是短链接，这里暂时不支持跳转解析（用户未要求），仅按规则提取或直接返回
    // 抖音 sec_uid 通常以 MS4wLjABAAAA 开头，长度较长
    if input.len() > 30 && input.starts_with("MS4wLjABAAAA") {
        return Ok(input.to_string());
    }

    // 3. 其他情况，如果看起来像链接但不是支持的格式
    if input.starts_with("http") {
        return Err("目前仅支持 sec_uid 或以 www.douyin.com/user/ 开头的主页链接".to_string());
    }

    // 默认返回原样，假设用户输入的是正确的 sec_uid
    Ok(input.to_string())
}

/// 启动采集任务
#[tauri::command]
async fn start_scrape(
    account_name: String,
    platform: String,
    sec_uid: String,
    scrape_type: String,   // video | comment | reply | all | follower | like
    limit: i32,
    skip_existing: bool,
    incremental: bool,
    index_kb: bool,
    state: State<'_, AppState>,
) -> Result<ScraperTask, String> {
    // 检查是否有任务正在运行
    {
        let current = state.current_task_id.lock().unwrap();
        if current.is_some() {
            return Err("已有任务正在运行中，请先停止或等待完成".to_string());
        }
    }

    if sec_uid.trim().is_empty() {
        return Err("sec_uid 不能为空".to_string());
    }

    // 确认账号存在
    let store = load_accounts();
    let _account = store.accounts.iter()
        .find(|a| a.platform == platform && a.name == account_name)
        .ok_or_else(|| format!("账号不存在: {}/{}", platform, account_name))?;

    let task_id = Uuid::new_v4().to_string();
    let cookie_file = get_account_dir(&platform, &account_name).join("cookie.txt");
    let script_path = PathBuf::from("..").join("scripts").join("douyin_scraper.py");

    // 日志文件
    let log_dir = get_data_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log_path = log_dir.join(format!("scrape_{}_{}.log", &task_id[..8], &sec_uid[..8]));
    let log_file = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let stderr_file = log_file.try_clone().map_err(|e| e.to_string())?;

    // 启动 Python 子进程
    let mut cmd = python_cmd();
    cmd.arg(&script_path)
        .arg("--task-id").arg(&task_id)
        .arg("--cookie-path").arg(&cookie_file)
        .arg("--sec-uid").arg(&sec_uid)
        .arg("--type").arg(&scrape_type)
        .arg("--limit").arg(limit.to_string());

    if skip_existing {
        cmd.arg("--skip-existing");
    }

    if incremental {
        cmd.arg("--incremental");
    }

    if index_kb {
        let config = get_config().await?;
        let config_str = serde_json::to_string(&config).unwrap();
        cmd.arg("--config").arg(config_str);
    }

    let child = cmd
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(stderr_file))
        .kill_on_drop(true)
        .spawn().map_err(|e| e.to_string())?;

    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.insert(format!("scrape_{}", task_id), child);
    }

    {
        let mut current = state.current_task_id.lock().unwrap();
        *current = Some(task_id.clone());
    }

    let task = ScraperTask {
        task_id: task_id.clone(),
        account_name,
        platform,
        sec_uid,
        scrape_type,
        limit,
        skip_existing,
        status: "running".to_string(),
        created_at: chrono_now(),
    };

    Ok(task)
}

/// 查询采集进度（读取 Python 写的进度文件）
#[tauri::command]
async fn get_scrape_progress(task_id: String) -> Result<ScraperProgress, String> {
    let progress_path = get_scraper_dir().join(format!("{}.json", task_id));
    if !progress_path.exists() {
        return Err("任务进度文件不存在".to_string());
    }
    let content = fs::read_to_string(&progress_path).map_err(|e| e.to_string())?;
    let progress: ScraperProgress = serde_json::from_str(&content)
        .map_err(|e| format!("解析进度文件失败: {}", e))?;
    Ok(progress)
}

/// 取消采集任务
#[tauri::command]
async fn cancel_scrape(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let key = format!("scrape_{}", task_id);
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(child) = handles.remove(&key) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
            }
        }
        #[cfg(not(unix))]
        {
            let mut child = child;
            let _ = child.start_kill();
        }
    }
    // 即使进程没关掉，也强制更新进度文件状态为 cancelled
    let progress_path = get_scraper_dir().join(format!("{}.json", task_id));
    if progress_path.exists() {
        let content = fs::read_to_string(&progress_path).unwrap_or_default();
        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
            val["status"] = serde_json::Value::String("cancelled".to_string());
            val["finished_at"] = serde_json::Value::Number(serde_json::Number::from_f64(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()).unwrap());
            let _ = fs::write(&progress_path, serde_json::to_string_pretty(&val).unwrap_or_default());
        }
    }

    {
        let mut current = state.current_task_id.lock().unwrap();
        if let Some(id) = current.as_ref() {
            if id == &task_id {
                *current = None;
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn get_current_task(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let current = state.current_task_id.lock().unwrap();
    Ok(current.clone())
}

#[tauri::command]
async fn clear_current_task(state: State<'_, AppState>) -> Result<(), String> {
    let mut current = state.current_task_id.lock().unwrap();
    *current = None;
    Ok(())
}

/// 内部共享函数：内容生成 + GEO 审计（供 Tauri 命令和聊天工具调用）
async fn studio_generate_internal(
    topic: String,
    material: String,
    mode: String,
    platform: String,
) -> Result<serde_json::Value, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    let query = if topic.is_empty() {
        material.chars().take(50).collect::<String>()
    } else {
        topic.clone()
    };
    let kb_context = match search_kb_internal(query).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut ctx = String::from("\n参考的企业知识库背景：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(5) {
                    if let Some(text) = item["text"].as_str() {
                        ctx.push_str(&format!("- {}\n", text));
                    }
                }
            }
            if ctx.len() < 20 { String::new() } else { ctx }
        }
        Err(_) => String::new(),
    };

    let platform_instructions = match platform.as_str() {
        "douyin" => "【抖音/短视频平台优化】：要求开头前 3 秒有极其吸引人的\"情绪钩子\"，中间事实密集，语言口语化，结尾有强引导。采用\"答案前置\"结构，直接在开头揭示核心价值。",
        "wechat" => "【微信公众号优化】：要求排版精美感，深度分析，事实密度极高，建立 E-E-A-T 权威感。采用\"答案前置\"结构，首段即总结全文精华。",
        "zhihu"  => "【知乎/专业社区优化】：要求专业严谨，大量引用事实和数据，逻辑性强。直接回答问题核心，避免废话。",
        _        => "采用答案前置结构，提高事实密度。",
    };

    let system_prompt = format!(
        "你是一位资深的 AI 内容创作者和 GEO（生成式引擎优化）专家。\n\
        你的任务是根据提供的素材和知识库内容，为用户创作或改造高质量内容。\n\n\
        核心准则：\n\
        1. **答案前置 (Answer-First)**：直接在内容开头回答核心问题或展示最核心价值。\n\
        2. **事实密度最大化**：大量使用知识库中的具体数据、技术指标和事实描述，避免空洞的形容词。\n\
        3. **权威性构建**：语言风格专业，逻辑严密。\n\n\
        {}\n\n{}",
        platform_instructions, kb_context
    );

    let user_content = if mode == "new" {
        format!("请围绕主题「{}」创作一篇全新的文章。补充素材：{}", topic, material)
    } else {
        format!("请对以下内容进行 GEO 深度改造和重写：\n\n{}", material)
    };

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    // 第一步：生成内容
    let gen_payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ],
        "temperature": 0.7
    });
    let gen_resp: serde_json::Value = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&gen_payload)
        .send().await.map_err(|e| format!("生成内容失败: {}", e))?
        .json().await.map_err(|e| e.to_string())?;
    let generated_content = gen_resp["choices"][0]["message"]["content"]
        .as_str().ok_or("LLM 返回内容为空")?.to_string();

    // 第二步：GEO 审计
    let audit_system = "你是一位冷静的内容审计员和舆情分析师。\n\
        请对提供的内容进行\"发布前压力测试\"，输出一份简洁的 Markdown 审计报告。\n\
        报告需包含：\n\
        1. **舆情预判**：模拟读者看到该内容后的潜在反应（积极、争议点）。\n\
        2. **GEO 评分**：针对\"答案前置\"和\"事实密度\"给出 0-100 的评分。\n\
        3. **改进建议**：如何让内容更专业、更具 AI 引擎可见性。\n\
        4. **敏感性核查**：是否存在不合规风险。";
    let audit_payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": audit_system },
            { "role": "user", "content": format!("请对以下内容进行审计分析：\n\n{}", generated_content) }
        ],
        "temperature": 0.3
    });
    let audit_resp: serde_json::Value = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&audit_payload)
        .send().await.map_err(|e| format!("审计分析失败: {}", e))?
        .json().await.map_err(|e| e.to_string())?;
    let audit_report = audit_resp["choices"][0]["message"]["content"]
        .as_str().unwrap_or("审计失败").to_string();

    Ok(serde_json::json!({ "content": generated_content, "audit": audit_report }))
}

/// 内部共享函数：仅审计（不生成新内容）
async fn audit_content_internal(content: String) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }
    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };
    let system = "你是一位冷静的内容审计员和舆情分析师。\n\
        请对提供的内容进行\"发布前压力测试\"，输出一份简洁的 Markdown 审计报告。\n\
        报告需包含：\n\
        1. **舆情预判**：模拟读者看到该内容后的潜在反应（积极、争议点）。\n\
        2. **GEO 评分**：针对\"答案前置\"和\"事实密度\"给出 0-100 的评分。\n\
        3. **改进建议**：如何让内容更专业、更具 AI 引擎可见性。\n\
        4. **敏感性核查**：是否存在不合规风险。";
    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": format!("请对以下内容进行审计分析：\n\n{}", content) }
        ],
        "temperature": 0.3
    });
    let resp: serde_json::Value = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("审计失败: {}", e))?
        .json().await.map_err(|e| e.to_string())?;
    Ok(resp["choices"][0]["message"]["content"]
        .as_str().ok_or("审计返回为空")?.to_string())
}

/// Tauri 命令：内容创作（代理 internal 函数）
#[tauri::command]
async fn studio_generate_content(
    topic: String,
    material: String,
    mode: String,
    platform: String,
) -> Result<serde_json::Value, String> {
    studio_generate_internal(topic, material, mode, platform).await
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GeoQueryResult {
    pub model_name: String,
    pub mentioned: bool,
    pub position: i32,       // 0=未提及，1=首位，2=次位，以此类推
    pub response: String,
    pub sources: Vec<String>,
    pub error: Option<String>,
}

#[tauri::command]
async fn geo_monitor_query(
    brand: String,
    keyword: String,
) -> Result<Vec<GeoQueryResult>, String> {
    let config = get_config().await?;
    let models: Vec<GeoModelConfig> = config.llm.geo_models.into_iter()
        .filter(|m| m.enabled && !m.api_key.is_empty() && !m.base_url.is_empty())
        .collect();

    if models.is_empty() {
        return Err("未配置任何 GEO 监控模型，请前往设置页添加模型".to_string());
    }

    let brand_clone = brand.clone();
    let keyword_clone = keyword.clone();

    let mut handles = Vec::new();
    for model in models {
        let b = brand_clone.clone();
        let k = keyword_clone.clone();
        handles.push(tokio::spawn(async move {
            query_geo_model(model, b, k).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(r) => results.push(r),
            Err(e) => results.push(GeoQueryResult {
                model_name: "未知".to_string(),
                mentioned: false,
                position: 0,
                response: String::new(),
                sources: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(results)
}

async fn query_geo_model(model: GeoModelConfig, brand: String, keyword: String) -> GeoQueryResult {
    let url = if model.base_url.ends_with("/chat/completions") {
        model.base_url.clone()
    } else {
        format!("{}/chat/completions", model.base_url.trim_end_matches('/'))
    };

    let prompt = format!(
        "请问关于「{}」，你能推荐一些相关的内容创作者或品牌吗？请给出具体名称，并说明你的信息来源。",
        keyword
    );

    let body = serde_json::json!({
        "model": model.model_id,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 800,
        "temperature": 0.3
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let resp = match client.post(&url)
        .header("Authorization", format!("Bearer {}", model.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return GeoQueryResult {
            model_name: model.name,
            mentioned: false,
            position: 0,
            response: String::new(),
            sources: vec![],
            error: Some(format!("请求失败: {}", e)),
        },
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => return GeoQueryResult {
            model_name: model.name,
            mentioned: false,
            position: 0,
            response: String::new(),
            sources: vec![],
            error: Some(format!("解析响应失败: {}", e)),
        },
    };

    let response_text = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // 判断品牌是否被提及，并估算位置
    let brand_lower = brand.to_lowercase();
    let resp_lower = response_text.to_lowercase();
    let mentioned = resp_lower.contains(&brand_lower);

    let position = if mentioned {
        // 按段落/列表项粗估排名位置
        let lines: Vec<&str> = response_text.lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        let pos = lines.iter().position(|l| l.to_lowercase().contains(&brand_lower))
            .map(|i| i + 1)
            .unwrap_or(1);
        pos as i32
    } else {
        0
    };

    // 提取 sources：抓取响应中出现的 URL 或「来源：xxx」格式
    let mut sources: Vec<String> = Vec::new();
    let url_re = regex::Regex::new(r"https?://[^\s\)]+").unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
    for cap in url_re.find_iter(&response_text) {
        sources.push(cap.as_str().to_string());
    }
    // 同时提取「来源」「参考」关键词后的内容
    for line in response_text.lines() {
        if line.contains("来源") || line.contains("参考") || line.contains("引用") {
            let cleaned = line.trim_start_matches(|c: char| !c.is_alphanumeric()).trim().to_string();
            if !cleaned.is_empty() && !sources.contains(&cleaned) {
                sources.push(cleaned);
            }
        }
    }

    GeoQueryResult {
        model_name: model.name,
        mentioned,
        position,
        response: response_text,
        sources,
        error: None,
    }
}

#[tauri::command]
async fn list_kb_files() -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("list")
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
async fn add_to_kb(file_path: String) -> Result<serde_json::Value, String> {
    let config = get_config().await?;
    let config_str = serde_json::to_string(&config).unwrap();

    let script_path = PathBuf::from("..").join("scripts").join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("add")
        .arg("--file").arg(file_path)
        .arg("--config").arg(config_str)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
async fn get_kb_file_details(filename: String) -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("details")
        .arg("--filename").arg(filename)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
async fn delete_kb_file(filename: String) -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("delete")
        .arg("--filename").arg(filename)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

async fn search_kb_internal(query: String) -> Result<String, String> {
    let config = get_config().await?;
    let config_str = serde_json::to_string(&config).unwrap();

    let script_path = PathBuf::from("..").join("scripts").join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("search")
        .arg("--query").arg(query)
        .arg("--config").arg(config_str)
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(result_str)
}

// ============ 结果查询命令 ============

#[tauri::command]
async fn list_scraped_users() -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("query_data.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("list_users")
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}", result_str))?;
    Ok(result)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn get_scraped_videos(secUid: String, limit: i32, offset: i32) -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("query_data.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("get_videos")
        .arg("--sec-uid").arg(&secUid)
        .arg("--limit").arg(limit.to_string())
        .arg("--offset").arg(offset.to_string())
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}", result_str))?;
    Ok(result)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn get_scraped_comments(secUid: String, awemeId: Option<String>, limit: i32, offset: i32) -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("query_data.py");
    let mut cmd = python_cmd();
    cmd.arg(&script_path)
        .arg("get_comments")
        .arg("--sec-uid").arg(&secUid)
        .arg("--limit").arg(limit.to_string())
        .arg("--offset").arg(offset.to_string());
    
    if let Some(id) = awemeId {
        cmd.arg("--aweme-id").arg(id);
    }

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}", result_str))?;
    Ok(result)
}

#[tauri::command]
async fn open_video_in_browser(aweme_id: String, account_name: String) -> Result<(), String> {
    let platform = "douyin";
    let cookie_json = get_account_dir(platform, &account_name).join("cookie.json");
    
    if !cookie_json.exists() {
        return Err(format!("账号 {} 的 Cookie 文件不存在", account_name));
    }

    let script_path = PathBuf::from("..").join("scripts").join("open_video.py");
    
    let mut cmd = python_cmd();
    cmd.arg(&script_path)
        .arg("--cookie-path").arg(&cookie_json)
        .arg("--video-id").arg(&aweme_id);

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("打开视频失败: {}", err));
    }

    Ok(())
}

// ============ 抖音私信命令 ============

fn run_douyin_im_bridge(args: Vec<String>) -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("douyin_im_bridge.py");
    let output = python_cmd_sync()
        .arg(&script_path)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    if !stderr_str.is_empty() {
        eprintln!("[douyin_im_bridge] Python stderr:\n{}", stderr_str);
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if result_str.is_empty() {
        return Err(format!("抖音私信脚本无输出，退出码: {:?}", output.status.code()));
    }

    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("抖音私信结果解析失败: {}", result_str))?;

    if !output.status.success() || result.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        // 把完整 JSON 作为错误字符串传回，让前端可以解析出 needs_refresh 等额外字段。
        // 格式：JSON_ERR:<json>  前端通过前缀区分普通错误和结构化错误。
        let json_err = serde_json::to_string(&result).unwrap_or_default();
        let message = result.get("error").and_then(|v| v.as_str()).unwrap_or("抖音私信命令执行失败");
        return Err(format!("JSON_ERR:{}\n{}", json_err, message));
    }

    Ok(result)
}

fn append_optional_arg(args: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        if !value.trim().is_empty() {
            args.push(name.to_string());
            args.push(value);
        }
    }
}

fn get_douyin_cookie_path(account_name: &str) -> Result<PathBuf, String> {
    let store = load_accounts();
    let _account = store.accounts.iter()
        .find(|a| a.platform == "douyin" && a.name == account_name)
        .ok_or_else(|| format!("抖音账号不存在: {}", account_name))?;
    let cookie_path = get_account_dir("douyin", account_name).join("cookie.txt");
    if !cookie_path.exists() {
        return Err(format!("账号 {} 的 Cookie 文件不存在", account_name));
    }
    Ok(cookie_path)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_check(accountName: String) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_douyin_im_bridge(vec![
            "check".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
        ])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_my_uid(accountName: String) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_douyin_im_bridge(vec![
            "my_uid".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
        ])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_user_info(accountName: String, userId: String) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_douyin_im_bridge(vec![
            "user_info".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
            "--user-id".to_string(),
            userId,
        ])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_contacts(accountName: String, uid: Option<String>, limit: Option<i64>) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec![
            "contacts".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
            "--limit".to_string(),
            limit.unwrap_or(50).to_string(),
        ];
        append_optional_arg(&mut args, "--uid", uid);
        run_douyin_im_bridge(args)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_messages(
    accountName: String,
    conversationId: Option<String>,
    peerUid: Option<String>,
    uid: Option<String>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec![
            "messages".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
            "--limit".to_string(),
            limit.unwrap_or(50).to_string(),
        ];
        append_optional_arg(&mut args, "--conversation-id", conversationId);
        append_optional_arg(&mut args, "--peer-uid", peerUid);
        append_optional_arg(&mut args, "--uid", uid);
        run_douyin_im_bridge(args)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_create_conversation(
    accountName: String,
    toUserId: String,
    webProtect: Option<String>,
    keys: Option<String>,
    uid: Option<String>,
) -> Result<serde_json::Value, String> {
    if toUserId.trim().is_empty() {
        return Err("对方 UID 不能为空".to_string());
    }
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec![
            "create_conversation".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
            "--to-user-id".to_string(),
            toUserId,
        ];
        if let Some(web_protect_value) = webProtect {
            if !web_protect_value.trim().is_empty() {
                args.push("--web-protect".to_string());
                args.push(web_protect_value);
            }
        }
        if let Some(keys_value) = keys {
            if !keys_value.trim().is_empty() {
                args.push("--keys".to_string());
                args.push(keys_value);
            }
        }
        if let Some(uid_value) = uid {
            if !uid_value.trim().is_empty() {
                args.push("--uid".to_string());
                args.push(uid_value);
            }
        }
        run_douyin_im_bridge(args)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_send(
    accountName: String,
    content: String,
    toUserId: Option<String>,
    conversationId: Option<String>,
    conversationShortId: Option<i64>,
    ticket: Option<String>,
    webProtect: Option<String>,
    keys: Option<String>,
    uid: Option<String>,
) -> Result<serde_json::Value, String> {
    if content.trim().is_empty() {
        return Err("消息内容不能为空".to_string());
    }
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec![
            "send".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
            "--content".to_string(),
            content,
        ];
        if let Some(web_protect_value) = webProtect {
            if !web_protect_value.trim().is_empty() {
                args.push("--web-protect".to_string());
                args.push(web_protect_value);
            }
        }
        if let Some(keys_value) = keys {
            if !keys_value.trim().is_empty() {
                args.push("--keys".to_string());
                args.push(keys_value);
            }
        }
        if let Some(uid_value) = uid {
            if !uid_value.trim().is_empty() {
                args.push("--uid".to_string());
                args.push(uid_value);
            }
        }
        if let Some(to_user_id) = toUserId {
            if !to_user_id.trim().is_empty() {
                args.push("--to-user-id".to_string());
                args.push(to_user_id);
            }
        } else {
            if let Some(conversation_id) = conversationId {
                args.push("--conversation-id".to_string());
                args.push(conversation_id);
            }
            if let Some(short_id) = conversationShortId {
                args.push("--conversation-short-id".to_string());
                args.push(short_id.to_string());
            }
            if let Some(ticket_value) = ticket {
                args.push("--ticket".to_string());
                args.push(ticket_value);
            }
        }
        run_douyin_im_bridge(args)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_refresh_credentials(
    accountName: String,
) -> Result<serde_json::Value, String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_douyin_im_bridge(vec![
            "refresh_credentials".to_string(),
            "--cookie-path".to_string(),
            cookie_path.to_string_lossy().to_string(),
        ])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_start_monitor(
    accountName: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cookie_path = get_douyin_cookie_path(&accountName)?;
    let key = format!("douyin_im_{}", accountName);
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if handles.contains_key(&key) {
        return Err("该账号私信监控已在运行".to_string());
    }

    let script_path = PathBuf::from("..").join("scripts").join("douyin_im_bridge.py");
    let mut child = python_cmd()
        .arg(&script_path)
        .arg("monitor")
        .arg("--cookie-path").arg(&cookie_path)
        .arg("--account-name").arg(&accountName)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn().map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().ok_or("无法打开 Python stdout")?;
    let app_handle = app.clone();
    let key_clone = key.clone();
    let account_name_clone = accountName.clone();

    tauri::async_runtime::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let _ = app_handle.emit("douyin-im-event", val);
            }
        }
        if let Ok(mut h) = app_handle.state::<AppState>().process_handles.lock() {
            h.remove(&key_clone);
        }
        let _ = app_handle.emit("douyin-im-event", serde_json::json!({
            "type": "status",
            "status": "stopped",
            "account": account_name_clone
        }));
    });

    handles.insert(key, child);
    Ok(())
}

#[tauri::command]
async fn get_active_douyin_im_monitors(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    let monitors: Vec<serde_json::Value> = handles.keys()
        .filter(|k| k.starts_with("douyin_im_"))
        .map(|k| {
            let account = k.replacen("douyin_im_", "", 1);
            serde_json::json!({
                "account": account,
                "status": "running"
            })
        })
        .collect();
    Ok(monitors)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn douyin_im_stop_monitor(accountName: String, state: State<'_, AppState>) -> Result<(), String> {
    let key = format!("douyin_im_{}", accountName);
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(child) = handles.remove(&key) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
        }
    }
    Ok(())
}

#[tauri::command]
async fn resolve_live_url(url: String) -> Result<String, String> {
    let url = url.trim();

    // 1. 如果是纯数字 ID，直接返回
    if !url.is_empty() && url.chars().all(|c| c.is_ascii_digit()) {
        return Ok(url.to_string());
    }

    // 2. 如果包含 live.douyin.com/，尝试从 URL 中提取 ID
    if url.contains("live.douyin.com/") {
        let parts: Vec<&str> = url.split("live.douyin.com/").collect();
        if parts.len() > 1 {
            let id_part = parts[1].split('?').next().unwrap_or("").split('/').next().unwrap_or("");
            if !id_part.is_empty() {
                return Ok(id_part.to_string());
            }
        }
    }

    // 3. 如果是其他形式的链接或非法输入
    if url.starts_with("http") {
        return Err("目前仅支持直播间 ID 或以 live.douyin.com/ 开头的直播间链接".to_string());
    }

    Err("请输入有效的直播间 ID 或直播间链接".to_string())
}

#[tauri::command]
async fn start_live_monitor(
    room_id: String,
    account_name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    let key = format!("live_{}", room_id);
    
    if handles.contains_key(&key) {
        return Err("该直播间已在监控中".to_string());
    }

    // 检查监控数量限制
    let live_count = handles.keys().filter(|k| k.starts_with("live_")).count();
    if live_count >= 10 {
        return Err("最多只能同时监控 10 个直播间".to_string());
    }

    let script_path = PathBuf::from("..").join("scripts").join("douyin_live_monitor.py");
    
    let mut child = python_cmd()
        .arg(&script_path)
        .arg("--room-id").arg(&room_id)
        .arg("--account-name").arg(&account_name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn().map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().ok_or("无法打开 Python stdout")?;
    let room_id_clone = room_id.clone();
    let app_handle = app.clone();
    
    // 在后台读取输出并发送事件
    tauri::async_runtime::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let _ = app_handle.emit("live-event", val);
            }
        }

        // 进程结束，从 handles 中移除自己
        {
            if let Ok(mut h) = app_handle.state::<AppState>().process_handles.lock() {
                h.remove(&format!("live_{}", room_id_clone));
            }
        }

        // 发送状态事件
        let _ = app_handle.emit("live-event", serde_json::json!({
            "type": "status",
            "status": "stopped",
            "live_id": room_id_clone
        }));
    });

    handles.insert(key, child);
    Ok(())
}

#[tauri::command]
async fn stop_live_monitor(room_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let key = format!("live_{}", room_id);
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(child) = handles.remove(&key) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_active_monitors(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    let ids: Vec<String> = handles.keys()
        .filter(|k| k.starts_with("live_"))
        .map(|k| k.replace("live_", ""))
        .collect();
    Ok(ids)
}

#[tauri::command]
async fn get_live_history(room_id: String) -> Result<Vec<serde_json::Value>, String> {
    let data_dir = get_data_dir().join("live_data").join(&room_id);
    let history_path = data_dir.join("history.jsonl");
    
    if !history_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    let mut history = vec![];
    
    // 取最后 100 条
    let lines: Vec<&str> = content.lines().collect();
    let start = if lines.len() > 100 { lines.len() - 100 } else { 0 };
    
    for line in &lines[start..] {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            history.push(val);
        }
    }
    
    Ok(history)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
            current_task_id: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config, save_config, get_default_config,
            list_kb_files, add_to_kb, delete_kb_file, get_kb_file_details,
            studio_generate_content,
            analyze_comments, generate_live_reply, generate_im_reply, delete_scraped_user,
            list_chat_sessions, create_chat_session, delete_chat_session,
            send_chat_message, get_chat_messages,
            list_accounts, verify_account, delete_account,
            sync_local_accounts, init_login_session, get_login_status,
            finish_login, cleanup_login_session,
            start_scrape, get_scrape_progress, cancel_scrape,
            get_current_task, clear_current_task,
            list_scraped_users, get_scraped_videos, get_scraped_comments,
            open_video_in_browser, resolve_user_sec_uid,
            douyin_im_check, douyin_im_my_uid, douyin_im_user_info,
            douyin_im_contacts, douyin_im_messages,
            douyin_im_create_conversation, douyin_im_send,
            douyin_im_refresh_credentials,
            douyin_im_start_monitor, douyin_im_stop_monitor,
            get_active_douyin_im_monitors,
            start_live_monitor, stop_live_monitor, get_active_monitors,
            get_live_history, resolve_live_url,
            geo_monitor_query
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let app_state = app_handle.state::<AppState>();
                let lock_result = app_state.process_handles.lock();
                if let Ok(mut handles) = lock_result {
                    // 先 SIGTERM 通知 Python 优雅关闭浏览器
                    #[cfg(unix)]
                    for (_, child) in handles.iter() {
                        if let Some(pid) = child.id() {
                            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                        }
                    }
                    // 给 Python 时间关 Chrome（异步 playwright close 通常 ~1s）
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    // 兜底 SIGKILL（drop Child 会触发 kill_on_drop）
                    for (_, mut child) in handles.drain() {
                        let _ = child.start_kill();
                    }
                }
            }
        });
}
