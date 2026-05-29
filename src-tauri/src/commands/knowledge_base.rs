use crate::utils::{get_scripts_dir, python_cmd};
use crate::commands::common::get_config;

#[tauri::command]
pub async fn list_kb_files() -> Result<serde_json::Value, String> {
    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("list")
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
pub async fn add_to_kb(file_path: String) -> Result<serde_json::Value, String> {
    let config = get_config().await?;
    let config_str = serde_json::to_string(&config).unwrap();

    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("add")
        .arg("--file").arg(file_path)
        .arg("--config").arg(config_str)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
pub async fn get_kb_file_details(filename: String) -> Result<serde_json::Value, String> {
    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("details")
        .arg("--filename").arg(filename)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_kb_file(filename: String) -> Result<serde_json::Value, String> {
    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("delete")
        .arg("--filename").arg(filename)
        .output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("脚本执行失败: {}", err));
    }

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result: serde_json::Value = serde_json::from_str(&result_str)
        .map_err(|_| format!("结果解析失败: {}, stderr: {}", result_str, String::from_utf8_lossy(&output.stderr)))?;
    Ok(result)
}

pub async fn search_kb_internal(query: String) -> Result<String, String> {
    let config = get_config().await?;
    let config_str = serde_json::to_string(&config).unwrap();

    let script_path = get_scripts_dir().join("kb_manager.py");
    let output = python_cmd()
        .arg(&script_path)
        .arg("search")
        .arg("--query").arg(query)
        .arg("--config").arg(config_str)
        .output().await.map_err(|e| e.to_string())?;

    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(result_str)
}

#[tauri::command]
pub async fn kb_search(query: String) -> Result<serde_json::Value, String> {
    let res = search_kb_internal(query).await?;
    let v: serde_json::Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
    Ok(v)
}
