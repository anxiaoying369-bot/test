use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::state::AppState;
use crate::utils::{get_scripts_dir, python_cmd};
use crate::commands::common::get_config;

const F5_INSTALL_EVENT: &str = "voice-clone-install-progress";
const F5_SYNTH_EVENT: &str = "voice-clone-synthesis-progress";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct F5InstallStatus {
    pub running: bool,
    pub progress: u8,
    pub stage: String,
    pub message: String,
    pub ready: bool,
    pub cli: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
}

impl Default for F5InstallStatus {
    fn default() -> Self {
        Self {
            running: false,
            progress: 0,
            stage: "idle".to_string(),
            message: "未开始安装".to_string(),
            ready: false,
            cli: None,
            error: None,
            started_at: None,
            finished_at: None,
        }
    }
}

static F5_INSTALL_STATUS: OnceLock<Mutex<F5InstallStatus>> = OnceLock::new();

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn install_status_lock() -> &'static Mutex<F5InstallStatus> {
    F5_INSTALL_STATUS.get_or_init(|| Mutex::new(F5InstallStatus::default()))
}

fn get_install_status() -> F5InstallStatus {
    install_status_lock()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

fn update_install_status(app: &AppHandle, patch: impl FnOnce(&mut F5InstallStatus)) -> F5InstallStatus {
    let status = {
        let mut guard = install_status_lock().lock().unwrap_or_else(|e| e.into_inner());
        patch(&mut guard);
        guard.clone()
    };
    let _ = app.emit(F5_INSTALL_EVENT, status.clone());
    status
}

fn parse_progress_line(line: &str) -> Option<(Option<u8>, Option<String>, Option<String>)> {
    let v = serde_json::from_str::<Value>(line).ok()?;
    if v.get("type").and_then(|v| v.as_str()) != Some("audio_lab_log") {
        return None;
    }
    let progress = v
        .get("progress")
        .and_then(|v| v.as_u64())
        .map(|n| n.min(100) as u8);
    let stage = v.get("stage").and_then(|v| v.as_str()).map(|s| s.to_string());
    let message = v
        .get("message")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| v.get("output").and_then(|v| v.as_str()).map(|s| s.to_string()));
    Some((progress, stage, message))
}

async fn run_audio_lab(args: Vec<String>, timeout_secs: u64) -> Result<Value, String> {
    run_audio_lab_inner(args, timeout_secs, None).await
}

async fn run_audio_lab_with_progress_event(
    args: Vec<String>,
    timeout_secs: u64,
    app: AppHandle,
    event_name: &'static str,
) -> Result<Value, String> {
    run_audio_lab_inner(args, timeout_secs, Some((app, event_name))).await
}

async fn run_audio_lab_with_install_progress(
    args: Vec<String>,
    timeout_secs: u64,
    app: AppHandle,
) -> Result<Value, String> {
    run_audio_lab_with_progress_event(args, timeout_secs, app, F5_INSTALL_EVENT).await
}

