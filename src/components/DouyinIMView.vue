<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  AlertCircle,
  CheckCircle,
  KeyRound,
  Loader2,
  MessageCircle,
  Send,
  ShieldCheck,
  Square,
  UserRound,
  Wand2,
} from 'lucide-vue-next';

interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

interface Contact {
  conversation_id?: string;
  conversation_short_id?: number | string | null;
  ticket?: string;
  peer_uid?: string;
  peer_nickname?: string;
  peer_avatar?: string;
  last_message?: { text?: string; raw?: any };
  unread_count?: number;
  raw?: any;
}

interface MonitorItem {
  account: string;
  status: 'starting' | 'running' | 'stopping' | 'stopped' | 'error';
  startedAt?: number;
  stoppedAt?: number;
  lastMessageAt?: number;
  error?: string;
}

interface MonitorMessage {
  id: string;
  account: string;
  senderUid: string;
  senderName: string;
  conversationId: string;
  conversationShortId?: number | string | null;
  ticket?: string;
  text: string;           // 文本内容或备用描述
  messageType?: number | string;
  mediaType?: 'sticker' | 'image' | 'voice' | 'video'; // 富媒体类型
  mediaUrl?: string;      // 图片/表情/语音 URL
  mediaCaption?: string;  // 表情关键词 / 视频 ID 等
  receivedAt: number;
  isSelf?: boolean;  // 自己发出的消息
  raw: any;
}

// ── 持久化 ───────────────────────────────────────────────────────────────────
const STORAGE_KEY = 'autocast_im_messages_v1';
const MAX_STORED   = 500;   // 最多保留条数

function loadPersistedMessages(): MonitorMessage[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as MonitorMessage[];
    return parsed.map((m) => ({ ...m, raw: null }));
  } catch { return []; }
}

function persistMessages() {
  try {
    const toSave = messages.value
      .slice(0, MAX_STORED)
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      .map(({ raw: _raw, ...m }) => m);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
  } catch (e) { console.warn('持久化消息失败:', e); }
}
// ─────────────────────────────────────────────────────────────────────────────

const accounts = ref<Account[]>([]);
const selectedAccount = ref('');
const contactMaps = ref<Record<string, Contact[]>>({});
const monitors = ref<MonitorItem[]>([]);
const messages = ref<MonitorMessage[]>(loadPersistedMessages());
const selectedMessage = ref<MonitorMessage | null>(null);
const myUid = ref('');
const replyContent = ref('');
const loading = ref(false);
const sending = ref(false);
const isGeneratingReply = ref(false);
const refreshingCreds = ref(false);
const sendReady = ref<boolean | null>(null);   // null=未检测, true=可发送, false=缺凭证
const statusMessage = ref('请选择抖音 Cookie 账号，然后点击开始监控');
const errorMessage = ref('');
const threadEl = ref<HTMLElement | null>(null);
let unlistenImEvent: UnlistenFn | null = null;
let persistTimer: ReturnType<typeof setTimeout> | null = null;

const douyinAccounts = computed(() => accounts.value.filter((a) => a.platform === 'douyin'));
const selectedAccountInfo = computed(() => douyinAccounts.value.find((a) => a.name === selectedAccount.value));
const selectedMonitor = computed(() => monitors.value.find((m) => m.account === selectedAccount.value));
const selectedMonitorRunning = computed(() => selectedMonitor.value?.status === 'running' || selectedMonitor.value?.status === 'starting');
const runningCount = computed(() => monitors.value.filter((m) => m.status === 'running' || m.status === 'starting').length);
const canStartMonitor = computed(() => Boolean(selectedAccount.value) && !loading.value && !selectedMonitorRunning.value);
const canReply = computed(() => Boolean(selectedMessage.value && replyContent.value.trim()) && !sending.value);
const selectedTitle = computed(() => selectedMessage.value ? `与 ${selectedMessage.value.senderName} 的对话` : '等待新私信');

// 侧边栏：按 (account + conversationId) 分组，每组取最新收到的消息（一行 = 一个会话）
const conversations = computed(() => {
  const map = new Map<string, MonitorMessage>();
  for (const msg of messages.value) {
    if (msg.isSelf) continue;
    const key = `${msg.account}:${msg.conversationId}`;
    const existing = map.get(key);
    if (!existing || msg.receivedAt > existing.receivedAt) {
      map.set(key, msg);
    }
  }
  return [...map.values()].sort((a, b) => b.receivedAt - a.receivedAt);
});

// 当前选中会话的完整消息线程（收到 + 发出），按时间升序
const conversationThread = computed(() => {
  if (!selectedMessage.value) return [];
  const convId = selectedMessage.value.conversationId;
  const account = selectedMessage.value.account;
  return [...messages.value]
    .filter((m) => m.conversationId === convId && m.account === account)
    .sort((a, b) => a.receivedAt - b.receivedAt);
});

