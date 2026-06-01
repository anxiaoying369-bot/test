use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

static FFMPEG_PATH: OnceLock<String> = OnceLock::new();
static FFPROBE_PATH: OnceLock<String> = OnceLock::new();

/// 获取 FFmpeg 二进制路径
pub fn get_ffmpeg_path() -> String {
    FFMPEG_PATH.get_or_init(|| resolve_binary_path("ffmpeg")).clone()
}

/// 获取 FFprobe 二进制路径
pub fn get_ffprobe_path() -> String {
    FFPROBE_PATH.get_or_init(|| resolve_binary_path("ffprobe")).clone()
}

fn resolve_binary_path(name: &str) -> String {
    let exe_name = if cfg!(windows) { format!("{}.exe", name) } else { name.to_string() };

    // 1. 环境变量
    let env_var = format!("AUTOCAST_{}", name.to_uppercase());
    if let Ok(path) = std::env::var(&env_var) {
        if !path.trim().is_empty() && PathBuf::from(&path).exists() {
            return path;
        }
    }

    // 2. 打包内的 ffmpeg-runtime
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Platform-specific runtime subdirectory
    let platform_runtime = if cfg!(windows) { "windows" } else { "macos" };

    if let Some(res) = crate::state::RESOURCE_DIR.get() {
        // Tauri 2 资源路径
        candidates.push(res.join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
        candidates.push(res.join("_up_").join("src-tauri").join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // macOS Bundle 结构
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
            }
            // 调试/普通结构
            candidates.push(parent.join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
        }
    }

    // dev 模式相对路径
    candidates.push(PathBuf::from("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
    candidates.push(PathBuf::from("src-tauri").join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));
    candidates.push(PathBuf::from("..").join("src-tauri").join("ffmpeg-runtime").join(platform_runtime).join(&exe_name));

    for c in &candidates {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }

    name.to_string()
}

#[derive(Debug, Serialize, Clone)]
pub struct FfmpegProgress {
    pub task_id: String,
    pub percentage: f32,
    pub speed: String,
    pub time: String,
    pub stage: String,
}

/// 运行 FFmpeg 并向前端发送进度
pub async fn run_ffmpeg_with_progress(
    task_id: String,
    args: Vec<String>,
    app: AppHandle,
    stage: String,
) -> Result<(), String> {
    let ffmpeg = get_ffmpeg_path();
    let mut cmd = Command::new(&ffmpeg);

    cmd.args(args)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(crate::utils::CREATE_NO_WINDOW);

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let mut reader = BufReader::new(stderr).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        if line.contains("time=") {
            let mut time = String::new();
            let mut speed = String::new();
            
            for part in line.split_whitespace() {
                if let Some(val) = part.strip_prefix("time=") {
                    time = val.to_string();
                } else if let Some(val) = part.strip_prefix("speed=") {
                    speed = val.to_string();
                }
            }

            if !time.is_empty() {
                let _ = app.emit("video-ffmpeg-progress", FfmpegProgress {
                    task_id: task_id.clone(),
                    percentage: 0.0,
                    speed,
                    time,
                    stage: stage.clone(),
                });
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("FFmpeg exited with error code: {:?}", status.code()))
    }
}
