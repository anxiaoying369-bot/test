// 多平台发布 / 定时排期 / 矩阵分发。
//
// 数据流（沿用本项目 Rust→python sidecar→前端事件 模式）：
//   create_publish_tasks 写入 publish_tasks 表（矩阵=每账号一条）→
//   tokio 调度循环 run_publish_scheduler 轮询到点的 pending 任务 →
//   spawn scripts/publisher/douyin_publisher.py（DrissionPage 驱动创作者中心）→
//   读 stdout 行 JSON 进度，更新 DB 并 emit("publish-progress")。
//
// 排期语义：scheduled_at = 我方调度器**开始执行**该任务的时间；到点后按「立即发布」走
// 创作者中心（不依赖抖音原生定时 UI，跨平台行为统一）。即时任务（无排期）用有头浏览器，
// 方便用户首发时人工过验证码；排期任务用无头后台发布。

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::db;
use crate::state::AppState;
use crate::utils::{get_account_dir, get_data_dir, get_scripts_dir, python_cmd};

/// 同时进行的发布数上限：每个任务起一个 Chrome，太多会拖垮机器。
const MAX_CONCURRENT_PUBLISH: usize = 2;

#[derive(serde::Deserialize)]
pub struct PublishRequest {
    pub platform: Option<String>,
    /// 矩阵分发：同一视频发布到这些账号，各生成一条任务。
    pub account_names: Vec<String>,
    pub video_path: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    pub cover_path: Option<String>,
    pub location: Option<String>,
    /// 排期开始时间（Unix 秒）；None/0 = 立即进入队列。
    pub scheduled_at: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct PublishableVideo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: i64,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 扫描视频工作室的成片输出目录，供发布页选片。
#[tauri::command]
pub async fn list_publishable_videos() -> Result<Vec<PublishableVideo>, String> {
    let root = get_data_dir().join("video_studio").join("output");
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    // output/<project_id>/<task_id>.mp4，向下两层扫 .mp4
    fn scan(dir: &PathBuf, out: &mut Vec<PublishableVideo>, depth: usize) {
        if depth > 3 {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan(&path, out, depth + 1);
            } else if path.extension().and_then(|e| e.to_str()) == Some("mp4") {
                let meta = entry.metadata().ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push(PublishableVideo {
                    path: path.to_string_lossy().to_string(),
                    name: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                    size,
                    modified,
                });
            }
        }
    }
    scan(&root, &mut out, 0);
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(out)
}

/// 创建发布任务（矩阵 = 每个账号一条）。立即任务由调度循环马上拾取。
#[tauri::command]
pub async fn create_publish_tasks(
    req: PublishRequest,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    if req.account_names.is_empty() {
        return Err("请至少选择一个发布账号".to_string());
    }
    if !PathBuf::from(&req.video_path).exists() {
        return Err(format!("视频文件不存在：{}", req.video_path));
    }
    if req.title.trim().is_empty() {
        return Err("标题不能为空".to_string());
    }

    let platform = req.platform.clone().unwrap_or_else(|| "douyin".to_string());
    let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());
    let scheduled = req.scheduled_at.filter(|t| *t > 0);

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for account in &req.account_names {
        let id = Uuid::new_v4().to_string();
        db::publish_create_task(
            &db,
            &id,
            &platform,
            account,
            &req.video_path,
            &req.title,
            &tags_json,
            &req.description,
            req.cover_path.as_deref(),
            req.location.as_deref(),
            scheduled,
        )
        .map_err(|e| e.to_string())?;
        ids.push(id);
    }
    Ok(ids)
}

#[tauri::command]
pub async fn list_publish_tasks(state: State<'_, AppState>) -> Result<Vec<db::PublishTaskRecord>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db::publish_list_tasks(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_publish_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    // 正在发布的不允许直接删，先取消
    kill_publish_process(&state, &id);
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db::publish_delete_task(&db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_publish_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    kill_publish_process(&state, &id);
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db::publish_set_status(&db, &id, "cancelled").map_err(|e| e.to_string())
}

/// 重新发布：把失败/取消的任务重置为 pending，调度循环会再次拾取。
#[tauri::command]
pub async fn retry_publish_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db::publish_update_progress(&db, &id, 0, "").map_err(|e| e.to_string())?;
    db::publish_set_status(&db, &id, "pending").map_err(|e| e.to_string())
}

