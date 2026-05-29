use base64 as base64_engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

mod ffmpeg;
mod db;

// ============ 全局打包资源路径 ============
// 在 run() 初始化时由 AppHandle 注入；脚本路径解析依赖它。
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();
static SCRIPTS_DIR: OnceLock<PathBuf> = OnceLock::new();
static BUNDLED_PYTHON: OnceLock<String> = OnceLock::new();

/// 返回 scripts/ 目录的绝对路径。优先级：
///   1. 已缓存的结果
///   2. AUTOCAST_SCRIPTS_DIR 环境变量
///   3. 打包资源 resource_dir 下的多种 Tauri 布局：
///        a. resource_dir/_up_/scripts   (Tauri 默认把 ../scripts/* 放这里)
///        b. resource_dir/scripts        (map 形式或自定义 dest)
///        c. resource_dir/resources/_up_/scripts (某些 Tauri 2 版本)
///   4. 可执行文件附近的常见位置（macOS: Contents/Resources, Windows: 同级）
///   5. 项目根目录的 ../scripts（dev 模式）
///   6. 通过 kb_manager.py 这类已知文件做递归探测
fn get_scripts_dir() -> PathBuf {
    if let Some(p) = SCRIPTS_DIR.get() {
        return p.clone();
    }
    let result = resolve_scripts_dir();
    let _ = SCRIPTS_DIR.set(result.clone());
    result
}

fn resolve_scripts_dir() -> PathBuf {
    // 1. 环境变量手动指定（用户/CI 兜底）
    if let Ok(env_dir) = std::env::var("AUTOCAST_SCRIPTS_DIR") {
        let p = PathBuf::from(&env_dir);
        if p.join("kb_manager.py").exists() {
            return p;
        }
    }

    // 2. resource_dir 下的所有候选位置
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = RESOURCE_DIR.get() {
        candidates.push(res.join("_up_").join("scripts"));
        candidates.push(res.join("scripts"));
        candidates.push(res.join("resources").join("_up_").join("scripts"));
        candidates.push(res.join("resources").join("scripts"));
    }

    // 3. 可执行文件附近探测（macOS .app/Contents/Resources、Windows 同级）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("scripts"));
            candidates.push(parent.join("_up_").join("scripts"));
            candidates.push(parent.join("resources").join("scripts"));
            candidates.push(parent.join("resources").join("_up_").join("scripts"));
            // macOS .app 包内位置：MacOS/<exe> → ../Resources/_up_/scripts
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("_up_").join("scripts"));
                candidates.push(pp.join("Resources").join("scripts"));
            }
        }
    }

    // 4. dev fallback
    candidates.push(PathBuf::from("..").join("scripts"));
    candidates.push(PathBuf::from(".").join("scripts"));

    for c in &candidates {
        // 用 kb_manager.py 这个我们一定打包了的文件作为存在性标记
        if c.join("kb_manager.py").exists() {
            return c.clone();
        }
    }

    // 5. 兜底：以 resource_dir 为根递归搜（最多 4 层），找 kb_manager.py
    if let Some(res) = RESOURCE_DIR.get() {
        if let Some(found) = find_file_upwards(res, "kb_manager.py", 4) {
            if let Some(parent) = found.parent() {
                return parent.to_path_buf();
            }
        }
    }

    // 真的找不到了，返回相对路径让上层报清晰错误
    eprintln!("[autocast] WARN: 未找到 scripts/ 目录！RESOURCE_DIR={:?}, CWD={:?}",
              RESOURCE_DIR.get(),
              std::env::current_dir().ok());
    PathBuf::from("..").join("scripts")
}

fn find_file_upwards(root: &std::path::Path, target: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 { return None; }
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().map(|n| n == target).unwrap_or(false) {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(found) = find_file_upwards(&p, target, max_depth - 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

// ============ 状态管理 ============
pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
    pub process_handles: Mutex<std::collections::HashMap<String, tokio::process::Child>>,
    pub current_task_id: Mutex<Option<String>>,
    pub video_db: Mutex<rusqlite::Connection>,
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
    #[serde(default)]
    pub system_prompt: String,
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

    // ── TTS（语音合成）相关 ──
    #[serde(default)]
    pub tts_provider: String,       // "mock" / "openai" / "volcengine"
    #[serde(default)]
    pub tts_api_key: String,
    #[serde(default)]
    pub tts_base_url: String,       // OpenAI 兼容服务用
    #[serde(default)]
    pub tts_model: String,          // 默认 "tts-1"
    #[serde(default)]
    pub default_tts_voice: String,
    #[serde(default)]
    pub default_tts_speed: f32,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub llm: LLMConfig,
    #[serde(default)]
    pub hermes: HermesConfig,
    #[serde(default)]
    pub video: VideoConfig,
}

// 持久化存储结构

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
/// 查找顺序：
///   1. 环境变量 AUTOCAST_PYTHON
///   2. 打包内 python-build-standalone（开箱即用方案）
///        - macOS/Linux: <res>/_up_/src-tauri/python-runtime/python/bin/python3
///        - Windows    : <res>\_up_\src-tauri\python-runtime\python\python.exe
///        - 同时探测 <res>/python-runtime/python/...（如果 tauri 路径布局不同）
///   3. 项目根目录的 .venv (dev)
///   4. dev 模式下 src-tauri/python-runtime/python/...
///   5. 系统 PATH 中的 python/python3
fn python_executable() -> String {
    if let Some(p) = BUNDLED_PYTHON.get() {
        return p.clone();
    }
    let result = resolve_python_executable();
    let _ = BUNDLED_PYTHON.set(result.clone());
    result
}

fn resolve_python_executable() -> String {
    // 1. 环境变量
    if let Ok(env_py) = std::env::var("AUTOCAST_PYTHON") {
        if !env_py.trim().is_empty() && PathBuf::from(&env_py).exists() {
            return env_py;
        }
    }

    let (rel_bin, fallback_cmd): (PathBuf, &str) = if cfg!(windows) {
        (PathBuf::from("python").join("python.exe"), "python")
    } else {
        (PathBuf::from("python").join("bin").join("python3"), "python3")
    };

    // 2. 打包内的 python-runtime
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = RESOURCE_DIR.get() {
        candidates.push(res.join("_up_").join("src-tauri").join("python-runtime").join(&rel_bin));
        candidates.push(res.join("python-runtime").join(&rel_bin));
        candidates.push(res.join("_up_").join("python-runtime").join(&rel_bin));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // macOS .app/Contents/MacOS/<exe> → ../Resources/_up_/src-tauri/python-runtime
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("_up_").join("src-tauri").join("python-runtime").join(&rel_bin));
                candidates.push(pp.join("Resources").join("python-runtime").join(&rel_bin));
            }
            candidates.push(parent.join("python-runtime").join(&rel_bin));
        }
    }
    // 3. dev 模式：src-tauri/python-runtime（CWD=src-tauri/）
    candidates.push(PathBuf::from("python-runtime").join(&rel_bin));
    // 4. dev 模式：项目根目录的 .venv 或 src-tauri/python-runtime
    let venv_rel = if cfg!(windows) {
        PathBuf::from(".venv").join("Scripts").join("python.exe")
    } else {
        PathBuf::from(".venv").join("bin").join("python3")
    };
    candidates.push(PathBuf::from("..").join(&venv_rel));
    candidates.push(PathBuf::from("..").join("src-tauri").join("python-runtime").join(&rel_bin));

    for c in &candidates {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }

    eprintln!("[autocast] WARN: 未找到 bundled Python，回退到系统 PATH。候选路径：");
    for c in &candidates {
        eprintln!("  - {}", c.display());
    }
    fallback_cmd.to_string()
}

/// 创建已预置 AUTOCAST_DATA_DIR 环境变量的 tokio Python 子进程 Command。
/// Python 脚本通过 compat.get_data_dir() 优先读取该变量，确保路径与 Rust 端严格一致。

/// 诊断命令：返回运行时关键路径 + Python 依赖检查结果
#[tauri::command]
async fn autocast_diagnostics() -> Result<serde_json::Value, String> {
    let scripts = get_scripts_dir();
    let py = python_executable();
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let resource = RESOURCE_DIR.get().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let kb_exists = scripts.join("kb_manager.py").exists();

    // 探测 Python 关键依赖
    let check_modules = [
        "DrissionPage", "lancedb", "pypdf", "openai",
        "websockets", "httpx", "yaml", "tqdm", "PIL",
    ];
    let probe_code = format!(
        "import importlib,json,sys; res={{}}\nfor m in {:?}:\n  try: importlib.import_module(m); res[m]=True\n  except Exception as e: res[m]=str(e)\nprint(json.dumps(res))",
        check_modules
    );
    let dep_result = tokio::process::Command::new(&py)
        .arg("-c")
        .arg(&probe_code)
        .output()
        .await;
    let deps = match dep_result {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            serde_json::from_str::<serde_json::Value>(&s)
                .unwrap_or(serde_json::json!({ "raw": s }))
        }
        Ok(o) => serde_json::json!({
            "error": String::from_utf8_lossy(&o.stderr).to_string()
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };

    Ok(serde_json::json!({
        "scripts_dir": scripts.to_string_lossy(),
        "kb_manager_exists": kb_exists,
        "python": py,
        "cwd": cwd,
        "exe": exe,
        "resource_dir": resource,
        "python_modules": deps,
    }))
}

// ============ Hermes Agent 网关控制 ============

/// 跨平台查找 hermes 可执行文件
fn which_hermes() -> String {
    let exe_name = if cfg!(windows) { "hermes.exe" } else { "hermes" };

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 用户 home 下常见安装位置
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local").join("bin").join(exe_name));
        if cfg!(windows) {
            // pipx / Python Scripts / 自定义安装路径
            candidates.push(home.join("AppData").join("Local").join("Programs").join("hermes").join(exe_name));
            candidates.push(home.join("AppData").join("Local").join("hermes").join("bin").join(exe_name));
            candidates.push(home.join("AppData").join("Roaming").join("Python").join("Scripts").join(exe_name));
            candidates.push(home.join("scoop").join("shims").join(exe_name));
        }
    }

    if cfg!(windows) {
        // Chocolatey / Program Files
        candidates.push(PathBuf::from(r"C:\Program Files\hermes").join(exe_name));
        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin").join(exe_name));
    } else {
        candidates.push(PathBuf::from("/usr/local/bin/hermes"));
        candidates.push(PathBuf::from("/opt/homebrew/bin/hermes"));
        candidates.push(PathBuf::from("/usr/bin/hermes"));
    }

    for c in candidates {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }
    // fallback：交给 PATH 解析
    if cfg!(windows) { "hermes.exe".to_string() } else { "hermes".to_string() }
}

