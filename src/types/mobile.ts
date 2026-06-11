// 手机无线控制相关类型（对应 src-tauri/src/commands/mobile.rs）

export interface MobileDevice {
  device_id: string;
  model: string;
  remark: string | null;
  online: boolean;
  /** 屏幕像素宽高（来自手机端 device_info，未上报时为 0） */
  width: number;
  height: number;
  /** Unix 秒 */
  last_seen: number;
}

export interface MobileServerInfo {
  port: number;
  ips: string[];
}

export interface MobileReceivedFile {
  device_id: string;
  name: string;
  path: string;
  size: number;
  file_type: string;
  received_at: number;
}

/** 已回传并落盘的通话录音 */
export interface RecordingItem {
  device_id: string;
  name: string;
  path: string;
  size: number;
  /** Unix 秒 */
  modified: number;
}
