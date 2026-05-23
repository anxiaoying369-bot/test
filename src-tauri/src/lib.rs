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

    let child = tokio::process::Command::new("python3")
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

    let output = tokio::process::Command::new("python3")
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

/// 启动采集任务
#[tauri::command]
async fn start_scrape(
    account_name: String,
    platform: String,
    sec_uid: String,
    scrape_type: String,   // video | comment | reply | all
    limit: i32,
    skip_existing: bool,
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
    let mut cmd = tokio::process::Command::new("python3");
    cmd.arg(&script_path)
        .arg("--task-id").arg(&task_id)
        .arg("--cookie-path").arg(&cookie_file)
        .arg("--sec-uid").arg(&sec_uid)
        .arg("--type").arg(&scrape_type)
        .arg("--limit").arg(limit.to_string());
    if skip_existing {
        cmd.arg("--skip-existing");
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

// ============ 结果查询命令 ============

#[tauri::command]
async fn list_scraped_users() -> Result<serde_json::Value, String> {
    let script_path = PathBuf::from("..").join("scripts").join("query_data.py");
    let output = tokio::process::Command::new("python3")
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
    let output = tokio::process::Command::new("python3")
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
    let mut cmd = tokio::process::Command::new("python3");
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
    
    let mut cmd = tokio::process::Command::new("python3");
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
    
    let mut child = tokio::process::Command::new("python3")
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
    if let Some(mut child) = handles.remove(&key) {
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
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
            current_task_id: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            list_accounts, verify_account, delete_account,
            sync_local_accounts, init_login_session, get_login_status,
            finish_login, cleanup_login_session,
            start_scrape, get_scrape_progress, cancel_scrape,
            get_current_task, clear_current_task,
            list_scraped_users, get_scraped_videos, get_scraped_comments,
            open_video_in_browser,
            start_live_monitor, stop_live_monitor, get_active_monitors,
            get_live_history
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
