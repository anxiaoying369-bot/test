// MoneyPrinterTurbo 视频引擎接入层。
//
// 视频创作流程：前端构造好参数对象（主题/脚本/关键词/画幅/配音/字幕/BGM/素材来源等）
// → 调用 `video_mpt_generate` → 本命令注入服务端配置（LLM 凭据 / Pexels Key /
// FFmpeg 路径 / 存储目录）后拉起 `scripts/mpt_generate.py` → 逐行解析 stdout 的 JSONL
// 进度并 `emit("video-mpt-progress")` → 结束后把成片写入 video_db（task / material /
// project.final_video_path）并返回成片路径。
//
// 关键词生成（前端「关键词」步骤）走 `video_mpt_generate_terms` → `scripts/mpt_helper.py`。

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use crate::commands::common::get_config;
use crate::ffmpeg::get_ffmpeg_path;
use crate::state::AppState;
use crate::utils::{extract_provider_error, get_data_dir, get_scripts_dir, python_cmd};

/// MPT 引擎的存储根目录：<data_dir>/mpt_engine。
/// 任务产物落在 <root>/tasks/<task_id>/，本地素材暂存在 <root>/local_videos/。
fn mpt_storage_dir() -> PathBuf {
    get_data_dir().join("mpt_engine")
}

/// 用我的应用配置 + 单次任务参数，构造注入给 Python 引擎的 MPT_CONFIG（JSON 字符串）。
fn build_mpt_config(
    config: &crate::models::AppConfig,
    subtitle_provider_override: Option<&str>,
) -> Result<String, String> {
    if config.llm.api_key.trim().is_empty() {
        return Err("请先在设置中配置 AI 助理的 LLM API Key（脚本/关键词生成需要）".to_string());
    }

    // Pexels Key 支持逗号分隔的多 key。
    let pexels_keys: Vec<String> = config
        .video
        .pexels_api_keys
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let subtitle_provider = subtitle_provider_override
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if config.video.mpt_subtitle_provider.is_empty() {
                "edge".to_string()
            } else {
                config.video.mpt_subtitle_provider.clone()
            }
        });

    let cfg = json!({
        "app": {
            "llm_provider": "openai",
            "openai_api_key": config.llm.api_key,
            "openai_base_url": config.llm.base_url,
            "openai_model_name": config.llm.model,
            "pexels_api_keys": pexels_keys,
            "subtitle_provider": subtitle_provider,
            "tls_verify": true,
            "video_codec": "libx264"
        },
        "whisper": {
            "model_size": "large-v3",
            "device": "cpu",
            "compute_type": "int8"
        },
        "ui": {
            "subtitle_position": "bottom",
            "custom_position": 70.0,
            "hide_log": true
        },
        "log_level": "INFO"
    });

    serde_json::to_string(&cfg).map_err(|e| e.to_string())
}

/// 把本地素材复制到 <storage>/local_videos 下，返回复制后的**绝对路径**列表。
/// MPT 的 file_security 要求本地素材位于该目录内；而 combine_videos 直接用 material.url
/// 打开素材（视频素材的 url 不会被 preprocess 改写），因此必须传可直接打开的绝对路径。
fn stage_local_materials(paths: &[String]) -> Result<Vec<String>, String> {
    let dir = mpt_storage_dir().join("local_videos");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut staged = Vec::new();
    for p in paths {
        let src = PathBuf::from(p);
        if !src.exists() {
            return Err(format!("本地素材不存在：{}", p));
        }
        let file_name = src
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("无效的素材路径：{}", p))?
            .to_string();
        let dest = dir.join(&file_name);
        // 已存在同名文件则跳过复制（素材缓存复用）。
        if !dest.exists() {
            fs::copy(&src, &dest).map_err(|e| format!("复制素材失败 {}: {}", p, e))?;
        }
        staged.push(dest.to_string_lossy().to_string());
    }
    Ok(staged)
}

