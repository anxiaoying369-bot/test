use std::env;
use std::path::PathBuf;

fn main() {
    // Patch tauri.conf.json resources to match current platform at build time.
    // We do NOT modify the source tauri.conf.json (which would pollute the repo).
    // Instead, we pass the patch via the TAURI_CONFIG env var, which tauri-build
    // json-merges into the parsed config at line ~489 of tauri-build/src/lib.rs.
    let platform = env::var("TARGET").unwrap_or_default();
    let runtime_platform = if platform.contains("windows") {
        "windows"
    } else if platform.contains("darwin") {
        "macos"
    } else if platform.contains("linux") {
        "linux"
    } else {
        "macos" // default for unknown
    };

    // Compute the patched resources (what the runtime should use on the current TARGET)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok();
    let conf_path: PathBuf = manifest_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tauri.conf.json");

    // Read the source conf (in repo) just to find the resources array — we never write it back.
    if let Ok(conf_text) = std::fs::read_to_string(&conf_path) {
        if let Ok(conf) = serde_json::from_str::<serde_json::Value>(&conf_text) {
            if let Some(resources) = conf
                .get("bundle")
                .and_then(|b| b.get("resources"))
                .and_then(|r| r.as_array())
            {
                // Build the patched resources array (only differs from source when not on macOS)
                let mut patched: Vec<String> = Vec::new();
                let mut any_changed = false;
                for r in resources.iter() {
                    let s = r.as_str().unwrap_or("");
                    let new_s = if runtime_platform != "macos" && s.contains("python-runtime/macos/python") {
                        any_changed = true;
                        s.replace("python-runtime/macos/python", &format!("python-runtime/{}", runtime_platform))
                    } else if runtime_platform != "macos" && s.contains("ffmpeg-runtime/macos") {
                        any_changed = true;
                        s.replace("ffmpeg-runtime/macos", &format!("ffmpeg-runtime/{}", runtime_platform))
                    } else {
                        s.to_string()
                    };
                    patched.push(new_s);
                }

                if any_changed {
                    // Build the JSON patch and pass via TAURI_CONFIG env var.
                    // tauri-build will json-merge this into the parsed config.
                    let patch = serde_json::json!({
                        "bundle": {
                            "resources": patched
                        }
                    });
                    println!(
                        "cargo:rustc-env=TAURI_CONFIG={}",
                        patch.to_string()
                    );
                    println!(
                        "[build] Resources patched → {} (via TAURI_CONFIG env)",
                        runtime_platform
                    );
                } else {
                    println!(
                        "[build] Resources already match platform: {}",
                        runtime_platform
                    );
                }
            }
        }
    }

    println!("cargo:rerun-if-env-changed=TARGET");

    tauri_build::build()
}