fn kill_publish_process(state: &State<'_, AppState>, id: &str) {
    if let Ok(mut handles) = state.process_handles.lock() {
        if let Some(child) = handles.remove(&format!("publish_{id}")) {
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
    }
}

/// 统计正在发布中的任务数，用于并发限流。
fn running_publish_count(app: &AppHandle) -> usize {
    app.state::<AppState>()
        .process_handles
        .lock()
        .map(|h| h.keys().filter(|k| k.starts_with("publish_")).count())
        .unwrap_or(0)
}

/// 启动单个发布任务：spawn python、流式读取进度、回写 DB、emit 事件。
/// 调用前任务在 DB 中应已置为 publishing，且未在运行中。
fn spawn_publish(app: AppHandle, task: db::PublishTaskRecord) {
    let id = task.id.clone();
    let headless = task.scheduled_at.is_some(); // 排期=无头后台；即时=有头便于过验证码
    let account_dir = get_account_dir(&task.platform, &task.account_name);
    let script = get_scripts_dir().join("publisher").join("douyin_publisher.py");

    let mut cmd = python_cmd();
    cmd.arg(&script)
        .arg("--account-dir").arg(&account_dir)
        .arg("--video").arg(&task.video_path)
        .arg("--title").arg(&task.title)
        .arg("--tags").arg(task.tags.join(","))
        .arg("--description").arg(&task.description);
    if let Some(cover) = &task.cover_path {
        cmd.arg("--cover").arg(cover);
    }
    if let Some(loc) = &task.location {
        cmd.arg("--location").arg(loc);
    }
    if headless {
        cmd.arg("--headless");
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            finalize(&app, &id, false, "", &format!("无法启动发布进程：{e}"));
            return;
        }
    };
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            finalize(&app, &id, false, "", "无法读取发布进程输出");
            return;
        }
    };

    if let Ok(mut handles) = app.state::<AppState>().process_handles.lock() {
        handles.insert(format!("publish_{id}"), child);
    }

    tauri::async_runtime::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut reader = BufReader::new(stdout).lines();
        let mut got_result = false;

        while let Ok(Some(line)) = reader.next_line().await {
            let val: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match val.get("type").and_then(|t| t.as_str()) {
                Some("progress") => {
                    let stage = val.get("stage").and_then(|s| s.as_str()).unwrap_or("");
                    let progress = val.get("progress").and_then(|p| p.as_i64()).unwrap_or(0);
                    if let Ok(db) = app.state::<AppState>().video_db.lock() {
                        let _ = db::publish_update_progress(&db, &id, progress, stage);
                    }
                    let _ = app.emit("publish-progress", serde_json::json!({
                        "id": id, "status": "publishing", "stage": stage, "progress": progress
                    }));
                }
                Some("result") => {
                    got_result = true;
                    let success = val.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
                    let url = val.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let error = val.get("error").and_then(|e| e.as_str()).unwrap_or("");
                    finalize(&app, &id, success, url, error);
                }
                _ => {}
            }
        }

        if !got_result {
            finalize(&app, &id, false, "", "发布进程异常退出（未返回结果）");
        }
        if let Ok(mut handles) = app.state::<AppState>().process_handles.lock() {
            handles.remove(&format!("publish_{id}"));
        }
    });
}

/// 写入最终状态并 emit。
fn finalize(app: &AppHandle, id: &str, success: bool, url: &str, error: &str) {
    if let Ok(db) = app.state::<AppState>().video_db.lock() {
        if success {
            let _ = db::publish_set_success(&db, id, if url.is_empty() { None } else { Some(url) });
        } else {
            let _ = db::publish_set_failed(&db, id, if error.is_empty() { "发布失败" } else { error });
        }
    }
    let _ = app.emit("publish-progress", serde_json::json!({
        "id": id,
        "status": if success { "success" } else { "failed" },
        "stage": if success { "发布成功" } else { "发布失败" },
        "progress": if success { 100 } else { 0 },
        "url": url,
        "error": error,
    }));
}

/// 发布调度循环：在 app setup 时启动，常驻轮询到点的 pending 任务并发布。
pub fn run_publish_scheduler(app: AppHandle) {
    // 启动时清理上次异常退出残留的 publishing 任务（无对应进程，否则会永远卡住）
    if let Ok(db) = app.state::<AppState>().video_db.lock() {
        let _ = db::publish_reset_stuck(&db);
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let slots = MAX_CONCURRENT_PUBLISH.saturating_sub(running_publish_count(&app));
            if slots == 0 {
                continue;
            }

            // 取到点任务并立刻在 DB 标记 publishing，避免下一轮重复拾取
            let due: Vec<db::PublishTaskRecord> = {
                let state = app.state::<AppState>();
                let db = match state.video_db.lock() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let list = db::publish_list_due(&db, now_secs()).unwrap_or_default();
                let picked: Vec<_> = list.into_iter().take(slots).collect();
                for t in &picked {
                    let _ = db::publish_set_status(&db, &t.id, "publishing");
                    let _ = db::publish_update_progress(&db, &t.id, 1, "排队中");
                }
                picked
            };

            for task in due {
                let _ = app.emit("publish-progress", serde_json::json!({
                    "id": task.id, "status": "publishing", "stage": "排队中", "progress": 1
                }));
                spawn_publish(app.clone(), task);
            }
        }
    });
}
