use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

// ============ 状态管理 ============
pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
    /// 活跃的 Python 登录进程（session_id -> Child）
    /// 存到这里防止 drop 时自动 kill
    pub process_handles: Mutex<std::collections::HashMap<String, tokio::process::Child>>,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub platform: String,
    pub user_id: Option<String>,
    pub name: String,
    pub status: String,
    pub cookie_file: String,
    pub avatar: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct AccountsStore {
    accounts: Vec<Account>,
}

impl Default for AccountsStore {
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
    #[allow(dead_code)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct PyCookies {
    cookies: Vec<CookieEntry>,
    user_info: Option<PyUserInfo>,
}

#[derive(Deserialize)]
struct PyUserInfo {
    user_id: String,
    name: String,
    avatar: Option<String>,
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

fn load_accounts() -> AccountsStore {
    let path = get_accounts_db_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AccountsStore::default()
    }
}

fn save_accounts(store: &AccountsStore) -> Result<(), String> {
    let path = get_accounts_db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn save_cookies(cookie_file: &str, data: &CookieData) -> Result<(), String> {
    let dir = get_cookies_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(cookie_file);
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

// ============ Tauri 命令 ============

#[tauri::command]
async fn get_accounts() -> Result<Vec<Account>, String> {
    // 每次获取账号列表前，尝试从本地 scripts 目录同步最新的 cookie
    let _ = sync_local_accounts().await;
    
    let store = load_accounts();
    Ok(store.accounts)
}

#[tauri::command]
async fn sync_local_accounts() -> Result<usize, String> {
    let platforms = vec!["douyin", "xiaohongshu"];
    let mut sync_count = 0;

    // 脚本现在位于项目根目录下的 scripts 文件夹
    // 在开发模式下，当前目录通常是 src-tauri，所以需要向上找一级
    let script_dir = PathBuf::from("..").join("scripts");

    for platform in platforms {
        let cookie_path = script_dir.join(platform).join("cookie.json");

        if !cookie_path.exists() {
            continue;
        }

        // 调用 validator.py 验证
        let validator_path = script_dir.join("validator.py");
        let mut cmd = tokio::process::Command::new("python3");
        cmd.arg(&validator_path)
            .arg(platform)
            .arg(&cookie_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd.spawn()
            .map_err(|e| format!("启动验证进程失败: {}", e))?
            .wait_with_output()
            .await
            .map_err(|e| format!("执行验证失败: {}", e))?;

        if !output.status.success() {
            continue;
        }

        let result_str = String::from_utf8_lossy(&output.stdout);
        let validation: serde_json::Value = serde_json::from_str(&result_str)
            .map_err(|e| format!("解析验证结果失败: {}", e))?;

        let status = validation["status"].as_str().unwrap_or("error");
        if status == "valid" || status == "expired" {
            // 同步到 accounts.json
            let user_id = validation["user_id"].as_str().unwrap_or("unknown").to_string();
            let name = validation["name"].as_str().unwrap_or("未知用户").to_string();
            let avatar = validation["avatar"].as_str().map(|s| s.to_string());
            
            // 构造 CookieData
            let py_data = &validation["data"];
            let cookies: Vec<CookieEntry> = serde_json::from_value(py_data["cookies"].clone()).unwrap_or_default();
            
            let cookie_data = CookieData {
                cookies,
                origins: None,
                user_info: Some(UserInfo {
                    user_id: user_id.clone(),
                    name: name.clone(),
                    avatar: avatar.clone(),
                }),
            };

            // 保存并同步
            save_new_account(
                platform.to_string(),
                name,
                Some(UserInfo { user_id, name: "".to_string(), avatar }),
                cookie_data,
                Some(status.to_string())
            ).await?;
            
            sync_count += 1;
        }
    }

    Ok(sync_count)
}

#[tauri::command]
async fn init_login_session(
    platform: String,
    state: State<'_, AppState>,
) -> Result<LoginSession, String> {
    let session_id = Uuid::new_v4().to_string();

    // 分配一个随机端口
    let port: u16 = 18000 + (rand_port() % 10000);

    // 准备 Python 脚本路径
    let script_name = match platform.as_str() {
        "xiaohongshu" => "xiaohongshu_login.py",
        "douyin" => "douyin_login.py",
        _ => return Err("不支持的平台".to_string()),
    };

    let script_path = PathBuf::from("..")
        .join("scripts")
        .join(script_name);

    if !script_path.exists() {
        return Err(format!("脚本不存在: {}", script_path.display()));
    }

    // 启动 Python HTTP 服务器作为后台进程
    // 注意：不 pipe stdout/stderr，避免 buffer 填满导致脚本阻塞
    let mut cmd = tokio::process::Command::new("python3");
    cmd.arg(&script_path)
        .arg("--port")
        .arg(port.to_string())
        .arg("--session-id")
        .arg(&session_id)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let mut child = cmd.spawn().map_err(|e| format!("启动 Python 进程失败: {}", e))?;

    // 等待服务器启动
    let base_url = format!("http://127.0.0.1:{}", port);
    let mut started = false;
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Ok(resp) = reqwest::get(format!("{}/status", base_url)).await {
            if resp.status().is_success() {
                started = true;
                break;
            }
        }
    }

    if !started {
        return Err(format!("Python 登录服务器启动失败 (port {})", port));
    }

    // 将进程句柄存入 AppState，防止 drop 时被 kill
    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.insert(session_id.clone(), child);
    }

    // 初始化登录状态
    let flow = LoginFlow {
        port,
        platform: platform.clone(),
        status: "pending".to_string(),
        qrcode_base64: None,
        user_name: None,
        user_id: None,
        cookie_data: None,
        error_msg: None,
    };

    {
        let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        flows.insert(session_id.clone(), flow);
    }

    // 返回 session，让前端开始轮询
    Ok(LoginSession {
        session_id,
        platform,
        status: "pending".to_string(),
        user_info: None,
        cookie_data: None,
        qrcode_base64: None,
    })
}

fn rand_port() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    (nanos % 10000) as u16
}