/**
 * 解析 Rust 层返回的错误字符串。
 * 格式：JSON_ERR:<json>\n<message>  ← 结构化错误（ok: false 的 Python 响应）
 * 其他格式：纯文本错误
 */
function parseInvokeError(e: unknown): { message: string; payload: Record<string, any> } {
  const raw = String(e);
  if (raw.startsWith('JSON_ERR:')) {
    const nl = raw.indexOf('\n');
    const jsonPart = raw.slice('JSON_ERR:'.length, nl > 0 ? nl : undefined);
    try {
      const payload = JSON.parse(jsonPart);
      return { message: payload.error || raw, payload };
    } catch {
      // ignore
    }
  }
  return { message: raw, payload: {} };
}

function setBusy(message: string) {
  loading.value = true;
  errorMessage.value = '';
  statusMessage.value = message;
}

function finish(message: string) {
  statusMessage.value = message;
}

function fail(e: any) {
  errorMessage.value = String(e);
  statusMessage.value = '操作失败';
}

function upsertMonitor(item: Partial<MonitorItem> & { account: string }) {
  const idx = monitors.value.findIndex((m) => m.account === item.account);
  if (idx >= 0) {
    monitors.value[idx] = { ...monitors.value[idx], ...item };
    return monitors.value[idx];
  }
  const created: MonitorItem = {
    status: 'starting',
    startedAt: Date.now(),
    ...item,
    account: item.account,
  };
  monitors.value.unshift(created);
  return created;
}

function removeStoppedMonitor(account: string) {
  const monitor = monitors.value.find((m) => m.account === account);
  if (monitor) {
    monitor.status = 'stopped';
    monitor.stoppedAt = Date.now();
  }
}

function monitorStatusLabel(status: MonitorItem['status']) {
  if (status === 'starting') return '启动中';
  if (status === 'running') return '运行中';
  if (status === 'stopping') return '停止中';
  if (status === 'stopped') return '已停止';
  return '异常';
}

function monitorStatusClass(status: MonitorItem['status']) {
  if (status === 'running') return 'border-green-700 bg-green-950/40 text-green-300';
  if (status === 'starting' || status === 'stopping') return 'border-amber-700 bg-amber-950/40 text-amber-300';
  if (status === 'error') return 'border-red-800 bg-red-950/40 text-red-300';
  return 'border-gray-700 bg-gray-900 text-gray-400';
}

function getMessageText(payload: any): string {
  const content = payload?.content;
  const msgType = Number(payload?.message_type);
  if (typeof content === 'string') return content;
  if (msgType === 5)  return content?.keyword ? `[表情: ${content.keyword}]` : '[表情包]';
  if (msgType === 17) return '[语音消息]';
  if (msgType === 27) return '[图片]';
  if (msgType === 8)  return '[视频分享]';
  if (content?.text)    return String(content.text);
  if (content?.content) return String(content.content);
  if (payload?.summary) return String(payload.summary);
  return '[消息]';
}

/** 从消息 payload 中提取富媒体信息（URL、类型、标题） */
function extractMedia(payload: any): Pick<MonitorMessage, 'mediaType' | 'mediaUrl' | 'mediaCaption'> {
  const msgType = Number(payload?.message_type);
  const c = payload?.content || {};
  if (msgType === 5) {
    return {
      mediaType: 'sticker',
      mediaUrl: c.url?.url_list?.[0] ?? '',
      mediaCaption: c.keyword ?? '',
    };
  }
  if (msgType === 27) {
    return {
      mediaType: 'image',
      mediaUrl: c.resource_url?.origin_url_list?.[0] ?? c.resource_url?.url_list?.[0] ?? '',
    };
  }
  if (msgType === 17) {
    return {
      mediaType: 'voice',
      mediaUrl: c.resource_url?.url_list?.[0] ?? '',
    };
  }
  if (msgType === 8) {
    return {
      mediaType: 'video',
      mediaCaption: String(c.itemId ?? ''),
    };
  }
  return {};
}

/**
 * 从 conversation_id（格式 "0:1:{uidA}:{uidB}"）提取对方 UID。
 * 取最后两段中不等于 myUid 且为纯数字的那一个。
 */
function parsePeerUidFromConversationId(convId: string): string {
  if (!convId) return '';
  const parts = convId.split(':');
  if (parts.length < 2) return '';
  const uidCandidates = parts.slice(-2).reverse(); // 优先取最末一段
  for (const uid of uidCandidates) {
    if (uid && /^\d+$/.test(uid) && uid !== myUid.value) return uid;
  }
  return '';
}

