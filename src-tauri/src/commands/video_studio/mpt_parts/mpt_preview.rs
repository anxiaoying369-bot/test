fn stage_label(progress: i64) -> &'static str {
    match progress {
        0..=9 => "生成脚本",
        10..=19 => "生成关键词",
        20..=29 => "合成配音",
        30..=39 => "生成字幕",
        40..=49 => "准备素材",
        50..=99 => "拼接与字幕烧录",
        _ => "完成",
    }
}

#[tauri::command]
pub async fn video_mpt_preview_voice(voice_name: String) -> Result<String, String> {
    // 试听只用 Edge TTS 合成一小段示例音频，不需要 LLM/Pexels，因此用最小配置。
    let storage = mpt_storage_dir();
    fs::create_dir_all(&storage).map_err(|e| e.to_string())?;
    let tmp = storage.join(format!("preview-{}.json", Uuid::new_v4().simple()));
    fs::write(&tmp, json!({ "voice_name": voice_name }).to_string()).map_err(|e| e.to_string())?;

    let helper_py = get_scripts_dir().join("mpt_helper.py");
    let mut cmd = python_cmd();
    cmd.arg(&helper_py)
        .arg("preview")
        .arg("--params")
        .arg(&tmp)
        .env("MPT_CONFIG", "{\"app\":{}}")
        .env("MPT_STORAGE_DIR", storage.to_string_lossy().to_string())
        .env("IMAGEIO_FFMPEG_EXE", get_ffmpeg_path());

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&tmp);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let last = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let res: Value =
        serde_json::from_str(last).map_err(|_| format!("音色试听失败：{}", stdout))?;
    if res.get("status").and_then(|v| v.as_str()) == Some("error") {
        return Err(extract_provider_error(&res, "音色试听失败"));
    }
    res.get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "音色试听未返回音频路径".to_string())
}

#[tauri::command]
pub async fn video_mpt_generate_terms(
    video_subject: String,
    video_script: String,
    amount: Option<u32>,
) -> Result<Vec<String>, String> {
    let config = get_config().await?;
    let mpt_config = build_mpt_config(&config, None)?;

    let storage = mpt_storage_dir();
    fs::create_dir_all(&storage).map_err(|e| e.to_string())?;
    let tmp = storage.join(format!("terms-{}.json", Uuid::new_v4().simple()));
    let payload = json!({
        "video_subject": video_subject,
        "video_script": video_script,
        "amount": amount.unwrap_or(5),
    });
    fs::write(&tmp, payload.to_string()).map_err(|e| e.to_string())?;

    let helper_py = get_scripts_dir().join("mpt_helper.py");
    let mut cmd = python_cmd();
    cmd.arg(&helper_py)
        .arg("terms")
        .arg("--params")
        .arg(&tmp)
        .env("MPT_CONFIG", &mpt_config)
        .env("MPT_STORAGE_DIR", storage.to_string_lossy().to_string());

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&tmp);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // 取最后一行 JSON（前面可能有空行）。
    let last = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let res: Value =
        serde_json::from_str(last).map_err(|_| format!("关键词生成失败：{}", stdout))?;
    if res.get("status").and_then(|v| v.as_str()) == Some("error") {
        return Err(extract_provider_error(&res, "关键词生成失败"));
    }
    let terms = res
        .get("terms")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(terms)
}
