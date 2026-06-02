use std::fs;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
use crate::models::VideoMaterial;
use crate::state::AppState;
use crate::utils::get_data_dir;

#[tauri::command]
pub async fn video_list_materials(state: State<'_, AppState>, project_id: String) -> Result<Vec<VideoMaterial>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, project_id, type, local_path, remote_url, meta, source, created_at
         FROM video_materials WHERE project_id = ?1 ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([&project_id], |row| {
        Ok(VideoMaterial {
            id: row.get(0)?,
            project_id: row.get(1)?,
            material_type: row.get(2)?,
            local_path: row.get(3)?,
            remote_url: row.get(4)?,
            meta: row.get::<_, Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
            source: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut materials = Vec::new();
    for r in rows {
        materials.push(r.map_err(|e| e.to_string())?);
    }
    Ok(materials)
}

#[tauri::command]
pub async fn video_upload_material(
    state: State<'_, AppState>,
    project_id: String,
    source_path: String,
    material_type: String,
) -> Result<VideoMaterial, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    
    let material_id = Uuid::new_v4().to_string();
    let source_p = PathBuf::from(&source_path);
    if !source_p.exists() {
        return Err("源文件不存在".to_string());
    }

    let ext = source_p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let dest_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest_filename = format!("{}.{}", material_id, ext);
    let dest_path = dest_dir.join(&dest_filename);

    fs::copy(&source_p, &dest_path).map_err(|e| e.to_string())?;

    let material = VideoMaterial {
        id: material_id,
        project_id: project_id.clone(),
        material_type,
        local_path: Some(dest_path.to_string_lossy().to_string()),
        remote_url: None,
        meta: None,
        source: "uploaded".to_string(),
        created_at: None,
    };

    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, source)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&material.id, &material.project_id, &material.material_type, &material.local_path, &material.source],
    ).map_err(|e| e.to_string())?;

    Ok(material)
}

#[tauri::command]
pub async fn video_delete_material(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    
    let (_project_id, local_path): (String, Option<String>) = db.query_row(
        "SELECT project_id, local_path FROM video_materials WHERE id = ?1",
        [&id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| e.to_string())?;

    if let Some(p) = local_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            let _ = fs::remove_file(pb);
        }
    }

    db.execute("DELETE FROM video_materials WHERE id = ?1", [&id]).map_err(|e| e.to_string())?;
    Ok(())
}
