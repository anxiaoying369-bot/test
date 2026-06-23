use serde_json::{json, Value};
use std::fs;
use std::process::Stdio;
use tauri::State;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use crate::commands::common::get_config;
use crate::state::AppState;
use crate::utils::{get_data_dir, get_scripts_dir, python_cmd};

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let (_, b64) = data_url
        .split_once(',')
        .ok_or("mask 数据格式不正确")?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("mask base64 解码失败: {e}"))
}

async fn run_image_manager(args: Vec<String>) -> Result<Value, String> {
    let script = get_scripts_dir().join("image_manager.py");
    if !script.exists() {
        return Err(format!("图片能力脚本缺失: {}", script.display()));
    }

    let mut cmd = python_cmd();
    cmd.arg(&script)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("启动图片能力进程失败: {e}"))?;
    let stdout = child.stdout.take().ok_or("无法捕获图片能力 stdout")?;
    let stderr = child.stderr.take().ok_or("无法捕获图片能力 stderr")?;

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut last_json: Option<Value> = None;
        let mut raw_tail = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            raw_tail.push(trimmed.to_string());
            if raw_tail.len() > 20 {
                raw_tail.remove(0);
            }
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                last_json = Some(v);
            } else {
                eprintln!("[image-studio stdout] {trimmed}");
            }
        }
        (last_json, raw_tail.join("\n"))
    });

    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut tail = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("[image-studio] {line}");
            tail.push(line);
            if tail.len() > 80 {
                tail.remove(0);
            }
        }
        tail.join("\n")
    });

    let status = match tokio::time::timeout(std::time::Duration::from_secs(240), child.wait()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("等待图片能力进程失败: {e}")),
        Err(_) => {
            let _ = child.start_kill();
            return Err("图片能力进程超时".to_string());
        }
    };

    let (last_json, stdout_tail) = stdout_task.await.unwrap_or((None, String::new()));
    let stderr_tail = stderr_task.await.unwrap_or_default();
    let res = last_json.ok_or_else(|| {
        format!("图片能力进程没有返回 JSON。stdout: {stdout_tail}\nstderr: {stderr_tail}")
    })?;

    if !status.success() || res.get("status").and_then(|v| v.as_str()) == Some("error") {
        let msg = res
            .get("error")
            .and_then(|v| v.as_str())
            .or_else(|| res.get("message").and_then(|v| v.as_str()))
            .unwrap_or("图片能力进程执行失败");
        return Err(format!("{msg}\n{stderr_tail}"));
    }

    Ok(res)
}

#[tauri::command]
pub async fn image_inpaint(
    image_path: String,
    mask_data_url: String,
    prompt: String,
    size: Option<String>,
    _state: State<'_, AppState>,
) -> Result<Value, String> {
    if image_path.trim().is_empty() {
        return Err("请先选择原图".to_string());
    }
    if prompt.trim().is_empty() {
        return Err("请输入修改描述".to_string());
    }
    let source = std::path::PathBuf::from(&image_path);
    if !source.exists() {
        return Err("原图文件不存在".to_string());
    }

    let job_id = Uuid::new_v4().to_string();
    let job_dir = get_data_dir().join("image_studio").join("jobs").join(&job_id);
    fs::create_dir_all(&job_dir).map_err(|e| e.to_string())?;
    let mask_path = job_dir.join("mask.png");
    fs::write(&mask_path, decode_data_url(&mask_data_url)?).map_err(|e| e.to_string())?;

    let config = get_config().await?;
    let provider = if config.video.default_provider.trim().is_empty() {
        "mock".to_string()
    } else {
        config.video.default_provider.clone()
    };
    let api_key = match provider.as_str() {
        "fal" => config.video.fal_key.clone(),
        "volcengine" | "volc" => config.video.volc_key.clone(),
        "openai" | "openai-compatible" | "openai-image" => {
            if config.video.openai_model_source == "relay" {
                config.llm.api_key.clone()
            } else {
                config.video.openai_api_key.clone()
            }
        }
        _ => String::new(),
    };
    let base_url = if config.video.openai_model_source == "relay" {
        config.llm.base_url.clone()
    } else {
        config.video.openai_base_url.clone()
    };
    let model = config.video.openai_model.clone();

    let mut args = vec![
        "--provider".to_string(),
        provider,
        "--api-key".to_string(),
        api_key,
        "--prompt".to_string(),
        prompt,
        "--reference-image".to_string(),
        image_path,
        "--mask-image".to_string(),
        mask_path.to_string_lossy().to_string(),
        "--size".to_string(),
        size.unwrap_or_else(|| "1024x1024".to_string()),
    ];
    if !base_url.trim().is_empty() {
        args.push("--base-url".to_string());
        args.push(base_url);
    }
    if !model.trim().is_empty() {
        args.push("--model".to_string());
        args.push(model);
    }

    let res = run_image_manager(args).await?;
    Ok(json!({
        "ok": true,
        "job_id": job_id,
        "mask_path": mask_path.to_string_lossy(),
        "image_url": res.get("image_url").and_then(|v| v.as_str()).unwrap_or_default(),
        "provider_status": res,
    }))
}
