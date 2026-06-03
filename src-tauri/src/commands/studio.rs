use crate::commands::common::get_config;
use crate::commands::knowledge_base::search_kb_internal;

#[tauri::command]
pub async fn studio_generate_content(
    topic: String,
    material: String,
    mode: String,
    platform: String,
    platform_prompt: Option<String>,
) -> Result<serde_json::Value, String> {
    studio_generate_internal(topic, material, mode, platform, platform_prompt).await
}

pub async fn studio_generate_internal(
    topic: String,
    material: String,
    mode: String,
    platform: String,
    platform_prompt: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() {
        return Err("请先在设置中配置 LLM API Key".to_string());
    }

    let query = if topic.is_empty() { material.chars().take(50).collect::<String>() } else { topic.clone() };
    let kb_context = match search_kb_internal(query).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut ctx = String::from("\n参考的企业知识库背景：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(5) {
                    if let Some(text) = item["text"].as_str() { ctx.push_str(&format!("- {}\n", text)); }
                }
            }
            if ctx.len() < 20 { String::new() } else { ctx }
        }
        Err(_) => String::new(),
    };

    let platform_instructions_owned;
    let platform_instructions: &str = if let Some(ref p) = platform_prompt {
        if !p.trim().is_empty() {
            platform_instructions_owned = p.clone();
            &platform_instructions_owned
        } else {
            get_default_platform_instructions(&platform)
        }
    } else {
        get_default_platform_instructions(&platform)
    };

    let system_prompt = format!(
        "你是一位资深的 AI 内容专家和 GEO（生成式引擎优化）专家。\n\
        你的任务是根据提供的素材和知识库内容，为用户创作或改造高质量内容。\n\n\
        核心准则：\n\
        1. **答案前置 (Answer-First)**：直接在内容开头回答核心问题或展示最核心价值。\n\
        2. **事实密度最大化**：大量使用知识库中的具体数据、技术指标和事实描述，避免空洞的形容词。\n\
        3. **权威性构建**：语言风格专业，逻辑严密。\n\n\
        {}\n\n{}",
        platform_instructions, kb_context
    );

    let user_content = if mode == "new" {
        format!("请围绕主题「{}」创作一篇全新的文章。补充素材：{}", topic, material)
    } else {
        format!("请对以下内容进行 GEO 深度改造和重写：\n\n{}", material)
    };

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") {
        config.llm.base_url.clone()
    } else {
        format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/'))
    };

    let gen_payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ],
        "temperature": 0.7
    });
    let gen_resp: serde_json::Value = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&gen_payload)
        .send().await.map_err(|e| format!("生成内容失败: {}", e))?
        .json().await.map_err(|e| e.to_string())?;
    let generated_content = gen_resp["choices"][0]["message"]["content"].as_str().ok_or("LLM 返回内容为空")?.to_string();

    let audit_report = audit_content_internal(generated_content.clone()).await.unwrap_or("审计失败".to_string());

    Ok(serde_json::json!({ "content": generated_content, "audit": audit_report }))
}

fn get_default_platform_instructions(platform: &str) -> &'static str {
    match platform {
        "douyin" => "【抖音/短视频平台优化】：要求开头前 3 秒有极其吸引人的\"情绪钩子\"，中间事实密集，语言口语化，结尾有强引导。采用\"答案前置\"结构，直接在开头揭示核心价值。",
        "wechat" => "【微信公众号优化】：要求排版精美感，深度分析，事实密度极高，建立 E-E-A-T 权威感。采用\"答案前置\"结构，首段即总结全文精华。",
        "zhihu"  => "【知乎/专业社区优化】：要求专业严谨，大量引用事实和数据，逻辑性强。直接回答问题核心，避免废话。",
        _        => "采用答案前置结构，提高事实密度。",
    }
}

