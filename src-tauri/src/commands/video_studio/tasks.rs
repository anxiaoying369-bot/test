use tauri::State;
use crate::models::VideoTask;
use crate::state::AppState;

#[tauri::command]
pub async fn video_list_tasks(state: State<'_, AppState>, project_id: Option<String>) -> Result<Vec<VideoTask>, String> {
    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    let mut tasks = Vec::new();
    
    if let Some(pid) = project_id {
        let mut stmt = db.prepare("SELECT id, project_id, type, status, progress, result_path, error_msg, created_at, updated_at FROM video_tasks WHERE project_id = ?1 ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([pid], |row| Ok(VideoTask { id: row.get(0)?, project_id: row.get(1)?, task_type: row.get(2)?, status: row.get(3)?, progress: row.get(4)?, result_path: row.get(5)?, error_msg: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)? })).map_err(|e| e.to_string())?;
        for row in rows { tasks.push(row.map_err(|e| e.to_string())?); }
    } else {
        let mut stmt = db.prepare("SELECT id, project_id, type, status, progress, result_path, error_msg, created_at, updated_at FROM video_tasks ORDER BY created_at DESC LIMIT 50").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| Ok(VideoTask { id: row.get(0)?, project_id: row.get(1)?, task_type: row.get(2)?, status: row.get(3)?, progress: row.get(4)?, result_path: row.get(5)?, error_msg: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)? })).map_err(|e| e.to_string())?;
        for row in rows { tasks.push(row.map_err(|e| e.to_string())?); }
    }
    Ok(tasks)
}
