use std::env;
use std::path::PathBuf;

fn main() {
    // Patch tauri.conf.json resources to match current platform at build time.
    //
    // We do NOT modify the source tauri.conf.json (which would pollute the repo).
    // Instead, we set the `TAURI_CONFIG` env var in this build-script's process;
    // tauri-build's `try_build()` reads `std::env::var("TAURI_CONFIG")` and
    // json-merges it into the parsed config BEFORE the resource glob validator
    // runs (see tauri-build-2.6.2/src/lib.rs:487-490 and the subsequent
    // `copy_resources` → `ResourcePaths::new(...).iter()` path).
    //
    // Earlier versions of this file used `println!("cargo:rustc-env=TAURI_CONFIG=...")`
    // which is a NO-OP here: that directive only sets a compile-time env var for
    // `rustc` invocations spawned by cargo, NOT for the current build.rs process
    // itself — so tauri-build kept reading the un-patched `macos/...` paths and
    // failed with "glob pattern ... path not found" on Windows runners.
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
                    let new_s = if runtime_platform != "macos"
                        && s.contains("python-runtime/macos/python")
                    {
                        any_changed = true;
                        s.replace(
                            "python-runtime/macos/python",
                            &format!("python-runtime/{}/python", runtime_platform),
                        )
                    } else if runtime_platform != "macos" && s.contains("ffmpeg-runtime/macos") {
                        any_changed = true;
                        s.replace(
                            "ffmpeg-runtime/macos",
                            &format!("ffmpeg-runtime/{}", runtime_platform),
                        )
                    } else {
                        s.to_string()
                    };
                    patched.push(new_s);
                }

                if any_changed {
                    // Build the JSON patch and set it in the current process env.
                    // tauri-build's try_build() will read this and json-merge it
                    // into the parsed config before the resource glob validator runs.
                    let patch = serde_json::json!({
                        "bundle": {
                            "resources": patched
                        }
                    });
                    env::set_var("TAURI_CONFIG", &patch.to_string());
                    eprintln!(
                        "[build] Resources patched → {} (via TAURI_CONFIG env, set in process)",
                        runtime_platform
                    );
                    eprintln!(
                        "[build] Patched resources: {}",
                        serde_json::to_string_pretty(&patched).unwrap_or_default()
                    );
                } else {
                    eprintln!(
                        "[build] Resources already match platform: {}",
                        runtime_platform
                    );
                }
            }
        }
    }

    // Tell cargo to re-run this build script if the env or source conf changes.
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=TAURI_CONFIG");
    println!("cargo:rerun-if-changed=tauri.conf.json");

    tauri_build::build()
}