pub async fn audit_content_internal(content: String) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() { return Err("请先在设置中配置 LLM API Key".to_string()); }
    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") { config.llm.base_url.clone() } else { format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/')) };
    let system = "你是一位冷静的内容审计员和舆情分析师。\n\
        请对提供的内容进行\"发布前压力测试\"，输出一份简洁的 Markdown 审计报告。\n\
        报告需包含：\n\
        1. **舆情预判**：模拟读者看到该内容后的潜在反应（积极、争议点）。\n\
        2. **GEO 评分**：针对\"答案前置\"和\"事实密度\"给出 0-100 的评分。\n\
        3. **改进建议**：如何让内容更专业、更具 AI 引擎可见性。\n\
        4. **敏感性核查**：是否存在不合规风险。";
    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": format!("请对以下内容进行审计分析：\n\n{}", content) }
        ],
        "temperature": 0.3
    });
    let resp: serde_json::Value = client.post(&url).header("Authorization", format!("Bearer {}", config.llm.api_key)).json(&payload).send().await.map_err(|e| format!("审计失败: {}", e))?.json().await.map_err(|e| e.to_string())?;
    Ok(resp["choices"][0]["message"]["content"].as_str().ok_or("审计返回为空")?.to_string())
}

#[tauri::command]
pub async fn studio_analyze_video_comments(comments: Vec<serde_json::Value>) -> Result<String, String> {
    let config = get_config().await?;
    if config.llm.api_key.is_empty() { return Err("请先在设置中配置 LLM API Key".to_string()); }
    if comments.is_empty() { return Err("没有可分析的评论内容".to_string()); }

    let mut text_to_analyze = String::new();
    for (idx, c) in comments.iter().take(50).enumerate() {
        let content = c["text"].as_str().unwrap_or("");
        if !content.is_empty() { text_to_analyze.push_str(&format!("{}. {}\n", idx + 1, content)); }
    }

    let system_prompt = if config.llm.analysis_prompt.is_empty() {
        "你是一位资深的社交媒体数据分析师。我会为你提供一组短视频评论数据，请从以下几个维度进行深度分析：\n1. 舆情氛围：整体情绪倾向（积极、消极、中立）及其占比。\n2. 核心热点：用户最关心的前3个话题或痛点。\n3. 用户意图：是否存在高潜力的咨询、购买意向或反馈建议。\n4. 互动建议：针对当前评论区，建议运营人员如何进行回复或引导。\n请用专业且简洁的 Markdown 格式输出分析报告。".to_string()
    } else {
        config.llm.analysis_prompt.clone()
    };

    let query_for_kb = comments.get(0).and_then(|c| c["text"].as_str()).unwrap_or("产品评价").to_string();
    let kb_context = match search_kb_internal(query_for_kb).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or(serde_json::json!([]));
            let mut ctx = String::from("\n相关背景/产品手册知识：\n");
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(5) {
                    if let Some(text) = item["text"].as_str() { ctx.push_str(&format!("- {}\n", text)); }
                }
            }
            if ctx.len() < 20 { String::new() } else { ctx }
        },
        Err(_) => String::new(),
    };

    let client = reqwest::Client::new();
    let url = if config.llm.base_url.ends_with("/chat/completions") { config.llm.base_url.clone() } else { format!("{}/chat/completions", config.llm.base_url.trim_end_matches('/')) };
    let payload = serde_json::json!({
        "model": config.llm.model,
        "messages": [
            { "role": "system", "content": format!("{}\n\n以下是与当前内容相关的企业知识库信息作为参考：\n{}", system_prompt, kb_context) },
            { "role": "user", "content": format!("请分析以下评论：\n\n{}", text_to_analyze) }
        ],
        "temperature": 0.7
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .json(&payload).send().await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = response.status();
    let body_text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("LLM API 错误 {}: {}", status, body_text.chars().take(300).collect::<String>()));
    }

    let resp_data: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("LLM 响应解析失败（{}）：{}", e, body_text.chars().take(300).collect::<String>()))?;

    let content = resp_data["choices"][0]["message"]["content"].as_str()
        .or_else(|| resp_data["choices"][0]["text"].as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());

    match content {
        Some(s) => Ok(s),
        None => {
            if let Some(err) = resp_data.get("error") {
                return Err(format!("LLM 返回错误：{}", err));
            }
            Err(format!("LLM 返回空内容。原始响应：{}", body_text.chars().take(400).collect::<String>()))
        }
    }
}