/// Write API_SERVER_ENABLED=true to ~/.hermes/.env so the gateway starts the HTTP API on :8642
/// Write API_SERVER_ENABLED=true to ~/.hermes/.env.
/// Returns the existing or newly generated API_SERVER_KEY (empty string = no key set).
#[tauri::command]
async fn hermes_enable_api_server() -> Result<String, String> {
    let env_path = dirs::home_dir()
        .ok_or("无法获取 home 目录")?
        .join(".hermes")
        .join(".env");

    let content = if env_path.exists() {
        fs::read_to_string(&env_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut api_key_value = String::new();

    // Ensure API_SERVER_ENABLED=true
    if !lines.iter().any(|l| l.trim() == "API_SERVER_ENABLED=true") {
        lines.push("API_SERVER_ENABLED=true".to_string());
    }

    // Extract existing API_SERVER_KEY if set
    for line in &lines {
        if let Some(val) = line.strip_prefix("API_SERVER_KEY=") {
            api_key_value = val.trim().to_string();
        }
    }

    // Generate a new key if missing
    if api_key_value.is_empty() {
        api_key_value = uuid::Uuid::new_v4().to_string().replace("-", "");
        lines.push(format!("API_SERVER_KEY={}", api_key_value));
    }

    let new_content = lines.join("\n") + "\n";
    fs::write(&env_path, new_content).map_err(|e| e.to_string())?;

    // Return the key (newly generated or existing)
    Ok(api_key_value)
}

/// Fire-and-forget restart of the launchd/systemd-managed Hermes gateway.
/// Returns immediately — caller should poll /health to detect when it's back up.
#[tauri::command]
async fn hermes_restart_service() -> Result<(), String> {
    let hermes_bin = which_hermes();
    // spawn() and drop — takes ~10s to complete, don't block the UI
    tokio::process::Command::new(&hermes_bin)
        .args(["gateway", "restart"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("重启网关失败: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn start_hermes_gateway(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let key = "hermes_gateway".to_string();
    // Kill old owned process
    let old_child = {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.remove(&key)
    };
    if let Some(mut child) = old_child {
        let _ = child.start_kill();
        let _ = child.wait().await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Fire-and-forget restart of launchd service (takes ~10s — let caller poll health)
    let hermes_bin = which_hermes();
    let spawned = tokio::process::Command::new(&hermes_bin)
        .args(["gateway", "restart"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match spawned {
        Ok(_) => Ok(()), // launchd restart in progress
        Err(_) => {
            // Fallback: start foreground process with API server enabled
            let child = tokio::process::Command::new(&hermes_bin)
                .args(["gateway", "run", "--replace"])
                .env("API_SERVER_ENABLED", "true")
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| format!("启动 Hermes 失败 ({}): {}", hermes_bin, e))?;

            let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
            handles.insert(key, child);
            Ok(())
        }
    }
}

#[tauri::command]
async fn stop_hermes_gateway(state: State<'_, AppState>) -> Result<(), String> {
    // Try stopping owned process
    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        if let Some(mut child) = handles.remove("hermes_gateway") {
            let _ = child.start_kill();
            return Ok(());
        }
    }
    // Try stopping launchd service
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin)
        .args(["gateway", "stop"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn check_hermes_status(state: State<'_, AppState>) -> Result<bool, String> {
    // Clean up exited owned process
    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        if let Some(child) = handles.get_mut("hermes_gateway") {
            if let Ok(Some(_)) = child.try_wait() {
                handles.remove("hermes_gateway");
            }
        }
    }
    // HTTP health check is authoritative — Hermes may run independently
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    match client.get("http://127.0.0.1:8642/health").send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Detailed health info from the gateway
#[tauri::command]
async fn check_hermes_gateway_health(gateway_url: String, api_key: String) -> Result<serde_json::Value, String> {
    let url = format!("{}/health", gateway_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    let response = req.send().await.map_err(|e| format!("连接失败: {}", e))?;
    if response.status().is_success() {
        let body = response.json::<serde_json::Value>().await
            .unwrap_or(serde_json::json!({"status": "ok"}));
        Ok(body)
    } else {
        Err(format!("HTTP {}", response.status().as_u16()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HermesSession {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub last_active: String,
}

/// List sessions from Hermes CLI (hermes sessions list).
/// Hermes stores sessions in a local SQLite DB; there is no REST endpoint for listing.
#[tauri::command]
async fn list_hermes_sessions() -> Result<Vec<HermesSession>, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin)
        .args(["sessions", "list", "--limit", "30"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        // Not an error — hermes might not be installed or sessions empty
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sessions = Vec::new();

    // Output format:
    // Title                            Preview                  Last Active   ID
    // ──────────────────────────────────────────────────────────────────────────────────
    // Session title here               Preview text here...     2h ago        some_id
    for line in stdout.lines().skip(2) {  // Skip header + separator
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('─') {
            continue;
        }
        // Tab or multiple-spaces separated; try splitting by 2+ spaces
        let parts: Vec<&str> = line.splitn(4, "  ").map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            let title = parts.first().unwrap_or(&"").to_string();
            let preview = parts.get(1).unwrap_or(&"").to_string();
            let last_active = parts.get(2).unwrap_or(&"").to_string();
            let id = parts.get(3).unwrap_or(&"").to_string();

            if !id.is_empty() || !last_active.is_empty() {
                // When there are only 3 parts, last_active might actually be the ID
                let (real_last_active, real_id) = if parts.len() == 3 {
                    (last_active.clone(), preview.clone())
                } else {
                    (last_active, id)
                };
                if !real_id.is_empty() {
                    sessions.push(HermesSession {
                        id: real_id,
                        title: if title == "—" || title.is_empty() { "未命名会话".to_string() } else { title },
                        preview: preview.chars().take(80).collect(),
                        last_active: real_last_active,
                    });
                }
            }
        }
    }
    Ok(sessions)
}

/// Get status of a specific run (GET /v1/runs/{run_id})
#[tauri::command]
async fn hermes_list_runs(gateway_url: String, api_key: String, run_id: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let Some(rid) = run_id else {
        return Ok(vec![]);
    };
    let url = format!("{}/v1/runs/{}", gateway_url.trim_end_matches('/'), rid);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) => Ok(vec![body]),
                Err(_) => Ok(vec![]),
            }
        }
        _ => Ok(vec![]),
    }
}

/// Stop an active run via POST /v1/runs/{id}/stop
#[tauri::command]
async fn hermes_stop_run(gateway_url: String, api_key: String, run_id: String) -> Result<(), String> {
    let url = format!("{}/v1/runs/{}/stop", gateway_url.trim_end_matches('/'), run_id);
    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&serde_json::json!({}));
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    req.send().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Approve or reject a tool call for a pending run
#[tauri::command]
async fn hermes_approve_run(
    gateway_url: String,
    api_key: String,
    run_id: String,
    approved: bool,
) -> Result<(), String> {
    let url = format!("{}/v1/runs/{}/approval", gateway_url.trim_end_matches('/'), run_id);
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(&serde_json::json!({"approved": approved}));
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    req.send().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Send a chat message to Hermes and stream the response via Tauri events.
/// Events emitted: hermes-chunk, hermes-done, hermes-error, hermes-run-id, hermes-tool-calls
/// Note: X-Hermes-Session-Id is only sent when api_key is set (403 without it).
#[tauri::command]
async fn hermes_send_message(
    app: tauri::AppHandle,
    gateway_url: String,
    api_key: String,
    messages: Vec<serde_json::Value>,
    session_id: Option<String>,
) -> Result<(), String> {
    use futures_util::StreamExt;

    let url = format!("{}/v1/chat/completions", gateway_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // agent tasks can be long
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream");

    // X-Hermes-Session-Id requires API_SERVER_KEY to be configured on the server.
    // Only send it when the user has set an api_key in settings.
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
        if let Some(ref sid) = session_id {
            if !sid.is_empty() {
                req = req.header("X-Hermes-Session-Id", sid.as_str());
            }
        }
    }

    // "model" is ignored by Hermes (it uses its configured model), but required by OpenAI spec
    let body = serde_json::json!({
        "messages": messages,
        "stream": true,
        "model": "hermes-agent"
    });

    let response = req.json(&body).send().await.map_err(|e| {
        let msg = format!("连接失败: {}", e);
        let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()}));
        msg
    })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        let msg = format!("HTTP {}: {}", status, text);
        let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()}));
        return Err(msg);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    let mut current_event = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            let msg = e.to_string();
            let _ = app.emit("hermes-error", serde_json::json!({"message": msg.clone()}));
            msg
        })?;

        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Drain complete SSE lines from buffer
        loop {
            match buffer.find('\n') {
                None => break,
                Some(pos) => {
                    let line = buffer[..pos].trim_end_matches('\r').to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.starts_with("event: ") {
                        current_event = line[7..].to_string();
                        continue;
                    }

                    if !line.starts_with("data: ") {
                        if line.is_empty() {
                            current_event = String::new();
                        }
                        continue;
                    }
                    let data = &line[6..];

                    if data == "[DONE]" {
                        let _ = app.emit("hermes-done", serde_json::json!({}));
                        return Ok(());
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                        // Check for error in stream (OpenAI format)
                        if let Some(err) = val.get("error") {
                            let msg = err["message"].as_str()
                                .or_else(|| err.as_str())
                                .unwrap_or("未知流错误");
                            let _ = app.emit("hermes-error", serde_json::json!({"message": msg}));
                            return Ok(());
                        }

                        // Emit custom events
                        if !current_event.is_empty() {
                            if current_event == "hermes.tool.progress" {
                                let _ = app.emit("hermes-tool-progress", val.clone());
                            } else {
                                let _ = app.emit("hermes-event", serde_json::json!({
                                    "event": current_event,
                                    "data": val
                                }));
                            }
                        }

                        // Emit run ID for stop/approval operations
                        if let Some(run_id) = val.get("id").and_then(|v| v.as_str()) {
                            let _ = app.emit("hermes-run-id", serde_json::json!({"run_id": run_id}));
                        }
                        
                        // Content delta (OpenAI format)
                        if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                            if let Some(choice) = choices.get(0) {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                        if !content.is_empty() {
                                            let _ = app.emit("hermes-chunk", serde_json::json!({"content": content}));
                                        }
                                    }
                                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                                        if !reasoning.is_empty() {
                                            let _ = app.emit("hermes-thinking", serde_json::json!({"content": reasoning}));
                                        }
                                    }
                                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                        if !tool_calls.is_empty() {
                                            let _ = app.emit("hermes-tool-calls", serde_json::json!({"tool_calls": tool_calls}));
                                        }
                                    }
                                }
                                
                                // Finish reason
                                let finish = choice.get("finish_reason").and_then(|v| v.as_str()).unwrap_or("");
                                if !finish.is_empty() && finish != "null" {
                                    let _ = app.emit("hermes-done", serde_json::json!({"finish_reason": finish}));
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit("hermes-done", serde_json::json!({}));
    Ok(())
}

/// Read the current API_SERVER_KEY from ~/.hermes/.env
#[tauri::command]
async fn hermes_read_api_key() -> Result<String, String> {
    let env_path = dirs::home_dir()
        .ok_or("无法获取 home 目录")?
        .join(".hermes")
        .join(".env");

    if !env_path.exists() {
        return Ok(String::new());
    }

    let content = fs::read_to_string(&env_path).map_err(|e| e.to_string())?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("API_SERVER_KEY=") {
            return Ok(val.trim().to_string());
        }
    }
    Ok(String::new())
}

/// Write/update API_SERVER_KEY in ~/.hermes/.env (empty key = remove the line)
#[tauri::command]
async fn hermes_set_api_key(key: String) -> Result<(), String> {
    let hermes_dir = dirs::home_dir()
        .ok_or("无法获取 home 目录")?
        .join(".hermes");
    fs::create_dir_all(&hermes_dir).map_err(|e| e.to_string())?;

    let env_path = hermes_dir.join(".env");
    let content = if env_path.exists() {
        fs::read_to_string(&env_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let existing_pos = lines.iter().position(|l| l.starts_with("API_SERVER_KEY="));

    if key.is_empty() {
        // Remove existing key line if present
        if let Some(pos) = existing_pos {
            lines.remove(pos);
        }
    } else {
        let new_line = format!("API_SERVER_KEY={}", key);
        if let Some(pos) = existing_pos {
            lines[pos] = new_line;
        } else {
            lines.push(new_line);
        }
    }

    let new_content = if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    };
    fs::write(&env_path, new_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 计算扩展后的 PATH，包含常见 Node.js / Homebrew / pipx 安装位置。
/// macOS GUI App 默认 PATH 只有 /usr/bin:/bin:/usr/sbin:/sbin，
/// 必须补充 /usr/local/bin 和 /opt/homebrew/bin 否则找不到 node、ffmpeg、hermes 等。
fn enhanced_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let extra_dirs = if cfg!(target_os = "macos") {
        vec![
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/local/sbin",
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            "/usr/local/bin",
            "/snap/bin",
        ]
    } else {
        vec![]
    };

    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut parts: Vec<String> = current.split(sep).map(|s| s.to_string()).collect();

    // 把 home/.local/bin 加上（pipx 默认安装位置）
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        let s = local_bin.to_string_lossy().to_string();
        if !parts.iter().any(|p| p == &s) {
            parts.push(s);
        }
    }

    for d in extra_dirs {
        if !parts.iter().any(|p| p == d) {
            parts.push(d.to_string());
        }
    }
    parts.join(sep)
}

fn python_cmd() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(python_executable());
    cmd.env("AUTOCAST_DATA_DIR", get_data_dir().to_string_lossy().to_string());
    cmd.env("PYTHONUNBUFFERED", "1"); // 强制 Python 立即输出
    cmd.env("PATH", enhanced_path());  // 补全 PATH，让 Python 子进程能找到 node 等工具
    cmd.arg("-u"); // 开启无缓冲模式
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

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return match status.as_u16() {
            401 => Err("LLM API Key 无效或已过期，请检查配置".to_string()),
            403 => Err("LLM 权限不足，请确认模型权限或 API 状态".to_string()),
            429 => Err("LLM 请求过于频繁，请稍后再试".to_string()),
            _ => Err(format!("LLM API 错误 ({}): {}", status, err_text)),
        };
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

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return match status.as_u16() {
            401 => Err("LLM API Key 无效或已过期，请检查配置".to_string()),
            403 => Err("LLM 权限不足，请确认模型权限或 API 状态".to_string()),
            429 => Err("LLM 请求过于频繁，请稍后再试".to_string()),
            _ => Err(format!("LLM API 错误 ({}): {}", status, err_text)),
        };
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
                        match start_scrape(acc, plat, uid, stype.clone(), lim, true, true, state.clone()).await {
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
                    "search_knowledge_base" => {

                        let query = func_args["query"].as_str().unwrap_or_default().to_string();
                        search_kb_internal(query).await?
                    },
                    "generate_content" => {
                        let topic    = func_args["topic"].as_str().unwrap_or_default().to_string();
                        let material = func_args["material"].as_str().unwrap_or_default().to_string();
                        let mode     = func_args["mode"].as_str().unwrap_or("new").to_string();
                        let platform = func_args["platform"].as_str().unwrap_or("douyin").to_string();
                        match studio_generate_internal(topic.clone(), material, mode, platform.clone(), None).await {
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
            live_theme: "".to_string(),
            live_content: "".to_string(),
            geo_models: vec![],
            geo_publish_platforms: vec![],
        },
        hermes: HermesConfig {
            enabled: false,
            gateway_url: default_hermes_url(),
            api_key: "".to_string(),
        },
        video: VideoConfig {
            fal_key: "".to_string(),
            volc_key: "".to_string(),
            openai_api_key: "".to_string(),
            openai_base_url: "https://api.openai.com/v1".to_string(),
            openai_model: "v0".to_string(),
            default_provider: "fal".to_string(),
            tts_provider: "mock".to_string(),
            tts_api_key: "".to_string(),
            tts_base_url: "https://api.openai.com/v1".to_string(),
            tts_model: "tts-1".to_string(),
            default_tts_voice: "".to_string(),
            default_tts_speed: 1.0,
        },
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

    let script_path = get_scripts_dir().join(script_name);

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
    let script_path = get_scripts_dir().join("verify_account.py");

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
    let script_path = get_scripts_dir().join("douyin_scraper.py");

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
    platform_prompt: Option<String>,
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

    let platform_instructions_owned;
    let platform_instructions: &str = if let Some(ref p) = platform_prompt {
        if !p.trim().is_empty() {
            platform_instructions_owned = p.clone();
            &platform_instructions_owned
        } else {
            match platform.as_str() {
                "douyin" => "【抖音/短视频平台优化】：要求开头前 3 秒有极其吸引人的\"情绪钩子\"，中间事实密集，语言口语化，结尾有强引导。采用\"答案前置\"结构，直接在开头揭示核心价值。",
                "wechat" => "【微信公众号优化】：要求排版精美感，深度分析，事实密度极高，建立 E-E-A-T 权威感。采用\"答案前置\"结构，首段即总结全文精华。",
                "zhihu"  => "【知乎/专业社区优化】：要求专业严谨，大量引用事实和数据，逻辑性强。直接回答问题核心，避免废话。",
                _        => "采用答案前置结构，提高事实密度。",
            }
        }
    } else {
        match platform.as_str() {
            "douyin" => "【抖音/短视频平台优化】：要求开头前 3 秒有极其吸引人的\"情绪钩子\"，中间事实密集，语言口语化，结尾有强引导。采用\"答案前置\"结构，直接在开头揭示核心价值。",
            "wechat" => "【微信公众号优化】：要求排版精美感，深度分析，事实密度极高，建立 E-E-A-T 权威感。采用\"答案前置\"结构，首段即总结全文精华。",
            "zhihu"  => "【知乎/专业社区优化】：要求专业严谨，大量引用事实和数据，逻辑性强。直接回答问题核心，避免废话。",
            _        => "采用答案前置结构，提高事实密度。",
        }
    };

    let system_prompt = format!(
        "你是一位资深的 AI 内容专家和 GEO（生成式引擎优化）专家。\n\
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
    platform_prompt: Option<String>,
) -> Result<serde_json::Value, String> {
    studio_generate_internal(topic, material, mode, platform, platform_prompt).await
}

/// 根据平台 ID 拿到对应的 system_prompt 风格指令。
/// 1. 先从用户在设置页配置的 geo_publish_platforms 中匹配（name 或拼音 id）
/// 2. 找不到就用内置默认（抖音/快手/视频号/小红书）
fn resolve_platform_prompt(config: &AppConfig, platform_id: &str) -> String {
    let id = platform_id.trim();
    if id.is_empty() { return String::new(); }

    // 先在用户配置里找：name 完全匹配 / 名称包含 id / id 包含名称（中英文宽松匹配）
    for p in &config.llm.geo_publish_platforms {
        let n = p.name.trim();
        if n.is_empty() { continue; }
        if n == id || n.contains(id) || id.contains(n) {
            if !p.system_prompt.trim().is_empty() {
                return format!("\n【平台风格 · {}】\n{}\n", n, p.system_prompt.trim());
            }
        }
    }

    // 内置默认（用户没配置时兜底）
    let (label, prompt) = match id {
        "douyin" | "抖音" => ("抖音",
            "前 3 秒强情绪钩子（疑问/反差/惊吓），口语化短句，每句不超过 12 字。\
             中间段卖点高密度，节奏快。结尾必带强 CTA（点购物车/关注/下方链接）。\
             不要书面语，禁止用'今天我要给大家介绍'这种开头。"),
        "kuaishou" | "快手" => ("快手",
            "走老铁文化路线：接地气、性价比、信任感。开头直白点出产品和价格优势，\
             多用'家人们''老铁''咱家'这类词。中段用对比/亲测展示效果。结尾给福利感（限时/包邮/赠品）。"),
        "wechat-channel" | "视频号" | "video-channel" => ("视频号",
            "调性偏朋友圈：稳重、信任、有人情味。可以中长（30-60s），叙述完整，\
             适当带'我自己用过''朋友推荐''家人都说好'这类背书。CTA 偏柔和，'点小心心''加个好友咨询'。"),
        "xiaohongshu" | "小红书" => ("小红书",
            "种草调性：精致、闺蜜推荐感、关键词扎堆。开头用 emoji + 关键词，\
             中段分点列卖点（'✅' 符号开头），强调真实体验和细节，植入热门 tag 关键词。\
             结尾'冲！''快囤''姐妹们跟上'类号召。"),
        _ => ("通用", "针对短视频平台优化：开头钩子强，中段信息密集，结尾有行动指令。"),
    };
    format!("\n【平台风格 · {}】\n{}\n", label, prompt)
}

/// 视频脚本生成 / 重生成。
/// 流程（对应用户期望）：
///   1. 用产品信息检索知识库（第 1 次）→ 拼成 system prompt
///   2. 再做一次"综合检索"（第 2 次）：用产品 + 参考脚本 + 比例做 query
///   3. 把两次检索结果 + 视频比例 + 产品 + 参考脚本 + （可选）反馈
///      统一交给 AI 助理用的 LLM（config.llm）生成短视频脚本
///   4. 返回纯文本脚本，前端预览
///
/// 重新生成只是在 user_message 里追加"上版脚本 + 用户反馈"，模型自然能改稿。
#[tauri::command]
async fn video_generate_script(
    product: String,
    reference_script: Option<String>,
    video_ratio: String,
    platform: Option<String>,        // "douyin" | "kuaishou" | "wechat-channel" | "xiaohongshu" | "" (任意/无)
    script_type: Option<String>,     // "voiceover" | "ai-video"（P3 用，P1 先接住参数）
    previous_script: Option<String>,
    feedback: Option<String>,
) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 AI 助理的 LLM API Key".to_string());
    }
    if product.trim().is_empty() {
        return Err("请先填写要卖的产品信息".to_string());
    }

    // 平台 prompt：先从用户配置的 geo_publish_platforms 找匹配，否则用内置默认
    let platform_id = platform.unwrap_or_default();
    let platform_prompt = resolve_platform_prompt(&config, &platform_id);

    // 剧本类型 prompt：P3 会进一步细化，这里先放一个占位
    let script_type_id = script_type.unwrap_or_else(|| "voiceover".to_string());

    // ─── 第 1 次知识库检索：用产品名做 query，捞品牌/规格/历史话术 ───
    let kb_brand_ctx = match search_kb_internal(product.clone()).await {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!([]));
            let mut buf = String::new();
            if let Some(arr) = v.as_array() {
                for item in arr.iter().take(8) {
                    if let Some(t) = item["text"].as_str() {
                        buf.push_str(&format!("- {}\n", t.trim()));
                    }
                }
            }
            buf
        }
        Err(_) => String::new(),
    };

    // ─── 第 2 次知识库检索：综合 query（产品 + 参考脚本片段 + 比例语义） ───
    let ratio_hint = match video_ratio.as_str() {
        "9:16" => "竖屏短视频（抖音/快手/小红书）",
        "16:9" => "横屏视频（B站/YouTube/视频号横屏）",
        "1:1"  => "方形视频（Instagram/朋友圈）",
        _      => "短视频",
    };
    let mix_query = format!(
        "{} {} {}",
        product.trim(),
        reference_script.as_deref().unwrap_or("").chars().take(80).collect::<String>(),
        ratio_hint
    );
    let kb_mix_ctx = match search_kb_internal(mix_query).await {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!([]));
            let mut buf = String::new();
            if let Some(arr) = v.as_array() {
                for item in arr.iter().take(6) {
                    if let Some(t) = item["text"].as_str() {
                        buf.push_str(&format!("- {}\n", t.trim()));
                    }
                }
            }
            buf
        }
        Err(_) => String::new(),
    };

    // ─── 构造 system prompt：通用部分 + 剧本类型差异化 ───
    let common_intro =
        "你是一位资深的短视频带货脚本编剧，擅长把产品卖点转化成口播脚本。\n\
         产出要求：\n\
         1. 开头 3 秒钩子要强，能立刻抓住注意力；\n\
         2. 中段卖点密集，每一句话都要有信息量；\n\
         3. 结尾给明确行动指令（点购物车/点关注/留言等）；\n\
         4. 整体节奏匹配视频比例（竖屏快节奏、横屏可稍长）。\n\n";

    let type_format = match script_type_id.as_str() {
        // 口播剧本：用素材库素材轮播 + TTS，所以剧本是连贯的口播段落，**不要分镜**
        "voiceover" => "【剧本类型：口播剧本】\n\
            该剧本会用 TTS 合成成连续的旁白音频，配合素材库里的图片/视频轮播。\n\
            因此请输出**连贯的口播稿**，不要分镜，不要画面描述。\n\
            【输出格式严格遵守】请直接输出 Markdown：\n\
            \n\
            # 视频标题：<一句话>\n\
            > 总时长：<秒数> | 语速：<慢/中/快> | 目标受众：<人群>\n\n\
            ## 口播文案\n\
            <第一段：开头钩子。每段 30-50 字，便于 TTS 合成。>\n\n\
            <第二段：核心卖点 1>\n\n\
            <第三段：核心卖点 2>\n\n\
            <...更多段落>\n\n\
            <最后一段：行动号召>\n\n\
            ---\n\
            **核心卖点关键词**：`关键词1` `关键词2` `关键词3`\n\
            **建议素材关键词**（给上层匹配素材用）：`素材关键词1` `素材关键词2` ...\n",

        // AI 视频：每个分镜会对应一次视频生成，需要画面描述
        _ /* "ai-video" 或其他 */ => "【剧本类型：AI 视频】\n\
            该剧本的每个分镜都会作为视频生成模型的 prompt，所以**画面描述必须具体**\n\
            （包含主体、场景、镜头语言、光线、风格）。\n\
            【输出格式严格遵守】请直接输出 Markdown：\n\
            \n\
            # 视频标题：<一句话>\n\
            > 总时长：<秒数> | 节奏：<节奏描述> | 目标受众：<人群>\n\n\
            ## 分镜 1 · 0-3s · 钩子\n\
            **口播**：「<口播台词>」\n\
            **画面**：<具体的画面描述，给 AI 视频模型用>\n\
            **运镜**：<运镜方式：推/拉/摇/移/特写...>\n\n\
            ## 分镜 2 · 3-10s · 卖点\n\
            ...\n\n\
            ## 分镜 N · 最后 3s · 行动号召\n\
            **口播**：「...」\n\
            **画面**：...\n\n\
            ---\n\
            **核心卖点关键词**：`关键词1` `关键词2` `关键词3`\n",
    };

    let mut system_prompt = format!("{}{}", common_intro, type_format);

    if !kb_brand_ctx.is_empty() {
        system_prompt.push_str("\n以下是企业知识库中关于该产品/品牌的背景资料，请在脚本中合理引用：\n");
        system_prompt.push_str(&kb_brand_ctx);
    }
    if !platform_prompt.is_empty() {
        system_prompt.push_str(&platform_prompt);
    }

    // ─── 构造 user message（第 2 次检索结果 + 用户输入 + 反馈） ───
    let mut user_msg = String::new();
    user_msg.push_str(&format!("【视频比例】{}（{}）\n", video_ratio, ratio_hint));
    user_msg.push_str(&format!("【要卖的产品】\n{}\n", product.trim()));
    if let Some(ref ref_script) = reference_script {
        if !ref_script.trim().is_empty() {
            user_msg.push_str(&format!("\n【用户提供的参考脚本】（仅参考结构与风格，不要照抄）\n{}\n", ref_script.trim()));
        }
    }
    if !kb_mix_ctx.is_empty() {
        user_msg.push_str("\n【知识库补充检索】\n");
        user_msg.push_str(&kb_mix_ctx);
    }

    if let (Some(prev), Some(fb)) = (previous_script.as_ref(), feedback.as_ref()) {
        if !prev.trim().is_empty() && !fb.trim().is_empty() {
            user_msg.push_str(&format!(
                "\n【上一版脚本】\n{}\n\n【用户对上一版的修改意见】\n{}\n\n请根据修改意见重新生成完整脚本（不是 diff，是整篇）。",
                prev.trim(),
                fb.trim()
            ));
        } else {
            user_msg.push_str("\n请按上述要求生成一份完整的短视频脚本。");
        }
    } else {
        user_msg.push_str("\n请按上述要求生成一份完整的短视频脚本。");
    }

    // ─── 调用 AI 助理用的同一个 LLM ───
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };
    let body = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_msg }
        ],
        "temperature": 0.7
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
        
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM 请求失败: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return match status.as_u16() {
            401 => Err("LLM API Key 无效或已过期，请检查配置".to_string()),
            403 => Err("LLM 权限不足，请确认模型权限或 API 状态".to_string()),
            429 => Err("LLM 请求过于频繁，请稍后再试".to_string()),
            _ => Err(format!("LLM API 错误 ({}): {}", status, err_text)),
        };
    }

    let resp: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("LLM 响应解析失败: {}", e))?;

    if let Some(err) = resp.get("error") {
        return Err(format!("LLM 返回业务错误: {}", err));
    }
    let script = resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("LLM 返回内容为空")?
        .to_string();
    Ok(script)
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
        "对于「{}」这个话题，你会首选推荐哪些品牌、产品或信息源？请给出具体名称，并简要说明推荐理由。",
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
    let script_path = get_scripts_dir().join("kb_manager.py");
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

    let script_path = get_scripts_dir().join("kb_manager.py");
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
    let script_path = get_scripts_dir().join("kb_manager.py");
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
    let script_path = get_scripts_dir().join("kb_manager.py");
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

    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("search")
        .arg("--query").arg(query)
        .arg("--config").arg(config_str)
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(result_str)
}