async fn run_audio_lab_inner(
    args: Vec<String>,
    timeout_secs: u64,
    progress_app: Option<(AppHandle, &'static str)>,
) -> Result<Value, String> {
    let script = get_scripts_dir().join("audio_lab.py");
    if !script.exists() {
        return Err(format!("音频能力脚本缺失: {}", script.display()));
    }

    let mut cmd = python_cmd();
    cmd.arg(&script)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("启动音频能力进程失败: {e}"))?;
    let stdout = child.stdout.take().ok_or("无法捕获音频能力 stdout")?;
    let stderr = child.stderr.take().ok_or("无法捕获音频能力 stderr")?;

    // stderr 只做日志/诊断，必须透传到 Tauri dev console，便于定位 Python 依赖/模型下载问题。
    // 对长任务，stderr 中的 audio_lab_log JSON 同时会转成前端进度事件。
    let stderr_handle = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut tail: Vec<String> = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("[audio-lab] {line}");
            if let Some((app, event_name)) = progress_app.as_ref() {
                if let Some((progress, stage, message)) = parse_progress_line(&line) {
                    if *event_name == F5_INSTALL_EVENT {
                        update_install_status(app, |s| {
                            s.running = true;
                            if let Some(p) = progress {
                                s.progress = p;
                            }
                            if let Some(stage) = stage.clone() {
                                s.stage = stage;
                            }
                            if let Some(message) = message.clone() {
                                s.message = message.chars().take(500).collect();
                            }
                        });
                    } else {
                        let _ = app.emit(*event_name, json!({
                            "running": true,
                            "progress": progress,
                            "stage": stage,
                            "message": message,
                        }));
                    }
                }
            }
            tail.push(line);
            if tail.len() > 80 {
                tail.remove(0);
            }
        }
        tail.join("\n")
    });

    let stdout_handle = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut last_json: Option<Value> = None;
        let mut raw_tail: Vec<String> = Vec::new();
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
                eprintln!("[audio-lab stdout] {trimmed}");
            }
        }
        (last_json, raw_tail.join("\n"))
    });

    let status = match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("等待音频能力进程失败: {e}")),
        Err(_) => {
            let _ = child.start_kill();
            return Err("音频能力进程超时".to_string());
        }
    };

    let stderr_tail = stderr_handle.await.unwrap_or_default();
    let (last_json, stdout_tail) = stdout_handle.await.unwrap_or((None, String::new()));

    let res = last_json.ok_or_else(|| {
        format!(
            "音频能力进程没有返回 JSON。stdout: {}\nstderr: {}",
            stdout_tail, stderr_tail
        )
    })?;

    if !status.success() || res.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        let msg = res
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("音频能力进程执行失败");
        return Err(format!("{msg}\n{stderr_tail}"));
    }

    Ok(res)
}

#[tauri::command]
pub async fn audio_asr_check_model(_state: State<'_, AppState>) -> Result<Value, String> {
    run_audio_lab(vec!["asr-check".to_string()], 30).await
}

#[tauri::command]
pub async fn audio_asr_download_model(_state: State<'_, AppState>) -> Result<Value, String> {
    run_audio_lab(vec!["asr-download".to_string()], 1800).await
}

#[tauri::command]
pub async fn audio_transcribe_file(path: String, _state: State<'_, AppState>) -> Result<Value, String> {
    run_audio_lab(
        vec!["asr-transcribe".to_string(), "--audio".to_string(), path],
        600,
    )
    .await
}

#[tauri::command]
pub async fn audio_polish_speech_text(text: String, _state: State<'_, AppState>) -> Result<Value, String> {
    let input = text.trim();
    if input.is_empty() {
        return Err("请输入需要润色的文本".to_string());
    }
    let config = get_config().await?;
    crate::commands::common::ensure_llm_configured(&config.llm)?;

    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };
    let system_prompt = format!(
        "你是一位中文短视频口播编剧，也懂语音合成和声音表演。\
        请把用户输入润色成更适合语音克隆朗读的自然口播文本。\
        要求：\
        1. 保留原意，不新增事实、价格、功效、承诺。\
        2. 语言更口语、更像真人说话。\
        3. 不要插入任何括号标签、声音动作、语气标注或 SSML，例如不要出现[叹气]、[喘息]、[停顿]。\
        4. 不要刻意加入“嗯”“啊”“呀”“呢”“哦”等口头语，除非原文已经有且保留更自然。\
        5. 可以调整句式和标点，让文本更顺口、更适合朗读。\
        6. 只输出润色后的正文，不要解释，不要 Markdown，不要加标题。"
    );
    let payload = json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": input }
        ],
        "temperature": 0.75
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("AI 润色请求失败: {e}"))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = body.chars().take(500).collect();
        return Err(format!("AI 润色失败 {}: {}", status.as_u16(), snippet));
    }
    let v: Value = serde_json::from_str(&body).map_err(|e| format!("AI 返回内容不是合法 JSON: {e}"))?;
    let polished = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("AI 润色返回为空")?
        .trim()
        .trim_matches('"')
        .trim()
        .to_string();
    if polished.is_empty() {
        return Err("AI 润色返回为空".to_string());
    }
    Ok(json!({
        "ok": true,
        "text": polished,
    }))
}

#[tauri::command]
pub async fn voice_clone_check(_state: State<'_, AppState>) -> Result<Value, String> {
    run_audio_lab(vec!["clone-check".to_string()], 30).await
}

