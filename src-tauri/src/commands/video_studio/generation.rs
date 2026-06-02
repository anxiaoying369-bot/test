use tauri::State;
use uuid::Uuid;
use crate::state::AppState;
use crate::utils::{get_data_dir, get_scripts_dir, python_cmd, extract_provider_error};
use crate::commands::common::get_config;
use crate::commands::knowledge_base::search_kb_internal;
use std::fs;

fn resolve_platform_prompt(config: &crate::models::AppConfig, platform_id: &str) -> String {
    let id = platform_id.trim();
    if id.is_empty() { return String::new(); }

    for p in &config.llm.geo_publish_platforms {
        let n = p.name.trim();
        if n.is_empty() { continue; }
        if n == id || n.contains(id) || id.contains(n) {
            if !p.system_prompt.trim().is_empty() {
                return format!("\n【平台风格 · {}】\n{}\n", n, p.system_prompt.trim());
            }
        }
    }

    let (label, prompt) = match id {
        "douyin" | "抖音" => ("抖音",
            "前 3 秒强情绪钩子（疑问/反差/惊吓），口语化短句，每句不超过 12 字。\
             中间段卖点高密度，节奏快。结尾必带强 CTA（点购物车/关注/下方链接）。\
             不要书面语，禁止用'今天我要给大家介绍'这种开头。"),
        "kuaishou" | "快手" => ("快手",
            "走老铁文化路线：接地气、性价比、信任感。开头直白点出产品和价格优势，\
             多用'家人们''老铁''咱家'这类词。中段用对比/亲测展示效果。结尾给福利感（限时/包邮/赠品）。"),
        "wechat-channel" | "视频号" | "video-channel" => ("视频号",
            "调性偏朋友圈：稳重、信任、有人情味。可以中长（30-60s），叙述完整，\
             适当带'我自己用过''朋友推荐''家人都说好'这类背书。CTA 偏柔和，'点小心心''加个好友咨询'。"),
        "xiaohongshu" | "小红书" => ("小红书",
            "种草调性：精致、闺蜜推荐感、关键词扎堆。开头用 emoji + 关键词，\
             中段分点列卖点（'✅' 符号开头），强调真实体验和细节，植入热门 tag 关键词。\
             结尾'冲！''快囤''姐妹们跟上'类号号召。"),
        _ => ("通用", "针对短视频平台优化：开头钩子强，中段信息密集，结尾有行动指令。"),
    };
    format!("\n【平台风格 · {}】\n{}\n", label, prompt)
}

