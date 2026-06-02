use tauri::State;
use crate::state::AppState;
use crate::models::Task;
use sysinfo::{System, ProcessRefreshKind, RefreshKind, Pid, ProcessesToUpdate};

#[tauri::command]
pub async fn list_active_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let mut tasks_lock = state.tasks.lock().map_err(|e| e.to_string())?;
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;

    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new().with_cpu().with_memory()),
    );
    sys.refresh_processes_specifics(ProcessesToUpdate::All, ProcessRefreshKind::new().with_cpu().with_memory());

    // 清理已结束的任务
    let mut to_remove = Vec::new();
    for (id, child) in handles.iter_mut() {
        if let Ok(Some(_)) = child.try_wait() {
            if let Some(task) = tasks_lock.get_mut(id) {
                task.status = "finished".to_string();
                task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
            to_remove.push(id.clone());
        }
    }
    for id in to_remove {
        handles.remove(&id);
    }

    // 更新存活任务的资源占用
    for (id, task) in tasks_lock.iter_mut() {
        if task.status == "running" {
            if let Some(pid_u32) = task.pid {
                if let Some(process) = sys.process(Pid::from(pid_u32 as usize)) {
                    task.cpu = process.cpu_usage();
                    task.memory = process.memory(); // 以字节为单位
                } else {
                    // 进程可能已经消失但 handles 还没清理
                    task.status = "finished".to_string();
                }
            }
        } else {
            task.cpu = 0.0;
            task.memory = 0;
        }
    }

    let mut res: Vec<Task> = tasks_lock.values().cloned().collect();
    res.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(res)
}

#[tauri::command]
pub async fn kill_task(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut handles = state.process_handles.lock().map_err(|e| e.to_string())?;
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;

    if let Some(mut child) = handles.remove(&task_id) {
        let _ = child.start_kill();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = "killed".to_string();
            task.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        Ok(())
    } else {
        Err("任务不存在或已结束".to_string())
    }
}