#[tauri::command]
pub async fn voice_clone_install(app: AppHandle, _state: State<'_, AppState>) -> Result<Value, String> {
    let current = get_install_status();
    if current.running {
        return Ok(json!({
            "ok": true,
            "started": false,
            "running": true,
            "status": current,
            "message": "F5-TTS 正在下载安装中",
        }));
    }

    update_install_status(&app, |s| {
        *s = F5InstallStatus {
            running: true,
            progress: 1,
            stage: "start".to_string(),
            message: "开始下载/安装 F5-TTS".to_string(),
            ready: false,
            cli: None,
            error: None,
            started_at: Some(now_secs()),
            finished_at: None,
        };
    });

    let app_for_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = run_audio_lab_with_install_progress(
            vec!["clone-install".to_string()],
            7200,
            app_for_task.clone(),
        )
        .await;

        match result {
            Ok(res) => {
                let cli = res.get("cli").and_then(|v| v.as_str()).map(|s| s.to_string());
                update_install_status(&app_for_task, |s| {
                    s.running = false;
                    s.progress = 100;
                    s.stage = "done".to_string();
                    s.message = "F5-TTS 安装完成".to_string();
                    s.ready = true;
                    s.cli = cli;
                    s.error = None;
                    s.finished_at = Some(now_secs());
                });
            }
            Err(err) => {
                update_install_status(&app_for_task, |s| {
                    s.running = false;
                    s.stage = "error".to_string();
                    s.message = "F5-TTS 安装失败".to_string();
                    s.error = Some(err.chars().take(2000).collect());
                    s.finished_at = Some(now_secs());
                });
            }
        }
    });

    Ok(json!({
        "ok": true,
        "started": true,
        "running": true,
        "status": get_install_status(),
        "message": "F5-TTS 安装任务已在后台启动",
    }))
}

#[tauri::command]
pub async fn voice_clone_install_status(_state: State<'_, AppState>) -> Result<Value, String> {
    Ok(json!({
        "ok": true,
        "status": get_install_status(),
    }))
}

#[tauri::command]
pub async fn voice_clone_register(
    name: String,
    reference_audio: String,
    reference_text: String,
    _state: State<'_, AppState>,
) -> Result<Value, String> {
    run_audio_lab(
        vec![
            "clone-register".to_string(),
            "--name".to_string(),
            name,
            "--audio".to_string(),
            reference_audio,
            "--text".to_string(),
            reference_text,
        ],
        120,
    )
    .await
}

#[tauri::command]
pub async fn voice_clone_list(_state: State<'_, AppState>) -> Result<Value, String> {
    run_audio_lab(vec!["clone-list".to_string()], 30).await
}

#[tauri::command]
pub async fn voice_clone_synthesize(
    app: AppHandle,
    voice: String,
    text: String,
    output: Option<String>,
    _state: State<'_, AppState>,
) -> Result<Value, String> {
    let _ = app.emit(F5_SYNTH_EVENT, json!({
        "running": true,
        "progress": 1,
        "stage": "start",
        "message": "准备语音克隆合成",
    }));
    let mut args = vec![
        "clone-synthesize".to_string(),
        "--voice".to_string(),
        voice,
        "--text".to_string(),
        text,
    ];
    if let Some(out) = output.filter(|s| !s.trim().is_empty()) {
        args.push("--output".to_string());
        args.push(out);
    }
    let result = run_audio_lab_with_progress_event(args, 1800, app.clone(), F5_SYNTH_EVENT).await;
    match &result {
        Ok(res) => {
            let _ = app.emit(F5_SYNTH_EVENT, json!({
                "running": false,
                "progress": 100,
                "stage": "done",
                "message": "语音克隆合成完成",
                "audio_path": res.get("audio_path").and_then(|v| v.as_str()),
            }));
        }
        Err(err) => {
            let _ = app.emit(F5_SYNTH_EVENT, json!({
                "running": false,
                "stage": "error",
                "message": "语音克隆合成失败",
                "error": err.chars().take(2000).collect::<String>(),
            }));
        }
    }
    result
}
