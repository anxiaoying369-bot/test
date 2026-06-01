import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { LiveRoom } from '../types/live-monitor';

// 直播监控全局状态：由 App 持有并下发给 LiveMonitorView
const liveMonitorRooms = ref<Record<string, LiveRoom>>({});
let activeLiveEventUnlisten: any = null;

export function useLiveEvents() {
  async function initLiveEventListener() {
    if (activeLiveEventUnlisten) return;
    activeLiveEventUnlisten = await listen('live-event', (event: any) => {
      const data = event.payload;
      const rid = data.live_id;

      // 忽略已删除或未初始化的房间的迟到事件
      if (!liveMonitorRooms.value[rid]) return;

      const room = liveMonitorRooms.value[rid];

      if (data.type === 'status') {
        if (data.status === 'starting') room.status = 'connecting';
        if (data.status === 'running') room.status = 'running';
        if (data.status === 'stopped') room.status = 'stopped';
        if (data.anchor_name) room.anchor_name = data.anchor_name;
      } else if (data.type === 'init') {
        room.status = 'running';
        if (data.anchor_name) room.anchor_name = data.anchor_name;
      } else if (data.type === 'data') {
        if (data.anchor_name && !room.anchor_name) room.anchor_name = data.anchor_name;
        room.messages.push({ type: data.data_type, payload: data.payload });
        if (room.messages.length > 1000) room.messages.shift();
      } else if (data.type === 'error') {
        room.status = 'error';
        room.error = data.message;
      }
    });
  }

  return { liveMonitorRooms, initLiveEventListener };
}
