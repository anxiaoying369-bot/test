import { ref, nextTick, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface ToolData {
  content?: string;
  audit?: string;
  platform?: string;
  topic?: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  tool_used?: string;
  tool_data?: ToolData;
}

export interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  created_at: number;
  updated_at: number;
}

// ---------- 模块级单例状态（切换页面后不会丢失）----------
const sessions = ref<ChatSession[]>([]);
const currentSessionId = ref<string | null>(null);
const messages = ref<ChatMessage[]>([]);
const userInput = ref('');
const isSending = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);
const expandedAudits = ref<Set<number>>(new Set());

export function useChat() {
  async function loadSessions() {
    try {
      const res = await invoke('list_chat_sessions') as ChatSession[];
      sessions.value = res;
      if (res.length > 0 && !currentSessionId.value) {
        await selectSession(res[0].id);
      }
    } catch (e) {
      console.error('加载会话失败:', e);
    }
  }

  async function createNewSession() {
    try {
      const session = await invoke('create_chat_session', { title: '新对话' }) as ChatSession;
      await loadSessions();
      await selectSession(session.id);
    } catch (e) {
      console.error('创建会话失败:', e);
    }
  }

  async function selectSession(id: string) {
    currentSessionId.value = id;
    try {
      const msgs = await invoke('get_chat_messages', { sessionId: id }) as ChatMessage[];
      messages.value = msgs;
      await scrollToBottom();
    } catch (e) {
      console.error('获取消息失败:', e);
    }
  }

  async function deleteSession(id: string, event: Event) {
    event.stopPropagation();
    if (!confirm('确定要删除此会话吗？')) return;
    try {
      await invoke('delete_chat_session', { sessionId: id });
      if (currentSessionId.value === id) {
        currentSessionId.value = null;
        messages.value = [];
      }
      await loadSessions();
    } catch (e) {
      console.error('删除会话失败:', e);
    }
  }

  async function sendMessage() {
    if (!userInput.value.trim() || isSending.value || !currentSessionId.value) return;

    try {
      const config: any = await invoke('get_config');
      if (!config.llm?.api_key || config.llm.api_key.trim() === '') {
        messages.value.push({
          role: 'system',
          content: '⚠️ **未配置 LLM API Key**\n\n当前尚未设置 AI 模型密钥，无法开始对话。请前往"系统设置" -> "AI 模型设置"完成配置。',
          timestamp: Date.now() / 1000,
        });
        userInput.value = '';
        return;
      }
    } catch (e) {
      console.error('检查配置失败:', e);
    }

    const content = userInput.value;
    userInput.value = '';
    isSending.value = true;

    // 乐观更新
    messages.value.push({
      role: 'user',
      content,
      timestamp: Date.now() / 1000,
    });
    await scrollToBottom();

    // 注意：invoke 返回的 Promise 持有对模块级 ref 的引用，
    // 即使用户切换了页面，当 Rust 后端完成后仍会正确更新这些 ref。
    try {
      const reply = await invoke('send_chat_message', {
        sessionId: currentSessionId.value,
        content: content,
      }) as ChatMessage;
      messages.value.push(reply);
      await scrollToBottom();
      await loadSessions();
    } catch (e) {
      console.error('发送消息失败:', e);
      messages.value.push({
        role: 'system',
        content: '错误: ' + e,
        timestamp: Date.now() / 1000,
      });
    } finally {
      isSending.value = false;
    }
  }

  async function scrollToBottom() {
    await nextTick();
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
    }
  }

  function toggleAudit(timestamp: number) {
    if (expandedAudits.value.has(timestamp)) {
      expandedAudits.value.delete(timestamp);
    } else {
      expandedAudits.value.add(timestamp);
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      console.error('复制失败:', e);
    }
  }

  // 每次页面挂载时刷新会话列表（不重置当前会话和消息）
  onMounted(loadSessions);

  return {
    sessions, currentSessionId, messages, userInput, isSending, scrollContainer, expandedAudits,
    loadSessions, createNewSession, selectSession, deleteSession, sendMessage, toggleAudit, copyToClipboard,
  };
}
