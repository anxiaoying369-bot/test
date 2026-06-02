use tauri::State;
use uuid::Uuid;
use crate::models::VideoProject;
use crate::state::AppState;

#[tauri::command]
pub async fn video_clone_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<VideoProject, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.prepare(
        "SELECT title, description, config FROM video_projects WHERE id = ?1"
    ).map_err(|e| e.to_string())?;
    let (title, description, config_str): (String, Option<String>, Option<String>) = stmt
        .query_row([&id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| format!("源项目不存在: {}", e))?;

    let mut config_val: serde_json::Value = config_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    if let Some(script_obj) = config_val.get_mut("script").and_then(|v| v.as_object_mut()) {
        script_obj.remove("generatedScript");
        script_obj.remove("generationPrompt");
        script_obj.insert("scriptConfirmed".into(), serde_json::json!(false));
    }

    let new_id = Uuid::new_v4().to_string();
    let new_title = format!("{}（副本）", title);
    let new_config_str = serde_json::to_string(&config_val).unwrap_or_default();

    db.execute(
        "INSERT INTO video_projects (id, title, description, config, status)
         VALUES (?1, ?2, ?3, ?4, 'draft')",
        rusqlite::params![&new_id, &new_title, &description, &new_config_str],
    ).map_err(|e| e.to_string())?;

    Ok(VideoProject {
        id: new_id,
        title: new_title,
        description,
        config: Some(config_val),
        status: "draft".to_string(),
        locked_at: None,
        final_video_path: None,
        created_at: None,
        updated_at: None,
    })
}

#[tauri::command]
pub async fn video_list_projects(state: State<'_, AppState>) -> Result<Vec<VideoProject>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, title, description, config, status, locked_at, final_video_path, created_at, updated_at
         FROM video_projects ORDER BY updated_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(VideoProject {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            config: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
            status: row.get(4)?,
            locked_at: row.get(5)?,
            final_video_path: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut projects = Vec::new();
    for r in rows {
        projects.push(r.map_err(|e| e.to_string())?);
    }
    Ok(projects)
}

#[tauri::command]
pub async fn video_upsert_project(state: State<'_, AppState>, project: VideoProject) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    
    let config_str = project.config.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());

    db.execute(
        "INSERT INTO video_projects (id, title, description, config, status)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            config = excluded.config,
            status = excluded.status,
            updated_at = CURRENT_TIMESTAMP",
        rusqlite::params![&project.id, &project.title, &project.description, &config_str, &project.status],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn video_delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    // 必须先删子表（有 FOREIGN KEY 引用 video_projects），最后删父表，否则触发外键约束失败
    db.execute("DELETE FROM video_materials WHERE project_id = ?1", [&id]).map_err(|e| e.to_string())?;
    db.execute("DELETE FROM video_tasks WHERE project_id = ?1", [&id]).map_err(|e| e.to_string())?;
    db.execute("DELETE FROM video_projects WHERE id = ?1", [&id]).map_err(|e| e.to_string())?;
    Ok(())
}
