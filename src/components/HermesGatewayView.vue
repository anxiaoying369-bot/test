<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  Plus, Search, Trash2, MessageSquare, Play, Square, RefreshCw,
  Send, User, Bot, Wrench, Loader2, Check, X,
  Binary, Info, LayoutGrid, BookOpen, Database, FileText, Upload, Zap, ZapOff
} from 'lucide-vue-next';
import { marked } from 'marked';

// ===== Types =====
interface ChatMessage {
  role: 'user' | 'assistant' | 'system' | 'tool' | 'thought';
  content: string;
  timestamp: number;
  toolName?: string;
  toolCallId?: string;
  toolStatus?: 'running' | 'completed';
  isStreaming?: boolean;
}

interface Session {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

// ===== Session Persistence (localStorage) =====
const STORAGE_KEY = 'hermes_sessions_v2';
const DELETED_KEY = 'hermes_deleted_session_ids';

function loadSessions(): Session[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return [];
}

function loadDeletedIds(): Set<string> {
  try {
    const raw = localStorage.getItem(DELETED_KEY);
    if (raw) return new Set(JSON.parse(raw));
  } catch {}
  return new Set();
}

function saveDeletedId(id: string) {
  try {
    const ids = loadDeletedIds();
    ids.add(id);
    // 最多保留 500 条，防止无限增长
    const arr = Array.from(ids).slice(-500);
    localStorage.setItem(DELETED_KEY, JSON.stringify(arr));
  } catch {}
}

function saveSessions() {
  try {
    // Keep last 30 sessions, last 200 messages each
    const trimmed = sessions.value.slice(-30).map(s => ({
      ...s,
      messages: s.messages.slice(-200),
    }));
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch {}
}

// ===== Config =====
const gatewayUrl = ref('http://127.0.0.1:8642');
const apiKey = ref('');

// Auto-save config when changed
watch([gatewayUrl, apiKey], async ([newUrl, newKey]) => {
  try {
    const config: any = await invoke('get_config');
    await invoke('save_config', {
      config: {
        ...config,
        hermes: { 
          ...(config?.hermes || {}), 
          gateway_url: newUrl,
          api_key: newKey 
        }
      }
    }).catch(() => {});
  } catch {}
});

async function loadConfig() {
  try {
    const config: any = await invoke('get_config');
    if (config?.hermes?.gateway_url) {
      const saved = config.hermes.gateway_url as string;
      // Migrate old WebSocket URLs (ws:// / wss://) to HTTP
      if (saved.startsWith('ws://') || saved.startsWith('wss://')) {
        gatewayUrl.value = 'http://127.0.0.1:8642';
        // Persist the corrected URL back to config
        await invoke('save_config', {
          config: {
            ...config,
            hermes: { ...config.hermes, gateway_url: gatewayUrl.value }
          }
        }).catch(() => {});
      } else {
        gatewayUrl.value = saved;
      }
    }
    if (config?.hermes?.api_key) apiKey.value = config.hermes.api_key;
  } catch {}
}

// ===== Gateway Status =====
const isConnected = ref(false);
const isChecking = ref(false);
const healthInfo = ref<Record<string, any> | null>(null);
const isStarting = ref(false);
const isStopping = ref(false);
const isEnablingApi = ref(false);
let healthInterval: ReturnType<typeof setInterval> | null = null;
let isMounted = true;

async function checkHealth() {
  if (isChecking.value) return;
  isChecking.value = true;
  try {
    const health = await invoke('check_hermes_gateway_health', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
    }) as Record<string, any>;
    isConnected.value = true;
    healthInfo.value = health;
  } catch {
    isConnected.value = false;
    healthInfo.value = null;
  } finally {
    isChecking.value = false;
  }
}

async function enableApiServerAndRestart() {
  isEnablingApi.value = true;
  try {
    // 1. Write API_SERVER_ENABLED=true to ~/.hermes/.env and get/generate key
    const key = await invoke('hermes_enable_api_server') as string;
    if (key) {
      apiKey.value = key;
      // Save key to app config so it persists
      const config: any = await invoke('get_config');
      await invoke('save_config', {
        config: {
          ...config,
          hermes: { 
            ...(config?.hermes || {}), 
            api_key: key,
            gateway_url: gatewayUrl.value 
          }
        }
      }).catch(() => {});
    }

    // 2. Trigger restart (fire-and-forget — returns immediately, takes ~10s to complete)
    try {
      await invoke('hermes_restart_service');
    } catch {
      await invoke('start_hermes_gateway');
    }
    pushSystemMsg('网关重启中，正在等待 API 服务上线...');
    // 3. Poll health every 2s for up to 30s
    await pollUntilConnected(30, 2000);
  } catch (e) {
    pushSystemMsg(`启用失败: ${e}`);
  } finally {
    isEnablingApi.value = false;
  }
}

/** Poll /health every intervalMs until connected or timeout. Cancels on unmount. */
async function pollUntilConnected(maxSeconds: number, intervalMs: number) {
  const deadline = Date.now() + maxSeconds * 1000;
  while (isMounted && Date.now() < deadline) {
    await new Promise(r => setTimeout(r, intervalMs));
    if (!isMounted) break;
    await checkHealth();
    if (isConnected.value) {
      pushSystemMsg('✓ Hermes API 服务已就绪');
      return;
    }
  }
  if (isMounted && !isConnected.value) {
    pushSystemMsg('等待超时，请检查 ~/.hermes/logs/gateway.log 排查问题');
  }
}

async function startGateway() {
  isStarting.value = true;
  try {
    await invoke('start_hermes_gateway');
    await pollUntilConnected(20, 2000);
  } catch (e) {
    pushSystemMsg(`启动失败: ${e}`);
  } finally {
    isStarting.value = false;
  }
}

async function stopGateway() {
  isStopping.value = true;
  try {
    await invoke('stop_hermes_gateway');
    isConnected.value = false;
    healthInfo.value = null;
  } catch (e) {
    pushSystemMsg(`停止失败: ${e}`);
  } finally {
    isStopping.value = false;
  }
}

// ===== Sessions =====
const sessions = ref<Session[]>(loadSessions());
const currentSession = ref<Session | null>(null);
const searchQuery = ref('');

const filteredSessions = computed(() => {
  const q = searchQuery.value.toLowerCase();
  const list = q
    ? sessions.value.filter(s => s.title.toLowerCase().includes(q))
    : sessions.value;
  return list.slice().reverse(); // Newest first
});

function newSession(): Session {
  return {
    id: crypto.randomUUID(),
    title: '新会话',
    messages: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

function startNewSession() {
  const s = newSession();
  sessions.value.push(s);
  currentSession.value = s;
  saveSessions();
}

async function selectSession(s: Session) {
  currentSession.value = s;
  
  // If session has no messages (likely just synced), fetch history from Hermes CLI
  if (s.messages.length <= 1 && s.id !== 'new-session') {
    try {
      const history = await invoke<any[]>('hermes_get_session_messages', { sessionId: s.id });
      if (history && history.length > 0) {
        // Map Hermes internal message format to our UI format
        s.messages = history.map(m => ({
          role: m.role as any,
          content: m.content || '',
          timestamp: m.timestamp ? m.timestamp * 1000 : Date.now(),
          toolName: m.tool_name,
          toolStatus: m.content ? 'completed' : 'running',
        }));
        saveSessions();
      }
    } catch (e) {
      console.error('Failed to fetch session history:', e);
    }
  }
  
  nextTick(scrollToBottom);
}

function deleteSession(s: Session, evt: Event) {
  evt.stopPropagation();
  sessions.value = sessions.value.filter(x => x.id !== s.id);
  if (currentSession.value?.id === s.id) {
    currentSession.value = sessions.value.length
      ? sessions.value[sessions.value.length - 1]
      : null;
  }
  saveDeletedId(s.id);
  saveSessions();
}

function updateTitle(s: Session, firstMsg: string) {
  if (s.title === '新会话') {
    s.title = firstMsg.slice(0, 28) + (firstMsg.length > 28 ? '…' : '');
  }
  s.updatedAt = Date.now();
}

// ===== Chat =====
const userInput = ref('');
const isStreaming = ref(false);
const currentRunId = ref<string | null>(null);
const scrollContainer = ref<HTMLElement | null>(null);

let unlistenChunk: UnlistenFn | null = null;
let unlistenThinking: UnlistenFn | null = null;
let unlistenDone: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;
let unlistenRunId: UnlistenFn | null = null;
let unlistenToolCalls: UnlistenFn | null = null;
let unlistenToolProgress: UnlistenFn | null = null;

async function setupListeners() {
  unlistenChunk = await listen<{ content: string }>('hermes-chunk', ({ payload }) => {
    if (!currentSession.value || !payload.content) return;
    const msgs = currentSession.value.messages;
    const last = msgs[msgs.length - 1];
    if (last?.role === 'assistant' && last.isStreaming) {
      last.content += payload.content;
    } else {
      msgs.push({ role: 'assistant', content: payload.content, timestamp: Date.now(), isStreaming: true });
    }
    scrollToBottom();
  });

  unlistenThinking = await listen<{ content: string }>('hermes-thinking', ({ payload }) => {
    if (!currentSession.value || !payload.content) return;
    const msgs = currentSession.value.messages;
    const last = msgs[msgs.length - 1];
    if (last?.role === 'thought' && last.isStreaming) {
      last.content += payload.content;
    } else {
      msgs.push({ role: 'thought', content: payload.content, timestamp: Date.now(), isStreaming: true });
    }
    scrollToBottom();
  });

  unlistenDone = await listen('hermes-done', () => {
    isStreaming.value = false;
    currentRunId.value = null;
    if (currentSession.value) {
      currentSession.value.messages.forEach(m => {
        if (m.isStreaming) m.isStreaming = false;
      });
      saveSessions();
    }
  });

  unlistenError = await listen<{ message: string }>('hermes-error', ({ payload }) => {
    isStreaming.value = false;
    currentRunId.value = null;
    pushSystemMsg(`错误: ${payload.message}`);
    saveSessions();
  });

  unlistenRunId = await listen<{ run_id: string }>('hermes-run-id', ({ payload }) => {
    currentRunId.value = payload.run_id;
  });

  unlistenToolCalls = await listen<{ tool_calls: any[] }>('hermes-tool-calls', ({ payload }) => {
    if (!currentSession.value) return;
    for (const tc of payload.tool_calls ?? []) {
      const name = tc?.function?.name || tc?.name || '未知工具';
      const id = tc?.id || tc?.toolCallId;
      // Avoid duplicates if tool progress already added it
      if (id && currentSession.value.messages.some(m => m.toolCallId === id)) continue;

      currentSession.value.messages.push({
        role: 'tool',
        content: `调用工具: **${name}**`,
        toolName: name,
        toolCallId: id,
        timestamp: Date.now(),
      });
    }
    scrollToBottom();
  });

  unlistenToolProgress = await listen<any>('hermes-tool-progress', ({ payload }) => {
    if (!currentSession.value) return;
    const id = payload.toolCallId;
    const msgs = currentSession.value.messages;
    
    // Find existing message for this tool call
    const existing = id ? msgs.find(m => m.toolCallId === id) : null;

    if (existing) {
      existing.toolStatus = payload.status;
      if (payload.label) existing.content = payload.label;
    } else {
      msgs.push({
        role: 'tool',
        content: payload.label || `正在运行 ${payload.tool}...`,
        toolName: payload.tool,
        toolCallId: id,
        toolStatus: payload.status,
        timestamp: Date.now(),
      });
    }
    scrollToBottom();
  });
}

async function sendMessage() {
  const text = userInput.value.trim();
  if (!text || isStreaming.value) return;

  if (!isConnected.value) {
    pushSystemMsg('Hermes 网关未连接，请先启动网关或等待连接');
    return;
  }

  // Ensure there's an active session
  if (!currentSession.value) startNewSession();
  const session = currentSession.value!;

  userInput.value = '';
  session.messages.push({ role: 'user', content: text, timestamp: Date.now() });
  updateTitle(session, text);
  scrollToBottom();

  isStreaming.value = true;

  // Build OpenAI-format messages array (only user/assistant messages)
  const historyMessages = session.messages
    .filter(m => m.role === 'user' || m.role === 'assistant')
    .map(m => ({ role: m.role, content: m.content }));

  // 知识库增强：若启用，搜索 KB 并将相关内容注入为 system 消息
  const sendWithKb = async () => {
    let apiMessages = [...historyMessages];
    if (kbEnabled.value) {
      try {
        const kbContext = await invoke<string>('hermes_search_kb', { query: text });
        if (kbContext && kbContext.trim().length > 0) {
          // 将 KB 上下文作为第一条 system 消息
          apiMessages = [
            { role: 'system', content: `以下是来自知识库的相关参考信息，请结合这些内容回答用户问题：\n${kbContext}` },
            ...apiMessages,
          ];
        }
      } catch {
        // KB 搜索失败不影响正常对话
      }
    }
    return apiMessages;
  };

  // Fire-and-forget; real-time updates come via events.
  sendWithKb().then(apiMessages =>
    invoke('hermes_send_message', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
      messages: apiMessages,
      sessionId: session.id,
    })
  ).catch(e => {
    isStreaming.value = false;
    pushSystemMsg(`发送失败: ${e}`);
  });
}

async function stopRun() {
  if (!currentRunId.value) {
    isStreaming.value = false;
    return;
  }
  try {
    await invoke('hermes_stop_run', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
      runId: currentRunId.value,
    });
    isStreaming.value = false;
    currentRunId.value = null;
    pushSystemMsg('已中止当前任务');
  } catch (e) {
    pushSystemMsg(`中止失败: ${e}`);
  }
}

function pushSystemMsg(content: string) {
  if (!currentSession.value) return;
  currentSession.value.messages.push({ role: 'system', content, timestamp: Date.now() });
  scrollToBottom();
}

function scrollToBottom() {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
    }
  });
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendMessage();
  }
}