#[tauri::command]
pub async fn video_generate_script(
    product: String,
    reference_script: Option<String>,
    video_ratio: String,
    platform: Option<String>,
    _script_type: Option<String>,
    previous_script: Option<String>,
    feedback: Option<String>,
) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 AI 助理的 LLM API Key".to_string());
    }
    if product.trim().is_empty() {
        return Err("请先填写要卖的产品信息".to_string());
    }

    let platform_id = platform.unwrap_or_default();
    let platform_prompt = resolve_platform_prompt(&config, &platform_id);

    let kb_brand_ctx = match search_kb_internal(product.clone()).await {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!([]));
            let mut buf = String::new();
            if let Some(arr) = v.as_array() {
                for item in arr.iter().take(8) {
                    if let Some(t) = item["text"].as_str() {
                        buf.push_str(&format!("- {}\n", t.trim()));
                    }
                }
            }
            buf
        }
        Err(_) => String::new(),
    };

    let ratio_hint = match video_ratio.as_str() {
        "9:16" => "竖屏短视频（抖音/快手/小红书）",
        "16:9" => "横屏视频（B站/YouTube/视频号横屏）",
        "1:1"  => "方形视频（Instagram/朋友圈）",
        _      => "短视频",
    };
    let mix_query = format!(
        "{} {} {}",
        product.trim(),
        reference_script.as_deref().unwrap_or("").chars().take(80).collect::<String>(),
        ratio_hint
    );
    let kb_mix_ctx = match search_kb_internal(mix_query).await {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!([]));
            let mut buf = String::new();
            if let Some(arr) = v.as_array() {
                for item in arr.iter().take(6) {
                    if let Some(t) = item["text"].as_str() {
                        buf.push_str(&format!("- {}\n", t.trim()));
                    }
                }
            }
            buf
        }
        Err(_) => String::new(),
    };

    // 系统提示词：用户在设置页配置的风格 + 系统强制的 JSON 结构约束
    let style_prompt = if config.video.script_system_prompt.trim().is_empty() {
        crate::models::default_script_system_prompt()
    } else {
        config.video.script_system_prompt.trim().to_string()
    };

    let system_prompt = format!(
        "{style}\n\n\
        【知识库背景（优先级最高，必须引用其中事实）】\n{kb1}\n\n\
        【相关素材检索】\n{kb2}\n\n\
        【平台风格指引】\n{platform}\n\n\
        【视频规格】比例：{ratio}\n\n\
        ====================\n\
        【严格输出要求】\n\
        你必须只返回一个合法的 JSON 对象，不要包含任何解释文字、不要用 ```json 代码块包裹。\n\
        JSON 字段如下（全部必填）：\n\
        {{\n\
          \"视频标题\": \"一句话标题，吸引点击\",\n\
          \"总时长\": \"预估秒数，如 '30秒'\",\n\
          \"语速\": \"建议语速倍数，数字，如 1.0 / 1.2\",\n\
          \"目标受众\": \"这条视频面向的人群\",\n\
          \"口播文案\": \"完整的口播文本，是主播要念出来的全部内容，连贯成段，不要分镜标记\",\n\
          \"核心卖点关键词\": [\"关键词1\", \"关键词2\", \"关键词3\"],\n\
          \"建议素材关键词\": [\"用于搜索配图/空镜的英文或中文关键词1\", \"关键词2\"]\n\
        }}\n\
        注意：\"语速\" 必须是数字（如 1.0），\"核心卖点关键词\" 和 \"建议素材关键词\" 必须是字符串数组。",
        style = style_prompt,
        kb1 = kb_brand_ctx,
        kb2 = kb_mix_ctx,
        platform = platform_prompt,
        ratio = video_ratio,
    );

    let mut user_message = format!("产品信息：{}\n", product);
    if let Some(ref ref_s) = reference_script {
        user_message.push_str(&format!("参考脚本/期望方向：{}\n", ref_s));
    }
    if let (Some(prev), Some(feed)) = (previous_script, feedback) {
        user_message.push_str(&format!("\n--- 上版脚本(JSON) ---\n{}\n\n--- 用户反馈 ---\n{}\n请基于反馈重新生成完整 JSON。", prev, feed));
    }

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    // 不强制 response_format：很多国产中转/模型不支持 json_object，会报错或忽略导致返回异常。
    // 改为靠 prompt 约束 + extract_json_object 容错解析。
    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ],
        "temperature": 0.8
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload)
        .send().await.map_err(|e| format!("脚本生成请求失败: {}", e))?;

    let status = response.status();
    let body_text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("LLM API 错误 {}: {}", status, body_text.chars().take(300).collect::<String>()));
    }

    let res_data: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("LLM 响应解析失败（{}）：{}", e, body_text.chars().take(300).collect::<String>()))?;

    // 容错取 content：标准 OpenAI 在 choices[0].message.content；
    // 个别中转放在 choices[0].text，或把内容直接放别处。
    let raw = res_data["choices"][0]["message"]["content"].as_str()
        .or_else(|| res_data["choices"][0]["text"].as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());

    let raw = match raw {
        Some(s) => s,
        None => {
            // 把实际返回结构暴露出来便于诊断（如 error 字段、内容过滤、空 choices 等）
            if let Some(err) = res_data.get("error") {
                return Err(format!("LLM 返回错误：{}", err));
            }
            return Err(format!(
                "LLM 返回空内容。原始响应：{}",
                body_text.chars().take(400).collect::<String>()
            ));
        }
    };

    // 校验/提取 JSON（兼容模型偶尔包裹 ```json 或夹带文字的情况）
    let cleaned = extract_json_object(&raw);
    let parsed: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("脚本未返回有效 JSON（{}）。原始内容：{}", e, raw.chars().take(300).collect::<String>()))?;

    // 返回规范化后的紧凑 JSON 字符串给前端
    Ok(parsed.to_string())
}

/// 从 LLM 输出里抠出 JSON 对象：去掉 ```json 围栏、取第一个 {...} 块。
fn extract_json_object(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")).unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s).trim();
    // 取第一个 { 到最后一个 } 之间
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        if end > start {
            return s[start..=end].to_string();
        }
    }
    s.to_string()
}