#[tauri::command]
pub async fn video_mpt_generate(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    mut params: Value,
) -> Result<String, String> {
    let config = get_config().await?;

    let subtitle_provider = params
        .get("subtitle_provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mpt_config = build_mpt_config(&config, subtitle_provider.as_deref())?;

    let storage = mpt_storage_dir();
    fs::create_dir_all(&storage).map_err(|e| e.to_string())?;

    let task_id = Uuid::new_v4().simple().to_string();

    // 本地素材模式：把素材搬到引擎可访问目录，参数里换成文件名。
    let video_source = params
        .get("video_source")
        .and_then(|v| v.as_str())
        .unwrap_or("pexels")
        .to_string();
    if video_source == "local" {
        let local_paths: Vec<String> = params
            .get("video_materials")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if local_paths.is_empty() {
            return Err("本地素材模式下请至少添加一个视频素材".to_string());
        }
        let staged = stage_local_materials(&local_paths)?;
        params["video_materials"] = json!(staged);
    }

    // 写参数文件。
    let params_path = storage.join(format!("params-{}.json", task_id));
    let params_str = serde_json::to_string_pretty(&params).map_err(|e| e.to_string())?;
    fs::write(&params_path, params_str).map_err(|e| e.to_string())?;

    // 任务入库（processing）。
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO video_tasks (id, project_id, type, status, progress) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&task_id, &project_id, "mpt", "processing", 5],
        )
        .map_err(|e| e.to_string())?;
    }

    let script_py = get_scripts_dir().join("mpt_generate.py");

    let mut cmd = python_cmd();
    cmd.arg(&script_py)
        .arg("--params")
        .arg(&params_path)
        .arg("--task-id")
        .arg(&task_id)
        .env("MPT_CONFIG", &mpt_config)
        .env("MPT_STORAGE_DIR", storage.to_string_lossy().to_string())
        .env("IMAGEIO_FFMPEG_EXE", get_ffmpeg_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动视频生成进程：{}", e))?;

    let stdout = child.stdout.take().ok_or("无法捕获子进程输出")?;
    let stderr = child.stderr.take().ok_or("无法捕获子进程错误输出")?;

    // 并发读取 stderr（业务日志）：仅保留尾部用于失败诊断，避免管道阻塞。
    let stderr_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut tail: Vec<String> = Vec::new();
        while let Ok(Some(line)) = reader.next_line().await {
            tail.push(line);
            if tail.len() > 40 {
                tail.remove(0);
            }
        }
        tail.join("\n")
    });

    let mut final_videos: Vec<String> = Vec::new();
    let mut error_message: Option<String> = None;
    let mut script_out: Option<String> = None;

    let mut reader = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // 非协议行（理论上不应出现）直接忽略
        };
        match parsed.get("event").and_then(|v| v.as_str()) {
            Some("progress") => {
                let progress = parsed.get("progress").and_then(|v| v.as_i64()).unwrap_or(0);
                let _ = app.emit(
                    "video-mpt-progress",
                    json!({
                        "task_id": task_id,
                        "progress": progress,
                        "stage": stage_label(progress),
                    }),
                );
                if let Ok(db) = state.video_db.lock() {
                    let _ = db.execute(
                        "UPDATE video_tasks SET progress=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                        rusqlite::params![progress, &task_id],
                    );
                }
            }
            Some("done") => {
                final_videos = parsed
                    .get("videos")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                script_out = parsed
                    .get("script")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
            Some("error") => {
                error_message = Some(extract_provider_error(&parsed, "视频生成失败"));
            }
            _ => {}
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let stderr_tail = stderr_handle.await.unwrap_or_default();

    // 清理参数文件。
    let _ = fs::remove_file(&params_path);

    if !status.success() || final_videos.is_empty() {
        let msg = error_message.unwrap_or_else(|| {
            if stderr_tail.is_empty() {
                "视频生成失败，请检查素材来源、LLM 配置与网络。".to_string()
            } else {
                format!("视频生成失败：{}", stderr_tail.lines().last().unwrap_or(""))
            }
        });
        if let Ok(db) = state.video_db.lock() {
            let _ = db.execute(
                "UPDATE video_tasks SET status='error', error_msg=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                rusqlite::params![&msg, &task_id],
            );
        }
        return Err(msg);
    }

    let final_video = final_videos[0].clone();

    // 成片入库：更新 task、project、并登记为 material。
    {
        let db = state.video_db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE video_tasks SET status='completed', progress=100, result_path=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            rusqlite::params![&final_video, &task_id],
        )
        .map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE video_projects SET final_video_path=?1, status='completed', updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            rusqlite::params![&final_video, &project_id],
        )
        .map_err(|e| e.to_string())?;

        let meta = json!({ "videos": final_videos, "script": script_out }).to_string();
        let material_id = Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO video_materials (id, project_id, type, local_path, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![&material_id, &project_id, "video", &final_video, &meta, "mpt"],
        )
        .map_err(|e| e.to_string())?;
    }

    let _ = app.emit(
        "video-mpt-progress",
        json!({ "task_id": task_id, "progress": 100, "stage": "完成" }),
    );

    Ok(final_video)
}

/// 根据进度百分比给出阶段标签（与 task.py 的进度节点对应），用于前端展示。
fn stage_label(progress: i64) -> &'static str {
    match progress {
        0..=9 => "生成脚本",
        10..=19 => "生成关键词",
        20..=29 => "合成配音",
        30..=39 => "生成字幕",
        40..=49 => "准备素材",
        50..=99 => "拼接与字幕烧录",
        _ => "完成",
    }
}

#[tauri::command]
pub async fn video_mpt_generate_terms(
    video_subject: String,
    video_script: String,
    amount: Option<u32>,
) -> Result<Vec<String>, String> {
    let config = get_config().await?;
    let mpt_config = build_mpt_config(&config, None)?;

    let storage = mpt_storage_dir();
    fs::create_dir_all(&storage).map_err(|e| e.to_string())?;
    let tmp = storage.join(format!("terms-{}.json", Uuid::new_v4().simple()));
    let payload = json!({
        "video_subject": video_subject,
        "video_script": video_script,
        "amount": amount.unwrap_or(5),
    });
    fs::write(&tmp, payload.to_string()).map_err(|e| e.to_string())?;

    let helper_py = get_scripts_dir().join("mpt_helper.py");
    let mut cmd = python_cmd();
    cmd.arg(&helper_py)
        .arg("terms")
        .arg("--params")
        .arg(&tmp)
        .env("MPT_CONFIG", &mpt_config)
        .env("MPT_STORAGE_DIR", storage.to_string_lossy().to_string());

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&tmp);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // 取最后一行 JSON（前面可能有空行）。
    let last = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let res: Value =
        serde_json::from_str(last).map_err(|_| format!("关键词生成失败：{}", stdout))?;
    if res.get("status").and_then(|v| v.as_str()) == Some("error") {
        return Err(extract_provider_error(&res, "关键词生成失败"));
    }
    let terms = res
        .get("terms")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(terms)
}