// ===== Active Runs & Approvals =====
const activeRuns = ref<any[]>([]);
const pendingApprovals = ref<any[]>([]);

async function fetchRuns() {
  if (!isConnected.value || !currentRunId.value) {
    activeRuns.value = [];
    pendingApprovals.value = [];
    return;
  }
  try {
    const runs = await invoke<any[]>('hermes_list_runs', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
      runId: currentRunId.value,
    });
    activeRuns.value = runs;
    pendingApprovals.value = runs.filter(r =>
      r.status === 'waiting_for_approval' || r.status === 'awaiting_approval' || r.requires_approval
    );
  } catch {
    activeRuns.value = [];
    pendingApprovals.value = [];
  }
}

async function approveRun(runId: string, approved: boolean) {
  try {
    await invoke('hermes_approve_run', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
      runId,
      approved,
    });
    pushSystemMsg(approved ? '已批准工具调用' : '已拒绝工具调用');
    await fetchRuns();
  } catch (e) {
    pushSystemMsg(`操作失败: ${e}`);
  }
}

// ===== Import Hermes CLI sessions =====
interface HermesCliSession {
  id: string;
  title: string;
  preview: string;
  last_active: string;
}

async function syncHermesSessions() {
  if (!isConnected.value) return;
  try {
    const result = await invoke<HermesCliSession[]>('list_hermes_sessions');
    const deletedIds = loadDeletedIds();
    let addedAny = false;
    for (const s of result) {
      // 跳过用户已手动删除的会话
      if (deletedIds.has(s.id)) continue;
      const existing = sessions.value.find(x => x.id === s.id);
      if (!existing) {
        sessions.value.push({
          id: s.id,
          title: s.title || '无标题会话',
          messages: [{
            role: 'system',
            content: `已同步 Hermes 会话: ${s.preview || s.id}`,
            timestamp: Date.now(),
          }],
          createdAt: Date.now(),
          updatedAt: Date.now(),
        });
        addedAny = true;
      }
    }
    if (addedAny) saveSessions();
  } catch (e) {
    console.error('Failed to sync Hermes sessions:', e);
  }
}

// ===== Rendering =====
function renderMarkdown(text: string): string {
  try {
    return marked(text, { breaks: true }) as string;
  } catch {
    return text;
  }
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
}

function formatRelativeTime(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return '刚刚';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}分钟前`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}小时前`;
  return `${Math.floor(diff / 86_400_000)}天前`;
}
// ===== Hermes Toolbox =====
const skills = ref<any[]>([]);
const tools = ref<any[]>([]);
const isLoadingSkills = ref(false);
const isLoadingTools = ref(false);
const showToolbox = ref(false);
// Track which skills/tools are currently being toggled (per-item loading state)
const togglingSkills = ref<Record<string, boolean>>({});
const togglingTools = ref<Record<string, boolean>>({});
const currentToolboxTab = ref<'mine' | 'custom' | 'library' | 'install'>('mine');
const rightSidebarTab = ref<'tasks' | 'gateway' | 'kb' | 'info'>('tasks');

// ===== Knowledge Base =====
const kbEnabled = ref<boolean>(
  localStorage.getItem('hermes_kb_enabled') === 'true'
);
watch(kbEnabled, v => localStorage.setItem('hermes_kb_enabled', String(v)));

