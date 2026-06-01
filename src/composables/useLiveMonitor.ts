import { ref, computed, onMounted, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Account, LiveRoom } from '../types/live-monitor';

// globalRooms 由父组件以 prop 传入（响应式对象），各方法直接读写其中的房间状态
export function useLiveMonitor(globalRooms: Record<string, LiveRoom>) {
  const selectedRoomId = ref<string | null>(null);
  const newRoomId = ref('');
  const accounts = ref<Account[]>([]);
  const selectedAccount = ref('');
  const messageContainer = ref<HTMLElement | null>(null);
  const copiedId = ref<string | null>(null);
  const aiReplies = ref<Record<string, { content: string, loading: boolean }>>({});

  // 消息过滤器状态
  const filters = ref({ chat: true, like: true, gift: true, member: true });

  const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));
  const activeRoomsCount = computed(() => Object.keys(globalRooms).length);
  const selectedRoom = computed(() => selectedRoomId.value ? globalRooms[selectedRoomId.value] : null);

  const filteredMessages = computed(() => {
    if (!selectedRoom.value) return [];
    return selectedRoom.value.messages.filter(msg => {
      if ((msg.type === 'chat' || msg.type === 'emoji') && !filters.value.chat) return false;
      if (msg.type === 'like' && !filters.value.like) return false;
      if (msg.type === 'gift' && !filters.value.gift) return false;
      if (msg.type === 'member' && !filters.value.member) return false;
      return true;
    });
  });

  async function generateAiReply(msgIdx: number, user_name: string, content: string) {
    const replyKey = `${selectedRoomId.value}_${msgIdx}`;
    aiReplies.value[replyKey] = { content: '', loading: true };
    try {
      const reply = await invoke('generate_live_reply', { userName: user_name, content }) as string;
      aiReplies.value[replyKey] = { content: reply, loading: false };
    } catch (e) {
      console.error('生成回复失败:', e);
      aiReplies.value[replyKey] = { content: '生成失败: ' + e, loading: false };
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copiedId.value = text;
      setTimeout(() => {
        if (copiedId.value === text) copiedId.value = null;
      }, 2000);
    } catch (err) {
      console.error('复制失败:', err);
    }
  }

  async function loadAccounts() {
    try {
      const res = await invoke('list_accounts', { platform: null }) as Account[];
      accounts.value = res;
      if (res.length > 0) selectedAccount.value = res[0].name;
    } catch (e) {
      console.error('加载账号失败:', e);
    }
  }

  async function loadHistory(rid: string) {
    try {
      const history = await invoke('get_live_history', { roomId: rid }) as any[];
      if (history.length > 0 && globalRooms[rid]) {
        // 避免重复加载
        if (globalRooms[rid].messages.length === 0) {
          globalRooms[rid].messages = history.map(h => ({ type: h.data_type, payload: h.payload }));
          const lastWithAnchor = [...history].reverse().find(h => h.anchor_name);
          if (lastWithAnchor) globalRooms[rid].anchor_name = lastWithAnchor.anchor_name;
        }
      }
    } catch (e) {
      console.error('加载历史失败:', e);
    }
  }

  async function addRoom() {
    const input = newRoomId.value.trim();
    if (!input) return;

    let rid = input;
    // 如果不是纯数字，则尝试从 URL 解析
    if (!/^\d+$/.test(input)) {
      try {
        rid = await invoke('resolve_live_url', { url: input });
      } catch (e: any) {
        alert(e);
        return;
      }
    }
    if (!/^\d+$/.test(rid)) {
      alert('无法获取有效的直播间 ID');
      return;
    }
    if (globalRooms[rid] && globalRooms[rid].status === 'running') {
      selectedRoomId.value = rid;
      newRoomId.value = '';
      return;
    }
    if (activeRoomsCount.value >= 10 && !globalRooms[rid]) {
      alert('最多只能同时监控 10 个直播间');
      return;
    }

    // 初始化或重置房间状态
    if (!globalRooms[rid]) {
      globalRooms[rid] = { id: rid, anchor_name: '', status: 'connecting', messages: [] };
    } else {
      globalRooms[rid].status = 'connecting';
      globalRooms[rid].error = undefined;
    }

    selectedRoomId.value = rid;
    newRoomId.value = '';

    await loadHistory(rid);

    try {
      await invoke('start_live_monitor', { roomId: rid, accountName: selectedAccount.value });
    } catch (e: any) {
      if (globalRooms[rid]) {
        globalRooms[rid].status = 'error';
        globalRooms[rid].error = String(e);
      }
    }
  }

  async function stopMonitor(rid: string) {
    try {
      await invoke('stop_live_monitor', { roomId: rid });
      if (globalRooms[rid]) globalRooms[rid].status = 'stopped';
    } catch (e) {
      console.error('停止监控失败:', e);
    }
  }

  function removeRoom(rid: string) {
    stopMonitor(rid);
    delete globalRooms[rid];
    if (selectedRoomId.value === rid) {
      selectedRoomId.value = Object.keys(globalRooms)[0] || null;
    }
  }

  function scrollToBottom(force = false) {
    if (messageContainer.value) {
      const el = messageContainer.value;
      const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 100;
      if (force || isAtBottom) {
        nextTick(() => { el.scrollTop = el.scrollHeight; });
      }
    }
  }

  // 房间列表变化时自动选中最新加入的房间
  watch(() => Object.keys(globalRooms).length, (newCount, oldCount) => {
    if (newCount > (oldCount || 0)) {
      const keys = Object.keys(globalRooms);
      const latestId = keys[keys.length - 1];
      if (latestId && !selectedRoomId.value) selectedRoomId.value = latestId;
    }
  }, { immediate: true });

  watch(selectedRoomId, () => {
    setTimeout(() => scrollToBottom(true), 100);
  }, { immediate: true });

  watch(() => selectedRoom.value?.messages.length, () => {
    if (selectedRoomId.value) scrollToBottom();
  });

  onMounted(async () => {
    await loadAccounts();
    // 恢复已有的监控
    try {
      const activeIds = await invoke('get_active_monitors') as string[];
      for (const id of activeIds) {
        if (!globalRooms[id]) {
          globalRooms[id] = { id, anchor_name: '', status: 'running', messages: [] };
        }
        await loadHistory(id);
      }
      if (activeIds.length > 0 && !selectedRoomId.value) selectedRoomId.value = activeIds[0];
    } catch (e) {
      console.error('恢复活跃监控失败:', e);
    }
    setTimeout(() => scrollToBottom(true), 150);
  });

  return {
    selectedRoomId, newRoomId, accounts, selectedAccount, messageContainer, copiedId, aiReplies, filters,
    douyinAccounts, activeRoomsCount, selectedRoom, filteredMessages,
    generateAiReply, copyToClipboard, addRoom, stopMonitor, removeRoom,
  };
}