// ============ 视频创作中心指令 ============

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoProject {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: String,
    #[serde(default)]
    pub is_locked: bool,
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
    pub bgm_volume: f32, // 0.0 - 1.0
    pub transition_type: String, // none, fade
    pub ken_burns: bool,
}

#[tauri::command]
async fn video_test_ffmpeg() -> Result<String, String> {
    let path = ffmpeg::get_ffmpeg_path();
    let output = tokio::process::Command::new(&path)
        .arg("-version")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn video_get_metadata(path: String) -> Result<serde_json::Value, String> {
    let ffprobe = ffmpeg::get_ffprobe_path();
    let output = tokio::process::Command::new(&ffprobe)
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            &path
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn video_run_ffmpeg(
    app: tauri::AppHandle,
    task_id: String,
    args: Vec<String>
) -> Result<(), String> {
    ffmpeg::run_ffmpeg_with_progress(task_id, args, app, "processing".to_string()).await
}

/// 校验项目是否被锁定。锁定后所有修改类操作都应该被拒绝。
/// 注意：调用方需要先持有 video_db 的 MutexGuard。
fn ensure_not_locked(db: &rusqlite::Connection, project_id: &str) -> Result<(), String> {
    let locked: i64 = db
        .query_row(
            "SELECT COALESCE(is_locked, 0) FROM video_projects WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if locked != 0 {
        return Err("项目已完成并锁定，无法修改。如需调整请克隆为新项目。".to_string());
    }
    Ok(())
}

/// 锁定项目：标记完成、设置 locked_at、记录最终视频路径。
#[tauri::command]
async fn video_lock_project(
    state: State<'_, AppState>,
    id: String,
    final_video_path: Option<String>,
) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE video_projects
         SET is_locked = 1,
             locked_at = CURRENT_TIMESTAMP,
             status = 'completed',
             final_video_path = COALESCE(?2, final_video_path),
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        rusqlite::params![&id, &final_video_path],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// 克隆项目：把现有项目（含 config）复制一份，但去掉"内容产物"字段，作为新草稿。
/// 不复制 materials（用户主动决定要不要带过去）。
#[tauri::command]
async fn video_clone_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<VideoProject, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;

    // 先读出来
    let mut stmt = db.prepare(
        "SELECT title, description, config FROM video_projects WHERE id = ?1"
    ).map_err(|e| e.to_string())?;
    let (title, description, config_str): (String, Option<String>, Option<String>) = stmt
        .query_row([&id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| format!("源项目不存在: {}", e))?;

    // 解析并清洗 config —— 去掉已生成的脚本/视频路径，保留产品信息和平台选择
    let mut config_val: serde_json::Value = config_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    if let Some(script_obj) = config_val.get_mut("script").and_then(|v| v.as_object_mut()) {
        script_obj.remove("generatedScript");
        script_obj.remove("generationPrompt");
        script_obj.insert("scriptConfirmed".into(), serde_json::json!(false));
        // 保留 productInfo / referenceScript / videoRatio / platform / scriptType
    }

    let new_id = Uuid::new_v4().to_string();
    let new_title = format!("{}（副本）", title);
    let new_config_str = serde_json::to_string(&config_val).unwrap_or_default();

    db.execute(
        "INSERT INTO video_projects (id, title, description, config, status, is_locked)
         VALUES (?1, ?2, ?3, ?4, 'draft', 0)",
        rusqlite::params![&new_id, &new_title, &description, &new_config_str],
    ).map_err(|e| e.to_string())?;

    Ok(VideoProject {
        id: new_id,
        title: new_title,
        description,
        config: Some(config_val),
        status: "draft".to_string(),
        is_locked: false,
        locked_at: None,
        final_video_path: None,
        created_at: None,
        updated_at: None,
    })
}

#[tauri::command]
async fn video_list_projects(state: State<'_, AppState>) -> Result<Vec<VideoProject>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, title, description, config, status, is_locked, locked_at, final_video_path, created_at, updated_at
         FROM video_projects ORDER BY updated_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let config_str: Option<String> = row.get(3)?;
        let config = config_str.and_then(|s| serde_json::from_str(&s).ok());
        let locked_raw: Option<i64> = row.get(5).ok();
        Ok(VideoProject {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            config,
            status: row.get(4)?,
            is_locked: locked_raw.unwrap_or(0) != 0,
            locked_at: row.get(6).ok(),
            final_video_path: row.get(7).ok(),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut projects = Vec::new();
    for row in rows {
        projects.push(row.map_err(|e| e.to_string())?);
    }
    Ok(projects)
}

#[tauri::command]
async fn video_upsert_project(state: State<'_, AppState>, project: VideoProject) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;

    // 已锁定项目只允许更新标题/描述这些元数据，不允许改 config / status
    let locked: i64 = db
        .query_row(
            "SELECT COALESCE(is_locked, 0) FROM video_projects WHERE id = ?1",
            [&project.id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if locked != 0 {
        // 锁定后仅更新 title / description
        db.execute(
            "UPDATE video_projects SET title=?2, description=?3, updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            rusqlite::params![&project.id, &project.title, &project.description],
        ).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let config_json = project.config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
    db.execute(
        "INSERT INTO video_projects (id, title, description, config, status, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
            title=excluded.title,
            description=excluded.description,
            config=excluded.config,
            status=excluded.status,
            updated_at=CURRENT_TIMESTAMP",
        (project.id, project.title, project.description, config_json, project.status),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn video_delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    
    // 1. 删除关联的任务
    db.execute("DELETE FROM video_tasks WHERE project_id = ?1", [&id]).map_err(|e| e.to_string())?;
    
    // 2. 删除关联的素材
    db.execute("DELETE FROM video_materials WHERE project_id = ?1", [&id]).map_err(|e| e.to_string())?;
    
    // 3. 删除项目本身
    db.execute("DELETE FROM video_projects WHERE id = ?1", [&id]).map_err(|e| e.to_string())?;
    
    // 4. 清理磁盘文件
    let data_dir = get_data_dir().join("video_studio");
    let dirs_to_clean = vec!["materials", "voiceovers", "output"];
    for sub in dirs_to_clean {
        let path = data_dir.join(sub).join(&id);
        if path.exists() {
            let _ = fs::remove_dir_all(path);
        }
    }
    
    Ok(())
}

/// 通用 AI 错误映射：将原始 API 报错或 Python 报错转换为用户友好的中文提示。
/// 从 Python provider_errors 输出的 JSON 中提取最友好的错误描述。
/// Python 端结构（provider_errors.classify_exception 的输出）：
///   { "status": "error", "error": "...", "error_code": "AUTH|RATE_LIMIT|...", "http_status": 401, "details": "..." }
/// 优先用 "error"（已是中文友好提示），失败时回退到 map_ai_error 兜底关键词匹配。
fn extract_provider_error(res: &serde_json::Value, fallback_label: &str) -> String {
    if let Some(msg) = res.get("error").and_then(|v| v.as_str()) {
        let code = res.get("error_code").and_then(|v| v.as_str()).unwrap_or("");
        let status = res.get("http_status").and_then(|v| v.as_i64()).unwrap_or(0);
        // 已经是 provider_errors 输出 → 直接给前端清晰文案
        if !code.is_empty() {
            return if status > 0 {
                format!("[{}] {} (HTTP {})", code, msg, status)
            } else {
                format!("[{}] {}", code, msg)
            };
        }
        // 老版 Provider 直接 str(e) 的情况，过一遍关键词映射
        return map_ai_error(msg);
    }
    format!("{}（未知错误）", fallback_label)
}

fn map_ai_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("401") || lower.contains("unauthorized") || lower.contains("invalid_api_key") {
        "API Key 无效或已过期，请检查设置中的配置".to_string()
    } else if lower.contains("403") || lower.contains("forbidden") || lower.contains("not allowed") {
        "权限不足，请确认您的账号有权访问该模型或 API".to_string()
    } else if lower.contains("429") || lower.contains("too many requests") || lower.contains("rate limit") {
        "请求过于频繁（限流），请稍后再试".to_string()
    } else if lower.contains("not supported on /v1/images/generations") || lower.contains("not supported on /v1/images/edits") {
        "中转站不支持该模型用于图片生成。请在设置中将模型名改为 'dall-e-3' 或中转站建议的名字".to_string()
    } else if lower.contains("insufficient_quota") || lower.contains("billing_hard_limit") || lower.contains("quota exceeded") {
        "API 额度不足或账号欠费，请前往 Provider 后台检查".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "连接超时，请检查网络设置或稍后重试".to_string()
    } else {
        raw.to_string()
    }
}

#[tauri::command]
async fn video_start_generation(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
    provider: String,
    api_key: String,
    mode: String,
    ratio: String,
    base_url: Option<String>,
    model: Option<String>,
    reference_material_id: Option<String>,   // ★ 新增：素材库中作为参考图的素材 ID
) -> Result<String, String> {
    // 查参考图路径（如果有）
    let reference_path: Option<String> = if let Some(mid) = reference_material_id.as_deref() {
        if mid.is_empty() {
            None
        } else {
            let db = state.video_db.lock().map_err(|e| e.to_string())?;
            ensure_not_locked(&db, &project_id)?;
            let row: Option<(Option<String>, String)> = db
                .query_row(
                    "SELECT local_path, type FROM video_materials WHERE id = ?1 AND project_id = ?2",
                    [mid, &project_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();
            match row {
                Some((Some(p), t)) if t == "image" => Some(p),
                Some((_, t)) if t != "image" => return Err(format!("参考图必须是图片素材，但选中的是 {}", t)),
                _ => return Err("参考图素材不存在".to_string()),
            }
        }
    } else {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        ensure_not_locked(&db, &project_id)?;
        None
    };

    let scripts = get_scripts_dir();
    let manager_py = scripts.join("video_manager.py");

    // 模式逻辑：有参考图 → 强制 image 模式；无参考图 → 用传入的 mode（通常是 text）
    let effective_mode = if reference_path.is_some() {
        "image".to_string()
    } else {
        mode.clone()
    };

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("start")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--prompt").arg(&prompt)
        .arg("--mode").arg(&effective_mode)
        .arg("--ratio").arg(&ratio);

    if let Some(ref p) = reference_path {
        cmd.arg("--image-url").arg(p);
    }

    if let Some(url) = base_url {
        if !url.is_empty() {
            cmd.arg("--base-url").arg(url);
        }
    }
    if let Some(m) = model {
        if !m.is_empty() {
            cmd.arg("--model").arg(m);
        }
    }

    let output = cmd.output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("Python error: {}", stdout))?;
    
    if res["status"] == "error" {
        return Err(extract_provider_error(&res, "AI 视频生成失败"));
    }

    let task_id = res["task_id"].as_str().ok_or("No task_id returned")?.to_string();

    // 存入本地数据库
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO video_tasks (id, project_id, type, status) VALUES (?1, ?2, ?3, ?4)",
        (&task_id, &project_id, "generation", "processing"),
    ).map_err(|e| e.to_string())?;

    Ok(task_id)
}

#[tauri::command]
async fn video_poll_task_status(
    state: State<'_, AppState>,
    task_id: String,
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let scripts = get_scripts_dir();
    let manager_py = scripts.join("video_manager.py");

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("poll")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--task-id").arg(&task_id);

    if let Some(url) = base_url {
        if !url.is_empty() {
            cmd.arg("--base-url").arg(url);
        }
    }
    if let Some(m) = model {
        if !m.is_empty() {
            cmd.arg("--model").arg(m);
        }
    }

    let output = cmd.output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("Python error: {}", stdout))?;

    // 如果状态是 error，覆写为分类后的友好信息
    if res["status"] == "error" {
        let friendly = extract_provider_error(&res, "任务查询失败");
        res["error"] = serde_json::json!(friendly);
    }

    // 更新数据库状态
    let status = res["status"].as_str().unwrap_or("processing");
    let result_url = res["video_url"].as_str();
    let error_msg = res["error"].as_str();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE video_tasks SET status=?1, result_path=?2, error_msg=?3, updated_at=CURRENT_TIMESTAMP WHERE id=?4",
        (status, result_url, error_msg, &task_id),
    ).map_err(|e| e.to_string())?;

    Ok(res)
}

#[tauri::command]
async fn video_list_materials(state: State<'_, AppState>, project_id: String) -> Result<Vec<VideoMaterial>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, project_id, type, local_path, remote_url, meta, source, created_at
         FROM video_materials WHERE project_id = ?1 ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([project_id], |row| {
        let meta_str: Option<String> = row.get(5)?;
        let meta = meta_str.and_then(|s| serde_json::from_str(&s).ok());
        let source: Option<String> = row.get(6).ok();
        Ok(VideoMaterial {
            id: row.get(0)?,
            project_id: row.get(1)?,
            material_type: row.get(2)?,
            local_path: row.get(3)?,
            remote_url: row.get(4)?,
            meta,
            source: source.unwrap_or_else(|| "uploaded".to_string()),
            created_at: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut materials = Vec::new();
    for row in rows {
        materials.push(row.map_err(|e| e.to_string())?);
    }
    Ok(materials)
}

// ============ TTS 语音合成 ============

#[tauri::command]
async fn tts_list_voices(
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let scripts = get_scripts_dir();
    let manager_py = scripts.join("tts_manager.py");

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("list-voices")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key);
    if let Some(u) = base_url.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--base-url").arg(u);
    }
    if let Some(m) = model.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--model").arg(m);
    }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err(format!("TTS list-voices 无输出: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|_| format!("TTS list-voices 返回非 JSON: {}", stdout))?;
    Ok(val)
}

/// 合成语音到本地文件，返回保存路径。
/// 文件落到 <data_dir>/video_studio/voiceovers/<project_id>/voice_<id8>.mp3
/// 同时插入 video_materials 表（type='audio', source='ai-generated'）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn tts_synthesize(
    state: State<'_, AppState>,
    project_id: String,
    text: String,
    voice_id: String,
    speed: f32,
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        ensure_not_locked(&db, &project_id)?;
    }
    if text.trim().is_empty() {
        return Err("文本不能为空".to_string());
    }

    let save_dir = get_data_dir()
        .join("video_studio")
        .join("voiceovers")
        .join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    let material_id = Uuid::new_v4().to_string();
    let filename = format!("voice_{}.mp3", &material_id[..8]);
    let save_path = save_dir.join(&filename);
    let save_path_str = save_path.to_string_lossy().to_string();

    let scripts = get_scripts_dir();
    let manager_py = scripts.join("tts_manager.py");

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("synthesize")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--text").arg(&text)
        .arg("--voice").arg(&voice_id)
        .arg("--speed").arg(format!("{}", speed))
        .arg("--output").arg(&save_path_str);
    if let Some(u) = base_url.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--base-url").arg(u);
    }
    if let Some(m) = model.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--model").arg(m);
    }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err(format!("TTS 无输出: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let res: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|_| format!("TTS 返回非 JSON: {}", stdout))?;
    if res["status"] == "error" {
        return Err(extract_provider_error(&res, "TTS 合成失败"));
    }
    let audio_path = res["audio_path"].as_str().unwrap_or(&save_path_str).to_string();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let meta_json = serde_json::json!({
        "voice_id": voice_id,
        "speed": speed,
        "provider": provider,
        "text_length": text.chars().count(),
    }).to_string();
    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, remote_url, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &material_id,
            &project_id,
            &"audio".to_string(),
            &audio_path,
            &Option::<String>::None,
            &meta_json,
            &"ai-generated".to_string(),
        ),
    ).map_err(|e| format!("写入数据库失败: {}", e))?;

    Ok(audio_path)
}

/// AI 文生图：调 image_manager.py → 下载到项目素材目录 → 入库（source='ai-generated'）。
///
/// 入参：
///   - provider:      "fal" | "volcengine" | "openai" | "mock"
///   - prompt:        提示词（必填）
///   - size:          "1024x1024" / "720x1280" 等
///   - reference_image_path: 可选，传本地路径或 URL（图生图）
///   - api_key / base_url / model: 各 Provider 配置
/// 返回：新插入的 material_id
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn video_generate_image(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
    size: String,
    provider: String,
    api_key: String,
    reference_image_path: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        ensure_not_locked(&db, &project_id)?;
    }

    if prompt.trim().is_empty() {
        return Err("提示词不能为空".to_string());
    }

    let scripts = get_scripts_dir();
    let manager_py = scripts.join("image_manager.py");

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--prompt").arg(&prompt)
        .arg("--size").arg(&size);

    if let Some(p) = reference_image_path.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--reference-image").arg(p);
    }
    if let Some(u) = base_url.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--base-url").arg(u);
    }
    if let Some(m) = model.as_ref().filter(|s| !s.is_empty()) {
        cmd.arg("--model").arg(m);
    }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stdout.is_empty() {
        return Err(format!("图片生成无输出 (exit={:?}): {}", output.status.code(), stderr));
    }

    let res: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|_| format!("图片生成返回非 JSON: {}", stdout))?;
    if res["status"] == "error" {
        return Err(extract_provider_error(&res, "AI 图片生成失败"));
    }
    let image_url = res["image_url"].as_str().ok_or("缺少 image_url 字段")?.to_string();

    // ─── 下载到项目目录 ───
    let material_id = Uuid::new_v4().to_string();
    let save_dir = get_data_dir()
        .join("video_studio")
        .join("materials")
        .join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| format!("创建素材目录失败: {}", e))?;

    let (bytes, ext) = if let Some(b64) = image_url.strip_prefix("data:") {
        // data:image/png;base64,XXXX
        let comma = b64.find(',').ok_or("data URL 格式错误")?;
        let header = &b64[..comma];
        let payload = &b64[comma + 1..];
        let mime = header.split(';').next().unwrap_or("image/png");
        let ext = if mime.contains("jpeg") || mime.contains("jpg") { "jpg" }
                  else if mime.contains("webp") { "webp" }
                  else { "png" };
        use base64_engine::{engine::general_purpose::STANDARD, Engine};
        let bytes = STANDARD.decode(payload).map_err(|e| format!("base64 解码失败: {}", e))?;
        (bytes, ext.to_string())
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build().map_err(|e| e.to_string())?;
        let resp = client.get(&image_url).send().await
            .map_err(|e| format!("下载图片失败: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("下载图片 HTTP {}", resp.status()));
        }
        // 从 URL 推断扩展名
        let ext = image_url
            .split('?').next().unwrap_or("")
            .rsplit('.').next().unwrap_or("png");
        let ext = if ["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.to_lowercase().as_str()) {
            ext.to_lowercase()
        } else {
            "png".to_string()
        };
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();
        (bytes, ext)
    };

    let filename = format!("ai_{}_{}.{}", &material_id[..8], chrono_like_now(), ext);
    let save_path = save_dir.join(&filename);
    fs::write(&save_path, bytes).map_err(|e| format!("写入图片文件失败: {}", e))?;
    let local_path_str = save_path.to_string_lossy().to_string();

    // 入库
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let meta_json = serde_json::json!({ "prompt": prompt, "size": size, "provider": provider }).to_string();
    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, remote_url, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &material_id,
            &project_id,
            &"image".to_string(),
            &local_path_str,
            &Option::<String>::None,
            &meta_json,
            &"ai-generated".to_string(),
        ),
    ).map_err(|e| format!("写入数据库失败: {}", e))?;

    Ok(material_id)
}

