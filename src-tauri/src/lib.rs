use tauri_plugin_shell::ShellExt;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SpyResult {
    status: String,
    target: String,
    data: serde_json::Value,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn spy_xiaohongshu(app: tauri::AppHandle, keyword: &str) -> Result<String, String> {
    let output = app.shell().command("npx")
        .args(["tsx", "scripts/xhs_spy.ts", keyword])
        .output().await.map_err(|e| e.to_string())?;
        
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            spy_xiaohongshu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
