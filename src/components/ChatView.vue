<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Plus, Trash2, Send, Bot, User, RefreshCw } from 'lucide-vue-next';
import { marked } from 'marked';

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
}

interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  created_at: number;
  updated_at: number;
}

const sessions = ref<ChatSession[]>([]);
const currentSessionId = ref<string | null>(null);
const messages = ref<ChatMessage[]>([]);
const userInput = ref('');
const isSending = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);

async function loadSessions() {
  try {
    const res = await invoke('list_chat_sessions') as ChatSession[];
    sessions.value = res;
    if (res.length > 0 && !currentSessionId.value) {
      selectSession(res[0].id);
    }
  } catch (e) {
    console.error('加载回话失败:', e);
  }
}

async function createNewChat() {
  try {
    const session = await invoke('create_chat_session', { title: '新对话' }) as ChatSession;
    sessions.value.unshift(session);
    selectSession(session.id);
  } catch (e) {
    console.error('创建回话失败:', e);
  }
}

async function selectSession(id: string) {
  currentSessionId.value = id;
  try {
    messages.value = await invoke('get_chat_messages', { sessionId: id }) as ChatMessage[];
    scrollToBottom();
  } catch (e) {
    console.error('加载消息失败:', e);
  }
}

async function deleteSession(id: string, event: Event) {
  event.stopPropagation();
  try {
    await invoke('delete_chat_session', { id });
    sessions.value = sessions.value.filter(s => s.id !== id);
    if (currentSessionId.value === id) {
      currentSessionId.value = sessions.value.length > 0 ? sessions.value[0].id : null;
      if (currentSessionId.value) {
        selectSession(currentSessionId.value);
      } else {
        messages.value = [];
      }
    }
  } catch (e) {
    console.error('删除回话失败:', e);
  }
}

async function sendMessage() {
  if (!userInput.value.trim() || isSending.value || !currentSessionId.value) return;

  const content = userInput.value;
  userInput.value = '';
  isSending.value = true;

  // 立即在 UI 上显示用户消息
  messages.value.push({
    role: 'user',
    content,
    timestamp: Math.floor(Date.now() / 1000)
  });
  scrollToBottom();

  try {
    const assistantMsg = await invoke('send_chat_message', {
      sessionId: currentSessionId.value,
      content
    }) as ChatMessage;
    
    messages.value.push(assistantMsg);
    // 更新会话列表中的标题和顺序
    await loadSessions();
    scrollToBottom();
  } catch (e) {
    console.error('发送消息失败:', e);
    messages.value.push({
      role: 'assistant',
      content: `错误: ${e}`,
      timestamp: Math.floor(Date.now() / 1000)
    });
  } finally {
    isSending.value = false;
  }
}

function scrollToBottom() {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
    }
  });
}

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function renderMarkdown(content: string) {
  return marked(content);
}

onMounted(() => {
  loadSessions();
});

watch(messages, () => {
  scrollToBottom();
}, { deep: true });
</script>