/// 用一个时间戳前缀让生成的图片文件名不重复
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{}", secs)
}

/// 用户主动上传素材：把本地任意路径的图片/视频复制进项目素材目录。
///
/// - `source_path`: 用户在文件对话框选中的绝对路径（图片或视频）
/// - `material_type`: 上层 Vue 端判断好的 "image" / "video"
/// - 复制到 `<data_dir>/video_studio/materials/<project_id>/uploaded_<id8>_<原文件名>`
/// - 插入 `video_materials` 表（`remote_url` 留空表示本地上传）
#[tauri::command]
async fn video_upload_material(
    state: State<'_, AppState>,
    project_id: String,
    source_path: String,
    material_type: String,
) -> Result<String, String> {
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        ensure_not_locked(&db, &project_id)?;
    }

    let src = PathBuf::from(&source_path);
    if !src.exists() {
        return Err(format!("源文件不存在: {}", source_path));
    }
    let filename = src.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("uploaded")
        .to_string();

    let material_id = Uuid::new_v4().to_string();
    let save_dir = get_data_dir()
        .join("video_studio")
        .join("materials")
        .join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| format!("创建素材目录失败: {}", e))?;

    let local_filename = format!("uploaded_{}_{}", &material_id[..8], filename);
    let save_path = save_dir.join(&local_filename);

    // 大文件用流式拷贝，避免一次性读到内存
    fs::copy(&src, &save_path).map_err(|e| format!("拷贝文件失败: {}", e))?;

    let local_path_str = save_path.to_string_lossy().to_string();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, remote_url, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            &material_id,
            &project_id,
            &material_type,
            &local_path_str,
            &Option::<String>::None,
            &"uploaded".to_string(),
        ),
    ).map_err(|e| format!("写入数据库失败: {}", e))?;

    Ok(local_path_str)
}