function findContactByUid(account: string, uid: string): Contact | undefined {
  if (!uid) return undefined;
  return (contactMaps.value[account] || []).find((c) => String(c.peer_uid || '') === uid);
}

function findContactByEvent(account: string, payload: any): Contact | undefined {
  const sender = String(payload?.sender || payload?.sender_uid || '');
  const convId = String(payload?.conversation_id || '');
  const contacts = contactMaps.value[account] || [];

  // 1. 优先按 sender UID 或 conversation_id 精确匹配
  const direct = contacts.find((c) => {
    const peerUid = String(c.peer_uid || '');
    const contactConvId = String(c.conversation_id || '');
    return (sender && peerUid && sender === peerUid) || (convId && contactConvId && convId === contactConvId);
  });
  if (direct) return direct;

  // 2. 退路：从 conversation_id 解析出对方 UID 再查
  const peerUid = parsePeerUidFromConversationId(convId);
  if (peerUid) return findContactByUid(account, peerUid);

  return undefined;
}

function getSenderName(payload: any, contact?: Contact): string {
  const candidates = [
    payload?.sender_name,
    payload?.sender_nickname,
    payload?.nickname,
    payload?.user?.nickname,
    payload?.content?.nickname,
    payload?.content?.user?.nickname,
    contact?.peer_nickname,
  ];
  const name = candidates.find((v) => String(v || '').trim());
  if (name) return String(name).trim();
  // 最后兜底：用 UID 显示，至少不是"未知用户"
  const uid = String(payload?.sender || payload?.sender_uid || '').trim();
  return uid ? `用户${uid}` : '未知用户';
}

// 只有这几种 type 是用户发出的真实消息，其余均为系统控制帧
const USER_MESSAGE_TYPES = new Set([7, 5, 17, 27, 8]);

function upsertIncomingMessage(event: any) {
  const payload = event?.event || event;
  if (!payload || payload.payload_type !== 'pb') return;

  // 过滤系统消息（50001 已读回执/会话更新、50002 空控制帧等）
  const msgType = Number(payload.message_type);
  if (!USER_MESSAGE_TYPES.has(msgType)) return;

  const account = String(event?.account || selectedAccount.value || '');
  const senderUid = String(payload.sender || payload.sender_uid || '');

  // 过滤自己发出的消息（sender 为 '0' 的系统帧也一并丢弃）
  if (!senderUid || senderUid === '0') return;
  if (myUid.value && senderUid === myUid.value) return;

  let contact = findContactByEvent(account, payload);
  const text = getMessageText(payload);
  const conversationId = String(payload.conversation_id || contact?.conversation_id || '');
  const id = String(payload.index || payload.message_id || `${account}-${conversationId}-${senderUid}-${Date.now()}`);

  if (messages.value.some((m) => m.id === id && m.conversationId === conversationId && m.account === account)) return;

  const senderName = getSenderName(payload, contact);

  const msg: MonitorMessage = {
    id,
    account,
    senderUid,
    senderName,
    conversationId,
    conversationShortId: payload.conversation_short_id || contact?.conversation_short_id || null,
    ticket: payload.ticket || contact?.ticket || '',
    text,
    messageType: payload.message_type,
    ...extractMedia(payload),
    receivedAt: Date.now(),
    isSelf: false,
    raw: payload,
  };
  messages.value.unshift(msg);
  // 当前没有选中会话，或收到的新消息就在当前会话里 → 自动切换（保持聚焦）
  const currentConvId = selectedMessage.value?.conversationId;
  const currentAccount = selectedMessage.value?.account;
  if (!selectedMessage.value || (currentConvId === conversationId && currentAccount === account)) {
    selectedMessage.value = msg;
  }
  upsertMonitor({ account, status: 'running', lastMessageAt: msg.receivedAt });
  statusMessage.value = `收到 ${account} / ${msg.senderName} 的私信`;
}

function selectConversation(repr: MonitorMessage) {
  // 找到该会话中最新的已收到消息作为代表（用于显示对方名称、UID 等）
  const latest = messages.value
    .filter((m) => m.conversationId === repr.conversationId && m.account === repr.account && !m.isSelf)
    .sort((a, b) => b.receivedAt - a.receivedAt)[0] ?? repr;
  selectedMessage.value = latest;
  selectedAccount.value = repr.account;
  replyContent.value = '';
}

async function loadAccounts() {
  setBusy('正在刷新账号列表...');
  try {
    const res = await invoke('list_accounts', { platform: null }) as Account[];
    accounts.value = res;
    if (!selectedAccount.value && douyinAccounts.value.length > 0) {
      selectedAccount.value = douyinAccounts.value[0].name;
    }
    finish(`已加载 ${douyinAccounts.value.length} 个抖音账号`);
  } catch (e) {
    fail(e);
  } finally {
    loading.value = false;
  }
}

