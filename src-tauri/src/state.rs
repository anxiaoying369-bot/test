use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use crate::models::{LoginFlow, Task};

pub static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static SCRIPTS_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static BUNDLED_PYTHON: OnceLock<String> = OnceLock::new();

pub struct AppState {
    pub login_flows: Mutex<std::collections::HashMap<String, LoginFlow>>,
    pub tasks: Mutex<std::collections::HashMap<String, Task>>,
    pub process_handles: Mutex<std::collections::HashMap<String, tokio::process::Child>>,
    pub current_task_id: Mutex<Option<String>>,
    pub video_db: Mutex<rusqlite::Connection>,
}
