<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import { 
  Send, User, Bot, Wrench, Loader2, Zap 
} from 'lucide-vue-next';
import { marked } from 'marked';
import type { Session } from '../../types/hermes';

const props = defineProps<{
  session: Session | null;
  isSending: boolean;
  streamingContent: string;
  streamingThinking: string;
  activeRunId: string | null;
  toolCallProgress: any | null;
}>();

const emit = defineEmits<{
  (e: 'send', content: string): void;
  (e: 'approveRun', id: string, approved: boolean): void;
  (e: 'stopRun', id: string): void;
}>();

const userInput = ref('');
const chatScrollContainer = ref<HTMLElement | null>(null);

const scrollToBottom = async () => {
  await nextTick();
  if (chatScrollContainer.value) {
    chatScrollContainer.value.scrollTop = chatScrollContainer.value.scrollHeight;
  }
};

watch(() => props.session?.messages.length, scrollToBottom);
watch(() => props.streamingContent, scrollToBottom);

const handleSend = () => {
  if (!userInput.value.trim() || props.isSending) return;
  emit('send', userInput.value);
  userInput.value = '';
};

const renderMarkdown = (content: string) => marked(content);
</script>

<template>
  <div class="flex-1 flex flex-col bg-gray-950 overflow-hidden relative">
    <div v-if="!session" class="flex-1 flex flex-col items-center justify-center p-12 text-center">
      <div class="w-24 h-24 rounded-3xl bg-blue-600/10 flex items-center justify-center mb-8 border border-blue-500/20 shadow-2xl">
        <Bot class="w-12 h-12 text-blue-500" />
      </div>
      <h2 class="text-2xl font-bold text-white mb-3">欢迎来到 Hermes Gateway</h2>
      <p class="text-gray-500 max-w-md leading-relaxed">
        AutoCast AI 的 Agent 枢纽。您可以直接通过对话调用各种专业技能和工具。
      </p>
    </div>

    <template v-else>
      <div ref="chatScrollContainer" class="flex-1 overflow-y-auto p-6 md:p-10 space-y-8 custom-scrollbar scroll-smooth">
        <div v-for="(m, idx) in session.messages" :key="idx" :class="['flex gap-4 md:gap-6 animate-in fade-in slide-in-from-bottom-2 duration-300', m.role === 'user' ? 'flex-row-reverse ml-auto max-w-[85%]' : 'max-w-[90%]']">
          <div :class="['w-9 h-9 md:w-11 md:h-11 rounded-2xl flex items-center justify-center flex-shrink-0 border shadow-lg', m.role === 'user' ? 'bg-blue-600 border-blue-500 text-white' : 'bg-gray-900 border-gray-800 text-blue-400']">
            <User v-if="m.role === 'user'" class="w-5 h-5 md:w-6 md:h-6" />
            <Bot v-else class="w-5 h-5 md:w-6 md:h-6" />
          </div>

          <div class="space-y-2 flex-1 min-w-0">
            <div :class="['p-4 md:p-6 rounded-3xl shadow-xl leading-relaxed break-words overflow-x-auto', m.role === 'user' ? 'bg-blue-600/10 border border-blue-500/20 text-blue-50 rounded-tr-none' : 'bg-gray-900 border border-gray-800 text-gray-200 rounded-tl-none']">
              <div v-if="m.role === 'thought'" class="flex items-center gap-2 mb-3 text-purple-400 font-bold text-xs uppercase tracking-widest border-b border-purple-500/20 pb-2">
                <Zap class="w-3.5 h-3.5" /> Thinking Process
              </div>
              <div v-if="m.role === 'tool'" class="flex items-center gap-2 mb-3 text-amber-400 font-bold text-xs uppercase tracking-widest border-b border-amber-500/20 pb-2">
                <Wrench class="w-3.5 h-3.5" /> Tool Action: {{ m.toolName }}
              </div>
              
              <div class="markdown-content prose prose-invert prose-sm md:prose-base max-w-none break-words [word-break:break-word]" v-html="renderMarkdown(m.content)" />
            </div>
            <div :class="['text-[10px] text-gray-600 font-mono px-2', m.role === 'user' ? 'text-right' : '']">
              {{ new Date(m.timestamp).toLocaleTimeString() }}
            </div>
          </div>
        </div>

        <div v-if="streamingThinking" class="flex gap-4 md:gap-6 animate-pulse">
           <div class="w-9 h-9 md:w-11 md:h-11 rounded-2xl bg-gray-900 border border-purple-500/30 text-purple-400 flex items-center justify-center flex-shrink-0 shadow-lg">
             <Zap class="w-5 h-5 animate-pulse" />
           </div>
           <div class="flex-1 space-y-2 min-w-0">
             <div class="bg-gray-900/60 border border-purple-500/20 p-5 rounded-3xl rounded-tl-none italic text-purple-300/80 text-sm leading-relaxed shadow-xl break-words overflow-x-auto">
               <div class="flex items-center gap-2 mb-3 text-purple-400 font-bold text-[10px] uppercase tracking-widest border-b border-purple-500/10 pb-2">Thinking...</div>
               {{ streamingThinking }}
             </div>
           </div>
        </div>

        <div v-if="streamingContent" class="flex gap-4 md:gap-6">
          <div class="w-9 h-9 md:w-11 md:h-11 rounded-2xl bg-gray-900 border border-blue-500/30 text-blue-400 flex items-center justify-center flex-shrink-0 shadow-lg">
            <Loader2 class="w-5 h-5 animate-spin" />
          </div>
          <div class="flex-1 space-y-2 min-w-0">
            <div class="bg-gray-900 border border-blue-500/20 p-5 rounded-3xl rounded-tl-none text-gray-200 leading-relaxed shadow-xl break-words overflow-x-auto">
              <div class="markdown-content prose prose-invert prose-sm md:prose-base max-w-none break-words [word-break:break-word]" v-html="renderMarkdown(streamingContent)" />
            </div>
          </div>
        </div>
      </div>

      <div class="p-4 md:p-8 bg-gray-950/80 backdrop-blur-xl border-t border-gray-800/50">
        <div v-if="activeRunId" class="mb-6 animate-in slide-in-from-bottom-4 duration-500">
           <div class="max-w-3xl mx-auto bg-amber-950/20 border border-amber-500/30 rounded-2xl p-5 shadow-2xl backdrop-blur-md">
             <div class="flex items-center justify-between gap-4">
               <div class="flex items-center gap-4">
                 <div class="w-12 h-12 rounded-xl bg-amber-500/20 flex items-center justify-center text-amber-500 border border-amber-500/30 shadow-inner">
                   <Wrench class="w-6 h-6 animate-bounce" />
                 </div>
                 <div>
                   <h4 class="text-sm font-bold text-amber-500 mb-1">等待操作授权</h4>
                   <p class="text-xs text-amber-200/60 leading-tight">Agent 正在尝试调用一个需要手动确认的工具。</p>
                 </div>
               </div>
               <div class="flex gap-3">
                 <button @click="emit('stopRun', activeRunId)" class="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs font-bold rounded-xl transition-all border border-gray-700">中止任务</button>
                 <button @click="emit('approveRun', activeRunId, true)" class="px-6 py-2.5 bg-amber-500 hover:bg-amber-400 text-black text-xs font-bold rounded-xl transition-all shadow-lg shadow-amber-900/40">确认授权</button>
               </div>
             </div>
           </div>
        </div>

        <div class="max-w-4xl mx-auto relative group">
          <textarea
            v-model="userInput"
            @keydown.enter.prevent="handleSend"
            :disabled="isSending"
            placeholder="与 Agent 对话，或输入 '/' 触发特定命令..."
            class="w-full bg-gray-900/80 border border-gray-800 rounded-3xl py-5 pl-6 pr-20 text-base text-gray-100 placeholder-gray-600 focus:outline-none focus:border-blue-500/50 focus:ring-4 focus:ring-blue-500/5 transition-all resize-none shadow-2xl min-h-[70px] max-h-48 overflow-y-auto custom-scrollbar"
            rows="1"
          ></textarea>
          <button
            @click="handleSend"
            :disabled="!userInput.trim() || isSending"
            class="absolute right-4 bottom-4 w-12 h-12 rounded-2xl bg-blue-600 hover:bg-blue-500 disabled:opacity-30 disabled:cursor-not-allowed text-white flex items-center justify-center transition-all shadow-xl shadow-blue-900/30 group-hover:scale-105 active:scale-95"
          >
            <Loader2 v-if="isSending" class="w-6 h-6 animate-spin" />
            <Send v-else class="w-6 h-6" />
          </button>
        </div>
      </div>
    </template>
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
@keyframes zoom-in { from { transform: scale(0.95); opacity: 0; } to { transform: scale(1); opacity: 1; } }
.animate-in { animation: fade-in 0.2s ease-out; }
.zoom-in { animation: zoom-in 0.2s ease-out; }
</style>
