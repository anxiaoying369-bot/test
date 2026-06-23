use crate::commands::common::get_config;
use crate::commands::knowledge_base::search_kb_internal;
use serde_json::json;

#[tauri::command]
pub async fn geo_monitor_query(
    brand: Option<String>,
    keyword: Option<String>,
) -> Result<serde_json::Value, String> {
    let brand = brand.unwrap_or_default().trim().to_string();
    let keyword = keyword.unwrap_or_default().trim().to_string();
    let query = if keyword.is_empty() { brand.clone() } else { keyword.clone() };
    if query.is_empty() {
        return Err("查询内容不能为空".to_string());
    }

    let config = get_config().await?;
    let enabled_models: Vec<_> = config.llm.geo_models
        .iter()
        .filter(|m| m.enabled && !m.name.trim().is_empty() && !m.base_url.trim().is_empty() && !m.api_key.trim().is_empty() && !m.model_id.trim().is_empty())
        .cloned()
        .collect();

    if enabled_models.is_empty() {
        return Err("请先在设置 → GEO 监控中配置至少一个启用的模型".to_string());
    }

    let kb_context = build_kb_context(&query).await;
    let client = reqwest::Client::new();
    let mut results = Vec::new();

    for model in enabled_models {
        let url = if model.base_url.ends_with("/chat/completions") {
            model.base_url.clone()
        } else {
            format!("{}/chat/completions", model.base_url.trim_end_matches('/'))
        };

        let system_prompt = "你是一个客观的 AI 搜索/推荐引擎。请直接回答用户问题，必要时列出品牌、产品或方案，并尽量给出信息来源名称或链接。";
        let user_prompt = if kb_context.is_empty() {
            query.clone()
        } else {
            format!("用户问题：{}\n\n可参考的企业知识库材料：\n{}", query, kb_context)
        };

        let payload = json!({
            "model": model.model_id,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": 0.2
        });

        let result = match client.post(&url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    json!({
                        "model_name": model.name,
                        "mentioned": false,
                        "position": 0,
                        "response": "",
                        "sources": [],
                        "error": format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>())
                    })
                } else {
                    parse_geo_response(&model.name, &brand, &body)
                }
            }
            Err(e) => json!({
                "model_name": model.name,
                "mentioned": false,
                "position": 0,
                "response": "",
                "sources": [],
                "error": format!("请求失败: {}", e)
            }),
        };
        results.push(result);
    }

    Ok(serde_json::Value::Array(results))
}

async fn build_kb_context(query: &str) -> String {
    match search_kb_internal(query.to_string()).await {
        Ok(res_str) => {
            let res: serde_json::Value = serde_json::from_str(&res_str).unwrap_or_else(|_| json!([]));
            let mut context = String::new();
            if let Some(arr) = res.as_array() {
                for item in arr.iter().take(5) {
                    if let Some(text) = item["text"].as_str() {
                        if !text.trim().is_empty() {
                            context.push_str("- ");
                            context.push_str(text.trim());
                            context.push('\n');
                        }
                    }
                }
            }
            context
        }
        Err(_) => String::new(),
    }
}

fn parse_geo_response(model_name: &str, brand: &str, body: &str) -> serde_json::Value {
    let resp_data: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json!({
            "model_name": model_name,
            "mentioned": false,
            "position": 0,
            "response": "",
            "sources": [],
            "error": format!("响应解析失败: {}", e)
        }),
    };

    let response = resp_data["choices"][0]["message"]["content"].as_str()
        .or_else(|| resp_data["choices"][0]["text"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    if response.is_empty() {
        return json!({
            "model_name": model_name,
            "mentioned": false,
            "position": 0,
            "response": "",
            "sources": [],
            "error": "模型返回空内容"
        });
    }

    let (mentioned, position) = detect_brand_position(&response, brand);
    let sources = extract_sources(&response);

    json!({
        "model_name": model_name,
        "mentioned": mentioned,
        "position": position,
        "response": response,
        "sources": sources,
        "error": serde_json::Value::Null
    })
}

fn detect_brand_position(response: &str, brand: &str) -> (bool, i64) {
    let brand = brand.trim();
    if brand.is_empty() {
        return (false, 0);
    }
    let response_lower = response.to_lowercase();
    let brand_lower = brand.to_lowercase();
    if !response_lower.contains(&brand_lower) {
        return (false, 0);
    }

    let mut position = 1_i64;
    for line in response.lines() {
        let clean = line.trim();
        if clean.is_empty() { continue; }
        if clean.to_lowercase().contains(&brand_lower) {
            return (true, position.max(1));
        }
        if looks_like_ranked_item(clean) {
            position += 1;
        }
    }
    (true, 1)
}

fn looks_like_ranked_item(line: &str) -> bool {
    let line = line.trim_start();
    if line.starts_with('-') || line.starts_with('*') || line.starts_with('•') {
        return true;
    }
    let mut chars = line.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_digit())
}

fn extract_sources(response: &str) -> Vec<String> {
    let mut sources = Vec::new();
    for token in response.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| matches!(c, '，' | '。' | ',' | '.' | ')' | '）' | ']' | '】' | '"' | '\''));
        if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
            let value = cleaned.to_string();
            if !sources.contains(&value) {
                sources.push(value);
            }
        }
    }
    sources.truncate(10);
    sources
}