#[tauri::command]
pub async fn video_start_generation(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
    provider: String,
    api_key: String,
    mode: String,
    ratio: String,
    base_url: Option<String>,
    model: Option<String>,
    reference_material_id: Option<String>,
) -> Result<String, String> {
    let reference_path: Option<String> = if let Some(mid) = reference_material_id.as_deref() {
        if mid.is_empty() {
            None
        } else {
            let db = state.video_db.lock().map_err(|e| e.to_string())?;
            let row: Option<(Option<String>, String)> = db
                .query_row(
                    "SELECT local_path, type FROM video_materials WHERE id = ?1 AND project_id = ?2",
                    [mid, &project_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();
            match row {
                Some((Some(p), t)) if t == "image" => Some(p),
                Some((_, t)) if t != "image" => return Err(format!("参考图必须是图片素材，但选中的是 {}", t)),
                _ => return Err("参考图素材不存在".to_string()),
            }
        }
    } else {
        None
    };

    let manager_py = get_scripts_dir().join("video_manager.py");
    let effective_mode = if reference_path.is_some() { "image".to_string() } else { mode.clone() };

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("start")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--prompt").arg(&prompt)
        .arg("--mode").arg(&effective_mode)
        .arg("--ratio").arg(&ratio);

    if let Some(ref p) = reference_path { cmd.arg("--image-url").arg(p); }
    if let Some(url) = base_url { if !url.is_empty() { cmd.arg("--base-url").arg(url); } }
    if let Some(m) = model { if !m.is_empty() { cmd.arg("--model").arg(m); } }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("Python error: {}", stdout))?;
    
    if res["status"] == "error" {
        return Err(extract_provider_error(&res, "AI 视频生成失败"));
    }

    let task_id = res["task_id"].as_str().ok_or("No task_id returned")?.to_string();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO video_tasks (id, project_id, type, status) VALUES (?1, ?2, ?3, ?4)",
        (&task_id, &project_id, "generation", "processing"),
    ).map_err(|e| e.to_string())?;

    Ok(task_id)
}

#[tauri::command]
pub async fn video_poll_task_status(
    state: State<'_, AppState>,
    task_id: String,
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let manager_py = get_scripts_dir().join("video_manager.py");

    let mut cmd = python_cmd();
    cmd.arg(&manager_py)
        .arg("poll")
        .arg("--provider").arg(&provider)
        .arg("--api-key").arg(&api_key)
        .arg("--task-id").arg(&task_id);

    if let Some(url) = base_url { if !url.is_empty() { cmd.arg("--base-url").arg(url); } }
    if let Some(m) = model { if !m.is_empty() { cmd.arg("--model").arg(m); } }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("Python error: {}", stdout))?;

    if res["status"] == "error" {
        let friendly = extract_provider_error(&res, "任务查询失败");
        res["error"] = friendly.into();
    }

    let status = res["status"].as_str().unwrap_or("processing");
    let result_url = res["video_url"].as_str();
    let error_msg = res["error"].as_str();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE video_tasks SET status=?1, result_path=?2, error_msg=?3, updated_at=CURRENT_TIMESTAMP WHERE id=?4",
        (status, result_url, error_msg, &task_id),
    ).map_err(|e| e.to_string())?;

    Ok(res)
}