interface KBFile { name: string }
const kbFiles = ref<KBFile[]>([]);
const isLoadingKb = ref(false);
const isUploadingKb = ref(false);
const kbSearchQuery = ref('');
const kbSearchResult = ref('');
const isSearchingKb = ref(false);

async function loadKbFiles() {
  isLoadingKb.value = true;
  try {
    const res = await invoke<any>('list_kb_files');
    if (Array.isArray(res)) {
      kbFiles.value = res.map((f: any) => ({ name: typeof f === 'string' ? f : f.name || String(f) }));
    } else if (res && Array.isArray(res.files)) {
      kbFiles.value = res.files.map((f: any) => ({ name: typeof f === 'string' ? f : f.name || String(f) }));
    }
  } catch (e) {
    console.error('KB list error:', e);
  } finally {
    isLoadingKb.value = false;
  }
}

async function uploadKbFile() {
  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: 'Documents', extensions: ['txt', 'pdf', 'json', 'md'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    isUploadingKb.value = true;
    const res = await invoke<any>('add_to_kb', { filePath: selected });
    if (res.status === 'success') {
      pushSystemMsg(`知识库：已添加 ${res.chunks_added} 个切片`);
      await loadKbFiles();
    } else {
      pushSystemMsg(`知识库添加失败: ${res.error || '未知错误'}`);
    }
  } catch (e) {
    pushSystemMsg(`知识库上传失败: ${e}`);
  } finally {
    isUploadingKb.value = false;
  }
}

async function deleteKbFile(filename: string) {
  try {
    await invoke('delete_kb_file', { filename });
    pushSystemMsg(`知识库：已删除 ${filename}`);
    await loadKbFiles();
  } catch (e) {
    pushSystemMsg(`删除失败: ${e}`);
  }
}

async function testKbSearch() {
  if (!kbSearchQuery.value.trim()) return;
  isSearchingKb.value = true;
  kbSearchResult.value = '';
  try {
    const result = await invoke<string>('hermes_search_kb', { query: kbSearchQuery.value.trim() });
    kbSearchResult.value = result || '（未找到相关内容）';
  } catch (e) {
    kbSearchResult.value = `搜索失败: ${e}`;
  } finally {
    isSearchingKb.value = false;
  }
}
const skillToInstall = ref('');
const isInstallingSkill = ref(false);

const pinnedToolNames = ref<string[]>(
  JSON.parse(localStorage.getItem('hermes_pinned_tools') || '["web", "terminal"]')
);

watch(pinnedToolNames, (newVal) => {
  localStorage.setItem('hermes_pinned_tools', JSON.stringify(newVal));
}, { deep: true });

function togglePin(name: string) {
  const idx = pinnedToolNames.value.indexOf(name);
  if (idx > -1) {
    pinnedToolNames.value.splice(idx, 1);
  } else {
    pinnedToolNames.value.push(name);
  }
}

async function loadSkills() {
  if (!isConnected.value) return;
  isLoadingSkills.value = true;
  try {
    skills.value = await invoke('hermes_list_skills');
  } catch (e) {
    console.error('Failed to load skills:', e);
  } finally {
    isLoadingSkills.value = false;
  }
}

async function loadTools() {
  if (!isConnected.value) return;
  isLoadingTools.value = true;
  try {
    tools.value = await invoke('hermes_list_tools');
  } catch (e) {
    console.error('Failed to load tools:', e);
  } finally {
    isLoadingTools.value = false;
  }
}

async function installSkill(name?: string) {
  const target = name || skillToInstall.value;
  if (!target.trim()) return;

  isInstallingSkill.value = true;
  try {
    pushSystemMsg(`正在安装技能 ${target}...`);
    await invoke('hermes_install_skill', { name: target });
    pushSystemMsg(`技能 ${target} 安装成功，正在重启网关以加载新技能...`);
    skillToInstall.value = '';
    // 重启网关，让新技能对 AI agent 可见（网关在启动时加载技能列表）
    await invoke('hermes_restart_service');
    pushSystemMsg('网关已重启，新技能现在可以使用');
    await loadSkills();
  } catch (e) {
    pushSystemMsg(`安装失败: ${e}`);
  } finally {
    isInstallingSkill.value = false;
  }
}

async function uninstallSkill(name: string) {
  try {
    pushSystemMsg(`正在卸载技能 ${name}...`);
    await invoke('hermes_uninstall_skill', { name });
    pushSystemMsg(`技能 ${name} 卸载成功，正在重启网关...`);
    await invoke('hermes_restart_service');
    pushSystemMsg('网关已重启');
    await loadSkills();
  } catch (e) {
    pushSystemMsg(`卸载失败: ${e}`);
  }
}

async function toggleSkillStatus(skill: any) {
  if (togglingSkills.value[skill.name]) return; // Prevent double-click
  const newEnabled = skill.status !== 'enabled';
  togglingSkills.value[skill.name] = true;
  // Optimistic update
  const oldStatus = skill.status;
  skill.status = newEnabled ? 'enabled' : 'disabled';
  try {
    await invoke('hermes_toggle_skill_status', { name: skill.name, enable: newEnabled });
  } catch (e) {
    // Rollback on failure
    skill.status = oldStatus;
    pushSystemMsg(`操作失败: ${e}`);
  } finally {
    delete togglingSkills.value[skill.name];
  }
}

async function toggleToolStatus(tool: any) {
  if (togglingTools.value[tool.name]) return;
  const newEnabled = !tool.enabled;
  togglingTools.value[tool.name] = true;
  const oldEnabled = tool.enabled;
  tool.enabled = newEnabled;
  try {
    await invoke('hermes_toggle_tool_status', { name: tool.name, enable: newEnabled });
  } catch (e) {
    tool.enabled = oldEnabled;
    pushSystemMsg(`操作失败: ${e}`);
  } finally {
    delete togglingTools.value[tool.name];
  }
}

function selectTool(tool: any) {
  const keyword = tool.keyword || `!${tool.name}`;
  if (userInput.value.startsWith('!')) {
    // Replace existing tool keyword
    const parts = userInput.value.split(' ');
    parts[0] = keyword;
    userInput.value = parts.join(' ');
  } else {
    userInput.value = `${keyword} ${userInput.value}`;
  }
}

const activeToolbarItems = computed(() => {
  const results: { name: string; keyword: string }[] = [];
  
  // From tools
  for (const t of tools.value) {
    if (t.enabled && pinnedToolNames.value.includes(t.name)) {
      results.push({ name: t.name, keyword: t.keyword });
    }
  }
  
  // From skills
  for (const s of skills.value) {
    if (s.status === 'enabled' && pinnedToolNames.value.includes(s.name)) {
      if (!results.some(r => r.name === s.name)) {
        results.push({ name: s.name, keyword: `!${s.name}` });
      }
    }
  }
  
  return results;
});

const builtinSkills = computed(() => skills.value.filter(s => s.source === 'builtin'));
const customSkills = computed(() => skills.value.filter(s => s.source !== 'builtin'));
const enabledTools = computed(() => tools.value.filter(t => t.enabled));
const disabledTools = computed(() => tools.value.filter(t => !t.enabled));

// ===== Lifecycle =====
onMounted(async () => {
  await loadConfig();
  await setupListeners();
  await checkHealth();

  if (isConnected.value) {
    loadSkills();
    loadTools();
    syncHermesSessions();
  }

  healthInterval = setInterval(async () => {
    const wasConnected = isConnected.value;
    await checkHealth();
    if (isConnected.value) {
      await fetchRuns();
      if (!wasConnected) {
        loadSkills();
        loadTools();
        syncHermesSessions();
      }
    }
  }, 5000);

  // Auto-select most recent session or create one
  if (sessions.value.length > 0) {
    currentSession.value = sessions.value[sessions.value.length - 1];
    nextTick(scrollToBottom);
  } else {
    startNewSession();
  }
});

onUnmounted(() => {
  isMounted = false;
  if (healthInterval) clearInterval(healthInterval);
  if (unlistenChunk) unlistenChunk();
  if (unlistenThinking) unlistenThinking();
  if (unlistenDone) unlistenDone();
  if (unlistenError) unlistenError();
  if (unlistenRunId) unlistenRunId();
  if (unlistenToolCalls) unlistenToolCalls();
  if (unlistenToolProgress) unlistenToolProgress();
});
</script>

<style scoped>
.prose {
  word-break: break-all;
  overflow-wrap: anywhere;
}

:deep(.prose pre) {
  white-space: pre-wrap;
  word-break: break-all;
}

