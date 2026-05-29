use std::fs;
use tauri::State;
use uuid::Uuid;
use crate::models::{Account, AccountView, AccountsStoreFile, LoginFlow, LoginSession, PyLoginStatus, UserInfo, VerifyResult, account_to_view};
use crate::state::AppState;
use crate::utils::{get_account_dir, get_accounts_db_path, get_cookies_dir, get_data_dir, get_scripts_dir, python_cmd, chrono_now};

pub fn load_accounts() -> AccountsStoreFile {
    let path = get_accounts_db_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AccountsStoreFile::default()
    }
}

pub fn save_accounts(store: &AccountsStoreFile) -> Result<(), String> {
    let path = get_accounts_db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn read_meta_json(platform: &str, name: &str) -> Option<UserInfo> {
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

pub fn register_account_on_disk(platform: &str, name: &str) -> Result<Account, String> {
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
        cookie_file: name.to_string(),
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

#[tauri::command]
pub async fn list_accounts(platform: Option<String>) -> Result<Vec<AccountView>, String> {
    let _ = sync_local_accounts().await;
    let store = load_accounts();
    let views = match platform {
        Some(p) => store.accounts.iter().filter(|a| a.platform == p).map(account_to_view).collect(),
        None => store.accounts.iter().map(account_to_view).collect(),
    };
    Ok(views)
}

#[tauri::command]
pub async fn sync_local_accounts() -> Result<usize, String> {
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
pub async fn init_login_session(platform: String, state: State<'_, AppState>) -> Result<LoginSession, String> {
    let session_id = Uuid::new_v4().to_string();
    let port: u16 = 18000 + (Uuid::new_v4().as_u128() as u16 % 10000);

    let script_name = match platform.as_str() {
        "douyin" => "douyin_login.py",
        _ => return Err("不支持的平台".to_string()),
    };

    let script_path = get_scripts_dir().join(script_name);
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
pub async fn get_login_status(session_id: String, state: State<'_, AppState>) -> Result<LoginSession, String> {
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

#[derive(serde::Deserialize)]
struct PyFinishResult {
    ok: bool,
    error: Option<String>,
    #[allow(dead_code)]
    user_info: Option<UserInfo>,
}

#[tauri::command]
pub async fn finish_login(
    session_id: String,
    account_name: String,
    state: State<'_, AppState>,
) -> Result<AccountView, String> {
    if account_name.trim().is_empty() {
        return Err("账号名称不能为空".to_string());
    }
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

    let account = register_account_on_disk(&platform, &account_name)?;
    Ok(account_to_view(&account))
}

#[tauri::command]
pub async fn verify_account(platform: String, name: String) -> Result<VerifyResult, String> {
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
pub async fn delete_account(platform: String, name: String) -> Result<(), String> {
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
pub async fn cleanup_login_session(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = handles.remove(&session_id) { let _ = child.start_kill(); }
    let mut flows = state.login_flows.lock().map_err(|e| e.to_string())?;
    flows.remove(&session_id);
    Ok(())
}