async function loadActiveMonitors() {
  try {
    const active = await invoke('get_active_douyin_im_monitors') as any[];
    for (const item of active) {
      upsertMonitor({ account: String(item.account), status: 'running', startedAt: Date.now() });
    }
  } catch (e) {
    console.warn('获取私信监控列表失败:', e);
  }
}

async function checkCapability() {
  if (!selectedAccount.value) return;
  setBusy('正在检查 Cookie 与私信能力...');
  try {
    const result: any = await invoke('douyin_im_check', { accountName: selectedAccount.value });
    if (result.uid && !myUid.value) myUid.value = String(result.uid);
    sendReady.value = Boolean(result.send_ready);

    const recv = result.receive_ready ? '可接收' : '接收缺字段';
    if (result.credentials_expired) {
      const age = result.credentials_age_hours != null ? `（已存 ${result.credentials_age_hours}h）` : '';
      finish(`发送凭证已过期${age}，正在自动刷新...`);
      loading.value = false;
      await refreshCredentials();   // 自动刷新
      return;
    }
    const send = result.send_ready ? '可回复' : '缺少发送凭证（点击刷新凭证）';
    finish(`检查完成：${recv} / ${send}`);
  } catch (e) {
    fail(e);
  } finally {
    loading.value = false;
  }
}

async function refreshCredentials() {
  const account = selectedMessage.value?.account || selectedAccount.value;
  if (!account) return;
  refreshingCreds.value = true;
  errorMessage.value = '';
  statusMessage.value = `正在打开 Chrome 刷新 ${account} 的认证凭证（约 15 秒）...`;
  try {
    const result: any = await invoke('douyin_im_refresh_credentials', { accountName: account });
    sendReady.value = Boolean(result.send_ready);
    finish(result.message || '凭证刷新完成');
  } catch (e) {
    fail(e);
  } finally {
    refreshingCreds.value = false;
  }
}

async function fetchMyUid(accountName = selectedAccount.value) {
  if (!accountName) return;
  try {
    const result: any = await invoke('douyin_im_my_uid', { accountName });
    if (result.uid && accountName === selectedAccount.value) myUid.value = String(result.uid);
  } catch (e) {
    console.warn('获取当前账号 UID 失败:', e);
  }
}

async function startMonitor() {
  const account = selectedAccount.value;
  if (!account) return;
  setBusy(`正在启动 ${account} 的私信监控...`);
  upsertMonitor({ account, status: 'starting', startedAt: Date.now(), error: '' });
  try {
    await fetchMyUid(account);
    await invoke('douyin_im_start_monitor', { accountName: account });
    upsertMonitor({ account, status: 'running', startedAt: Date.now() });
    finish(`${account} 私信监控已启动`);
  } catch (e) {
    upsertMonitor({ account, status: 'error', error: String(e) });
    fail(e);
  } finally {
    loading.value = false;
  }
}

async function stopMonitor(account: string) {
  if (!account) return;
  upsertMonitor({ account, status: 'stopping' });
  if (account === selectedAccount.value) setBusy(`正在停止 ${account} 的私信监控...`);
  try {
    await invoke('douyin_im_stop_monitor', { accountName: account });
    removeStoppedMonitor(account);
    finish(`${account} 私信监控已停止`);
  } catch (e) {
    upsertMonitor({ account, status: 'error', error: String(e) });
    fail(e);
  } finally {
    loading.value = false;
  }
}

async function generateAiReply() {
  if (!selectedMessage.value || isGeneratingReply.value) return;
  
  isGeneratingReply.value = true;
  errorMessage.value = '';
  try {
    // 提取最近 10 条对话作为上下文
    const history = conversationThread.value.map(m => ({
      role: m.isSelf ? 'assistant' : 'user',
      content: m.text
    })).slice(-10);

    const reply = await invoke('generate_im_reply', { 
      messages: history 
    }) as string;
    
    replyContent.value = reply;
    finish('AI 回复建议已生成');
  } catch (e) {
    console.error('AI 生成失败:', e);
    fail('AI 生成失败: ' + e);
  } finally {
    isGeneratingReply.value = false;
  }
}

