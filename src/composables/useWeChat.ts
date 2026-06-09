import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 微信聊天监控 composable：封装与 Rust wechat_* 命令的交互，
// 并监听 wechat-new-message 事件把新消息推进 newMessages。

export interface WeChatSession {
  username: string;
  displayName: string;
  [key: string]: any;
}

export interface WeChatContact {
  username: string;
  displayName: string;
  isGroup: boolean;
  category: 'friend' | 'group';
}

export interface WeChatMessage {
  localId?: number;
  svrId?: number;
  localType?: number;
  createTime: number;
  senderUsername?: string;
  senderName?: string;
  content?: string;
  parsedContent?: string;
  isSender?: number | boolean;
  [key: string]: any;
}

export interface NewMessageEvent {
  sessionId: string;
  displayName: string;
  messages: WeChatMessage[];
  receivedAt: number;
}

// 全局单例状态（跨视图共享）
const connected = ref(false);
const monitoring = ref(false);
const accountDir = ref('');
const hexKey = ref('');
const statusText = ref('');
const busy = ref(false);

const sessions = ref<WeChatSession[]>([]);
const contacts = ref<WeChatContact[]>([]);
const friendCount = ref(0);
const groupCount = ref(0);
const currentSessionId = ref<string>('');
const messages = ref<WeChatMessage[]>([]);

// 被监控的会话集合（sessionId → displayName）
const watched = ref<Record<string, string>>({});
// 监控推送过来的新消息（最新在前）
const newMessages = ref<NewMessageEvent[]>([]);

let unlisten: any = null;