/// 删除一条素材记录 + 磁盘文件（若文件还在项目目录里）。
#[tauri::command]
async fn video_delete_material(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;

    // 先查 local_path + 所属 project_id（用于锁定校验）
    let row: Option<(Option<String>, String)> = db
        .query_row(
            "SELECT local_path, project_id FROM video_materials WHERE id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (local_path, project_id) = match row {
        Some((p, pid)) => (p, pid),
        None => return Err("素材不存在".to_string()),
    };

    // 锁定项目不允许删素材
    ensure_not_locked(&db, &project_id)?;

    db.execute("DELETE FROM video_materials WHERE id = ?1", [&id])
        .map_err(|e| format!("删除记录失败: {}", e))?;

    // 仅删除我们项目目录里的文件，避免误删用户原始文件
    if let Some(p) = local_path {
        let path = PathBuf::from(&p);
        let video_studio_root = get_data_dir().join("video_studio");
        if path.starts_with(&video_studio_root) && path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

#[tauri::command]
async fn video_download_material(
    state: State<'_, AppState>,
    project_id: String,
    url: String,
    material_type: String,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    // 确定保存路径: AUTOCAST_DATA_DIR/video_studio/materials/{project_id}/{filename}
    let filename = url.split('/').last().unwrap_or("video.mp4").split('?').next().unwrap_or("video.mp4");
    let material_id = Uuid::new_v4().to_string();
    let save_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;
    
    // 为了防止重名，带上 ID
    let local_filename = format!("{}_{}", &material_id[..8], filename);
    let save_path = save_dir.join(&local_filename);
    
    let content = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&save_path, content).map_err(|e| e.to_string())?;
    
    let local_path_str = save_path.to_string_lossy().to_string();

    // 记录到数据库（远程下载的视频来自 AI 生成 → source='ai-generated'）
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, remote_url, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (&material_id, &project_id, &material_type, &local_path_str, &url, &"ai-generated".to_string()),
    ).map_err(|e| e.to_string())?;

    Ok(local_path_str)
}