#[tauri::command]
async fn get_login_status(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<LoginSession, String> {
    // 先克隆需要的数据，避免跨越 await 的借用
    let (platform, cached_status, cached_user_name, cached_user_id, cached_cookie_data, cached_qrcode, port) = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        match flows.get(&session_id) {
            Some(flow) => (
                flow.platform.clone(),
                flow.status.clone(),
                flow.user_name.clone(),
                flow.user_id.clone(),
                flow.cookie_data.clone(),
                flow.qrcode_base64.clone(),
                flow.port,
            ),
            None => return Err("Session not found".to_string()),
        }
    }; // flows 在此释放

    // 如果已经有结果直接返回（不需要查 Python）
    if cached_status == "confirmed" || cached_status == "error" {
        return Ok(LoginSession {
            session_id: session_id.clone(),
            platform,
            status: cached_status,
            user_info: cached_user_name.map(|name| UserInfo {
                user_id: cached_user_id.unwrap_or_default(),
                name,
                avatar: None,
            }),
            cookie_data: cached_cookie_data,
            qrcode_base64: cached_qrcode,
        });
    }

    // 从 Python 服务器获取状态
    let base_url = format!("http://127.0.0.1:{}", port);

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/status", base_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求 Python 服务器失败: {}", e))?;

    let py_status: PyLoginStatus = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    Ok(LoginSession {
        session_id,
        platform,
        status: py_status.status,
        user_info: py_status.user_name.map(|name| UserInfo {
            user_id: py_status.user_id.unwrap_or_default(),
            name,
            avatar: None,
        }),
        cookie_data: None,
        qrcode_base64: py_status.qrcode_base64,
    })
}

fn infer_port(session_id: &str, _platform: &str) -> u16 {
    // 端口通过 session_id 的 hash 分配，与 init 时一致
    let hash: u16 = session_id.bytes().fold(0u16, |acc, b| acc.wrapping_add(b as u16));
    18000 + (hash % 10000)
}

