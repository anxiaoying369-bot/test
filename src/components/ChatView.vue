<script setup lang="ts">
import {
  MessageSquare, Plus, Trash2, Send,
  Bot, User, Sparkles, Copy, ChevronDown, BarChart3
} from 'lucide-vue-next';
import { marked } from 'marked';
import { inject } from 'vue';
import { useChat } from '../composables/useChat';

const {
  sessions, currentSessionId, messages, userInput, isSending, scrollContainer, expandedAudits,
  createNewSession, selectSession, deleteSession, sendMessage, toggleAudit, copyToClipboard,
} = useChat();

const navigateTo = inject('navigateTo') as (page: string, tab?: string) => void;
</script>

<template>
  <div class="flex h-full bg-gray-950 text-gray-100 overflow-hidden">
    <!-- 左侧：会话列表 -->
    <aside class="w-64 border-r border-gray-800 bg-gray-900/30 flex flex-col flex-shrink-0">
      <div class="p-4 border-b border-gray-800 flex items-center justify-between">
        <h2 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
          <MessageSquare class="w-4 h-4" /> 聊天记录
        </h2>
        <button
          @click="createNewSession"
          class="p-1.5 hover:bg-gray-800 rounded-lg text-gray-400 hover:text-white transition-colors"
          title="新对话"
        >
          <Plus class="w-4 h-4" />
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar">
        <div
          v-for="s in sessions"
          :key="s.id"
          @click="selectSession(s.id)"
          :class="['group flex items-center justify-between p-3 rounded-xl cursor-pointer transition-all',
                    currentSessionId === s.id ? 'bg-blue-600/10 border border-blue-500/50' : 'hover:bg-gray-800 border border-transparent']"
        >
          <div class="flex items-center gap-3 min-w-0">
            <MessageSquare :class="['w-4 h-4 flex-shrink-0', currentSessionId === s.id ? 'text-blue-400' : 'text-gray-500']" />
            <span :class="['text-sm truncate', currentSessionId === s.id ? 'text-white font-medium' : 'text-gray-400 group-hover:text-gray-200']">
              {{ s.title }}
            </span>
          </div>
          <button
            @click="deleteSession(s.id, $event)"
            class="p-1 opacity-0 group-hover:opacity-100 hover:text-red-400 transition-all"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </aside>

    <!-- 右侧：聊天窗口 -->
    <main class="flex-1 flex flex-col min-w-0 relative">
      <!-- 消息列表 -->
      <div
        ref="scrollContainer"
        class="flex-1 overflow-y-auto p-6 space-y-8 custom-scrollbar"
      >
        <div v-if="!currentSessionId" class="h-full flex flex-col items-center justify-center text-gray-600 space-y-4">
          <MessageSquare class="w-12 h-12 opacity-10" />
          <p class="text-sm">选择一个会话或开启新对话</p>
        </div>

        <template v-else>
          <div
            v-for="msg in messages"
            :key="msg.timestamp"
            :class="['flex gap-4 max-w-4xl mx-auto', msg.role === 'user' ? 'justify-end' : 'justify-start']"
          >
            <!-- 机器人头像 -->
            <div v-if="msg.role !== 'user'" class="flex-shrink-0">
              <div :class="['w-8 h-8 rounded-xl flex items-center justify-center', msg.role === 'assistant' ? 'bg-blue-600' : 'bg-gray-800']">
                <Bot v-if="msg.role === 'assistant'" class="w-5 h-5 text-white" />
                <span v-else class="text-xs text-gray-500">S</span>
              </div>
            </div>

            <!-- 消息主体 -->
            <div :class="['flex flex-col space-y-2 max-w-[85%] min-w-0', msg.role === 'user' ? 'items-end' : 'items-start']">
              <!-- 消息气泡 -->
              <div
                :class="['p-4 rounded-2xl text-sm leading-relaxed border shadow-sm overflow-x-auto break-words w-full',
                          msg.role === 'user' ? 'bg-blue-600 border-blue-500 text-white' : 
                          msg.role === 'system' ? 'bg-amber-950/20 border-amber-900/50 text-amber-200/80' :
                          'bg-gray-900 border-gray-800 text-gray-200']"
              >
                <div class="markdown-content break-words [word-break:break-word]" v-html="marked(msg.content)"></div>
                
                <!-- 引导配置按钮 -->
                <div v-if="msg.role === 'system' && msg.content.includes('未配置 LLM API Key')" class="mt-3">
                  <button 
                    @click="navigateTo('settings', 'llm')"
                    class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white text-xs font-bold rounded-lg transition-colors shadow-lg shadow-amber-900/20"
                  >
                    立即前往设置
                  </button>
                </div>

                <!-- 工具调用标记 -->
                <div v-if="msg.tool_used" class="mt-4 pt-3 border-t border-gray-800 flex flex-col gap-2">
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2 text-[10px] font-bold text-gray-500 uppercase tracking-widest">
                      <Sparkles class="w-3 h-3 text-purple-400" />
                      Executed Tool: {{ msg.tool_used }}
                    </div>
                    <button
                      v-if="msg.tool_data?.audit"
                      @click="toggleAudit(msg.timestamp)"
                      class="flex items-center gap-1 text-[10px] text-blue-400 hover:text-blue-300 transition-colors"
                    >
                      <BarChart3 class="w-3 h-3" />
                      {{ expandedAudits.has(msg.timestamp) ? '隐藏审计报告' : '查看审计报告' }}
                      <ChevronDown :class="['w-3 h-3 transition-transform', expandedAudits.has(msg.timestamp) ? 'rotate-180' : '']" />
                    </button>
                  </div>

                  <!-- 审计报告内容 -->
                  <div v-if="expandedAudits.has(msg.timestamp) && msg.tool_data?.audit" 
                       class="mt-2 p-3 bg-gray-950 rounded-xl border border-gray-800 text-[11px] text-gray-400 font-mono leading-relaxed animate-in fade-in slide-in-from-top-1">
                    <div class="markdown-content break-words [word-break:break-word]" v-html="marked(msg.tool_data.audit)"></div>
                  </div>
                </div>
              </div>

              <!-- 底部操作 -->
              <div class="flex items-center gap-3 px-1">
                <span class="text-[10px] text-gray-600">{{ new Date(msg.timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
                <button
                  v-if="msg.role === 'assistant'"
                  @click="copyToClipboard(msg.content)"
                  class="text-[10px] text-gray-600 hover:text-blue-400 transition-colors flex items-center gap-1"
                >
                  <Copy class="w-2.5 h-2.5" /> 复制
                </button>
              </div>
            </div>

            <!-- 用户头像 -->
            <div v-if="msg.role === 'user'" class="flex-shrink-0">
              <div class="w-8 h-8 rounded-xl bg-gray-800 flex items-center justify-center border border-gray-700">
                <User class="w-5 h-5 text-gray-400" />
              </div>
            </div>
          </div>
        </template>

        <!-- 发送中状态 -->
        <div v-if="isSending" class="flex gap-4 max-w-4xl mx-auto">
          <div class="flex-shrink-0">
            <div class="w-8 h-8 rounded-xl bg-blue-600 flex items-center justify-center">
              <Bot class="w-5 h-5 text-white" />
            </div>
          </div>
          <div class="bg-gray-900 border border-gray-800 p-4 rounded-2xl flex items-center gap-2">
            <div class="flex gap-1">
              <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce"></div>
              <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 0.2s"></div>
              <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 0.4s"></div>
            </div>
            <span class="text-xs text-gray-500 ml-1 italic font-serif">AI 正在思考并创作中...</span>
          </div>
        </div>
      </div>

      <!-- 输入区域 -->
      <div class="p-6 bg-gradient-to-t from-gray-950 via-gray-950 to-transparent">
        <div class="max-w-4xl mx-auto relative group">
          <textarea
            v-model="userInput"
            @keydown.enter.prevent="sendMessage"
            :disabled="isSending || !currentSessionId"
            rows="1"
            placeholder="向 AI 提问，它将结合企业知识库为你解答..."
            class="w-full bg-gray-900 border border-gray-800 rounded-2xl px-6 py-4 pr-16 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all resize-none shadow-2xl disabled:opacity-50"
          ></textarea>
          <button
            @click="sendMessage"
            :disabled="!userInput.trim() || isSending || !currentSessionId"
            class="absolute right-3 bottom-3 p-2 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-800 text-white rounded-xl transition-all shadow-lg shadow-blue-900/40"
          >
            <Send class="w-5 h-5" />
          </button>
        </div>
        <p class="text-[10px] text-center text-gray-600 mt-3 uppercase tracking-tighter">
          Power by AutoCast AI Engine • Knowledge Base Integration Enabled
        </p>
      </div>
    </main>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #374151;
}

/* Markdown 样式优化 */
.markdown-content :deep(p) { margin-bottom: 1rem; line-height: 1.75; }
.markdown-content :deep(p:last-child) { margin-bottom: 0; }
.markdown-content :deep(h1), .markdown-content :deep(h2), .markdown-content :deep(h3) { 
  color: #fff; margin-top: 1.5rem; margin-bottom: 0.75rem; font-weight: 700;
}
.markdown-content :deep(h1) { font-size: 1.25rem; border-bottom: 1px solid #374151; padding-bottom: 0.5rem; }
.markdown-content :deep(h2) { font-size: 1.1rem; }
.markdown-content :deep(h3) { font-size: 1rem; }

.markdown-content :deep(ul), .markdown-content :deep(ol) { margin-bottom: 1rem; padding-left: 1.5rem; }
.markdown-content :deep(li) { margin-bottom: 0.25rem; }

.markdown-content :deep(code) {
  background: #1f2937; padding: 0.2rem 0.4rem; border-radius: 0.375rem; 
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.9em; color: #60a5fa;
}
.markdown-content :deep(pre) { 
  background: #0a0a0a !important; border: 1px solid #1f2937 !important;
  border-radius: 0.75rem !important; margin: 1.25rem 0 !important;
  padding: 1rem !important;
}
.markdown-content :deep(pre code) { background: transparent; padding: 0; color: inherit; font-size: 0.85rem; }

/* 表格优化 */
.markdown-content :deep(table) {
  width: 100%; border-collapse: collapse; margin: 1rem 0; font-size: 0.85rem;
  background: #111827; border-radius: 0.75rem; overflow: hidden;
  border: 1px solid #374151;
}
.markdown-content :deep(th) {
  background: #1f2937; color: #9ca3af; font-weight: 600; text-align: left;
  padding: 0.75rem; border: 1px solid #374151;
}
.markdown-content :deep(td) {
  padding: 0.75rem; border: 1px solid #374151; color: #d1d5db;
}
.markdown-content :deep(tr:nth-child(even)) { background: rgba(31, 41, 55, 0.3); }

.markdown-content :deep(blockquote) {
  border-left: 4px solid #3b82f6; padding: 0.75rem 1rem; color: #9ca3af;
  font-style: italic; margin: 1rem 0; background: rgba(59, 130, 246, 0.05);
  border-top-right-radius: 0.5rem; border-bottom-right-radius: 0.5rem;
}

@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes slide-in { from { transform: translateY(4px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
.animate-in { animation: slide-in 0.3s ease-out; }
</style>