<template>
  <div class="flex h-full bg-gray-950 overflow-hidden">
    <!-- 会话列表侧边栏 -->
    <aside class="w-64 border-r border-gray-800 flex flex-col bg-gray-950">
      <div class="p-4">
        <button
          @click="createNewChat"
          class="w-full flex items-center justify-center gap-2 bg-gray-900 hover:bg-gray-800 border border-gray-800 text-gray-200 py-2 rounded-lg transition-colors text-sm font-medium"
        >
          <Plus class="w-4 h-4" />
          新建对话
        </button>
      </div>

      <div class="flex-1 overflow-y-auto px-2 space-y-1">
        <div
          v-for="s in sessions"
          :key="s.id"
          @click="selectSession(s.id)"
          :class="[
            'group flex items-center justify-between px-3 py-2.5 rounded-lg cursor-pointer transition-colors text-sm',
            currentSessionId === s.id ? 'bg-gray-900 text-white' : 'text-gray-400 hover:bg-gray-900/50 hover:text-gray-200'
          ]"
        >
          <div class="flex items-center gap-2 truncate">
            <MessageSquare class="w-4 h-4 flex-shrink-0" />
            <span class="truncate">{{ s.title }}</span>
          </div>
          <button
            @click="deleteSession(s.id, $event)"
            class="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 transition-all"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </aside>

    <!-- 聊天区域 -->
    <main class="flex-1 flex flex-col bg-gray-950 relative">
      <!-- 顶部状态栏 -->
      <div v-if="currentSessionId" class="h-14 border-b border-gray-900 flex items-center px-6 bg-gray-950/50 backdrop-blur-md sticky top-0 z-10">
        <div class="flex items-center gap-3">
          <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
          <span class="text-sm font-medium text-gray-200">{{ sessions.find(s => s.id === currentSessionId)?.title }}</span>
        </div>
      </div>

      <div v-if="!currentSessionId" class="flex-1 flex flex-col items-center justify-center text-gray-500 space-y-4">
        <div class="w-20 h-20 rounded-3xl bg-gray-900 flex items-center justify-center mb-2">
          <Bot class="w-10 h-10 text-gray-700" />
        </div>
        <h3 class="text-gray-300 font-medium text-lg">AutoCast AI 助理</h3>
        <p class="text-sm">选择一个对话，开启高效工作</p>
        <button @click="createNewChat" class="mt-4 px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-full text-sm transition-all shadow-lg shadow-blue-900/20">
          开启新对话
        </button>
      </div>

      <template v-else>
        <!-- 消息列表 -->
        <div
          ref="scrollContainer"
          class="flex-1 overflow-y-auto p-4 md:p-8 space-y-8 scroll-smooth"
        >
          <div
            v-for="(msg, idx) in messages"
            :key="idx"
            :class="['flex gap-4 max-w-3xl mx-auto group', msg.role === 'user' ? 'flex-row-reverse' : '']"
          >
            <!-- 头像 -->
            <div
              :class="[
                'w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 shadow-sm transition-transform group-hover:scale-105',
                msg.role === 'user' ? 'bg-blue-600' : 'bg-gray-800'
              ]"
            >
              <User v-if="msg.role === 'user'" class="w-5 h-5 text-white" />
              <Bot v-else class="w-5 h-5 text-blue-400" />
            </div>

            <div :class="['flex flex-col space-y-2', msg.role === 'user' ? 'items-end max-w-[65%]' : 'max-w-[80%] md:max-w-[70%]']">
              <div
                v-if="msg.role === 'user'"
                class="px-4 py-3 rounded-2xl text-[14px] leading-relaxed whitespace-pre-wrap break-all shadow-sm bg-blue-600 text-white rounded-tr-none w-fit"
              >
                {{ msg.content }}
              </div>
              <div
                v-else
                class="markdown-content px-4 py-3 rounded-2xl text-[14px] leading-relaxed break-words shadow-sm bg-gray-900 text-gray-200 rounded-tl-none border border-gray-800"
                v-html="renderMarkdown(msg.content)"
              ></div>
              <div class="flex items-center gap-2 px-1">
                <span class="text-[11px] text-gray-600 font-mono">{{ formatTime(msg.timestamp) }}</span>
                <span v-if="msg.role === 'assistant'" class="text-[10px] text-gray-700 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-800">AI</span>
              </div>
            </div>
          </div>

          <div v-if="isSending" class="flex gap-4 max-w-3xl mx-auto">
            <div class="w-9 h-9 rounded-xl bg-gray-800 flex items-center justify-center flex-shrink-0">
              <Bot class="w-5 h-5 text-blue-400" />
            </div>
            <div class="bg-gray-900 border border-gray-800 rounded-2xl rounded-tl-none px-5 py-3">
              <div class="flex gap-1">
                <div class="w-1.5 h-1.5 bg-gray-600 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                <div class="w-1.5 h-1.5 bg-gray-600 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                <div class="w-1.5 h-1.5 bg-gray-600 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
              </div>
            </div>
          </div>
          
          <!-- 占位底部，确保最后一条消息不被遮挡 -->
          <div class="h-4"></div>
        </div>

        <!-- 输入区域 -->
        <div class="p-4 md:p-8 bg-gradient-to-t from-gray-950 via-gray-950 to-transparent">
          <div class="max-w-3xl mx-auto bg-gray-900/50 backdrop-blur-xl border border-gray-800 rounded-2xl p-2 pr-3 shadow-2xl focus-within:border-blue-500/50 focus-within:ring-1 focus-within:ring-blue-500/20 transition-all">
            <div class="flex items-end gap-2">
              <textarea
                v-model="userInput"
                @keydown.enter.prevent="sendMessage"
                placeholder="询问任何问题..."
                rows="1"
                class="flex-1 bg-transparent border-none py-3 px-4 text-sm text-white placeholder-gray-500 focus:outline-none resize-none max-h-40"
                @input="(e) => {
                  const target = e.target as HTMLTextAreaElement;
                  target.style.height = 'auto';
                  target.style.height = target.scrollHeight + 'px';
                }"
              ></textarea>
              <button
                @click="sendMessage"
                :disabled="!userInput.trim() || isSending"
                :class="[
                  'p-2.5 rounded-xl transition-all mb-1 flex items-center justify-center',
                  userInput.trim() && !isSending 
                    ? 'bg-blue-600 text-white shadow-lg shadow-blue-900/40 hover:scale-105 active:scale-95' 
                    : 'bg-gray-800 text-gray-600'
                ]"
              >
                <Send class="w-5 h-5" />
              </button>
            </div>
          </div>
          <div class="flex items-center justify-center gap-4 mt-4 opacity-30">
             <div class="h-[1px] w-12 bg-gray-600"></div>
             <p class="text-[10px] text-gray-500 uppercase tracking-widest font-medium">AutoCast AI Assistant</p>
             <div class="h-[1px] w-12 bg-gray-600"></div>
          </div>
        </div>
      </template>
    </main>
  </div>