#[tauri::command]
pub async fn tts_list_voices(
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let manager_py = get_scripts_dir().join("tts_manager.py");
    let mut cmd = python_cmd();
    cmd.arg(&manager_py).arg("list-voices").arg("--provider").arg(&provider).arg("--api-key").arg(&api_key);
    if let Some(u) = base_url.as_ref().filter(|s| !s.is_empty()) { cmd.arg("--base-url").arg(u); }
    if let Some(m) = model.as_ref().filter(|s| !s.is_empty()) { cmd.arg("--model").arg(m); }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("TTS list-voices 返回非 JSON: {}", stdout))?;
    
    if let Some(_err) = res.get("status").and_then(|s| s.as_str()).filter(|s| *s == "error") {
        return Err(extract_provider_error(&res, "获取音色列表失败"));
    }

    Ok(res)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn tts_synthesize(
    state: State<'_, AppState>,
    project_id: String,
    text: String,
    voice_id: String,
    speed: f32,
    // 以下参数仅作兼容保留，实际以 config.json 的 video.tts_* 为准，避免前端缓存了旧值
    provider: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    if text.trim().is_empty() { return Err("文本不能为空".to_string()); }
    let _ = (provider, api_key, base_url, model); // 忽略前端传值

    // 直接从 config.json 读 TTS 配置（最权威，杜绝前端 appConfig 缓存导致用错 provider）
    let config = get_config().await?;
    let v = &config.video;
    let provider = if v.tts_provider.is_empty() { "mock".to_string() } else { v.tts_provider.clone() };
    let api_key = v.tts_api_key.clone();
    let base_url = v.tts_base_url.clone();
    let model = v.tts_model.clone();

    let save_dir = get_data_dir().join("video_studio").join("voiceovers").join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    let material_id = Uuid::new_v4().to_string();
    let filename = format!("voice_{}.mp3", &material_id[..8]);
    let save_path = save_dir.join(&filename);
    let save_path_str = save_path.to_string_lossy().to_string();

    let manager_py = get_scripts_dir().join("tts_manager.py");
    let mut cmd = python_cmd();
    cmd.arg(&manager_py).arg("synthesize").arg("--provider").arg(&provider).arg("--api-key").arg(&api_key).arg("--text").arg(&text).arg("--voice").arg(&voice_id).arg("--speed").arg(format!("{}", speed)).arg("--output").arg(&save_path_str);
    if !base_url.is_empty() { cmd.arg("--base-url").arg(&base_url); }
    if !model.is_empty() { cmd.arg("--model").arg(&model); }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("TTS 返回非 JSON: {}", stdout))?;
    if res["status"] == "error" { return Err(extract_provider_error(&res, "TTS 合成失败")); }
    let audio_path = res["audio_path"].as_str().unwrap_or(&save_path_str).to_string();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let meta_json = serde_json::json!({ "voice_id": voice_id, "speed": speed, "provider": provider, "text_length": text.chars().count() }).to_string();
    db.execute("INSERT INTO video_materials (id, project_id, type, local_path, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", (&material_id, &project_id, "audio", &audio_path, &meta_json, "ai-generated")).map_err(|e| e.to_string())?;

    Ok(audio_path)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn video_generate_image(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
    size: String,
    provider: String,
    api_key: String,
    reference_image_path: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    let manager_py = get_scripts_dir().join("image_manager.py");
    let mut cmd = python_cmd();
    cmd.arg(&manager_py).arg("--provider").arg(&provider).arg("--api-key").arg(&api_key).arg("--prompt").arg(&prompt).arg("--size").arg(&size);
    if let Some(p) = reference_image_path.as_ref().filter(|s| !s.is_empty()) { cmd.arg("--reference-image").arg(p); }
    if let Some(u) = base_url.as_ref().filter(|s| !s.is_empty()) { cmd.arg("--base-url").arg(u); }
    if let Some(m) = model.as_ref().filter(|s| !s.is_empty()) { cmd.arg("--model").arg(m); }

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let res: serde_json::Value = serde_json::from_str(&stdout).map_err(|_| format!("图片生成返回非 JSON: {}", stdout))?;
    if res["status"] == "error" { return Err(extract_provider_error(&res, "AI 图片生成失败")); }
    let image_url = res["image_url"].as_str().ok_or("缺少 image_url 字段")?.to_string();

    let material_id = Uuid::new_v4().to_string();
    let save_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    let (bytes, ext) = if let Some(b64) = image_url.strip_prefix("data:") {
        let comma = b64.find(',').ok_or("data URL 格式错误")?;
        let payload = &b64[comma + 1..];
        use base64::{engine::general_purpose::STANDARD, Engine};
        (STANDARD.decode(payload).map_err(|e| e.to_string())?, "png".to_string())
    } else {
        let resp = reqwest::get(&image_url).await.map_err(|e| e.to_string())?;
        (resp.bytes().await.map_err(|e| e.to_string())?.to_vec(), "png".to_string())
    };

    let filename = format!("ai_{}_{}.{}", &material_id[..8], crate::utils::chrono_now(), ext);
    let save_path = save_dir.join(&filename);
    fs::write(&save_path, bytes).map_err(|e| e.to_string())?;
    let local_path_str = save_path.to_string_lossy().to_string();

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let meta_json = serde_json::json!({ "prompt": prompt, "size": size, "provider": provider }).to_string();
    db.execute("INSERT INTO video_materials (id, project_id, type, local_path, meta, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", (&material_id, &project_id, "image", &local_path_str, &meta_json, "ai-generated")).map_err(|e| e.to_string())?;

    Ok(material_id)
}
