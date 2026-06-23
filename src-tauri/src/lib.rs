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
            // 手机无线控制 WebSocket Server（autocast-mobile 连接 1422 端口）
            let mobile_devices = app.state::<AppState>().mobile.devices.clone();
            tauri::async_runtime::spawn(crate::commands::mobile::run_ws_server(
                app.handle().clone(),
                mobile_devices,
            ));
            // 发布排期调度循环：常驻轮询到点的待发布任务
            crate::commands::publisher::run_publish_scheduler(app.handle().clone());
            Ok(())
        })
        .manage(AppState {
            login_flows: Mutex::new(std::collections::HashMap::new()),
            process_handles: Mutex::new(std::collections::HashMap::new()),
            current_task_id: Mutex::new(None),
            video_db: Mutex::new(db::init_db(get_data_dir()).expect("Failed to init video database")),
            wechat: tokio::sync::Mutex::new(None),
            mobile: Default::default(),
        })
        .invoke_handler(tauri::generate_handler![
            // Diagnostics
            crate::commands::diagnostics::autocast_diagnostics,

            // Config
            crate::commands::common::get_config,
            crate::commands::common::save_config,
            crate::commands::common::get_default_config,
            crate::commands::common::test_llm_connection,
            crate::commands::common::list_relay_models,

            // Knowledge Base
            crate::commands::knowledge_base::list_kb_files,
            crate::commands::knowledge_base::add_to_kb,
            crate::commands::knowledge_base::delete_kb_file,
            crate::commands::knowledge_base::get_kb_file_details,
            crate::commands::knowledge_base::kb_search,

            // Studio
            crate::commands::studio::studio_generate_content,
            crate::commands::studio::studio_analyze_video_comments,

            // Accounts
            crate::commands::account::list_accounts,
            crate::commands::account::verify_account,
            crate::commands::account::delete_account,
            crate::commands::account::sync_local_accounts,
            crate::commands::account::init_login_session,
            crate::commands::account::get_login_status,
            crate::commands::account::finish_login,
            crate::commands::account::refresh_account_credential,
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
            crate::commands::scraper::fetch_douyin_user_info,
            crate::commands::scraper::open_video_in_browser,

            // User Cards (用户信息库)
            crate::commands::user_cards::list_user_cards,
            crate::commands::user_cards::delete_user_card,
            crate::commands::user_cards::query_and_save_user,
            crate::commands::user_cards::refresh_user_card,

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
            crate::commands::chat::confirm_tool_execution,
            crate::commands::chat::cancel_tool_execution,

            // Hermes
            crate::commands::hermes::check_hermes_installed,
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

            // Common
            crate::commands::common::open_file_in_finder,

            // Video Studio Project
            crate::commands::video_studio::project::video_list_projects,
            crate::commands::video_studio::project::video_upsert_project,
            crate::commands::video_studio::project::video_delete_project,
            crate::commands::video_studio::project::video_clone_project,

            // Video Studio Material
            crate::commands::video_studio::material::video_list_materials,
            crate::commands::video_studio::material::video_upload_material,
            crate::commands::video_studio::material::video_delete_material,

            // Video Studio Generation（脚本生成保留：含知识库 + 平台风格注入，作为 MPT 管线脚本来源）
            crate::commands::video_studio::generation::video_generate_script,

            // Video Studio · MoneyPrinterTurbo 引擎（素材拼接成片，取代旧 AI 生成式视频链路）
            crate::commands::video_studio::mpt::video_mpt_generate,
            crate::commands::video_studio::mpt::video_mpt_generate_terms,
            crate::commands::video_studio::mpt::video_mpt_preview_voice,

            // Video Studio Rendering
            crate::commands::video_studio::rendering::video_test_ffmpeg,
            crate::commands::video_studio::rendering::video_get_metadata,
            crate::commands::video_studio::rendering::video_run_ffmpeg,
            crate::commands::video_studio::rendering::video_concat_materials,
            crate::commands::video_studio::rendering::video_render_advanced,
            crate::commands::video_studio::rendering::video_export_render,

            // Video Studio Tasks
            crate::commands::video_studio::tasks::video_list_tasks,

            // WeChat 聊天监控
            crate::commands::wechat::wechat_get_key,
            crate::commands::wechat::wechat_open,
            crate::commands::wechat::wechat_list_sessions,
            crate::commands::wechat::wechat_list_contacts,
            crate::commands::wechat::wechat_get_messages,
            crate::commands::wechat::wechat_get_voice,
            crate::commands::wechat::wechat_transcribe_voice,
            crate::commands::wechat::wechat_get_media,
            crate::commands::wechat::wechat_get_image,
            crate::commands::wechat::wechat_open_video,
            crate::commands::wechat::wechat_resolve_session,
            crate::commands::wechat::wechat_start_monitor,
            crate::commands::wechat::wechat_start_monitor_auto,
            crate::commands::wechat::wechat_stop_monitor,
            crate::commands::wechat::wechat_get_status,
            crate::commands::wechat::wechat_check_stt_model,
            crate::commands::wechat::wechat_download_stt_model,
            crate::commands::wechat::wechat_save_credentials,
            crate::commands::wechat::wechat_load_credentials,

            // 手机控制（autocast-mobile 无线控制）
            crate::commands::mobile::mobile_get_server_info,
            crate::commands::mobile::mobile_list_devices,
            crate::commands::mobile::mobile_set_device_remark,
            crate::commands::mobile::mobile_delete_device,
            crate::commands::mobile::mobile_tap,
            crate::commands::mobile::mobile_swipe,
            crate::commands::mobile::mobile_key,
            crate::commands::mobile::mobile_request_screenshot,
            crate::commands::mobile::mobile_sync_recordings,
            crate::commands::mobile::mobile_adb_sync_recordings,
            crate::commands::mobile::mobile_list_recordings,
            crate::commands::mobile::mobile_delete_recording,

            // 发布排期 / 矩阵分发
            crate::commands::publisher::list_publishable_videos,
            crate::commands::publisher::create_publish_tasks,
            crate::commands::publisher::list_publish_tasks,
            crate::commands::publisher::delete_publish_task,
            crate::commands::publisher::cancel_publish_task,
            crate::commands::publisher::retry_publish_task,

            // 本地音频能力（ASR / 语音克隆）
            crate::commands::audio_lab::audio_asr_check_model,
            crate::commands::audio_lab::audio_asr_download_model,
            crate::commands::audio_lab::audio_transcribe_file,
            crate::commands::audio_lab::audio_polish_speech_text,
            crate::commands::audio_lab::voice_clone_check,
            crate::commands::audio_lab::voice_clone_install,
            crate::commands::audio_lab::voice_clone_install_status,
            crate::commands::audio_lab::voice_clone_register,
            crate::commands::audio_lab::voice_clone_list,
            crate::commands::audio_lab::voice_clone_synthesize,

            // AI 图像工作台
            crate::commands::image_studio::image_inpaint,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let app_state = app_handle.state::<AppState>();
                let lock_result = app_state.process_handles.lock();
                if let Ok(mut handles) = lock_result {
                    // Give children a brief grace period to flush & exit cleanly,
                    // then hard-kill anything that didn't honor it.
                    //
                    // On Unix we send SIGTERM first; on Windows we only call
                    // start_kill (which maps to TerminateProcess) because tokio's
                    // Child on Windows has no equivalent of kill(pid, SIGTERM) —
                    // sending a console CTRL_BREAK_EVENT would require us to
                    // attach to the child's console and is not worth the complexity
                    // for a Tauri desktop app on shutdown.
                    #[cfg(unix)]
                    {
                        for (_, child) in handles.iter() {
                            if let Some(pid) = child.id() {
                                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                    }
                    for (_, mut child) in handles.drain() {
                        let _ = child.start_kill();
                    }
                }
            }
        });
}