export function useWeChat() {
  async function initListener() {
    if (unlisten) return;
    unlisten = await listen('wechat-new-message', (event: any) => {
      const data = event.payload as { sessionId: string; displayName: string; messages: WeChatMessage[] };
      newMessages.value.unshift({
        sessionId: data.sessionId,
        displayName: data.displayName,
        messages: data.messages || [],
        receivedAt: Date.now(),
      });
      if (newMessages.value.length > 200) newMessages.value.pop();

      // 如果新消息属于当前正在查看的会话，追加到消息列表尾部
      if (data.sessionId === currentSessionId.value) {
        messages.value.push(...(data.messages || []));
      }
    });
  }

  async function loadCredentials() {
    try {
      const creds = await invoke('wechat_load_credentials') as { accountDir?: string; hexKey?: string };
      if (creds.accountDir) accountDir.value = creds.accountDir;
      if (creds.hexKey) hexKey.value = creds.hexKey;
    } catch (e) {
      console.warn('load credentials failed', e);
    }
  }

  async function refreshStatus() {
    try {
      const s = await invoke('wechat_get_status') as { connected: boolean; monitoring: boolean; accountDir?: string };
      connected.value = s.connected;
      monitoring.value = s.monitoring;
      if (s.accountDir) accountDir.value = s.accountDir;
    } catch (e) {
      console.warn('status failed', e);
    }
  }

  // 自动提取密钥 + 账号目录（mac 会弹出 sudo 授权）
  async function autoGetKey() {
    busy.value = true;
    statusText.value = '正在提取密钥（可能需要管理员授权，请保持微信在前台）...';
    try {
      const res = await invoke('wechat_get_key') as { success: boolean; hexKey?: string; accountDir?: string; error?: string };
      if (res.accountDir) accountDir.value = res.accountDir;
      if (res.success && res.hexKey) {
        hexKey.value = res.hexKey;
        statusText.value = '密钥提取成功';
        return true;
      }
      statusText.value = '提取失败：' + (res.error || '未知错误');
      return false;
    } catch (e: any) {
      statusText.value = '提取失败：' + (e?.message || e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  // 连接（打开数据库）
  async function connect() {
    if (!accountDir.value.trim() || !hexKey.value.trim()) {
      statusText.value = '请先填写账号目录与密钥';
      return false;
    }
    busy.value = true;
    statusText.value = '正在连接微信数据库...';
    try {
      const ok = await invoke('wechat_open', {
        accountDir: accountDir.value.trim(),
        hexKey: hexKey.value.trim(),
      }) as boolean;
      connected.value = ok;
      if (ok) {
        statusText.value = '已连接';
        await invoke('wechat_save_credentials', {
          accountDir: accountDir.value.trim(),
          hexKey: hexKey.value.trim(),
        }).catch(() => {});
        await loadContacts();
      }
      return ok;
    } catch (e: any) {
      connected.value = false;
      statusText.value = '连接失败：' + (e?.message || e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function loadSessions() {
    try {
      const res = await invoke('wechat_list_sessions') as { sessions: WeChatSession[] };
      sessions.value = res.sessions || [];
    } catch (e: any) {
      statusText.value = '获取会话失败：' + (e?.message || e);
    }
  }

  // 加载通讯录（好友 + 群聊），左侧列表用这个而非最近会话
  async function loadContacts() {
    try {
      const res = await invoke('wechat_list_contacts') as { contacts: WeChatContact[]; friendCount: number; groupCount: number };
      contacts.value = res.contacts || [];
      friendCount.value = res.friendCount || 0;
      groupCount.value = res.groupCount || 0;
    } catch (e: any) {
      statusText.value = '获取通讯录失败：' + (e?.message || e);
    }
  }

  async function openSession(sessionId: string) {
    currentSessionId.value = sessionId;
    messages.value = [];
    try {
      const res = await invoke('wechat_get_messages', { sessionId, limit: 100, offset: 0 }) as { messages: WeChatMessage[] };
      // 接口默认按时间倒序返回，倒一下让最新在底部
      messages.value = (res.messages || []).slice().reverse();
    } catch (e: any) {
      statusText.value = '获取消息失败：' + (e?.message || e);
    }
  }

  async function resolveSession(keyword: string) {
    try {
      const res = await invoke('wechat_resolve_session', { keyword }) as { found: boolean; sessionId?: string; displayName?: string };
      return res;
    } catch (e: any) {
      statusText.value = '查找失败：' + (e?.message || e);
      return { found: false };
    }
  }

  function toggleWatch(session: WeChatSession) {
    if (watched.value[session.username]) {
      delete watched.value[session.username];
    } else {
      watched.value[session.username] = session.displayName;
    }
  }

  async function startMonitor(intervalSecs = 5) {
    const targets = Object.entries(watched.value).map(([sessionId, displayName]) => ({ sessionId, displayName }));
    if (targets.length === 0) {
      statusText.value = '请至少勾选一个要监控的会话';
      return;
    }
    try {
      await invoke('wechat_start_monitor', { targets, intervalSecs });
      monitoring.value = true;
      statusText.value = `已开始监控 ${targets.length} 个会话`;
    } catch (e: any) {
      statusText.value = '开始监控失败：' + (e?.message || e);
    }
  }

  async function stopMonitor() {
    try {
      await invoke('wechat_stop_monitor');
      monitoring.value = false;
      statusText.value = '已停止监控';
    } catch (e: any) {
      statusText.value = '停止监控失败：' + (e?.message || e);
    }
  }

  function clearNewMessages() {
    newMessages.value = [];
  }

  // 取语音的可播放 data URL（SILK→WAV）
  async function getVoiceUrl(m: WeChatMessage): Promise<string | null> {
    try {
      const res = await invoke('wechat_get_voice', {
        sessionId: currentSessionId.value,
        svrId: m.svrId || 0,
        localId: m.localId || 0,
        createTime: m.createTime || 0,
      }) as { ok: boolean; mime: string; base64: string };
      return res.ok ? `data:${res.mime};base64,${res.base64}` : null;
    } catch (e) {
      console.warn('get_voice failed', e);
      return null;
    }
  }

  // 取图片的 data URL（wantFull=false 缩略图 / true 大图）
  async function getImageUrl(m: WeChatMessage, wantFull = false): Promise<string | null> {
    try {
      const res = await invoke('wechat_get_image', {
        sessionId: currentSessionId.value,
        localId: m.localId || 0,
        wantFull,
      }) as { ok: boolean; mime: string; base64: string };
      return res.ok ? `data:${res.mime};base64,${res.base64}` : null;
    } catch (e) {
      console.warn('get_image failed', e);
      return null;
    }
  }

  // 用系统播放器打开视频
  async function openVideo(m: WeChatMessage): Promise<void> {
    try {
      await invoke('wechat_open_video', { sessionId: currentSessionId.value, localId: m.localId || 0 });
    } catch (e: any) {
      statusText.value = '打开视频失败：' + (e?.message || e);
    }
  }

  // 取媒体（视频缩略图等）的 data URL
  async function getMediaUrl(m: WeChatMessage): Promise<string | null> {
    try {
      const res = await invoke('wechat_get_media', {
        sessionId: currentSessionId.value,
        localId: m.localId || 0,
        localType: m.localType || 0,
        svrId: m.svrId || 0,
        createTime: m.createTime || 0,
      }) as { ok: boolean; mime: string; base64: string };
      return res.ok ? `data:${res.mime};base64,${res.base64}` : null;
    } catch (e) {
      console.warn('get_media failed', e);
      return null;
    }
  }

  return {
    // state
    connected, monitoring, accountDir, hexKey, statusText, busy,
    sessions, contacts, friendCount, groupCount, currentSessionId, messages, watched, newMessages,
    // actions
    initListener, loadCredentials, refreshStatus, autoGetKey, connect,
    loadSessions, loadContacts, openSession, resolveSession, toggleWatch,
    startMonitor, stopMonitor, clearNewMessages, getVoiceUrl, getMediaUrl, getImageUrl, openVideo,
  };
}
