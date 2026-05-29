use crate::commands::knowledge_base::search_kb_internal;

#[tauri::command]
pub async fn geo_monitor_query(query: String) -> Result<serde_json::Value, String> {
    let kb_res = search_kb_internal(query.clone()).await?;
    let kb_v: serde_json::Value = serde_json::from_str(&kb_res).unwrap_or(serde_json::json!([]));
    
    let mut context = String::new();
    if let Some(arr) = kb_v.as_array() {
        for item in arr.iter().take(3) {
            if let Some(t) = item["text"].as_str() {
                context.push_str(t);
                context.push('\n');
            }
        }
    }

    Ok(serde_json::json!({
        "query": query,
        "kb_context": context,
        "status": "ok"
    }))
}
