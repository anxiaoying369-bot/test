import { ref, onUnmounted } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { MobileDevice, MobileServerInfo, MobileReceivedFile, RecordingItem } from '../types/mobile';

/**
 * 手机无线控制：设备列表 / 实时画面 / 触控指令。
 * 截图与文件由后端通过事件异步推送（mobile-screenshot / mobile-file-received）。
 */
export function useMobileControl() {
  const devices = ref<MobileDevice[]>([]);
  const serverInfo = ref<MobileServerInfo | null>(null);
  /** device_id -> 最新一帧截图 dataURL */
  const frames = ref<Record<string, string>>({});
  /** device_id -> 最新帧到达时间（ms） */
  const frameAt = ref<Record<string, number>>({});
  const receivedFiles = ref<MobileReceivedFile[]>([]);
  const recordings = ref<RecordingItem[]>([]);
  /** 正在实时刷新的设备 id（同一时刻只串流一台） */
  const streamingId = ref<string | null>(null);
  const lastError = ref('');

  let streamTimer: ReturnType<typeof setInterval> | null = null;
  let unlistens: Array<() => void> = [];

  async function refreshDevices() {
    try {
      devices.value = await invoke<MobileDevice[]>('mobile_list_devices');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function init() {
    try {
      serverInfo.value = await invoke<MobileServerInfo>('mobile_get_server_info');
    } catch (e) {
      lastError.value = String(e);
    }
    await refreshDevices();

    unlistens.push(
      await listen('mobile-devices-changed', () => {
        // 事件只做触发器：列表（含备注）以后端命令返回为准
        refreshDevices();
      }),
      await listen<{ device_id: string; data: string }>('mobile-screenshot', (event) => {
        const { device_id, data } = event.payload;
        frames.value[device_id] = `data:image/jpeg;base64,${data}`;
        frameAt.value[device_id] = Date.now();
        lastError.value = ''; // 收到画面说明设备可达，清掉可能残留的“不在线”提示
      }),
      await listen<Omit<MobileReceivedFile, 'received_at'>>('mobile-file-received', (event) => {
        receivedFiles.value.unshift({ ...event.payload, received_at: Date.now() });
        if (receivedFiles.value.length > 50) receivedFiles.value.pop();
        // 新录音到达，刷新通话记录列表
        if (event.payload.file_type === 'audio') refreshRecordings();
      }),
    );

    await refreshRecordings();
  }

  // ─── 通话录音记录 ───

  async function refreshRecordings(deviceId?: string) {
    try {
      recordings.value = await invoke<RecordingItem[]>('mobile_list_recordings', {
        deviceId: deviceId ?? null,
      });
    } catch (e) {
      lastError.value = String(e);
    }
  }

  /** 本地录音文件转成可播放 URL（走 Tauri asset 协议） */
  function recordingUrl(path: string): string {
    return convertFileSrc(path);
  }

  async function deleteRecording(path: string) {
    try {
      await invoke('mobile_delete_recording', { path });
      recordings.value = recordings.value.filter((r) => r.path !== path);
    } catch (e) {
      lastError.value = String(e);
    }
  }

  function dispose() {
    stopStream();
    unlistens.forEach((fn) => fn());
    unlistens = [];
  }

  // ─── 实时画面：按固定间隔向手机请求截图 ───

  function startStream(deviceId: string, intervalMs = 800) {
    stopStream();
    streamingId.value = deviceId;
    const tick = () => {
      // 后台轮询：设备抖动/断连的瞬时失败不写顶部错误栏（在线状态以设备徽标为准），
      // 否则一次“不在线”会一直挂在顶上即便设备已恢复。
      invoke('mobile_request_screenshot', { deviceId }).catch(() => {});
    };
    tick();
    streamTimer = setInterval(tick, intervalMs);
  }

  function stopStream() {
    if (streamTimer) clearInterval(streamTimer);
    streamTimer = null;
    streamingId.value = null;
  }

  async function requestScreenshot(deviceId: string) {
    try {
      await invoke('mobile_request_screenshot', { deviceId });
      lastError.value = '';
    } catch (e) {
      lastError.value = String(e);
    }
  }

  // ─── 触控 / 按键 ───

  async function tap(deviceId: string, x: number, y: number) {
    try {
      await invoke('mobile_tap', { deviceId, x, y });
      lastError.value = '';
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function swipe(deviceId: string, x1: number, y1: number, x2: number, y2: number, duration: number) {
    try {
      await invoke('mobile_swipe', { deviceId, x1, y1, x2, y2, duration });
      lastError.value = '';
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function pressKey(deviceId: string, name: 'back' | 'home' | 'recents' | 'notifications') {
    try {
      await invoke('mobile_key', { deviceId, name });
      lastError.value = '';
    } catch (e) {
      lastError.value = String(e);
    }
  }

  // ─── 备注 / 记录管理 ───

  async function setRemark(deviceId: string, remark: string) {
    try {
      await invoke('mobile_set_device_remark', { deviceId, remark });
      await refreshDevices();
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function deleteDevice(deviceId: string) {
    try {
      await invoke('mobile_delete_device', { deviceId });
      await refreshDevices();
    } catch (e) {
      lastError.value = String(e);
    }
  }

  onUnmounted(dispose);

  return {
    devices,
    serverInfo,
    frames,
    frameAt,
    receivedFiles,
    recordings,
    streamingId,
    lastError,
    init,
    refreshDevices,
    refreshRecordings,
    recordingUrl,
    deleteRecording,
    startStream,
    stopStream,
    requestScreenshot,
    tap,
    swipe,
    pressKey,
    setRemark,
    deleteDevice,
  };
}
