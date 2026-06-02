use std::sync::Mutex;
use tauri::Manager;

pub mod models;
pub mod state;
pub mod utils;
pub mod db;
pub mod ffmpeg;
pub mod commands;

use crate::state::{AppState, RESOURCE_DIR};
use crate::utils::get_data_dir;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Ok(res_dir) = app.path().resource_dir() {
                let _ = RESOURCE_DIR.set(res_dir);
            }
            Ok(())
        })
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            tasks: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
            current_task_id: Mutex::new(None),
            video_db: Mutex::new(db::init_db(get_data_dir()).expect("Failed to init video database")),
        })
        .invoke_handler(tauri::generate_handler![
            // Diagnostics
            crate::commands::diagnostics::autocast_diagnostics,

            // Config
            crate::commands::common::get_config,
            crate::commands::common::save_config,
            crate::commands::common::get_default_config,

            // Knowledge Base
            crate::commands::knowledge_base::list_kb_files,
            crate::commands::knowledge_base::add_to_kb,
            crate::commands::knowledge_base::delete_kb_file,
            crate::commands::knowledge_base::get_kb_file_details,
            crate::commands::knowledge_base::kb_search,

            // Studio
            crate::commands::studio::studio_generate_content,
            crate::commands::studio::analyze_video_comments,

            // Accounts
            crate::commands::account::list_accounts,
            crate::commands::account::verify_account,
            crate::commands::account::delete_account,
            crate::commands::account::sync_local_accounts,
            crate::commands::account::init_login_session,
            crate::commands::account::get_login_status,
            crate::commands::account::finish_login,
            crate::commands::account::cleanup_login_session,

            // Scraper
            crate::commands::scraper::start_scrape,
            crate::commands::scraper::get_scrape_progress,
            crate::commands::scraper::cancel_scrape,
            crate::commands::scraper::get_current_task,
            crate::commands::scraper::clear_current_task,
            crate::commands::scraper::list_scraped_users,
            crate::commands::scraper::get_scraped_videos,
            crate::commands::scraper::get_scraped_comments,
            crate::commands::scraper::delete_scraped_user,
            crate::commands::scraper::resolve_user_sec_uid,
            crate::commands::scraper::open_video_in_browser,

            // Live Monitor
            crate::commands::live_monitor::start_live_monitor,
            crate::commands::live_monitor::stop_live_monitor,
            crate::commands::live_monitor::get_active_monitors,
            crate::commands::live_monitor::get_live_history,
            crate::commands::live_monitor::resolve_live_url,
            crate::commands::live_monitor::generate_live_reply,

            // Chat
            crate::commands::chat::list_chat_sessions,
            crate::commands::chat::create_chat_session,
            crate::commands::chat::delete_chat_session,
            crate::commands::chat::send_chat_message,
            crate::commands::chat::get_chat_messages,

            // Hermes
            crate::commands::hermes::start_hermes_gateway,
            crate::commands::hermes::stop_hermes_gateway,
            crate::commands::hermes::check_hermes_status,
            crate::commands::hermes::check_hermes_gateway_health,
            crate::commands::hermes::list_hermes_sessions,
            crate::commands::hermes::hermes_enable_api_server,
            crate::commands::hermes::hermes_restart_service,
            crate::commands::hermes::hermes_read_api_key,
            crate::commands::hermes::hermes_set_api_key,
            crate::commands::hermes::hermes_send_message,
            crate::commands::hermes::hermes_list_runs,
            crate::commands::hermes::hermes_stop_run,
            crate::commands::hermes::hermes_approve_run,
            crate::commands::hermes::hermes_list_skills,
            crate::commands::hermes::hermes_install_skill,
            crate::commands::hermes::hermes_uninstall_skill,
            crate::commands::hermes::hermes_list_tools,
            crate::commands::hermes::hermes_get_session_messages,
            crate::commands::hermes::hermes_toggle_skill_status,
            crate::commands::hermes::hermes_toggle_tool_status,
            crate::commands::hermes::hermes_search_kb,

            // Geo
            crate::commands::geo::geo_monitor_query,

            // Dashboard
            crate::commands::dashboard::list_active_tasks,
            crate::commands::dashboard::kill_task,

            // Common
            crate::commands::common::open_file_in_finder,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let app_state = app_handle.state::<AppState>();
                let lock_result = app_state.process_handles.lock();
                if let Ok(mut handles) = lock_result {
                    #[cfg(unix)]
                    for (_, child) in handles.iter() {
                        if let Some(pid) = child.id() {
                            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    for (_, mut child) in handles.drain() {
                        let _ = child.start_kill();
                    }
                }
            }
        });
}