#[tauri::command]
async fn video_list_tasks(state: State<'_, AppState>, project_id: Option<String>) -> Result<Vec<VideoTask>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    
    let mut tasks = Vec::new();
    
    if let Some(pid) = project_id {
        let mut stmt = db.prepare("SELECT id, project_id, type, status, progress, result_path, error_msg, created_at, updated_at FROM video_tasks WHERE project_id = ?1 ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map([pid], |row| {
            Ok(VideoTask {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_type: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                result_path: row.get(5)?,
                error_msg: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        for row in rows {
            tasks.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let mut stmt = db.prepare("SELECT id, project_id, type, status, progress, result_path, error_msg, created_at, updated_at FROM video_tasks ORDER BY created_at DESC LIMIT 50")
            .map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map([], |row| {
            Ok(VideoTask {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_type: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                result_path: row.get(5)?,
                error_msg: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        for row in rows {
            tasks.push(row.map_err(|e| e.to_string())?);
        }
    }

    Ok(tasks)
}

async fn get_video_duration(path: &str) -> Result<f64, String> {
    let ffprobe = ffmpeg::get_ffprobe_path();
    let output = tokio::process::Command::new(&ffprobe)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path
        ])
        .output()
        .await
        .map_err(|e| format!("无法执行 ffprobe ({})，请确保已安装 FFmpeg 环境: {}", ffprobe, e))?;

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    s.parse::<f64>().map_err(|e| format!("Failed to parse duration '{}': {}", s, e))
}

#[tauri::command]
async fn video_render_advanced(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    video_paths: Vec<String>,
    bgm_path: Option<String>,
    config: RenderConfig,
) -> Result<String, String> {
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        ensure_not_locked(&db, &project_id)?;
    }
    if video_paths.is_empty() {
        return Err("No video clips provided".to_string());
    }

    let task_id = format!("render_{}", &Uuid::new_v4().to_string()[..8]);
    let project_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&project_dir).map_err(|e| e.to_string())?;

    let output_path = project_dir.join(format!("{}_final.mp4", task_id));
    let output_path_str = output_path.to_string_lossy().to_string();

    // 1. 获取所有片段的时长 (用于转场计算)
    let mut durations = Vec::new();
    for path in &video_paths {
        durations.push(get_video_duration(path).await?);
    }

    // 2. 构建 FFmpeg 参数
    let mut args = Vec::new();
    for path in &video_paths {
        args.push("-i".to_string());
        args.push(path.clone());
    }
    if let Some(ref bgm) = bgm_path {
        args.push("-i".to_string());
        args.push(bgm.clone());
    }

    // 3. 构建滤镜链
    let mut filter_complex = String::new();
    let clip_count = video_paths.len();
    let trans_dur = 1.0; // 默认转场时长 1s

    // a. 预处理：缩放、裁剪及可选的 Ken Burns 效果
    for i in 0..clip_count {
        let base_v = format!("pre{}", i);
        filter_complex.push_str(&format!(
            "[{}:v]scale={}:{}:force_original_aspect_ratio=increase,crop={0}:{1}:(iw-{0})/2:(ih-{1})/2,setsar=1[{}]",
            i, config.width, config.height, base_v
        ));
        
        if config.ken_burns {
            // Ken Burns: 缓慢放大 1.0 -> 1.1
            filter_complex.push_str(&format!(
                ";[{}]zoompan=z='min(zoom+0.0015,1.1)':d=1:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':s={}x{}[v{}]",
                base_v, config.width, config.height, i
            ));
        } else {
            filter_complex.push_str(&format!(";[{}]null[v{}]", base_v, i));
        }
        filter_complex.push_str(";");
    }

    // b. 视频转场与拼接
    if config.transition_type != "none" && clip_count > 1 {
        let mut last_v = "v0".to_string();
        let mut cumulative_dur = durations[0];
        
        for i in 1..clip_count {
            let offset = cumulative_dur - trans_dur;
            let next_v = format!("v_trans{}", i);
            filter_complex.push_str(&format!(
                "[{}][v{}]xfade=transition={}:duration={}:offset={}[{}];",
                last_v, i, config.transition_type, trans_dur, offset, next_v
            ));
            last_v = next_v;
            cumulative_dur = cumulative_dur + durations[i] - trans_dur;
        }
        filter_complex.push_str(&format!("[{}]null[v_final];", last_v));

        // 音频转场 (使用 acrossfade)
        let mut last_a = "0:a".to_string();
        for i in 1..clip_count {
            let next_a = format!("a_trans{}", i);
            filter_complex.push_str(&format!(
                "[{}][{}:a]acrossfade=d={}:c1=tri:c2=tri[{}];",
                last_a, i, trans_dur, next_a
            ));
            last_a = next_a;
        }
        filter_complex.push_str(&format!("[{}]anull[a_full];", last_a));
    } else {
        // 无转场，直接 concat
        let mut concat_inputs = String::new();
        for i in 0..clip_count {
            concat_inputs.push_str(&format!("[v{}][{}:a]", i, i));
        }
        filter_complex.push_str(&format!("{}concat=n={}:v=1:a=1[v_final][a_full];", concat_inputs, clip_count));
    }

    // c. BGM 混音 (Audio Ducking)
    if bgm_path.is_some() {
        let bgm_idx = clip_count;
        filter_complex.push_str(&format!(
            "[{}:a]volume={}[bgm_pre];\
             [bgm_pre][a_full]sidechaincompress=threshold=0.1:ratio=20:attack=100:release=1000[bgm_ducked];\
             [a_full][bgm_ducked]amix=inputs=2:duration=first[a_final]",
            bgm_idx, config.bgm_volume
        ));
    } else {
        filter_complex.push_str("[a_full]anull[a_final]");
    }

    args.push("-filter_complex".to_string());
    args.push(filter_complex);
    args.push("-map".to_string()); args.push("[v_final]".to_string());
    args.push("-map".to_string()); args.push("[a_final]".to_string());
    
    // 编码设置
    args.push("-c:v".to_string());
    args.push("libx264".to_string());
    args.push("-preset".to_string());
    args.push("veryfast".to_string());
    args.push("-crf".to_string());
    args.push("23".to_string());
    args.push("-c:a".to_string());
    args.push("aac".to_string());
    args.push("-shortest".to_string()); // 以视频长度为准
    args.push("-y".to_string());
    args.push(output_path_str.clone());

    // 3. 记录任务
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO video_tasks (id, project_id, type, status) VALUES (?1, ?2, ?3, ?4)",
            (&task_id, &project_id, "export", "processing"),
        ).map_err(|e| e.to_string())?;
    }

    // 4. 异步执行
    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    tauri::async_runtime::spawn(async move {
        match ffmpeg::run_ffmpeg_with_progress(task_id_clone.clone(), args, app_clone.clone(), "rendering".to_string()).await {
            Ok(_) => {
                let state = app_clone.state::<AppState>();
                let db = state.video_db.lock().unwrap();
                let _ = db.execute(
                    "UPDATE video_tasks SET status='completed', result_path=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    (&output_path_str, &task_id_clone),
                );
            }
            Err(e) => {
                {
                    let state = app_clone.state::<AppState>();
                    let db = state.video_db.lock().unwrap();
                    let _ = db.execute(
                        "UPDATE video_tasks SET status='error', error_msg=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                        (&e, &task_id_clone),
                    );
                }
            }
        }
    });

    Ok(task_id)
}

/// 导出合成管线（不锁定项目）：用户选择1个音频+多张图片/多个视频，按音频时长拼接。
#[tauri::command]
async fn video_export_render(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    audio_path: String,
    image_paths: Vec<String>,
    video_paths: Vec<String>,
    config: RenderConfig,
) -> Result<String, String> {
    let mut all_visual_paths: Vec<String> = Vec::new();
    all_visual_paths.extend(image_paths);
    all_visual_paths.extend(video_paths);

    if all_visual_paths.is_empty() {
        return Err("请至少选择一个视觉素材".to_string());
    }

    let audio_duration = get_video_duration(&audio_path).await?;

    let task_id = format!("export_{}", &Uuid::new_v4().to_string()[..8]);
    let output_dir = get_data_dir().join("video_studio").join("output").join(&project_id);
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let output_path = output_dir.join(format!("export_{}.mp4", &task_id[..8]));
    let output_path_str = output_path.to_string_lossy().to_string();

    let n = all_visual_paths.len();

    // 构建 FFmpeg 参数
    let mut args = Vec::new();
    for path in &all_visual_paths {
        args.push("-i".to_string());
        args.push(path.clone());
    }
    args.push("-i".to_string());
    args.push(audio_path.clone());

    // 滤镜链：图片最多3秒，视频循环
    let mut filter_complex = String::new();
    for i in 0..n {
        let path = &all_visual_paths[i];
        let is_image = path.to_lowercase().ends_with(".png")
            || path.to_lowercase().ends_with(".jpg")
            || path.to_lowercase().ends_with(".jpeg")
            || path.to_lowercase().ends_with(".webp")
            || path.to_lowercase().ends_with(".gif")
            || path.to_lowercase().ends_with(".bmp");
        let base_v = format!("pre{}", i);
        if is_image {
            filter_complex.push_str(&format!(
                "[{}:v]scale={}:{}:force_original_aspect_ratio=increase,crop={1}:{2}:(iw-{1})/2:(ih-{2})/2,setsar=1,trim=duration=3,setpts=PTS-STARTPTS[{}];",
                i, config.width, config.height, base_v
            ));
        } else {
            filter_complex.push_str(&format!(
                "[{}:v]scale={}:{}:force_original_aspect_ratio=increase,crop={1}:{2}:(iw-{1})/2:(ih-{2})/2,setsar=1,loop=loop=-1:size=0:start=0[{}];",
                i, config.width, config.height, base_v
            ));
        }
        filter_complex.push_str(&format!(";[{}]null[v{}];", base_v, i));
    }

    let mut concat_inputs = String::new();
    for i in 0..n { concat_inputs.push_str(&format!("[v{}]", i)); }
    filter_complex.push_str(&format!(
        "{}concat=n={}:v=1:a=0[v_final];",
        concat_inputs, n
    ));

    let tts_idx = n;
    filter_complex.push_str(&format!("[{}:a]anull[a_final]", tts_idx));

    args.push("-filter_complex".to_string());
    args.push(filter_complex);
    args.push("-map".to_string()); args.push("[v_final]".to_string());
    args.push("-map".to_string()); args.push("[a_final]".to_string());
    args.push("-c:v".to_string()); args.push("libx264".to_string());
    args.push("-preset".to_string()); args.push("veryfast".to_string());
    args.push("-crf".to_string()); args.push("23".to_string());
    args.push("-c:a".to_string()); args.push("aac".to_string());
    args.push("-shortest".to_string());
    args.push("-y".to_string());
    args.push(output_path_str.clone());

    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO video_tasks (id, project_id, type, status) VALUES (?1, ?2, ?3, ?4)",
            (&task_id, &project_id, "export", "processing"),
        ).map_err(|e| e.to_string())?;
    }

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    let project_id_clone = project_id.clone();
    let final_path_clone = output_path_str.clone();

    tauri::async_runtime::spawn(async move {
        match ffmpeg::run_ffmpeg_with_progress(task_id_clone.clone(), args, app_clone.clone(), "rendering".to_string()).await {
            Ok(_) => {
                let state = app_clone.state::<AppState>();
                let db = state.video_db.lock().unwrap();
                let _ = db.execute(
                    "UPDATE video_tasks SET status='completed', result_path=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    (&final_path_clone, &task_id_clone),
                );
                let mat_id = Uuid::new_v4().to_string();
                let _ = db.execute(
                    "INSERT INTO video_materials (id, project_id, type, local_path, remote_url, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    (&mat_id, &project_id_clone, &"video".to_string(), &final_path_clone, &Option::<String>::None, &"{}".to_string(), &"exported".to_string()),
                );
            }
            Err(e) => {
                let state = app_clone.state::<AppState>();
                let db = state.video_db.lock().unwrap();
                let _ = db.execute(
                    "UPDATE video_tasks SET status='error', error_msg=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    (&e, &task_id_clone),
                );
            }
        }
    });

    Ok(task_id)
}


