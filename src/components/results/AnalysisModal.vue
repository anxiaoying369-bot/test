<script setup lang="ts">
import { Sparkles, X, Wand2 } from 'lucide-vue-next';
import type { ScrapedVideo } from '../../types/scraped';

defineProps<{
  open: boolean;
  analyzing: boolean;
  analyzingVideo: ScrapedVideo | null;
  analysisReport: string;
  renderedReport: string;
}>();

const emit = defineEmits<{ (e: 'close'): void }>();
</script>

<template>
  <div v-if="open" class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-6">
    <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden animate-in fade-in zoom-in duration-200">
      <!-- 弹窗头部 -->
      <header class="p-4 border-b border-gray-800 flex items-center justify-between bg-gray-900/50">
        <div class="flex items-center gap-2 text-blue-400">
          <Sparkles class="w-5 h-5" />
          <h3 class="font-bold">AI 深度分析报告</h3>
        </div>
        <button @click="emit('close')" class="text-gray-500 hover:text-white p-1 hover:bg-gray-800 rounded-full transition-colors">
          <X class="w-5 h-5" />
        </button>
      </header>

      <!-- 弹窗内容 -->
      <div class="flex-1 overflow-y-auto p-6 custom-scrollbar bg-gray-950/30">
        <!-- 正在分析状态 -->
        <div v-if="analyzing" class="flex flex-col items-center justify-center py-20">
          <div class="relative w-16 h-16 mb-6">
            <div class="absolute inset-0 border-4 border-blue-500/20 rounded-full"></div>
            <div class="absolute inset-0 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            <Wand2 class="absolute inset-0 m-auto w-6 h-6 text-blue-400 animate-pulse" />
          </div>
          <p class="text-gray-400 animate-pulse text-sm">正在调动 LLM 分析作品及 {{ analyzingVideo?.comment_count }} 条评论内容...</p>
        </div>

        <!-- 分析结果展示 -->
        <div v-else-if="analysisReport" class="prose prose-invert max-w-none">
          <div
            class="markdown-content text-gray-300 leading-relaxed text-sm font-sans bg-gray-900/50 p-6 rounded-2xl border border-gray-800 shadow-inner"
            v-html="renderedReport"
          ></div>
        </div>
      </div>

      <!-- 弹窗底部 -->
      <footer class="p-4 border-t border-gray-800 bg-gray-900/50 flex justify-end">
        <button @click="emit('close')" class="px-6 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded-xl text-sm font-medium transition-colors">
          返回列表
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #1f2937; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #374151; }

@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes zoom-in { from { transform: scale(0.95); opacity: 0; } to { transform: scale(1); opacity: 1; } }
.animate-in { animation: fade-in 0.2s ease-out; }
.zoom-in { animation: zoom-in 0.2s ease-out; }

/* Markdown 样式覆盖 */
:deep(.markdown-content) h1,
:deep(.markdown-content) h2,
:deep(.markdown-content) h3 {
  color: #f3f4f6;
  margin-top: 1.5rem;
  margin-bottom: 0.75rem;
  font-weight: 600;
}
:deep(.markdown-content) h1 { font-size: 1.25rem; border-bottom: 1px solid #374151; padding-bottom: 0.5rem; }
:deep(.markdown-content) h2 { font-size: 1.1rem; }
:deep(.markdown-content) h3 { font-size: 1rem; }
:deep(.markdown-content) p { margin-bottom: 1rem; }
:deep(.markdown-content) ul,
:deep(.markdown-content) ol { margin-bottom: 1rem; padding-left: 1.25rem; }
:deep(.markdown-content) li { margin-bottom: 0.25rem; }
:deep(.markdown-content) strong { color: #60a5fa; font-weight: 600; }
:deep(.markdown-content) blockquote {
  border-left: 4px solid #3b82f6;
  padding-left: 1rem;
  font-style: italic;
  color: #9ca3af;
  margin: 1rem 0;
}
:deep(.markdown-content) code {
  background-color: #1f2937;
  padding: 0.2rem 0.4rem;
  border-radius: 0.25rem;
  font-family: monospace;
  font-size: 0.875rem;
}
</style>