#[tauri::command]
async fn get_cookies(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<CookieData, String> {
    let port = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        match flows.get(&session_id) {
            Some(flow) => flow.port,
            None => return Err("Session not found".to_string()),
        }
    }; // flows 释放

    let base_url = format!("http://127.0.0.1:{}", port);

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/cookies", base_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求 Python 服务器失败: {}", e))?;

    let py_cookies: PyCookies = resp
        .json()
        .await
        .map_err(|e| format!("解析 cookie 失败: {}", e))?;

    let cookie_data = CookieData {
        cookies: py_cookies.cookies,
        origins: None,
        user_info: py_cookies.user_info.map(|u| UserInfo {
            user_id: u.user_id,
            name: u.name,
            avatar: u.avatar,
        }),
    };

    // 更新 flow 中的 cookie_data（用于 save_new_account）
    {
        let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        if let Some(f) = flows.get_mut(&session_id) {
            f.cookie_data = Some(cookie_data.clone());
        }
    }

    Ok(cookie_data)
}

#[tauri::command]
async fn save_new_account(
    platform: String,
    name: String,
    user_info: Option<UserInfo>,
    cookie_data: CookieData,
    status: Option<String>,
) -> Result<Account, String> {
    let user_id = user_info
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let cookie_file = format!("{}_{}.json", platform, &user_id[..8.min(user_id.len())]);

    save_cookies(&cookie_file, &cookie_data)?;

    let now = chrono_now();
    let account = Account {
        id: Uuid::new_v4().to_string(),
        platform: platform.clone(),
        user_id: Some(user_id.clone()),
        name,
        status: status.unwrap_or_else(|| "active".to_string()),
        cookie_file,
        avatar: user_info.as_ref().and_then(|u| u.avatar.clone()),
        created_at: now.clone(),
        updated_at: now,
    };

    let mut store = load_accounts();
    
    // 强制每个平台只能登录一个账号：删除该平台已有的所有账号
    let mut i = 0;
    while i < store.accounts.len() {
        if store.accounts[i].platform == platform {
            let old_account = store.accounts.remove(i);
            // 删除对应的 cookie 文件
            let cookie_path = get_cookies_dir().join(&old_account.cookie_file);
            if cookie_path.exists() {
                let _ = fs::remove_file(cookie_path);
            }
            // 不需要增加 i，因为 remove 后后面的元素往前移了
        } else {
            i += 1;
        }
    }

    store.accounts.push(account.clone());
    save_accounts(&store)?;

    Ok(account)
}

#[tauri::command]
fn delete_account(account_id: String) -> Result<(), String> {
    let mut store = load_accounts();

    if let Some(pos) = store.accounts.iter().position(|a| a.id == account_id) {
        let account = store.accounts.remove(pos);

        let cookie_path = get_cookies_dir().join(&account.cookie_file);
        if cookie_path.exists() {
            fs::remove_file(cookie_path).map_err(|e| e.to_string())?;
        }

        save_accounts(&store)?;
    }

    Ok(())
}

#[tauri::command]
fn cleanup_login_session(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // 先从 process_handles 中取出并杀掉进程
    let child = {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.remove(&session_id)
    };
    if let Some(mut child) = child {
        let _ = child.start_kill();
    }

    // 清理 login_flows
    let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
    flows.remove(&session_id);
    Ok(())
}

// ============ LoginSession（返回给前端）============

#[derive(Clone, Serialize)]
pub struct LoginSession {
    pub session_id: String,
    pub platform: String,
    pub status: String,
    pub user_info: Option<UserInfo>,
    pub cookie_data: Option<CookieData>,
    pub qrcode_base64: Option<String>,
}

// ============ 入口 ============
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_accounts,
            sync_local_accounts,
            init_login_session,
            get_login_status,
            get_cookies,
            save_new_account,
            delete_account,
            cleanup_login_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
