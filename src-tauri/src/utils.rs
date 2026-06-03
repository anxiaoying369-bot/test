use std::fs;
use std::path::{Path, PathBuf};
use crate::state::{BUNDLED_PYTHON, RESOURCE_DIR, SCRIPTS_DIR};

pub fn get_scripts_dir() -> PathBuf {
    if let Some(p) = SCRIPTS_DIR.get() {
        return p.clone();
    }
    let result = resolve_scripts_dir();
    let _ = SCRIPTS_DIR.set(result.clone());
    result
}

fn resolve_scripts_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("AUTOCAST_SCRIPTS_DIR") {
        let p = PathBuf::from(&env_dir);
        if p.join("kb_manager.py").exists() {
            return p;
        }
    }

    // dev / debug 模式：优先用项目源 scripts（cwd = src-tauri/，源在 ../scripts）。
    // 否则会用到 target/debug/_up_/scripts 这个"编译时复制的快照"，
    // 导致改了 Python 脚本却不生效（必须重新编译才会同步）。
    #[cfg(debug_assertions)]
    {
        let src = PathBuf::from("..").join("scripts");
        if src.join("kb_manager.py").exists() {
            return src;
        }
        // 兜底：从可执行文件向上找项目根的 scripts（绝对路径，避免 cwd 不确定）
        if let Ok(exe) = std::env::current_exe() {
            let mut dir = exe.parent().map(|p| p.to_path_buf());
            while let Some(d) = dir {
                let cand = d.join("scripts");
                if cand.join("kb_manager.py").exists() {
                    return cand;
                }
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = RESOURCE_DIR.get() {
        candidates.push(res.join("_up_").join("scripts"));
        candidates.push(res.join("scripts"));
        candidates.push(res.join("resources").join("_up_").join("scripts"));
        candidates.push(res.join("resources").join("scripts"));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("scripts"));
            candidates.push(parent.join("_up_").join("scripts"));
            candidates.push(parent.join("resources").join("scripts"));
            candidates.push(parent.join("resources").join("_up_").join("scripts"));
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("_up_").join("scripts"));
                candidates.push(pp.join("Resources").join("scripts"));
            }
        }
    }

    candidates.push(PathBuf::from("..").join("scripts"));
    candidates.push(PathBuf::from(".").join("scripts"));

    for c in &candidates {
        if c.join("kb_manager.py").exists() {
            return c.clone();
        }
    }

    if let Some(res) = RESOURCE_DIR.get() {
        if let Some(found) = find_file_upwards(res, "kb_manager.py", 4) {
            if let Some(parent) = found.parent() {
                return parent.to_path_buf();
            }
        }
    }

    PathBuf::from("..").join("scripts")
}

fn find_file_upwards(root: &Path, target: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().map(|n| n == target).unwrap_or(false) {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(found) = find_file_upwards(&p, target, max_depth - 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("AutoCastAI")
}

pub fn get_accounts_db_path() -> PathBuf {
    get_data_dir().join("accounts.json")
}

pub fn get_cookies_dir() -> PathBuf {
    get_data_dir().join("cookies")
}

pub fn get_account_dir(platform: &str, account_name: &str) -> PathBuf {
    get_cookies_dir().join(platform).join(account_name)
}

pub fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

pub fn enhanced_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let mut extra_dirs: Vec<String> = if cfg!(target_os = "macos") {
        ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin", "/usr/local/sbin"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else if cfg!(target_os = "linux") {
        ["/usr/local/bin", "/snap/bin"].iter().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };

    // 注入内置 Node.js 路径
    if let Some(node_bin) = resolve_node_executable() {
        if let Some(parent) = std::path::Path::new(&node_bin).parent() {
            extra_dirs.push(parent.to_string_lossy().to_string());
        }
    }

    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut parts: Vec<String> = current.split(sep).map(|s| s.to_string()).collect();

    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        let s = local_bin.to_string_lossy().to_string();
        if !parts.iter().any(|p| p == &s) {
            parts.push(s);
        }
    }

    for d in extra_dirs {
        if !parts.iter().any(|p| p == &d) {
            parts.push(d);
        }
    }
    parts.join(sep)
}

pub fn resolve_node_executable() -> Option<String> {
    let (platform_runtime, rel_bin_inside): (&str, PathBuf) = if cfg!(windows) {
        ("windows", PathBuf::from("node.exe"))
    } else {
        ("macos", PathBuf::from("bin").join("node"))
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = RESOURCE_DIR.get() {
        candidates.push(res.join("node-runtime").join(platform_runtime).join(&rel_bin_inside));
        candidates.push(res.join("_up_").join("node-runtime").join(platform_runtime).join(&rel_bin_inside));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("node-runtime").join(platform_runtime).join(&rel_bin_inside));
            if let Some(pp) = parent.parent() {
                candidates.push(pp.join("Resources").join("node-runtime").join(platform_runtime).join(&rel_bin_inside));
            }
        }
    }
    candidates.push(PathBuf::from("node-runtime").join(platform_runtime).join(&rel_bin_inside));
    candidates.push(PathBuf::from("..").join("src-tauri").join("node-runtime").join(platform_runtime).join(&rel_bin_inside));

    for c in candidates {
        if c.exists() {
            return Some(c.to_string_lossy().to_string());
        }
    }
    None
}

