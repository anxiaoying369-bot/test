use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

// ============ 状态管理 ============
pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
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
    let _ = sync_local_accounts().await;
    let store = load_accounts();
    Ok(store.accounts)
}

#[tauri::command]
async fn sync_local_accounts() -> Result<usize, String> {
    let platforms = vec!["douyin", "xiaohongshu"];
    let mut sync_count = 0;
    let script_dir = PathBuf::from("..").join("scripts");

    for platform in platforms {
        let cookie_path = script_dir.join(platform).join("cookie.json");
        if !cookie_path.exists() { continue; }

        let validator_path = script_dir.join("validator.py");
        let output = tokio::process::Command::new("python3")
            .arg(&validator_path).arg(platform).arg(&cookie_path)
            .output().await.map_err(|e| e.to_string())?;

        if !output.status.success() { continue; }
        let result_str = String::from_utf8_lossy(&output.stdout);
        let validation: serde_json::Value = serde_json::from_str(&result_str).map_err(|e| e.to_string())?;

        let status = validation["status"].as_str().unwrap_or("error");
        if status == "valid" || status == "expired" {
            let user_id = validation["user_id"].as_str().unwrap_or("unknown").to_string();
            let name = validation["name"].as_str().unwrap_or("未知用户").to_string();
            let avatar = validation["avatar"].as_str().map(|s| s.to_string());
            let py_data = &validation["data"];
            let cookies: Vec<CookieEntry> = serde_json::from_value(py_data["cookies"].clone()).unwrap_or_default();
            
            let cookie_data = CookieData {
                cookies,
                origins: None,
                user_info: Some(UserInfo { user_id: user_id.clone(), name: name.clone(), avatar: avatar.clone() }),
            };

            save_new_account(platform.to_string(), name, Some(UserInfo { user_id, name: "".to_string(), avatar }), cookie_data, Some(status.to_string())).await?;
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
        "xiaohongshu" => "xiaohongshu_login.py",
        "douyin" => "douyin_login.py",
        _ => return Err("不支持的平台".to_string()),
    };

    let script_path = PathBuf::from("..").join("scripts").join(script_name);
    let child = tokio::process::Command::new("python3")
        .arg(&script_path).arg("--port").arg(port.to_string()).arg("--session-id").arg(&session_id)
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
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
async fn get_login_status(session_id: String, platform: String, state: State<'_, AppState>) -> Result<LoginSession, String> {
    let port = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        flows.get(&session_id).map(|f| f.port).ok_or("Session not found")?
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

#[tauri::command]
async fn get_cookies(session_id: String, state: State<'_, AppState>) -> Result<CookieData, String> {
    let port = {
        let flows = state.login_flows.lock().map_err(|e| e.to_string())?;
        flows.get(&session_id).map(|f| f.port).ok_or("Session not found")?
    };

    let client = reqwest::Client::new();
    let py_cookies: PyCookies = client.get(format!("http://127.0.0.1:{}/cookies", port))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    Ok(CookieData {
        cookies: py_cookies.cookies,
        origins: None,
        user_info: py_cookies.user_info.map(|u| UserInfo { user_id: u.user_id, name: u.name, avatar: u.avatar }),
    })
}

#[tauri::command]
async fn save_new_account(platform: String, name: String, user_info: Option<UserInfo>, cookie_data: CookieData, status: Option<String>) -> Result<Account, String> {
    let user_id = user_info.as_ref().map(|u| u.user_id.clone()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let cookie_file = format!("{}_{}.json", platform, &user_id[..8.min(user_id.len())]);
    save_cookies(&cookie_file, &cookie_data)?;

    let now = chrono_now();
    let account = Account {
        id: Uuid::new_v4().to_string(), platform: platform.clone(), user_id: Some(user_id),
        name, status: status.unwrap_or_else(|| "active".to_string()),
        cookie_file, avatar: user_info.as_ref().and_then(|u| u.avatar.clone()),
        created_at: now.clone(), updated_at: now,
    };

    let mut store = load_accounts();
    store.accounts.retain(|a| a.platform != platform);
    store.accounts.push(account.clone());
    save_accounts(&store)?;
    Ok(account)
}

#[tauri::command]
async fn delete_account(accountId: String) -> Result<(), String> {
    let mut store = load_accounts();
    if let Some(pos) = store.accounts.iter().position(|a| a.id == accountId) {
        let account = store.accounts.remove(pos);
        let _ = fs::remove_file(get_cookies_dir().join(&account.cookie_file));
        let _ = fs::remove_file(PathBuf::from("..").join("scripts").join(&account.platform).join("cookie.json"));
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { login_flows: Mutex::new(std::collections::HashMap::new()), process_handles: Mutex::new(std::collections::HashMap::new()) })
        .invoke_handler(tauri::generate_handler![
            get_accounts, sync_local_accounts, init_login_session, get_login_status, get_cookies, save_new_account, delete_account, cleanup_login_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
