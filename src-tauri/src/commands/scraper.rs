use std::fs;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
use crate::models::{ScraperTask, ScraperProgress};
use crate::state::AppState;
use crate::utils::{get_account_dir, get_data_dir, get_scripts_dir, python_cmd, chrono_now};
use crate::commands::account::load_accounts;

pub fn get_scraper_dir() -> PathBuf {
    get_data_dir().join("scraper")
}

#[tauri::command]
pub async fn resolve_user_sec_uid(input: String) -> Result<String, String> {
    let input = input.trim();
    if input.contains("www.douyin.com/user/") {
        let parts: Vec<&str> = input.split("www.douyin.com/user/").collect();
        if parts.len() > 1 {
            let id_part = parts[1].split('?').next().unwrap_or("").split('/').next().unwrap_or("");
            if !id_part.is_empty() {
                return Ok(id_part.to_string());
            }
        }
    }
    if input.len() > 30 && input.starts_with("MS4wLjABAAAA") {
        return Ok(input.to_string());
    }
    if input.starts_with("http") {
        return Err("目前仅支持 sec_uid 或以 www.douyin.com/user/ 开头的主页链接".to_string());
    }
    Ok(input.to_string())
}

#[tauri::command]
pub async fn start_scrape(
    account_name: String,
    platform: String,
    sec_uid: String,
    scrape_type: String,
    limit: i32,
    skip_existing: bool,
    incremental: bool,
    state: State<'_, AppState>,
) -> Result<ScraperTask, String> {
    {
        let current = state.current_task_id.lock().unwrap();
        if current.is_some() {
            return Err("已有任务正在运行中，请先停止或等待完成".to_string());
        }
    }

    if sec_uid.trim().is_empty() {
        return Err("sec_uid 不能为空".to_string());
    }

    let store = load_accounts();
    let _account = store.accounts.iter()
        .find(|a| a.platform == platform && a.name == account_name)
        .ok_or_else(|| format!("账号不存在: {}/{}", platform, account_name))?;

    let task_id = Uuid::new_v4().to_string();
    let cookie_file = get_account_dir(&platform, &account_name).join("cookie.txt");
    let script_path = get_scripts_dir().join("douyin_scraper.py");

    let log_dir = get_data_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log_path = log_dir.join(format!("scrape_{}_{}.log", &task_id[..8], &sec_uid[..8]));
    let log_file = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let stderr_file = log_file.try_clone().map_err(|e| e.to_string())?;

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

    let task_key = format!("scrape_{}", task_id);

    {
        let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
        handles.insert(task_key, child);
    }
    {
        let mut current = state.current_task_id.lock().unwrap();
        *current = Some(task_id.clone());
    }

    Ok(ScraperTask {
        task_id: task_id.clone(),
        account_name,
        platform,
        sec_uid,
        scrape_type,
        limit,
        skip_existing,
        status: "running".to_string(),
        created_at: chrono_now(),
    })
}

#[tauri::command]
pub async fn get_scrape_progress(task_id: String) -> Result<ScraperProgress, String> {
    let progress_path = get_scraper_dir().join(format!("{}.json", task_id));
    if !progress_path.exists() {
        return Err("任务进度文件不存在".to_string());
    }
    let content = fs::read_to_string(&progress_path).map_err(|e| e.to_string())?;
    let progress: ScraperProgress = serde_json::from_str(&content)
        .map_err(|e| format!("解析进度文件失败: {}", e))?;
    Ok(progress)
}

#[tauri::command]
pub async fn cancel_scrape(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
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
pub async fn get_current_task(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let current = state.current_task_id.lock().unwrap();
    Ok(current.clone())
}

#[tauri::command]
pub async fn clear_current_task(state: State<'_, AppState>) -> Result<(), String> {
    let mut current = state.current_task_id.lock().unwrap();
    *current = None;
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_scraped_user(secUid: String) -> Result<(), String> {
    let path = get_data_dir().join("scraper_data").join(&secUid);
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_scraped_users() -> Result<serde_json::Value, String> {
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
pub async fn get_scraped_videos(secUid: String, limit: i32, offset: i32) -> Result<serde_json::Value, String> {
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
pub async fn get_scraped_comments(secUid: String, awemeId: Option<String>, limit: i32, offset: i32) -> Result<serde_json::Value, String> {
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
pub async fn fetch_douyin_user_info(
    account_name: String,
    user_id: String,
) -> Result<serde_json::Value, String> {
    if user_id.trim().is_empty() {
        return Err("用户 ID 不能为空".to_string());
    }

    let store = load_accounts();
    let _account = store.accounts.iter()
        .find(|a| a.platform == "douyin" && a.name == account_name)
        .ok_or_else(|| format!("账号不存在: douyin/{}", account_name))?;

    // 该脚本走浏览器(CDP)注入，优先用更完整的 cookie.json，回退 cookie.txt
    let account_dir = get_account_dir("douyin", &account_name);
    let cookie_file = {
        let json = account_dir.join("cookie.json");
        if json.exists() { json } else { account_dir.join("cookie.txt") }
    };
    if !cookie_file.exists() {
        return Err(format!("账号 {} 的 Cookie 文件不存在，请先在「账号管理」中授权", account_name));
    }

    let script_path = get_scripts_dir().join("douyin_get_user_info.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("--cookie-path").arg(&cookie_file)
        .arg("--user-id").arg(user_id.trim())
        .arg("--no-save")
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| {
            let err = String::from_utf8_lossy(&output.stderr);
            format!("结果解析失败: {} | stderr: {}", result_str, err)
        })?;
    Ok(result)
}

#[tauri::command]
pub async fn open_video_in_browser(aweme_id: String, account_name: String) -> Result<(), String> {
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