#[tauri::command]
async fn video_concat_materials(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    material_paths: Vec<String>,
) -> Result<String, String> {
    if material_paths.is_empty() {
        return Err("No materials to concat".to_string());
    }

    let task_id = format!("concat_{}", &Uuid::new_v4().to_string()[..8]);
    let project_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&project_dir).map_err(|e| e.to_string())?;

    // 1. 创建 FFmpeg concat 列表文件
    let list_path = project_dir.join(format!("{}_list.txt", task_id));
    let mut content = String::new();
    for path in &material_paths {
        // FFmpeg concat 格式: file '/path/to/file'
        content.push_str(&format!("file '{}'\n", path.replace("'", "'\\''")));
    }
    fs::write(&list_path, content).map_err(|e| e.to_string())?;

    // 2. 准备输出路径
    let output_filename = format!("{}_merged.mp4", task_id);
    let output_path = project_dir.join(&output_filename);
    let output_path_str = output_path.to_string_lossy().to_string();

    // 3. 记录任务到数据库
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO video_tasks (id, project_id, type, status) VALUES (?1, ?2, ?3, ?4)",
            (&task_id, &project_id, "editing", "processing"),
        ).map_err(|e| e.to_string())?;
    }

    // 4. 后台执行 FFmpeg
    // 使用 -c copy 是最快的方法，前提是素材格式一致。MVP 暂时使用 copy。
    let args = vec![
        "-f".to_string(), "concat".to_string(),
        "-safe".to_string(), "0".to_string(),
        "-i".to_string(), list_path.to_string_lossy().to_string(),
        "-c".to_string(), "copy".to_string(),
        "-y".to_string(), // 覆盖已存在
        output_path_str.clone(),
    ];

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    
    // 异步执行并更新数据库
    tauri::async_runtime::spawn(async move {
        match ffmpeg::run_ffmpeg_with_progress(task_id_clone.clone(), args, app_clone.clone(), "concatenating".to_string()).await {
            Ok(_) => {
                let state = app_clone.state::<AppState>();
                let db = state.video_db.lock().unwrap();
                let _ = db.execute(
                    "UPDATE video_tasks SET status='completed', result_path=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    (&output_path_str, &task_id_clone),
                );
                // 清理临时列表文件
                let _ = fs::remove_file(list_path);
            }
            Err(e) => {
                {
                    let state = app_clone.state::<AppState>();
                    let db = state.video_db.lock().unwrap();
                    let _ = db.execute(
                        "UPDATE video_tasks SET status='error', error_msg=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                        (&e, &task_id_clone),
                    );
                }
            }
        }
    });

    Ok(task_id)
}