</template>

<style scoped>
/* 自定义滚动条 */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
::-webkit-scrollbar-thumb:hover {
  background: #374151;
}

/* Markdown 样式覆盖 */
:deep(.markdown-content) {
  word-break: break-word;
}
:deep(.markdown-content) p {
  margin-bottom: 0.75rem;
}
:deep(.markdown-content) p:last-child {
  margin-bottom: 0;
}
:deep(.markdown-content) h1,
:deep(.markdown-content) h2,
:deep(.markdown-content) h3 {
  color: #f3f4f6;
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  font-weight: 600;
  line-height: 1.25;
}
:deep(.markdown-content) h1 { font-size: 1.15rem; }
:deep(.markdown-content) h2 { font-size: 1.05rem; }
:deep(.markdown-content) h3 { font-size: 1rem; }

:deep(.markdown-content) ul, 
:deep(.markdown-content) ol { 
  margin-bottom: 0.75rem; 
  padding-left: 1.25rem; 
}
:deep(.markdown-content) li { 
  margin-bottom: 0.25rem; 
}
:deep(.markdown-content) strong { 
  color: #60a5fa; 
  font-weight: 600; 
}
:deep(.markdown-content) a {
  color: #3b82f6;
  text-decoration: underline;
  text-underline-offset: 2px;
}
:deep(.markdown-content) blockquote {
  border-left: 3px solid #3b82f6;
  padding-left: 0.75rem;
  font-style: italic;
  color: #9ca3af;
  margin: 0.75rem 0;
}
:deep(.markdown-content) code {
  background-color: #1f2937;
  padding: 0.15rem 0.3rem;
  border-radius: 0.25rem;
  font-family: monospace;
  font-size: 0.85em;
  color: #e5e7eb;
}
:deep(.markdown-content) pre {
  background-color: #030712;
  padding: 1rem;
  border-radius: 0.75rem;
  margin: 0.75rem 0;
  overflow-x: auto;
  border: 1px solid #1f2937;
}
:deep(.markdown-content) pre code {
  background-color: transparent;
  padding: 0;
  border-radius: 0;
  font-size: 0.85rem;
}
</style>
