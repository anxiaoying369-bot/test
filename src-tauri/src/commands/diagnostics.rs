use tauri::State;
use crate::state::{AppState, RESOURCE_DIR};
use crate::utils::{get_scripts_dir, python_executable};

#[tauri::command]
pub async fn autocast_diagnostics() -> Result<serde_json::Value, String> {
    let scripts = get_scripts_dir();
    let py = python_executable();
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let resource = RESOURCE_DIR.get().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let kb_exists = scripts.join("kb_manager.py").exists();

    let check_modules = [
        "DrissionPage", "lancedb", "pypdf", "openai",
        "websockets", "httpx", "yaml", "tqdm", "PIL",
    ];
    let probe_code = format!(
        "import importlib,json,sys; res={{}}\nfor m in {:?}:\n  try: importlib.import_module(m); res[m]=True\n  except Exception as e: res[m]=str(e)\nprint(json.dumps(res))",
        check_modules
    );
    let dep_result = tokio::process::Command::new(&py)
        .arg("-c")
        .arg(&probe_code)
        .output()
        .await;
    let deps = match dep_result {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            serde_json::from_str::<serde_json::Value>(&s)
                .unwrap_or(serde_json::json!({ "raw": s }))
        }
        Ok(o) => serde_json::json!({
            "error": String::from_utf8_lossy(&o.stderr).to_string()
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };

    Ok(serde_json::json!({
        "scripts_dir": scripts.to_string_lossy(),
        "kb_manager_exists": kb_exists,
        "python": py,
        "cwd": cwd,
        "exe": exe,
        "resource_dir": resource,
        "python_modules": deps,
    }))
}