:deep(.prose code) {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>

<template>
  <div class="flex h-full bg-gray-950 overflow-hidden text-sm">

    <!-- ===== Left: Session Sidebar ===== -->
    <aside class="w-60 border-r border-gray-800/60 flex flex-col flex-shrink-0 bg-gray-950">
      <!-- New session -->
      <div class="p-3 border-b border-gray-800/60">
        <button
          @click="startNewSession"
          class="w-full flex items-center justify-center gap-2 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg font-medium transition-colors text-xs"
        >
          <Plus class="w-3.5 h-3.5" />
          新建会话
        </button>
      </div>

      <!-- Search -->
      <div class="px-3 py-2 border-b border-gray-800/60">
        <div class="relative">
          <Search class="absolute left-2.5 top-2 w-3.5 h-3.5 text-gray-500" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索会话..."
            class="w-full bg-gray-900 border border-gray-800 rounded-md pl-8 pr-3 py-1.5 text-xs text-gray-300 placeholder-gray-600 focus:outline-none focus:border-indigo-500/60 transition-colors"
          />
        </div>
      </div>

      <!-- Session List -->
      <div class="flex-1 overflow-y-auto py-2 custom-scrollbar">
        <div v-if="filteredSessions.length === 0" class="flex flex-col items-center justify-center py-10 text-gray-600">
          <MessageSquare class="w-8 h-8 mb-2 opacity-30" />
          <p class="text-xs">暂无会话</p>
        </div>

        <button
          v-for="s in filteredSessions"
          :key="s.id"
          @click="selectSession(s)"
          :class="[
            'w-full text-left px-3 py-2.5 transition-all group relative border-l-2',
            currentSession?.id === s.id
              ? 'bg-indigo-600/10 border-indigo-500 text-white'
              : 'border-transparent hover:bg-gray-900/60 text-gray-400 hover:text-gray-200'
          ]"
        >
          <div class="flex justify-between items-start gap-1">
            <span class="text-xs font-medium truncate flex-1">{{ s.title }}</span>
            <button
              @click="deleteSession(s, $event)"
              class="opacity-0 group-hover:opacity-100 p-0.5 hover:text-red-400 transition-all flex-shrink-0"
            >
              <Trash2 class="w-3 h-3" />
            </button>
          </div>
          <div class="flex items-center gap-1 mt-0.5">
            <span class="text-[10px] text-gray-600">{{ formatRelativeTime(s.updatedAt) }}</span>
            <span v-if="s.messages.length > 0" class="text-[10px] text-gray-700">
              · {{ s.messages.filter(m => m.role !== 'system').length }} 条
            </span>
          </div>
        </button>
      </div>

      <!-- Sidebar footer -->
      <div class="p-3 border-t border-gray-800/60 text-[10px] text-gray-600 flex items-center gap-1.5">
        <Binary class="w-3 h-3" />
        <span>{{ sessions.length }} 个会话</span>
      </div>
    </aside>

    <!-- ===== Center: Chat ===== -->
    <div class="flex-1 flex flex-col min-w-0">
      <!-- Chat header -->
      <div class="border-b border-gray-800/60 px-4 py-3 flex items-center justify-between flex-shrink-0 bg-gray-950/80">
        <div class="flex items-center gap-2">
          <div :class="isConnected ? 'bg-green-500' : 'bg-gray-600'" class="w-2 h-2 rounded-full flex-shrink-0" />
          <div>
            <span class="text-sm font-semibold text-white">{{ currentSession?.title || 'Hermes Agent' }}</span>
            <span v-if="currentSession?.id" class="text-[10px] text-gray-600 ml-2 font-mono">
              #{{ currentSession.id.slice(0, 8) }}
            </span>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <span v-if="isStreaming" class="text-xs text-indigo-400 flex items-center gap-1">
            <Loader2 class="w-3 h-3 animate-spin" />
            生成中...
          </span>
          <button
            v-if="isStreaming"
            @click="stopRun"
            class="flex items-center gap-1 px-3 py-1.5 bg-red-500/10 text-red-400 hover:bg-red-500/20 border border-red-500/30 rounded-lg text-xs transition-colors"
          >
            <Square class="w-3 h-3 fill-current" />
            停止
          </button>
        </div>
      </div>

      <!-- Messages area -->
      <div ref="scrollContainer" class="flex-1 overflow-y-auto p-4 space-y-4 custom-scrollbar">
        <!-- Empty state -->
        <div v-if="!currentSession || currentSession.messages.length === 0"
             class="flex flex-col items-center justify-center h-full text-gray-600 pb-16">
          <Bot class="w-12 h-12 mb-3 opacity-20" />
          <p class="text-base font-medium text-gray-500">Hermes Agent</p>
          <p class="text-xs mt-1 text-gray-600">
            {{ isConnected ? '发送消息开始对话' : '请先连接 Hermes 网关' }}
          </p>
        </div>

        <!-- Messages -->
        <template v-for="(msg, idx) in currentSession?.messages ?? []" :key="idx">
          <!-- Thinking / Reasoning block -->
          <div v-if="msg.role === 'thought'" class="flex items-start gap-2 opacity-80">
            <div class="w-7 h-7 rounded-lg bg-gray-800 border border-gray-700 flex items-center justify-center flex-shrink-0 mt-0.5">
              <Binary class="w-3.5 h-3.5 text-gray-500" />
            </div>
            <div class="flex-1 min-w-0 max-w-[85%]">
              <div class="bg-gray-900/50 border border-gray-800 rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm italic text-gray-400 text-xs border-dashed">
                <div class="flex items-center gap-2 mb-1.5 text-gray-500 font-medium">
                  <RefreshCw v-if="msg.isStreaming" class="w-3 h-3 animate-spin" />
                  <span>AI 思考中...</span>
                </div>
                <div class="whitespace-pre-wrap break-words leading-relaxed">{{ msg.content }}</div>
              </div>
            </div>
          </div>

          <!-- System message (including errors) -->
          <div v-else-if="msg.role === 'system'" class="flex justify-center">
            <div
              :class="[
                'flex items-center gap-1.5 text-[11px] px-3 py-1.5 rounded-full border transition-colors',
                msg.content.includes('错误') || msg.content.includes('失败')
                  ? 'bg-red-500/10 text-red-400 border-red-500/20'
                  : 'bg-gray-900 text-gray-500 border-gray-800'
              ]"
            >
              <X v-if="msg.content.includes('错误') || msg.content.includes('失败')" class="w-3 h-3 flex-shrink-0" />
              <Info v-else class="w-3 h-3 flex-shrink-0" />
              <span class="break-words max-w-[280px] sm:max-w-md">{{ msg.content }}</span>
            </div>
          </div>

          <!-- Tool call message -->
          <div v-else-if="msg.role === 'tool'" class="flex items-start gap-2">
            <div :class="[
              'w-6 h-6 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5 transition-colors',
              msg.toolStatus === 'completed' ? 'bg-green-500/10 border border-green-500/20' : 'bg-amber-500/10 border border-amber-500/20'
            ]">
              <Check v-if="msg.toolStatus === 'completed'" class="w-3.5 h-3.5 text-green-400" />
              <Loader2 v-else class="w-3.5 h-3.5 text-amber-400 animate-spin" />
            </div>
            <div :class="[
              'rounded-xl rounded-tl-sm px-3 py-2 text-xs font-mono break-all max-w-[85%] border transition-colors',
              msg.toolStatus === 'completed' 
                ? 'bg-green-500/5 border-green-500/20 text-green-300/80' 
                : 'bg-amber-500/5 border-amber-500/20 text-amber-300/80'
            ]">
              {{ msg.content }}
            </div>
          </div>

          <!-- User message -->
          <div v-else-if="msg.role === 'user'" class="flex items-start gap-2 justify-end">
            <div class="max-w-[85%] bg-indigo-600 text-white rounded-2xl rounded-tr-sm px-4 py-3 text-sm whitespace-pre-wrap break-all shadow-sm">
              {{ msg.content }}
            </div>
            <div class="w-7 h-7 rounded-lg bg-gray-800 border border-gray-700 flex items-center justify-center flex-shrink-0 mt-0.5">
              <User class="w-3.5 h-3.5 text-gray-400" />
            </div>
          </div>

          <!-- Assistant message -->
          <div v-else-if="msg.role === 'assistant'" class="flex items-start gap-2">
            <div class="w-7 h-7 rounded-lg bg-indigo-600/20 border border-indigo-500/30 flex items-center justify-center flex-shrink-0 mt-0.5">
              <Bot class="w-3.5 h-3.5 text-indigo-400" />
            </div>
            <div class="flex-1 min-w-0 max-w-[85%]">
              <div class="bg-gray-900 border border-gray-800/80 rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm overflow-x-auto overflow-y-hidden">
                <div
                  class="prose prose-invert prose-sm max-w-none text-gray-200 break-all"
                  v-html="renderMarkdown(msg.content || '')"
                />
                <span v-if="msg.isStreaming" class="inline-block w-1 h-3.5 bg-indigo-400 animate-pulse ml-0.5 rounded-sm" />
              </div>
              <div class="text-[10px] text-gray-600 mt-1 ml-2">{{ formatTime(msg.timestamp) }}</div>
            </div>
          </div>
        </template>
      </div>

      <!-- Input area -->
      <div class="border-t border-gray-800/60 p-3 flex-shrink-0 bg-gray-950/80">
        <!-- Tool selection bar -->
        <div v-if="isConnected" class="flex gap-2 items-center overflow-x-auto pb-2 mb-2 custom-scrollbar no-scrollbar">
          <button
            @click="showToolbox = true"
            class="flex-shrink-0 p-1.5 bg-gray-900 border border-gray-800 hover:border-indigo-500/50 rounded text-gray-500 hover:text-indigo-400 transition-colors"
            title="管理工具箱"
          >
            <LayoutGrid class="w-3.5 h-3.5" />
          </button>
          
          <div class="h-4 w-px bg-gray-800 mx-1 flex-shrink-0" />

          <button
            v-for="tool in activeToolbarItems"
            :key="tool.name"
            @click="selectTool(tool)"
            class="flex-shrink-0 px-2 py-1 bg-indigo-500/5 border border-indigo-500/20 hover:border-indigo-500/50 rounded text-[10px] text-gray-400 hover:text-indigo-400 transition-colors flex items-center gap-1"
          >
            <span class="text-[8px] opacity-60 font-mono">!</span>{{ tool.name }}
          </button>
          
          <div v-if="activeToolbarItems.length === 0" class="text-[10px] text-gray-600 italic px-1">
            点击方块图标配置快捷工具...
          </div>
        </div>

        <div
          :class="[
            'flex items-end gap-2 bg-gray-900 border rounded-xl p-2 transition-colors',
            isConnected && !isStreaming
              ? 'border-gray-800 focus-within:border-indigo-500/60'
              : 'border-gray-800/40 opacity-60'
          ]"
        >
          <textarea
            v-model="userInput"
            @keydown="handleKeydown"
            :disabled="!isConnected || isStreaming"
            :placeholder="isConnected ? '输入消息... (Enter 发送, Shift+Enter 换行)' : '等待连接 Hermes 网关...'"
            rows="1"
            class="flex-1 bg-transparent border-none text-white text-sm px-2 py-1.5 resize-none min-h-[36px] max-h-32 focus:outline-none placeholder-gray-600"
          />
          <button
            @click="sendMessage"
            :disabled="!userInput.trim() || !isConnected || isStreaming"
            :class="[
              'p-2 rounded-lg transition-colors flex-shrink-0',
              userInput.trim() && isConnected && !isStreaming
                ? 'bg-indigo-600 hover:bg-indigo-500 text-white shadow'
                : 'bg-gray-800 text-gray-600 cursor-not-allowed'
            ]"
          >
            <Send class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- ===== Right: Task & Info Panel ===== -->
    <aside class="w-64 border-l border-gray-800/60 flex flex-col flex-shrink-0 bg-gray-950 overflow-hidden">
      
      <!-- Sidebar Tabs -->
      <div class="flex border-b border-gray-800 bg-gray-900/40">
        <button
          v-for="t in [
            { id: 'tasks',   label: '任务', icon: Play },
            { id: 'gateway', label: '网关', icon: RefreshCw },
            { id: 'kb',      label: '知识库', icon: Database },
            { id: 'info',    label: '说明', icon: Info }
          ]"
          :key="t.id"
          @click="rightSidebarTab = t.id as any; if(t.id === 'kb') loadKbFiles()"
          :class="[
            'flex-1 py-3 flex flex-col items-center gap-1 transition-all relative',
            rightSidebarTab === t.id
              ? 'bg-indigo-600/10 text-indigo-400'
              : 'text-gray-600 hover:text-gray-400 hover:bg-gray-800/30'
          ]"
        >
          <component :is="t.icon" :class="rightSidebarTab === 'gateway' && t.id === 'gateway' && isChecking ? 'animate-spin' : ''" class="w-3.5 h-3.5" />
          <span class="text-[9px] font-bold uppercase tracking-tighter">{{ t.label }}</span>
          <!-- KB 已启用时显示绿点 -->
          <div v-if="t.id === 'kb' && kbEnabled"
               class="absolute top-2 right-2 w-1.5 h-1.5 rounded-full bg-green-400 shadow-[0_0_4px_rgba(74,222,128,0.8)]" />
        </button>
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar">
        <!-- Tab: Tasks -->
        <div v-if="rightSidebarTab === 'tasks'">
          <!-- Active Runs -->
          <div class="p-4 border-b border-gray-800/60">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider">运行中的任务</h3>
              <span class="text-[10px] text-gray-600 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-800">{{ activeRuns.length }}</span>
            </div>

            <div v-if="activeRuns.length === 0" class="text-[11px] text-gray-600 text-center py-8 italic border border-dashed border-gray-800/40 rounded-xl mx-2">
              暂无活动任务
            </div>

            <div v-for="run in activeRuns" :key="run.id" class="mb-3 bg-gray-900 border border-gray-800 rounded-xl p-3 shadow-sm group">
              <div class="flex items-center justify-between mb-2">
                <span class="text-[10px] font-mono text-gray-500 truncate group-hover:text-gray-300 transition-colors">{{ (run.id || '').slice(0, 12) }}</span>
                <span :class="{
                  'text-green-400 bg-green-500/10 border-green-500/20': run.status === 'running',
                  'text-yellow-400 bg-yellow-500/10 border-yellow-500/20': run.status?.includes('waiting') || run.status?.includes('approval'),
                  'text-gray-400 bg-gray-800 border-transparent': !run.status?.includes('running') && !run.status?.includes('waiting'),
                }" class="text-[9px] px-1.5 py-0.5 rounded border">{{ run.status }}</span>
              </div>
              <div v-if="run.status?.includes('waiting') || run.status?.includes('approval')"
                   class="flex gap-2 mt-2">
                <button @click="approveRun(run.id, true)"
                        class="flex-1 flex items-center justify-center gap-1 py-1.5 bg-green-600 hover:bg-green-500 text-white rounded-lg text-[10px] font-bold transition-colors">
                  <Check class="w-3 h-3" /> 批准
                </button>
                <button @click="approveRun(run.id, false)"
                        class="flex-1 flex items-center justify-center gap-1 py-1.5 bg-gray-800 hover:bg-red-600 hover:text-white text-gray-400 rounded-lg text-[10px] font-bold transition-colors">
                  <X class="w-3 h-3" /> 拒绝
                </button>
              </div>
            </div>
          </div>

          <!-- Current Run Control -->
          <div v-if="isStreaming || currentRunId" class="p-4">
            <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">当前实时任务</h3>
            <div class="bg-indigo-600/5 border border-indigo-500/20 rounded-2xl p-4">
              <div class="flex items-center gap-2 mb-3">
                <div class="relative">
                  <div class="w-3 h-3 bg-indigo-500 rounded-full animate-ping absolute inset-0 opacity-75"></div>
                  <div class="w-3 h-3 bg-indigo-500 rounded-full relative"></div>
                </div>
                <span class="text-xs font-semibold text-indigo-300">正在生成回答...</span>
              </div>
              <div v-if="currentRunId" class="text-[10px] font-mono text-gray-500 mb-4 bg-gray-900/50 p-2 rounded border border-gray-800 truncate">
                ID: {{ currentRunId }}
              </div>
              <button @click="stopRun"
                      class="w-full flex items-center justify-center gap-2 py-2 bg-red-500/10 hover:bg-red-500 hover:text-white text-red-400 border border-red-500/20 rounded-xl text-xs font-bold transition-all shadow-sm">
                <Square class="w-3.5 h-3.5 fill-current" />
                立即中止当前任务
              </button>
            </div>
          </div>
        </div>

        <!-- Tab: Gateway -->
        <div v-if="rightSidebarTab === 'gateway'" class="p-4 space-y-4">
          <!-- Connection Status Card -->
          <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-4">
            <div class="flex items-center justify-between mb-4">
              <h4 class="text-xs font-bold text-gray-400 uppercase tracking-widest">网关状态</h4>
              <div class="flex items-center gap-2">
                <div :class="isConnected ? 'bg-green-500' : 'bg-gray-600'"
                     class="w-2 h-2 rounded-full shadow-sm" />
                <span :class="isConnected ? 'text-green-400' : 'text-gray-500'" class="text-[10px] font-bold">
                  {{ isConnected ? 'ONLINE' : 'OFFLINE' }}
                </span>
              </div>
            </div>

            <div class="space-y-3">
              <div>
                <label class="block text-[9px] text-gray-600 uppercase mb-1 ml-1 font-bold">Endpoint</label>
                <input
                  v-model="gatewayUrl"
                  @change="checkHealth"
                  class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-1.5 text-xs font-mono text-gray-400 focus:outline-none focus:border-indigo-500/50 transition-colors"
                  placeholder="http://127.0.0.1:8642"
                />
              </div>

              <div>
                <label class="block text-[9px] text-gray-600 uppercase mb-1 ml-1 font-bold">API Token</label>
                <input
                  v-model="apiKey"
                  @change="checkHealth"
                  type="password"
                  class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-1.5 text-xs font-mono text-gray-400 focus:outline-none focus:border-indigo-500/50 transition-colors"
                  placeholder="Bearer Token"
                />
              </div>

              <!-- Health info summary -->
              <div v-if="healthInfo" class="bg-gray-950/50 rounded-lg p-2 border border-gray-800/30 space-y-1">
                <div v-for="(val, key) in healthInfo" :key="key"
                     class="flex justify-between text-[9px] text-gray-500">
                  <span class="opacity-60">{{ key }}</span>
                  <span class="font-mono truncate ml-2 text-gray-400">{{ val }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Service Controls -->
          <div class="space-y-2">
            <div v-if="!isConnected" class="bg-amber-500/5 border border-amber-500/20 rounded-2xl p-4 mb-3">
              <p class="text-[10px] text-amber-300/80 leading-relaxed mb-3">
                检测到网关未开启 API 服务，本 App 的历史同步和实时监控依赖此服务。
              </p>
              <button
                @click="enableApiServerAndRestart"
                :disabled="isEnablingApi"
                class="w-full flex items-center justify-center gap-2 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded-xl text-[10px] font-bold transition-all shadow-sm disabled:opacity-50"
              >
                <Loader2 v-if="isEnablingApi" class="w-3 h-3 animate-spin" />
                <Play v-else class="w-3 h-3 fill-current" />
                一键启用并重启
              </button>
            </div>

            <button
              @click="startGateway"
              :disabled="isStarting || isConnected"
              class="w-full flex items-center justify-center gap-2 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl text-xs font-bold transition-all shadow-md disabled:opacity-50"
            >
              <Loader2 v-if="isStarting" class="w-4 h-4 animate-spin" />
              <Play v-else class="w-4 h-4 fill-current" />
              启动 Hermes 网关
            </button>
            
            <div class="flex gap-2">
              <button
                @click="startGateway"
                :disabled="isStarting || !isConnected"
                class="flex-1 flex items-center justify-center gap-1.5 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-xl text-[10px] font-bold transition-colors disabled:opacity-50"
              >
                <RefreshCw class="w-3 h-3" /> 重启
              </button>
              <button
                @click="stopGateway"
                :disabled="isStopping || !isConnected"
                class="flex-1 flex items-center justify-center gap-1.5 py-2 bg-gray-900 border border-red-500/20 text-red-500 hover:bg-red-600 hover:text-white rounded-xl text-[10px] font-bold transition-all disabled:opacity-50"
              >
                <Square class="w-3 h-3 fill-current" /> 停止
              </button>
            </div>
          </div>
        </div>

        <!-- Tab: Knowledge Base -->
        <div v-if="rightSidebarTab === 'kb'" class="flex flex-col h-full">
          <!-- KB 开关 -->
          <div class="p-4 border-b border-gray-800/60">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Database class="w-4 h-4 text-indigo-400" />
                <span class="text-xs font-bold text-gray-300">知识库增强</span>
              </div>
              <button
                @click="kbEnabled = !kbEnabled"
                :class="kbEnabled ? 'bg-green-600 border-green-500' : 'bg-gray-800 border-gray-700'"
                class="w-10 h-5 rounded-full border transition-all relative flex items-center px-0.5 flex-shrink-0"
                :title="kbEnabled ? '点击关闭知识库增强' : '点击开启知识库增强'"
              >
                <div
                  :class="kbEnabled ? 'translate-x-5 bg-white shadow' : 'translate-x-0 bg-gray-500'"
                  class="w-3.5 h-3.5 rounded-full transition-transform duration-200"
                />
              </button>
            </div>
            <p class="text-[10px] text-gray-600 mt-2 leading-relaxed">
              开启后，每次发送消息时会自动检索知识库，将相关内容作为参考注入给 Hermes AI。
            </p>
            <div v-if="kbEnabled" class="mt-2 flex items-center gap-1.5 text-[10px] text-green-400">
              <Zap class="w-3 h-3" /> 知识库增强已激活
            </div>
            <div v-else class="mt-2 flex items-center gap-1.5 text-[10px] text-gray-600">
              <ZapOff class="w-3 h-3" /> 已停用
            </div>
          </div>

          <!-- 文件列表 -->
          <div class="p-4 border-b border-gray-800/60 flex-1 min-h-0 overflow-y-auto custom-scrollbar">
            <div class="flex items-center justify-between mb-3">
              <h4 class="text-[10px] font-bold text-gray-500 uppercase tracking-widest">知识库文件</h4>
              <div class="flex items-center gap-1">
                <button
                  @click="loadKbFiles"
                  :disabled="isLoadingKb"
                  class="p-1 text-gray-600 hover:text-gray-300 transition-colors rounded"
                  title="刷新"
                >
                  <RefreshCw class="w-3 h-3" :class="isLoadingKb ? 'animate-spin' : ''" />
                </button>
                <button
                  @click="uploadKbFile"
                  :disabled="isUploadingKb"
                  class="p-1 text-indigo-400 hover:text-indigo-300 transition-colors rounded"
                  title="添加文件"
                >
                  <Loader2 v-if="isUploadingKb" class="w-3 h-3 animate-spin" />
                  <Upload v-else class="w-3 h-3" />
                </button>
              </div>
            </div>

            <!-- Loading -->
            <div v-if="isLoadingKb" class="py-8 flex justify-center">
              <Loader2 class="w-4 h-4 text-indigo-400 animate-spin" />
            </div>

            <!-- Empty -->
            <div v-else-if="kbFiles.length === 0" class="py-8 text-center">
              <Database class="w-8 h-8 text-gray-800 mx-auto mb-2" />
              <p class="text-[10px] text-gray-600">知识库暂无文件</p>
              <button
                @click="uploadKbFile"
                class="mt-2 text-[10px] text-indigo-400 hover:text-indigo-300 underline"
              >上传第一个文件</button>
            </div>

            <!-- File list -->
            <div v-else class="space-y-1.5">
              <div
                v-for="f in kbFiles" :key="f.name"
                class="flex items-center justify-between bg-gray-900 border border-gray-800/60 rounded-lg px-3 py-2 group hover:border-gray-700 transition-colors"
              >
                <div class="flex items-center gap-2 min-w-0">
                  <FileText class="w-3 h-3 text-gray-600 flex-shrink-0" />
                  <span class="text-[10px] text-gray-400 truncate">{{ f.name }}</span>
                </div>
                <button
                  @click="deleteKbFile(f.name)"
                  class="opacity-0 group-hover:opacity-100 p-1 text-gray-600 hover:text-red-400 transition-all rounded flex-shrink-0"
                  title="删除"
                >
                  <Trash2 class="w-3 h-3" />
                </button>
              </div>
            </div>
          </div>

          <!-- 检索测试 -->
          <div class="p-4">
            <h4 class="text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-2">检索测试</h4>
            <div class="flex gap-1.5">
              <input
                v-model="kbSearchQuery"
                @keydown.enter="testKbSearch"
                placeholder="输入关键词测试检索..."
                class="flex-1 bg-gray-900 border border-gray-800 rounded-lg px-3 py-1.5 text-[11px] text-gray-300 focus:outline-none focus:border-indigo-500/60 transition-colors placeholder-gray-700"
              />
              <button
                @click="testKbSearch"
                :disabled="!kbSearchQuery.trim() || isSearchingKb"
                class="p-1.5 bg-indigo-600/20 hover:bg-indigo-600/40 text-indigo-400 border border-indigo-500/30 rounded-lg transition-colors disabled:opacity-40"
              >
                <Loader2 v-if="isSearchingKb" class="w-3.5 h-3.5 animate-spin" />
                <Search v-else class="w-3.5 h-3.5" />
              </button>
            </div>
            <div v-if="kbSearchResult" class="mt-2 bg-gray-900/80 border border-gray-800 rounded-lg p-3 max-h-40 overflow-y-auto custom-scrollbar">
              <pre class="text-[9px] text-gray-400 whitespace-pre-wrap leading-relaxed">{{ kbSearchResult }}</pre>
            </div>
          </div>
        </div>

        <!-- Tab: Info -->
        <div v-if="rightSidebarTab === 'info'" class="p-4">
          <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-4">使用说明</h3>
          <div class="space-y-4">
            <div class="bg-gray-900/50 border border-gray-800 rounded-xl p-3">
              <div class="text-[10px] font-bold text-indigo-400 mb-1">键盘快捷键</div>
              <ul class="text-[10px] text-gray-600 space-y-1.5">
                <li class="flex justify-between"><span>发送消息</span><code class="bg-gray-800 px-1 rounded text-gray-400">Enter</code></li>
                <li class="flex justify-between"><span>换行</span><code class="bg-gray-800 px-1 rounded text-gray-400">Shift+Enter</code></li>
              </ul>
            </div>
            
            <div class="bg-gray-900/50 border border-gray-800 rounded-xl p-3">
              <div class="text-[10px] font-bold text-indigo-400 mb-1">功能提示</div>
              <div class="space-y-2 text-[10px] text-gray-600 leading-relaxed">
                <p>• 消息开头的 <code class="text-indigo-500">!keyword</code> 会被识别为特定工具调用。</p>
                <p>• 自动同步功能会每 5 秒从 Hermes 获取一次最新会话状态。</p>
                <p>• 所有的工具执行都需要在「任务」标签页中手动审批（如果开启了审批模式）。</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </aside>

    <!-- ===== Toolbox Modal ===== -->
    <div v-if="showToolbox" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm">
      <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl animate-in fade-in zoom-in duration-200 overflow-hidden">
        <div class="flex-shrink-0 p-4 border-b border-gray-800 flex items-center justify-between bg-gray-900/50 rounded-t-2xl">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-indigo-600/20 rounded-lg">
              <LayoutGrid class="w-5 h-5 text-indigo-400" />
            </div>
            <div>
              <h2 class="text-base font-semibold text-white">Hermes 工具箱</h2>
              <p class="text-[10px] text-gray-500">管理你的 AI 技能与快捷工具</p>
            </div>
          </div>
          <div class="flex items-center gap-1">
            <button
              @click="loadSkills(); loadTools()"
              :disabled="isLoadingSkills"
              class="p-2 text-gray-500 hover:text-white transition-colors hover:bg-gray-800 rounded-full disabled:opacity-40"
              title="刷新技能列表"
            >
              <RefreshCw class="w-4 h-4" :class="isLoadingSkills ? 'animate-spin' : ''" />
            </button>
            <button @click="showToolbox = false" class="p-2 text-gray-500 hover:text-white transition-colors hover:bg-gray-800 rounded-full">
              <X class="w-5 h-5" />
            </button>
          </div>
        </div>

        <!-- Tab Headers -->
        <div class="flex-shrink-0 px-4 border-b border-gray-800 flex gap-5 bg-gray-900/30 overflow-x-auto">
          <button
            v-for="t in [
              { id: 'mine',    label: '核心工具',   icon: Wrench,  badge: enabledTools.length   },
              { id: 'custom',  label: '自定义技能', icon: Binary,  badge: customSkills.length   },
              { id: 'library', label: '内置技能库', icon: BookOpen, badge: builtinSkills.length  },
              { id: 'install', label: '安装技能',   icon: Plus,    badge: 0                     }
            ]"
            :key="t.id"
            @click="currentToolboxTab = t.id as any"
            :class="[
              'py-3 text-xs font-medium border-b-2 transition-all flex items-center gap-1.5 whitespace-nowrap flex-shrink-0',
              currentToolboxTab === t.id
                ? 'border-indigo-500 text-indigo-400'
                : 'border-transparent text-gray-500 hover:text-gray-300'
            ]"
          >
            <component :is="t.icon" class="w-3.5 h-3.5" />
            {{ t.label }}
            <span v-if="t.badge > 0"
                  :class="currentToolboxTab === t.id ? 'bg-indigo-500/30 text-indigo-300' : 'bg-gray-800 text-gray-500'"
                  class="text-[9px] px-1.5 py-0.5 rounded-full font-mono">
              {{ t.badge }}
            </span>
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto min-h-0 p-4 custom-scrollbar">
          <!-- Tab: Mine (Core Toolsets only) -->
          <div v-if="currentToolboxTab === 'mine'">
            <div v-if="enabledTools.length > 0" class="mb-6">
              <h4 class="text-[10px] font-bold text-gray-500 mb-3 uppercase tracking-widest flex items-center justify-between px-1">
                <div class="flex items-center gap-2"><Wrench class="w-3 h-3" /> 已启用的核心工具</div>
                <span class="text-[9px] font-normal lowercase italic text-gray-600">点击垃圾桶可禁用</span>
              </h4>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div v-for="tool in enabledTools" :key="tool.name"
                     class="bg-gray-950 border border-gray-800 rounded-xl p-3 flex flex-col gap-3 group hover:border-gray-700 transition-colors">
                  <div class="flex items-start justify-between">
                    <div class="min-w-0 pr-2">
                      <div class="text-sm font-medium text-gray-200 truncate flex items-center gap-1.5">
                        <Wrench class="w-3 h-3 text-gray-600" />
                        {{ tool.name }}
                      </div>
                      <div class="text-[10px] text-gray-500 truncate mt-0.5">{{ tool.description }}</div>
                    </div>
                    <button
                      @click="toggleToolStatus(tool)"
                      :disabled="!!togglingTools[tool.name]"
                      class="p-1 text-gray-600 hover:text-red-400 transition-all rounded-md hover:bg-red-500/10 disabled:opacity-50"
                      title="禁用此工具"
                    >
                      <Loader2 v-if="togglingTools[tool.name]" class="w-3.5 h-3.5 animate-spin" />
                      <Trash2 v-else class="w-3.5 h-3.5" />
                    </button>
                  </div>
                  <div class="flex items-center justify-between pt-2 border-t border-gray-800/50">
                    <div class="flex items-center gap-1.5">
                      <div class="w-1.5 h-1.5 rounded-full bg-green-500" />
                      <span class="text-[10px] text-green-500/80 font-bold uppercase">已启用</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-[9px] text-gray-600">快捷栏</span>
                      <button
                        @click="togglePin(tool.name)"
                        :class="pinnedToolNames.includes(tool.name) ? 'bg-indigo-600 border-indigo-500' : 'bg-gray-800 border-gray-700'"
                        class="w-7 h-4 rounded-full border transition-all relative flex items-center px-0.5 flex-shrink-0"
                      >
                        <div
                          :class="pinnedToolNames.includes(tool.name) ? 'translate-x-3 bg-white shadow-sm' : 'translate-x-0 bg-gray-500'"
                          class="w-2.5 h-2.5 rounded-full transition-transform duration-200"
                        />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div v-if="enabledTools.length === 0" class="py-20 text-center text-gray-600 italic text-sm">
              暂无已启用的核心工具
            </div>
          </div>

          <!-- Tab: Custom Skills -->
          <div v-if="currentToolboxTab === 'custom'">
            <div v-if="isLoadingSkills" class="py-20 flex justify-center">
              <Loader2 class="w-5 h-5 text-indigo-400 animate-spin" />
            </div>
            <template v-else>
              <div v-if="customSkills.length > 0" class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div v-for="skill in customSkills" :key="skill.name"
                     class="bg-indigo-900/5 border border-indigo-500/20 rounded-xl p-3 flex flex-col gap-3 group hover:border-indigo-500/40 transition-colors">
                  <div class="flex items-start justify-between">
                    <div class="min-w-0 pr-2">
                      <div class="text-sm font-medium text-gray-200 truncate flex items-center gap-1.5">
                        <Binary class="w-3.5 h-3.5 text-indigo-400 flex-shrink-0" />
                        {{ skill.name }}
                      </div>
                      <div class="text-[10px] text-gray-500 truncate mt-0.5">
                        {{ skill.category || 'Custom' }} · {{ skill.source }}
                      </div>
                    </div>
                    <!-- Uninstall -->
                    <button
                      @click="uninstallSkill(skill.name)"
                      class="p-1 text-gray-600 hover:text-red-400 transition-all rounded-md hover:bg-red-500/10 flex-shrink-0"
                      title="从系统中彻底删除此技能"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>

                  <div class="flex items-center justify-between pt-2 border-t border-indigo-500/10">
                    <!-- Enable/disable toggle -->
                    <div class="flex items-center gap-2">
                      <span :class="skill.status === 'enabled' ? 'text-green-400' : 'text-gray-600'"
                            class="text-[9px] font-medium w-8 text-right">
                        {{ skill.status === 'enabled' ? '开启' : '关闭' }}
                      </span>
                      <button
                        @click="toggleSkillStatus(skill)"
                        :disabled="!!togglingSkills[skill.name]"
                        :class="skill.status === 'enabled' ? 'bg-green-600 border-green-500' : 'bg-gray-800 border-gray-700'"
                        class="w-8 h-[18px] rounded-full border transition-all relative flex items-center px-0.5 flex-shrink-0 disabled:cursor-wait"
                        :title="skill.status === 'enabled' ? '点击禁用' : '点击启用'"
                      >
                        <Loader2 v-if="togglingSkills[skill.name]"
                                 class="w-2.5 h-2.5 text-white/70 animate-spin mx-auto" />
                        <div v-else
                          :class="skill.status === 'enabled' ? 'translate-x-[14px] bg-white shadow-sm' : 'translate-x-0 bg-gray-500'"
                          class="w-3 h-3 rounded-full transition-transform duration-200"
                        />
                      </button>
                    </div>
                    <!-- Shortcut pin -->
                    <div class="flex items-center gap-2">
                      <span class="text-[9px] text-gray-600">快捷栏</span>
                      <button
                        @click="togglePin(skill.name)"
                        :class="pinnedToolNames.includes(skill.name) ? 'bg-indigo-600 border-indigo-500' : 'bg-gray-800 border-gray-700'"
                        class="w-7 h-4 rounded-full border transition-all relative flex items-center px-0.5"
                      >
                        <div
                          :class="pinnedToolNames.includes(skill.name) ? 'translate-x-3 bg-white shadow-sm' : 'translate-x-0 bg-gray-500'"
                          class="w-2.5 h-2.5 rounded-full transition-transform duration-200"
                        />
                      </button>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Empty state -->
              <div v-else class="py-20 text-center">
                <Binary class="w-10 h-10 text-gray-700 mx-auto mb-3" />
                <p class="text-gray-500 text-sm font-medium mb-1">尚未安装自定义技能</p>
                <p class="text-gray-600 text-xs mb-4">前往「安装技能」标签，从 GitHub 或技能市场安装第三方技能。</p>
                <button
                  @click="currentToolboxTab = 'install'"
                  class="px-4 py-2 bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-300 border border-indigo-500/30 rounded-lg text-xs transition-colors"
                >
                  去安装技能
                </button>
              </div>
            </template>
          </div>

          <!-- Tab: Library (Built-in) -->
          <div v-if="currentToolboxTab === 'library'">
            <!-- Disabled Core Tools -->
            <div v-if="disabledTools.length > 0" class="mb-8 border-b border-gray-800 pb-8">
              <h4 class="text-[10px] font-bold text-amber-500 mb-3 uppercase tracking-widest flex items-center gap-2">
                <Wrench class="w-3 h-3" /> 已禁用的核心工具
              </h4>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div v-for="tool in disabledTools" :key="tool.name" class="bg-amber-500/5 border border-amber-500/20 rounded-xl p-3 flex flex-col gap-3 group transition-colors">
                  <div class="flex items-start justify-between">
                    <div class="min-w-0 pr-2">
                      <div class="text-sm font-medium text-amber-200/80 truncate flex items-center gap-1.5">
                        {{ tool.name }}
                      </div>
                      <div class="text-[10px] text-amber-500/60 truncate mt-0.5">{{ tool.description }}</div>
                    </div>
                  </div>
                  <div class="flex items-center justify-between pt-2 border-t border-amber-500/10">
                    <button 
                      @click="toggleToolStatus(tool)"
                      class="px-3 py-1 bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 rounded-lg text-[10px] font-bold uppercase transition-all border border-amber-500/30"
                    >
                      重新启用
                    </button>
                    <span class="text-[9px] text-amber-500/40 italic text-right">已从主页移除</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Built-in Skills -->
            <h4 class="text-[10px] font-bold text-gray-500 mb-3 uppercase tracking-widest flex items-center gap-2">
              <Binary class="w-3 h-3" /> 内置技能库 (Built-in Library)
            </h4>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div v-for="skill in builtinSkills" :key="skill.name" class="bg-gray-950 border border-gray-800 rounded-xl p-3 flex flex-col gap-3 group hover:border-gray-700 transition-colors">
                <div class="flex items-start justify-between">
                  <div class="min-w-0 pr-2">
                    <div class="text-sm font-medium text-gray-200 truncate flex items-center gap-1.5">
                      {{ skill.name }}
                    </div>
                    <div class="text-[10px] text-gray-600 truncate mt-0.5">{{ skill.category || 'System' }}</div>
                  </div>
                </div>

                <div class="flex items-center justify-between pt-2 border-t border-gray-800/50">
                  <!-- Built-in skill enable/disable toggle -->
                  <div class="flex items-center gap-2">
                    <span :class="skill.status === 'enabled' ? 'text-green-400' : 'text-gray-600'"
                          class="text-[9px] font-medium w-8 text-right">
                      {{ skill.status === 'enabled' ? '开启' : '关闭' }}
                    </span>
                    <button
                      @click="toggleSkillStatus(skill)"
                      :disabled="!!togglingSkills[skill.name]"
                      :class="skill.status === 'enabled'
                        ? 'bg-green-600 border-green-500'
                        : 'bg-gray-800 border-gray-700'"
                      class="w-8 h-[18px] rounded-full border transition-all relative flex items-center px-0.5 flex-shrink-0 disabled:cursor-wait"
                      :title="skill.status === 'enabled' ? '点击禁用技能' : '点击启用技能'"
                    >
                      <Loader2 v-if="togglingSkills[skill.name]"
                               class="w-2.5 h-2.5 text-white/70 animate-spin mx-auto" />
                      <div v-else
                        :class="skill.status === 'enabled'
                          ? 'translate-x-[14px] bg-white shadow-sm'
                          : 'translate-x-0 bg-gray-500'"
                        class="w-3 h-3 rounded-full transition-transform duration-200"
                      />
                    </button>
                  </div>
                  <span class="text-[9px] text-gray-700 italic">内置技能不可置顶</span>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Tab: Install -->
          <div v-if="currentToolboxTab === 'install'" class="py-4">
            <div class="max-w-md mx-auto">
              <div class="bg-indigo-600/10 border border-indigo-500/20 rounded-2xl p-6 mb-6">
                <h4 class="text-sm font-semibold text-white mb-2">从互联网安装技能</h4>
                <p class="text-xs text-gray-400 leading-relaxed mb-4">
                  你可以从 GitHub 仓库、skills.sh 平台或其他技能源安装。请输入技能名称或仓库地址。
                </p>
                <div class="flex gap-2">
                  <input 
                    v-model="skillToInstall"
                    @keydown.enter="installSkill()"
                    placeholder="例如: google-workspace, obsidian..."
                    class="flex-1 bg-gray-950 border border-gray-800 rounded-xl px-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-indigo-500/50 transition-colors"
                    :disabled="isInstallingSkill"
                  />
                  <button
                    @click="installSkill()"
                    :disabled="!skillToInstall.trim() || isInstallingSkill"
                    class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-800 disabled:text-gray-500 text-white rounded-xl text-sm font-medium transition-colors flex items-center gap-2"
                  >
                    <Loader2 v-if="isInstallingSkill" class="w-4 h-4 animate-spin" />
                    <span v-else>安装</span>
                  </button>
                </div>
              </div>
              
              <div class="space-y-4 px-2">
                <h5 class="text-[10px] font-bold text-gray-500 uppercase tracking-widest">推荐尝试</h5>
                <div class="grid grid-cols-2 gap-3">
                  <button 
                    @click="skillToInstall = 'google-workspace'"
                    class="text-left p-3 rounded-xl border border-gray-800 hover:border-gray-700 bg-gray-900/50 transition-all group"
                  >
                    <div class="text-xs font-medium text-gray-300 group-hover:text-white">Google Workspace</div>
                    <div class="text-[10px] text-gray-600">处理文档与邮件</div>
                  </button>
                  <button 
                    @click="skillToInstall = 'obsidian'"
                    class="text-left p-3 rounded-xl border border-gray-800 hover:border-gray-700 bg-gray-900/50 transition-all group"
                  >
                    <div class="text-xs font-medium text-gray-300 group-hover:text-white">Obsidian</div>
                    <div class="text-[10px] text-gray-600">同步笔记内容</div>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div class="p-4 border-t border-gray-800 bg-gray-950/50 rounded-b-2xl">
          <div class="flex items-center gap-2 text-[10px] text-gray-500 italic">
            <Info class="w-3 h-3" />
            开启开关后，工具将以 !keyword 形式出现在输入框上方的快捷栏中。
          </div>
        </div>
      </div>
    </div>

  </div>
</template>
