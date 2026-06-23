use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceInfo {
    pub id: String,
    pub model: String,
    pub last_seen: u64,
    pub status: String, // "online" | "offline"
}

pub struct DeviceConnection {
    pub info: DeviceInfo,
    pub tx: mpsc::UnboundedSender<String>,
}

pub struct DeviceManager {
    pub devices: Arc<Mutex<HashMap<String, DeviceConnection>>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(&self, info: DeviceInfo, tx: mpsc::UnboundedSender<String>) {
        let mut devices = self.devices.lock().unwrap();
        devices.insert(info.id.clone(), DeviceConnection { info, tx });
    }

    pub fn unregister(&self, id: &str) {
        let mut devices = self.devices.lock().unwrap();
        devices.remove(id);
    }

    pub fn get_online_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.lock().unwrap();
        devices.values().map(|d| d.info.clone()).collect()
    }

    pub fn send_command(&self, device_id: &str, cmd: &str) -> Result<(), String> {
        let devices = self.devices.lock().unwrap();
        if let Some(conn) = devices.get(device_id) {
            conn.tx.send(cmd.to_string()).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }
}
