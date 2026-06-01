use std::env;

fn main() {
    // Patch tauri.conf.json resources to match current platform at build time.
    // This runs before tauri-build processes the config.
    let platform = env::var("TARGET").unwrap_or_default();
    let runtime_platform = if platform.contains("windows") {
        "windows"
    } else if platform.contains("darwin") {
        "macos"
    } else {
        // Default to macos for unknown (e.g. Linux when cross-compiling from macOS)
        // Linux builds will explicitly set TARGET=...-linux-gnu
        if platform.contains("linux") {
            "linux"
        } else {
            "macos"
        }
    };

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok();
    let conf_path: std::path::PathBuf = manifest_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tauri.conf.json");

    if let Ok(conf_text) = std::fs::read_to_string(&conf_path) {
        if let Ok(mut conf) = serde_json::from_str::<serde_json::Value>(&conf_text) {
            if let Some(resources) = conf.get_mut("bundle")
                .and_then(|b| b.get_mut("resources"))
                .and_then(|r| r.as_array_mut())
            {
                let patched: Vec<serde_json::Value> = resources
                    .iter()
                    .map(|r| {
                        let s = r.as_str().unwrap_or("");
                        let replacement = if runtime_platform != "macos" && s.contains("python-runtime/macos") {
                            s.replace("python-runtime/macos", &format!("python-runtime/{}", runtime_platform))
                        } else if runtime_platform != "macos" && s.contains("ffmpeg-runtime/macos") {
                            s.replace("ffmpeg-runtime/macos", &format!("ffmpeg-runtime/{}", runtime_platform))
                        } else {
                            s.to_string()
                        };
                        serde_json::Value::String(replacement)
                    })
                    .collect();

                let patched_any = resources.iter().zip(patched.iter()).any(|(a, b)| {
                    let sa = a.as_str().unwrap_or("");
                    let sb = b.as_str().unwrap_or("");
                    sa != sb
                });

                if patched_any {
                    conf["bundle"]["resources"] = serde_json::Value::Array(patched);
                    let out = serde_json::to_string_pretty(&conf).unwrap_or_default();
                    let _ = std::fs::write(&conf_path, out);
                    println!("[build] Patched tauri.conf.json → {runtime_platform}");
                }
            }
        }
    }

    tauri_build::build()
}