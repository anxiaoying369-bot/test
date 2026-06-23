use tauri::State;
use crate::device_manager::{DeviceManager, DeviceInfo};

#[tauri::command]
pub async fn list_remote_devices(manager: State<'_, DeviceManager>) -> Result<Vec<DeviceInfo>, String> {
    Ok(manager.get_online_devices())
}

#[tauri::command]
pub async fn send_remote_command(
    device_id: String,
    command_json: String,
    manager: State<'_, DeviceManager>,
) -> Result<(), String> {
    manager.send_command(&device_id, &command_json)
}