/// Windows: 隐藏子进程控制台窗口，避免 GUI 应用调用 python/ffmpeg 时黑框闪烁。
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 构建一个 tokio Command，并在 Windows 上隐藏控制台窗口。
/// 用于 ffmpeg/ffprobe 等非 python 的子进程调用。
pub fn tokio_command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 同步版本：构建一个 std Command，并在 Windows 上隐藏控制台窗口。
pub fn std_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 访问本机 127.0.0.1 服务（登录用的 Python HTTP server、Hermes 本地网关等）专用的 reqwest 客户端：
/// **禁用系统代理**。否则 Windows 上配置了系统/公司代理时，本地请求会被代理转发并返回
/// `502 Bad Gateway`（用户在登录"我已登录完成"时遇到的报错根因）。
pub fn local_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub fn python_cmd() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(python_executable());
    cmd.env(
        "AUTOCAST_DATA_DIR",
        get_data_dir().to_string_lossy().to_string(),
    );
    if let Some(node) = resolve_node_executable() {
        cmd.env("AUTOCAST_NODE", node);
    }
    cmd.env("PYTHONUNBUFFERED", "1");
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PATH", enhanced_path());
    cmd.arg("-u");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub fn extract_provider_error(res: &serde_json::Value, fallback_label: &str) -> String {
    let status = res["status"].as_str().unwrap_or("error");
    if status != "error" {
        return "未知错误".to_string();
    }

    let code = res["error_code"].as_str().unwrap_or("UNKNOWN");
    let msg = res["error"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| res["error_message"].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("");
    let details = res["details"].as_str().unwrap_or("");

    let friendly = match code {
        "AUTH" => "API Key 验证失败，请检查配置是否正确。",
        "RATE_LIMIT" => "请求过于频繁，已被限流，请稍后再试。",
        "QUOTA" => "余额不足或超出配额，请检查服务商账户状态。",
        "NETWORK" => "网络请求超时或连接失败，请检查网络。",
        "INVALID" | "INVALID_PARAMS" => {
            if !msg.is_empty() {
                msg
            } else {
                "输入参数无效（如提示词违禁、尺寸不支持、或接口参数不兼容）。"
            }
        }
        _ => {
            if !msg.is_empty() {
                msg
            } else {
                fallback_label
            }
        }
    };

    if !details.is_empty() {
        format!(
            "{} ({}) — {}",
            friendly,
            code,
            details.chars().take(200).collect::<String>()
        )
    } else {
        format!("{} ({})", friendly, code)
    }
}

pub fn python_executable() -> String {
    if let Some(p) = BUNDLED_PYTHON.get() {
        return p.clone();
    }
    let result = resolve_python_executable();
    let _ = BUNDLED_PYTHON.set(result.clone());
    result
}

fn resolve_python_executable() -> String {
    if let Ok(env_py) = std::env::var("AUTOCAST_PYTHON") {
        if !env_py.trim().is_empty() && PathBuf::from(&env_py).exists() {
            return env_py;
        }
    }

    // Platform-specific runtime subdirectory + executable path inside it
    // Structure: python-runtime/<platform>/python/bin/python3 (macos, python-build-standalone)
    //            python-runtime/<platform>/python/python.exe   (windows, python-build-standalone)
    let (platform_runtime, rel_bin_inside): (&str, PathBuf) = if cfg!(windows) {
        ("windows", PathBuf::from("python").join("python.exe"))
    } else {
        (
            "macos",
            PathBuf::from("python").join("bin").join("python3"),
        )
    };

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(res) = RESOURCE_DIR.get() {
        candidates.push(
            res.join("_up_")
                .join("src-tauri")
                .join("python-runtime")
                .join(platform_runtime)
                .join(&rel_bin_inside),
        );
        candidates.push(
            res.join("python-runtime")
                .join(platform_runtime)
                .join(&rel_bin_inside),
        );
        candidates.push(
            res.join("_up_")
                .join("python-runtime")
                .join(platform_runtime)
                .join(&rel_bin_inside),
        );
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Some(pp) = parent.parent() {
                candidates.push(
                    pp.join("Resources")
                        .join("_up_")
                        .join("src-tauri")
                        .join("python-runtime")
                        .join(platform_runtime)
                        .join(&rel_bin_inside),
                );
                candidates.push(
                    pp.join("Resources")
                        .join("python-runtime")
                        .join(platform_runtime)
                        .join(&rel_bin_inside),
                );
            }
            candidates.push(
                parent
                    .join("python-runtime")
                    .join(platform_runtime)
                    .join(&rel_bin_inside),
            );
        }
    }
    candidates.push(
        PathBuf::from("python-runtime")
            .join(platform_runtime)
            .join(&rel_bin_inside),
    );

    let venv_rel = if cfg!(windows) {
        PathBuf::from(".venv").join("Scripts").join("python.exe")
    } else {
        PathBuf::from(".venv").join("bin").join("python3")
    };
    candidates.push(PathBuf::from("..").join(&venv_rel));
    candidates.push(
        PathBuf::from("..")
            .join("src-tauri")
            .join("python-runtime")
            .join(platform_runtime)
            .join(&rel_bin_inside),
    );

    for c in &candidates {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }

    if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}