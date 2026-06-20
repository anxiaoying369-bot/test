use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use crate::models::{AppConfig, LLMConfig};
use crate::utils::get_data_dir;

/// 调用 LLM 前的统一校验：api_key / base_url / model 任一为空都视为"未配置"，
/// 返回清晰的提醒（指向设置页）。各 LLM 入口命令应先调用它。
pub fn ensure_llm_configured(llm: &LLMConfig) -> Result<(), String> {
    let mut missing = Vec::new();
    if llm.api_key.trim().is_empty() { missing.push("API Key"); }
    if llm.base_url.trim().is_empty() { missing.push("Base URL"); }
    if llm.model.trim().is_empty() { missing.push("模型名称"); }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "尚未配置 LLM 模型（缺少：{}）。请到「系统设置 → AI 模型」填写并点「测试连接」通过后保存。",
            missing.join("、")
        ))
    }
}

/// 拼出 chat/completions 完整 URL（兼容 base_url 是否已带该后缀）。
fn chat_completions_url(base_url: &str) -> String {
    if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    }
}

/// 测试 LLM 连接：用传入的（可能尚未保存的）配置发一次最小请求，验证 key/地址/模型是否可用。
/// 供设置页「测试连接」按钮调用，测试通过前不允许保存。
#[tauri::command]
pub async fn test_llm_connection(
    api_key: String,
    base_url: String,
    model: String,
) -> Result<String, String> {
    if api_key.trim().is_empty() || base_url.trim().is_empty() || model.trim().is_empty() {
        return Err("请先填写 API Key、Base URL 和模型名称".to_string());
    }

    let url = chat_completions_url(base_url.trim());
    let payload = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 1,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("连接失败：{}（请检查 Base URL 与网络）", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let snippet: String = body.chars().take(200).collect();
        let hint = match status.as_u16() {
            401 | 403 => "（API Key 可能无效）",
            404 => "（Base URL 或模型名称可能不对）",
            429 => "（请求过于频繁或余额不足）",
            _ => "",
        };
        return Err(format!("LLM 返回错误 {} {}: {}", status.as_u16(), hint, snippet));
    }

    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| "返回内容不是合法 JSON，Base URL 可能不是 OpenAI 兼容接口".to_string())?;
    if let Some(err) = v.get("error") {
        return Err(format!("LLM 返回错误：{}", err));
    }
    if v.get("choices").and_then(|c| c.as_array()).map(|a| !a.is_empty()).unwrap_or(false) {
        Ok(format!("连接成功，模型「{}」可用 ✓", model))
    } else {
        Err("返回结构异常（无 choices），请确认这是 chat/completions 兼容接口".to_string())
    }
}

#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    let path = get_data_dir().join("config.json");
    if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: AppConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

#[tauri::command]
pub async fn save_config(config: AppConfig) -> Result<(), String> {
    let path = get_data_dir().join("config.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_config() -> Result<AppConfig, String> {
    Ok(AppConfig::default())
}

#[tauri::command]
pub async fn open_file_in_finder(path: String) -> Result<(), String> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err("文件路径不存在".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = p.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