/// 暴露给 HermesGatewayView 的 KB 搜索命令，返回格式化好的上下文字符串
#[tauri::command]
async fn hermes_search_kb(query: String) -> Result<String, String> {
    let raw = search_kb_internal(query).await?;
    let res: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
    let mut context = String::new();
    if let Some(arr) = res.as_array() {
        for item in arr.iter().take(6) {
            if let Some(text) = item["text"].as_str() {
                if !text.trim().is_empty() {
                    context.push_str(&format!("- {}\n", text.trim()));
                }
            }
        }
    }
    Ok(context)
}

// ============ 结果查询命令 ============

#[tauri::command]
async fn list_scraped_users() -> Result<serde_json::Value, String> {
    let script_path = get_scripts_dir().join("query_data.py");
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
    let script_path = get_scripts_dir().join("query_data.py");
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
    let script_path = get_scripts_dir().join("query_data.py");
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

    let script_path = get_scripts_dir().join("open_video.py");
    
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

// ============ 直播监控命令 ============

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

    let script_path = get_scripts_dir().join("douyin_live_monitor.py");
    
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

#[tauri::command]
async fn hermes_list_skills() -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin)
        .arg("skills")
        .arg("list")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut skills = vec![];

    // Simple parsing for the ASCII table
    for line in stdout.lines() {
        if line.starts_with('│') && !line.contains(" Name ") {
            let parts: Vec<&str> = line.split('│').collect();
            if parts.len() >= 6 {
                skills.push(serde_json::json!({
                    "name": parts[1].trim(),
                    "category": parts[2].trim(),
                    "source": parts[3].trim(),
                    "trust": parts[4].trim(),
                    "status": parts[5].trim(),
                }));
            }
        }
    }

    Ok(skills)
}

#[tauri::command]
async fn hermes_install_skill(name: String) -> Result<String, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin)
        .arg("skills")
        .arg("install")
        .arg(&name)
        .arg("--yes")   // Skip confirmation prompt (required in non-interactive mode)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn hermes_uninstall_skill(name: String) -> Result<String, String> {
    let hermes_bin = which_hermes();
    let output = tokio::process::Command::new(&hermes_bin)
        .arg("skills")
        .arg("uninstall")
        .arg(&name)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn hermes_list_tools() -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin)
        .arg("tools")
        .arg("--summary")
        .arg("list")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tools = vec![];

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.ends_with(':') { continue; }
        
        let enabled = line.contains("✓ enabled");
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Skip checkmark and status
        if parts.len() >= 3 {
            let name = parts[2];
            let description = parts[3..].join(" ");
            tools.push(serde_json::json!({
                "name": name,
                "enabled": enabled,
                "description": description,
                "keyword": format!("!{}", name)
            }));
        }
    }

    Ok(tools)
}

#[tauri::command]
async fn hermes_get_session_messages(session_id: String) -> Result<Vec<serde_json::Value>, String> {
    let hermes_bin = which_hermes();
    let output = std::process::Command::new(&hermes_bin)
        .arg("sessions")
        .arg("export")
        .arg("-")
        .arg("--session-id")
        .arg(&session_id)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().next() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(messages) = val.get("messages").and_then(|m| m.as_array()) {
                return Ok(messages.clone());
            }
        }
    }

    Ok(vec![])
}

#[tauri::command]
async fn hermes_toggle_skill_status(name: String, enable: bool) -> Result<(), String> {
    let hermes_bin = which_hermes();
    let action = if enable { "enable" } else { "disable" };
    let output = tokio::process::Command::new(&hermes_bin)
        .arg("skills")
        .arg(action)
        .arg(&name)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(if !stderr.is_empty() { stderr } else { stdout })
    }
}

#[tauri::command]
async fn hermes_toggle_tool_status(name: String, enable: bool) -> Result<(), String> {
    let hermes_bin = which_hermes();
    let action = if enable { "enable" } else { "disable" };
    let output = tokio::process::Command::new(&hermes_bin)
        .arg("tools")
        .arg(action)
        .arg(&name)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// 在 Finder 中显示文件（支持 macOS）
#[tauri::command]
async fn open_file_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 缓存打包资源目录，供 get_scripts_dir/python_executable 使用
            if let Ok(res_dir) = app.path().resource_dir() {
                let _ = RESOURCE_DIR.set(res_dir);
            }
            Ok(())
        })
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
            current_task_id: Mutex::new(None),
            video_db: Mutex::new(db::init_db(get_data_dir()).expect("Failed to init video database")),
        })
        .invoke_handler(tauri::generate_handler![
            autocast_diagnostics,
            get_config, save_config, get_default_config,
            list_kb_files, add_to_kb, delete_kb_file, get_kb_file_details,
            studio_generate_content,
            video_generate_script,
            analyze_comments, generate_live_reply, delete_scraped_user,
            list_chat_sessions, create_chat_session, delete_chat_session,
            send_chat_message, get_chat_messages,
            list_accounts, verify_account, delete_account,
            sync_local_accounts, init_login_session, get_login_status,
            finish_login, cleanup_login_session,
            start_scrape, get_scrape_progress, cancel_scrape,
            get_current_task, clear_current_task,
            list_scraped_users, get_scraped_videos, get_scraped_comments,
            open_video_in_browser, resolve_user_sec_uid,
            start_live_monitor, stop_live_monitor, get_active_monitors,
            get_live_history, resolve_live_url,
            geo_monitor_query,
            start_hermes_gateway, stop_hermes_gateway, check_hermes_status,
            check_hermes_gateway_health, list_hermes_sessions,
            hermes_enable_api_server, hermes_restart_service,
            hermes_read_api_key, hermes_set_api_key,
            hermes_send_message, hermes_list_runs, hermes_stop_run, hermes_approve_run,
            hermes_list_skills, hermes_install_skill, hermes_uninstall_skill, hermes_list_tools,
            hermes_get_session_messages,
            hermes_toggle_skill_status, hermes_toggle_tool_status,
            hermes_search_kb,
            video_test_ffmpeg, video_get_metadata, video_run_ffmpeg,
            video_list_projects, video_upsert_project, video_delete_project, video_start_generation, video_poll_task_status,
            video_lock_project, video_clone_project,
            video_list_materials, video_download_material,
            video_upload_material, video_delete_material,
            video_generate_image,
            tts_list_voices, tts_synthesize,
            video_list_tasks, video_concat_materials,
            video_render_advanced, video_export_render,
            open_file_in_finder,
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