async function sendReply() {
  if (!canReply.value || !selectedMessage.value) return;
  sending.value = true;
  errorMessage.value = '';
  const outgoingText = replyContent.value.trim();
  const target = selectedMessage.value;
  try {
    const payload: any = {
      accountName: target.account,
      content: outgoingText,
      uid: myUid.value.trim() || null,
      toUserId: null,
      conversationId: null,
      conversationShortId: null,
      ticket: null,
    };

    if (target.conversationId && target.conversationShortId && target.ticket) {
      payload.conversationId = target.conversationId;
      payload.conversationShortId = Number(target.conversationShortId);
      payload.ticket = target.ticket;
    } else {
      payload.toUserId = target.senderUid || null;
    }

    let sendResult: any;
    try {
      sendResult = await invoke('douyin_im_send', payload);
    } catch (sendErr) {
      const { message, payload: errPayload } = parseInvokeError(sendErr);
      if (errPayload.needs_refresh) {
        // 凭证缺失或已过期 → 自动刷新后重试一次
        finish('凭证已过期，正在自动刷新...');
        sending.value = false;
        await refreshCredentials();
        if (!sendReady.value) {
          fail('凭证刷新失败，请手动点击「刷新凭证」后重试');
          return;
        }
        // 刷新完成，用新凭证重发
        sending.value = true;
        sendResult = await invoke('douyin_im_send', payload);
      } else {
        throw new Error(message);
      }
    }

    // 从发送结果中提取会话信息，下次发送直接走 conversationId 路径，避免重复调用 create_conversation
    const resultData = (sendResult as any)?.result || {};
    const freshConvShortId = resultData.conversation_short_id ?? null;
    const freshTicket: string = resultData.ticket ?? '';

    // 将会话信息回填到当前会话的所有消息（包括已收到的），使后续发送能复用
    if (freshConvShortId || freshTicket) {
      for (const m of messages.value) {
        if (m.conversationId === target.conversationId && m.account === target.account) {
          if (freshConvShortId && !m.conversationShortId) m.conversationShortId = freshConvShortId;
          if (freshTicket && !m.ticket) m.ticket = freshTicket;
        }
      }
    }

    replyContent.value = '';
    // 将发出的消息插入会话线程，以便在主区域的对话气泡中展示
    const sentMsg: MonitorMessage = {
      id: `sent-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      account: target.account,
      senderUid: myUid.value || 'me',
      senderName: '我',
      conversationId: target.conversationId,
      conversationShortId: freshConvShortId || target.conversationShortId,
      ticket: freshTicket || target.ticket,
      text: outgoingText,
      messageType: 7,
      receivedAt: Date.now(),
      isSelf: true,
      raw: null,
    };
    messages.value.unshift(sentMsg);

    // 更新 selectedMessage，确保下一条回复也能走 conversationId 路径
    if (freshConvShortId || freshTicket) {
      const latestReceived = messages.value
        .filter((m) => m.conversationId === target.conversationId && m.account === target.account && !m.isSelf)
        .sort((a, b) => b.receivedAt - a.receivedAt)[0];
      if (latestReceived) selectedMessage.value = latestReceived;
    }

    finish(`已通过 ${target.account} 回复 ${target.senderName}`);
  } catch (e) {
    fail(e);
  } finally {
    sending.value = false;
  }
}

// 消息变化时防抖持久化（1 秒后写 localStorage）
watch(
  () => messages.value.length,
  () => {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(persistMessages, 1000);
  },
);

// 新消息到达时自动滚到底部
watch(conversationThread, async () => {
  await nextTick();
  if (threadEl.value) {
    threadEl.value.scrollTop = threadEl.value.scrollHeight;
  }
});

onMounted(async () => {
  await loadAccounts();
  await loadActiveMonitors();
  unlistenImEvent = await listen('douyin-im-event', (event) => {
    const payload: any = event.payload;
    if (payload?.type === 'status') {
      const account = String(payload.account || '');
      const status = String(payload.status || '');
      if (account) {
        if (status === 'error') {
          upsertMonitor({
            account,
            status: 'error',
            error: payload.error || payload.message || '未知错误',
            stoppedAt: Date.now(),
          });
          statusMessage.value = `${account} 私信监控失败：${payload.error || payload.message || '未知错误'}`;
        } else if (status === 'starting') {
          upsertMonitor({
            account,
            status: 'starting',
            startedAt: Date.now(),
            error: '',
          });
          statusMessage.value = `${account} 正在建立私信 WebSocket 连接...`;
        } else if (status === 'running' || status === 'connected') {
          upsertMonitor({
            account,
            status: 'running',
            startedAt: Date.now(),
            error: '',
          });
          statusMessage.value = status === 'connected'
            ? `${account} 私信连接已建立，监听中`
            : `${account} 私信监控运行中`;
        } else if (status === 'disconnected') {
          // 暂时断开但 auto_reconnect 通常会重连，先保持 running 状态等下一次事件
          statusMessage.value = `${account} 私信连接断开 (code=${payload.code || '?'})，等待自动重连`;
        } else if (status === 'stopped') {
          upsertMonitor({
            account,
            status: 'stopped',
            stoppedAt: Date.now(),
          });
          statusMessage.value = `${account} 私信监控已停止`;
        }
      }
      return;
    }
    if (payload?.type === 'im_message') {
      upsertIncomingMessage(payload);
    }
  });
});

onUnmounted(() => {
  if (unlistenImEvent) unlistenImEvent();
  // 立即持久化，不等防抖
  if (persistTimer) clearTimeout(persistTimer);
  persistMessages();
});
</script>

<template>
  <div class="flex h-full bg-gray-950 text-gray-50 overflow-hidden">
    <aside class="w-[390px] border-r border-gray-800 bg-gray-900/50 flex flex-col">
      <div class="p-5 border-b border-gray-800">
        <div class="flex items-center gap-3 mb-2">
          <MessageCircle class="w-6 h-6 text-pink-500" />
          <h2 class="text-lg font-bold">私信监控</h2>
        </div>
        <p class="text-xs text-gray-500 leading-relaxed">
          选择已保存的抖音 Cookie 创建监控；下方可查看监控列表、状态，并单独停止某个账号。
        </p>
      </div>

      <div class="p-4 space-y-4 border-b border-gray-800">
        <section class="bg-gray-950 border border-gray-800 rounded-xl p-4 space-y-3">
          <label class="text-xs text-gray-400 flex items-center gap-2">
            <UserRound class="w-4 h-4" /> 新增监控账号
          </label>
          <select v-model="selectedAccount" class="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm">
            <option value="" disabled>选择抖音 Cookie</option>
            <option v-for="acc in douyinAccounts" :key="acc.id" :value="acc.name">
              {{ acc.name }} {{ acc.meta?.nickname ? `(${acc.meta.nickname})` : '' }}
            </option>
          </select>
          <div v-if="selectedAccountInfo" class="text-xs text-gray-500">
            当前：{{ selectedAccountInfo.name }} / {{ selectedAccountInfo.meta?.user_id || myUid || '未记录 UID' }}
          </div>
          <div class="grid grid-cols-2 gap-2">
            <button @click="refreshCredentials" :disabled="refreshingCreds || !selectedAccount" class="bg-gray-800 hover:bg-gray-700 disabled:opacity-50 rounded-lg px-3 py-2 text-xs flex items-center justify-center gap-1">
              <Loader2 v-if="refreshingCreds" class="w-3 h-3 animate-spin" />
              <KeyRound v-else class="w-3 h-3" />
              刷新凭证
            </button>
            <button @click="checkCapability" :disabled="loading || !selectedAccount" class="bg-gray-800 hover:bg-gray-700 disabled:opacity-50 rounded-lg px-3 py-2 text-xs flex items-center justify-center gap-1">
              <ShieldCheck class="w-3 h-3" /> 检查能力
            </button>
          </div>
          <button @click="startMonitor" :disabled="!canStartMonitor" class="w-full rounded-lg px-3 py-2 text-sm font-medium border border-pink-700 bg-pink-600 hover:bg-pink-700 text-white disabled:opacity-50 transition-colors">
            <span v-if="loading" class="inline-flex items-center gap-2"><Loader2 class="w-4 h-4 animate-spin" />处理中...</span>
            <span v-else>{{ selectedMonitorRunning ? '该账号监控中' : '开始监控' }}</span>
          </button>
        </section>
      </div>

      <div class="px-4 py-3 border-b border-gray-800 flex items-center justify-between">
        <h3 class="text-sm font-semibold">监控列表</h3>
        <span class="text-xs text-gray-500">运行中 {{ runningCount }} 个</span>
      </div>

      <div class="max-h-[240px] overflow-y-auto divide-y divide-gray-800 border-b border-gray-800">
        <div v-for="monitor in monitors" :key="monitor.account" class="p-3 space-y-2">
          <div class="flex items-center justify-between gap-2">
            <div class="min-w-0">
              <div class="text-sm font-medium truncate">{{ monitor.account }}</div>
              <div class="text-[11px] text-gray-500">
                {{ monitor.lastMessageAt ? `最近消息 ${new Date(monitor.lastMessageAt).toLocaleTimeString()}` : monitor.startedAt ? `启动 ${new Date(monitor.startedAt).toLocaleTimeString()}` : '未启动' }}
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[11px] px-2 py-1 rounded-lg border" :class="monitorStatusClass(monitor.status)">{{ monitorStatusLabel(monitor.status) }}</span>
              <button @click="stopMonitor(monitor.account)" :disabled="monitor.status !== 'running' && monitor.status !== 'starting'" class="text-xs bg-red-900/40 hover:bg-red-800 disabled:opacity-40 text-red-300 rounded-lg px-2 py-1 flex items-center gap-1">
                <Square class="w-3 h-3" /> 停止
              </button>
            </div>
          </div>
          <div v-if="monitor.error" class="text-[11px] text-red-300 bg-red-950/30 border border-red-900 rounded-lg px-2 py-1">{{ monitor.error }}</div>
        </div>
        <div v-if="monitors.length === 0" class="p-4 text-xs text-gray-500 text-center">暂无监控，选择 Cookie 后点击开始监控。</div>
      </div>

      <div class="px-4 py-3 border-b border-gray-800 flex items-center justify-between">
        <h3 class="text-sm font-semibold">私信会话</h3>
        <span class="text-xs text-gray-500">{{ conversations.length }} 个会话</span>
      </div>

      <div class="flex-1 overflow-y-auto divide-y divide-gray-800">
        <button
          v-for="conv in conversations"
          :key="`${conv.account}:${conv.conversationId}`"
          @click="selectConversation(conv)"
          class="w-full text-left p-4 hover:bg-gray-900 transition-colors"
          :class="selectedMessage?.conversationId === conv.conversationId && selectedMessage?.account === conv.account ? 'bg-gray-900 border-l-2 border-pink-500' : ''"
        >
          <div class="flex items-center justify-between gap-2 mb-1">
            <span class="text-sm font-medium truncate">{{ conv.senderName }}</span>
            <span class="text-[10px] text-gray-500">{{ new Date(conv.receivedAt).toLocaleTimeString() }}</span>
          </div>
          <div class="text-[11px] text-gray-500 font-mono truncate mb-1">{{ conv.account }} · UID {{ conv.senderUid || '?' }}</div>
          <p class="text-xs text-gray-400 line-clamp-1">{{ conv.text }}</p>
        </button>
        <div v-if="conversations.length === 0" class="p-6 text-xs text-gray-500 text-center">
          暂无私信。监控运行后，新消息会实时出现在这里。
        </div>
      </div>
    </aside>

    <main class="flex-1 flex flex-col overflow-hidden">
      <div class="p-5 border-b border-gray-800 flex items-center justify-between">
        <div>
          <h1 class="text-xl font-bold">{{ selectedTitle }}</h1>
          <p class="text-xs text-gray-500 mt-1">
            {{ selectedMessage ? `本次会话共 ${conversationThread.length} 条` : '收到消息后，点击左侧会话进行回复' }}
          </p>
        </div>
        <div class="flex items-center gap-2 text-xs px-3 py-2 rounded-lg border" :class="errorMessage ? 'border-red-800 bg-red-950/40 text-red-300' : 'border-gray-800 bg-gray-900 text-gray-400'">
          <Loader2 v-if="loading || sending" class="w-3 h-3 animate-spin" />
          <AlertCircle v-else-if="errorMessage" class="w-3 h-3" />
          <CheckCircle v-else class="w-3 h-3" />
          {{ errorMessage || statusMessage }}
        </div>
      </div>

      <!-- 会话信息栏（仅在选中消息时显示）-->
      <div v-if="selectedMessage" class="px-6 py-2 border-b border-gray-800 bg-gray-900/40 flex items-center justify-between gap-4">
        <div class="text-xs text-gray-500 font-mono">
          账号: {{ selectedMessage.account }} / 对方UID: {{ selectedMessage.senderUid || '未知' }} / 会话: {{ selectedMessage.conversationId || '未获取' }}
        </div>
        <div class="flex items-center gap-2">
          <input v-model="myUid" class="bg-gray-950 border border-gray-700 rounded-lg px-2 py-1 text-xs font-mono w-44" placeholder="当前账号 UID" />
          <!-- 发送凭证状态 -->
          <div class="flex items-center gap-1 rounded-lg px-2 py-1 border text-[11px]"
            :class="sendReady === null ? 'border-gray-700 bg-gray-950 text-gray-400'
                   : sendReady ? 'border-green-800 bg-green-950/40 text-green-300'
                   : 'border-amber-800 bg-amber-950/30 text-amber-300'">
            <CheckCircle v-if="sendReady" class="w-3 h-3 shrink-0" />
            <AlertCircle v-else class="w-3 h-3 shrink-0" />
            <span v-if="sendReady === null">凭证未检测</span>
            <span v-else-if="sendReady">可发送</span>
            <span v-else>缺少凭证</span>
          </div>
        </div>
      </div>

      <!-- 对话气泡区 -->
      <div class="flex-1 overflow-y-auto p-5 bg-gray-950 space-y-3" ref="threadEl">
        <div v-if="!selectedMessage" class="h-full flex items-center justify-center text-sm text-gray-500">
          请选择左侧收到的私信。
        </div>
        <template v-else>
          <div
            v-for="msg in conversationThread"
            :key="msg.id"
            class="flex"
            :class="msg.isSelf ? 'justify-end' : 'justify-start'"
          >
            <!-- 固定最大宽度，防止长消息挤压其他元素 -->
            <div class="w-[320px] max-w-[320px] min-w-0 space-y-1">
              <!-- 发送者名称 + 时间 -->
              <div class="flex items-center gap-1 text-[10px] text-gray-500"
                   :class="msg.isSelf ? 'justify-end' : 'justify-start'">
                <span>{{ msg.isSelf ? '我' : msg.senderName }}</span>
                <span>{{ new Date(msg.receivedAt).toLocaleTimeString() }}</span>
              </div>

              <!-- ── 表情包 / 图片 ────────────────────────────── -->
              <template v-if="msg.mediaType === 'sticker' || msg.mediaType === 'image'">
                <div class="inline-block overflow-hidden rounded-2xl"
                     :class="msg.isSelf ? 'rounded-tr-sm' : 'rounded-tl-sm'">
                  <img
                    v-if="msg.mediaUrl"
                    :src="msg.mediaUrl"
                    :alt="msg.mediaCaption || msg.text"
                    :class="msg.mediaType === 'sticker' ? 'max-w-[140px] max-h-[140px]' : 'max-w-[240px] max-h-[240px]'"
                    class="object-contain block"
                    loading="lazy"
                  />
                  <span v-else class="px-4 py-2.5 text-sm bg-gray-800 text-gray-400 block">{{ msg.text }}</span>
                </div>
                <div v-if="msg.mediaCaption && msg.mediaType === 'sticker'"
                     class="text-[10px] text-gray-500 px-1"
                     :class="msg.isSelf ? 'text-right' : 'text-left'">
                  {{ msg.mediaCaption }}
                </div>
              </template>

              <!-- ── 语音消息 ─────────────────────────────────── -->
              <template v-else-if="msg.mediaType === 'voice'">
                <div class="flex items-center gap-2 px-4 py-2.5 text-sm rounded-2xl"
                     :class="msg.isSelf ? 'bg-pink-600 text-white rounded-tr-sm' : 'bg-gray-800 text-gray-100 rounded-tl-sm'">
                  🎤 <span>语音消息</span>
                </div>
              </template>

              <!-- ── 视频分享 ─────────────────────────────────── -->
              <template v-else-if="msg.mediaType === 'video'">
                <div class="flex items-center gap-2 px-4 py-2.5 text-sm rounded-2xl"
                     :class="msg.isSelf ? 'bg-pink-600 text-white rounded-tr-sm' : 'bg-gray-800 text-gray-100 rounded-tl-sm'">
                  📹 <span class="truncate">视频分享{{ msg.mediaCaption ? `（${msg.mediaCaption}）` : '' }}</span>
                </div>
              </template>

              <!-- ── 普通文本 ─────────────────────────────────── -->
              <template v-else>
                <div
                  class="px-4 py-2.5 text-sm break-words leading-relaxed overflow-hidden"
                  :class="msg.isSelf
                    ? 'bg-pink-600 text-white rounded-2xl rounded-tr-sm'
                    : 'bg-gray-800 text-gray-100 rounded-2xl rounded-tl-sm'"
                >
                  {{ msg.text }}
                </div>
              </template>
            </div>
          </div>
        </template>
      </div>

      <div class="border-t border-gray-800 p-4 bg-gray-900/60">
        <div class="flex gap-2">
          <div class="flex-1 relative">
            <textarea v-model="replyContent" rows="3" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm pr-12" :placeholder="selectedMessage ? `回复 ${selectedMessage.senderName}` : '请先点击一条收到的私信'"></textarea>
            <button 
              @click="generateAiReply"
              :disabled="!selectedMessage || isGeneratingReply"
              class="absolute right-2 bottom-2 p-2 text-gray-500 hover:text-blue-400 disabled:opacity-30 transition-colors"
              title="AI 生成回复建议"
            >
              <Loader2 v-if="isGeneratingReply" class="w-4 h-4 animate-spin" />
              <Wand2 v-else class="w-4 h-4" />
            </button>
          </div>
          <button @click="sendReply" :disabled="!canReply" class="bg-pink-600 hover:bg-pink-700 disabled:opacity-50 rounded-lg px-5 py-2 text-sm flex items-center gap-2 self-end">
            <Loader2 v-if="sending" class="w-4 h-4 animate-spin" />
            <Send v-else class="w-4 h-4" /> 回复
          </button>
        </div>
      </div>
    </main>
  </div>
</template>
