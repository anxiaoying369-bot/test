import { ref, nextTick, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface ToolCall {
  name: string;
  args: Record<string, any>;
  status?: 'executed' | 'pending_confirmation';
  /** Phase 3 多动作时，每个 call 对应的 confirmation_id */
  confirmation_id?: string;
}

export interface ToolData {
  content?: string;
  audit?: string;
  platform?: string;
  topic?: string;
  calls?: ToolCall[];
  /** Phase 3 human-in-the-loop: 等待用户确认的 confirmation_id（单数，向后兼容） */
  pending_confirmation_id?: string;
  /** Phase 3 多动作场景：所有待确认 id 列表 */
  pending_confirmation_ids?: string[];
  /** 多动作场景下的"主 id"（用于前端路由） */
  primary_confirmation_id?: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  tool_used?: string;
  tool_data?: ToolData;
}

/** Phase 3 待用户确认的工具调用。前端监听 pendingConfirmation.value 自动弹窗。 */
export interface PendingConfirmation {
  /** 1 个或多个 confirmation_id。一次会话允许多个动作同时触发确认。 */
  confirmationIds: string[];
  /** 第一个工具名（向后兼容旧 UI）。 */
  toolName: string;
  /** 每个 id 对应的 call 详情。 */
  calls: (ToolCall & { confirmation_id?: string })[];
  /** assistant 消息的时间戳，用于高亮对应气泡 */
  messageTimestamp: number;
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

/** Phase 3：当前等待用户确认的工具调用。UI 监听这个 ref 显示弹窗。 */
const pendingConfirmation = ref<PendingConfirmation | null>(null);

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

      // Phase 3：检测到 pending_confirmation → 触发确认弹窗
      // 支持多动作：pending_confirmation_ids（数组）或向后兼容 pending_confirmation_id（单数）
      const td = reply.tool_data;
      if (td) {
        const pendingIds: string[] =
          (td.pending_confirmation_ids as string[] | undefined) ||
          (td.pending_confirmation_id ? [td.pending_confirmation_id] : []);
        if (pendingIds.length > 0) {
          const calls = td.calls || [];
          // 把每个 pending_id 映射到对应的 call
          const pendingCalls = pendingIds.map(id => {
            const c = calls.find(c => c.confirmation_id === id) || calls.find(c => c.name === reply.tool_used);
            return c || { name: reply.tool_used || '?', args: {}, confirmation_id: id };
          });
          pendingConfirmation.value = {
            confirmationIds: pendingIds,
            toolName: pendingCalls[0]?.name || '?',
            calls: pendingCalls,
            messageTimestamp: reply.timestamp,
          };
        }
      }
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

  /**
   * Phase 3：用户点了「允许」。一次性确认所有动作（如果有多个 confirmation_id）。
   * 调 confirm_tool_execution 让 Rust 真正执行工具，把结果作为新消息插入会话。
   */
  async function confirmTool() {
    const pending = pendingConfirmation.value;
    if (!pending) return;
    pendingConfirmation.value = null;
    isSending.value = true;
    try {
      // 一次性把所有 confirmation_id 交给后端批量执行；
      // 后端真正执行动作、按开关决定是否 LLM 总结，并已把结果持久化到会话文件，
      // 直接返回这条（已落库的）assistant 消息。
      const reply = await invoke('confirm_tool_execution', {
        confirmationIds: pending.confirmationIds,
      }) as ChatMessage;
      messages.value.push(reply);
      await scrollToBottom();
      await loadSessions();
    } catch (e) {
      console.error('确认执行失败:', e);
      messages.value.push({
        role: 'system',
        content: '❌ 执行失败: ' + e,
        timestamp: Date.now() / 1000,
      });
    } finally {
      isSending.value = false;
    }
  }

  /**
   * Phase 3：用户点了「拒绝」。一次性取消所有动作。
   * 调 cancel_tool_execution 让 Rust 释放资源，前端插入一条取消消息。
   */
  async function cancelTool() {
    const pending = pendingConfirmation.value;
    if (!pending) return;
    pendingConfirmation.value = null;
    try {
      // 批量取消，后端持久化一条取消消息并返回
      const reply = await invoke('cancel_tool_execution', {
        confirmationIds: pending.confirmationIds,
      }) as ChatMessage;
      messages.value.push(reply);
      await scrollToBottom();
      await loadSessions();
    } catch (e) {
      console.error('取消执行失败:', e);
    }
  }

  // 每次页面挂载时刷新会话列表（不重置当前会话和消息）
  onMounted(async () => {
    await loadSessions();
    // 重新进入页面时，单例状态已保留会话与消息，但 DOM 是全新渲染、滚动位置默认在顶部。
    // 主动回到底部展示最新消息；再用 rAF 兜底一次，等 markdown(v-html) 渲染撑开高度后再滚。
    await scrollToBottom();
    requestAnimationFrame(() => { scrollToBottom(); });
  });

  return {
    sessions, currentSessionId, messages, userInput, isSending, scrollContainer, expandedAudits,
    pendingConfirmation,
    loadSessions, createNewSession, selectSession, deleteSession, sendMessage, toggleAudit, copyToClipboard,
    confirmTool, cancelTool,
  };
}
