use serde::{Deserialize, Serialize};

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

fn default_true() -> bool { true }

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct GeoPublishPlatform {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub system_prompt: String,
}

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
    #[serde(default)]
    pub live_theme: String,
    #[serde(default)]
    pub live_content: String,
    #[serde(default)]
    pub geo_models: Vec<GeoModelConfig>,
    #[serde(default)]
    pub geo_publish_platforms: Vec<GeoPublishPlatform>,
    /// AI 助理：动作工具确认执行后，是否额外调用 LLM 生成自然语言总结。
    /// 默认 false（省配额，仅展示结构化要点）；开启后体验更好但每次执行多一次 API 调用。
    #[serde(default)]
    pub ai_summarize_actions: bool,
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_live_reply_prompt() -> String {
    "你是一位正在直播的主播。请根据直播主题和直播内容，简短地回复用户的弹幕。回复必须非常简短（20字以内），语气亲切自然，像真人在直播间说话一样。".to_string()
}

fn default_analysis_prompt() -> String {
    "你是一位资深的社交媒体数据分析师。我会为你提供一组短视频评论数据，请从以下几个维度进行深度分析：\n1. 舆情氛围：整体情绪倾向（积极、消极、中立）及其占比。\n2. 核心热点：用户最关心的前3个话题或痛点。\n3. 用户意图：是否存在高潜力的咨询、购买意向或反馈建议。\n4. 互动建议：针对当前评论区，建议运营人员如何进行回复或引导。\n请用专业且简洁的 Markdown 格式输出分析报告。".to_string()
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct HermesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_hermes_url")]
    pub gateway_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_hermes_url() -> String {
    "http://127.0.0.1:8642".to_string()
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct VideoConfig {
    #[serde(default)]
    pub fal_key: String,
    #[serde(default)]
    pub volc_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub openai_base_url: String,
    #[serde(default)]
    pub openai_model: String,
    #[serde(default)]
    pub default_provider: String,

    #[serde(default)]
    pub tts_provider: String,
    #[serde(default)]
    pub tts_api_key: String,
    #[serde(default)]
    pub tts_base_url: String,
    #[serde(default)]
    pub tts_model: String,
    #[serde(default)]
    pub default_tts_voice: String,
    #[serde(default)]
    pub default_tts_speed: f32,
    /// 用户在设置页自定义的音色组：voice_id 传给 OpenAI 协议，name 仅前端显示
    #[serde(default)]
    pub tts_voices: Vec<TtsVoice>,
    /// 脚本生成的系统提示词（可在设置页编辑；脚本生成页不可见）
    #[serde(default = "default_script_system_prompt")]
    pub script_system_prompt: String,
    /// 用户定义的语气词和声调列表，用于引导表演脚本生成
    #[serde(default)]
    pub tts_prosody_tags: String,
}

pub fn default_script_system_prompt() -> String {
    "你是一位资深的短视频带货编剧和内容营销专家，擅长编写极具传播力和转化力的口播脚本。\n\n\
    【核心创作准则 (GEO)】\n\
    1. 答案前置：第一句话必须抓住用户眼球（情绪钩子），直接展示产品最核心的痛点解决或价值。\n\
    2. 事实密度：拒绝空洞赞美（如\"非常好用\"），多使用具体规格、成分、数据和使用场景。\n\
    3. 场景化：让脚本听起来像真实推荐或体验过程，而非生硬广告。\n\
    4. 必须充分引用企业知识库中的事实资料，并遵循指定平台的风格。".to_string()
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TtsVoice {
    /// 音色 ID（OpenAI 协议的 voice 字段实际值，如 alloy / nova，或第三方服务的音色编号）
    pub voice_id: String,
    /// 音色名称（仅前端展示用的友好名）
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SttConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub llm: LLMConfig,
    #[serde(default)]
    pub hermes: HermesConfig,
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub stt: SttConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_used: Option<String>,
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

pub fn account_to_view(a: &Account) -> AccountView {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ScraperTask {
    pub task_id: String,
    pub account_name: String,
    pub platform: String,
    pub sec_uid: String,
    pub scrape_type: String,
    pub limit: i32,
    pub skip_existing: bool,
    pub status: String,
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
pub struct AccountsStoreFile {
    pub accounts: Vec<Account>,
}

impl Default for AccountsStoreFile {
    fn default() -> Self {
        Self { accounts: Vec::new() }
    }
}

/// 已查询并持久化的抖音用户卡片（sec_uid 为主键）。
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UserCard {
    pub sec_uid: String,
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub unique_id: String, // 抖音号
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub follower_count: i64,
    #[serde(default)]
    pub following_count: i64,
    #[serde(default)]
    pub total_favorited: i64,
    #[serde(default)]
    pub aweme_count: i64,
    #[serde(default)]
    pub ip_location: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserCardsStoreFile {
    pub cards: Vec<UserCard>,
}

#[derive(Deserialize)]
pub struct PyLoginStatus {
    pub status: String,
    pub qrcode_base64: Option<String>,
    pub user_name: Option<String>,
    pub user_id: Option<String>,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoProject {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: String,
    pub locked_at: Option<String>,
    pub final_video_path: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoTask {
    pub id: String,
    pub project_id: Option<String>,
    pub task_type: String, 
    pub status: String,
    pub progress: i32,
    pub result_path: Option<String>,
    pub error_msg: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoMaterial {
    pub id: String,
    pub project_id: String,
    pub material_type: String,
    pub local_path: Option<String>,
    pub remote_url: Option<String>,
    pub meta: Option<serde_json::Value>,
    #[serde(default = "default_source")]
    pub source: String,
    pub created_at: Option<String>,
}

fn default_source() -> String { "uploaded".to_string() }

#[derive(Serialize, Deserialize, Clone)]
pub struct RenderConfig {
    pub width: i32,
    pub height: i32,
    pub bgm_volume: f32,
    pub transition_type: String,
    pub ken_burns: bool,
}
