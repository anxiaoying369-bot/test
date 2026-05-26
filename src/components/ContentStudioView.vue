<script setup lang="ts">
import { ref, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  Sparkles, FileText, BarChart3,
  RefreshCw, Copy,
  Layout, Zap, Eye, ShieldCheck,
  Wand2, History,
  PanelLeftClose, PanelLeftOpen
} from 'lucide-vue-next';
import { marked } from 'marked';

// ============ 状态 ============

const topic = ref('');
const sourceMaterial = ref('');
const generationMode = ref<'new' | 'rewrite'>('new');
const targetPlatform = ref<'douyin' | 'wechat' | 'zhihu'>('douyin');

const isGenerating = ref(false);
const generatedContent = ref('');
const auditReport = ref('');

// ============ 布局 ============

const containerRef = ref<HTMLElement | null>(null);
const studioWidth = ref(360);
const auditWidth = ref(340);
const isStudioCollapsed = ref(false);
const activeResizer = ref<'left' | 'right' | null>(null);

function startResizing(type: 'left' | 'right', e: MouseEvent) {
  e.preventDefault(); // 阻止文字被选中
  activeResizer.value = type;

  // 拖动期间全局禁止文字选择，并锁定光标样式
  document.body.style.userSelect = 'none';
  document.body.style.webkitUserSelect = 'none';
  document.body.style.cursor = 'col-resize';

  document.addEventListener('mousemove', handleResizing, { passive: true });
  document.addEventListener('mouseup', stopResizing, { once: true });
}

function handleResizing(e: MouseEvent) {
  if (!activeResizer.value || !containerRef.value) return;
  const rect = containerRef.value.getBoundingClientRect();

  if (activeResizer.value === 'left') {
    // 鼠标 X 相对于容器左边沿就是左侧面板宽度
    const newWidth = e.clientX - rect.left;
    studioWidth.value = Math.min(Math.max(newWidth, 260), 560);
  } else {
    // 右侧面板宽度 = 容器右边沿 - 鼠标 X
    const newWidth = rect.right - e.clientX;
    auditWidth.value = Math.min(Math.max(newWidth, 220), 560);
  }
}

function stopResizing() {
  activeResizer.value = null;
  document.removeEventListener('mousemove', handleResizing);
  document.body.style.userSelect = '';
  document.body.style.webkitUserSelect = '';
  document.body.style.cursor = '';
}

onUnmounted(stopResizing);

// ============ 计算属性 ============

const renderedContent = computed(() => marked(generatedContent.value));
const renderedAudit = computed(() => marked(auditReport.value));

// ============ 方法 ============

async function handleGenerate() {
  if (!topic.value.trim() && generationMode.value === 'new') return;
  if (!sourceMaterial.value.trim() && generationMode.value === 'rewrite') return;

  isGenerating.value = true;
  generatedContent.value = '';
  auditReport.value = '';

  try {
    const res = await invoke('studio_generate_content', {
      topic: topic.value,
      material: sourceMaterial.value,
      mode: generationMode.value,
      platform: targetPlatform.value
    }) as { content: string; audit: string };
    generatedContent.value = res.content;
    auditReport.value = res.audit;
  } catch (e: any) {
    generatedContent.value = `## 生成失败\n\n错误信息: ${e}`;
  } finally {
    isGenerating.value = false;
  }
}

function copyContent() {
  navigator.clipboard.writeText(generatedContent.value);
}
</script>

<template>
  <!-- 容器：relative 让展开按钮的 absolute 定位生效 -->
  <div ref="containerRef" class="flex h-full bg-gray-950 overflow-hidden relative">

    <!-- ===== 左侧：工作区设置 ===== -->
    <aside
      class="flex-shrink-0 border-r border-gray-800 bg-gray-900/30 flex flex-col overflow-hidden"
      :class="activeResizer === 'left' ? '' : 'transition-[width] duration-300'"
      :style="{
        width: isStudioCollapsed ? '0px' : studioWidth + 'px',
        minWidth: isStudioCollapsed ? '0px' : undefined,
      }"
    >
      <!-- 内层可滚动容器，宽度跟随父级 aside -->
      <div class="flex flex-col flex-1 overflow-y-auto w-full p-6">
        <!-- 标题栏 -->
        <div class="flex items-center justify-between mb-8 flex-shrink-0">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-purple-500/10 rounded-xl">
              <Sparkles class="w-5 h-5 text-purple-400" />
            </div>
            <h2 class="text-lg font-bold text-white whitespace-nowrap">AI 创作中心</h2>
          </div>
          <button
            @click="isStudioCollapsed = true"
            class="p-1.5 text-gray-500 hover:text-white hover:bg-gray-800 rounded-lg transition-colors flex-shrink-0"
            title="收起"
          >
            <PanelLeftClose class="w-4 h-4" />
          </button>
        </div>

        <div class="space-y-6">
          <!-- 创作模式 -->
          <div class="space-y-2.5">
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider flex items-center gap-1.5">
              <Layout class="w-3 h-3" /> 创作模式
            </label>
            <div class="grid grid-cols-2 gap-1.5 p-1 bg-gray-950 rounded-xl border border-gray-800">
              <button
                @click="generationMode = 'new'"
                :class="['py-2 px-3 rounded-lg text-xs font-medium transition-all', generationMode === 'new' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300']"
              >全新内容生成</button>
              <button
                @click="generationMode = 'rewrite'"
                :class="['py-2 px-3 rounded-lg text-xs font-medium transition-all', generationMode === 'rewrite' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300']"
              >已有内容改造</button>
            </div>
          </div>

          <!-- 目标平台 -->
          <div class="space-y-2.5">
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider flex items-center gap-1.5">
              <Zap class="w-3 h-3" /> 目标平台 (GEO 优化)
            </label>
            <div class="grid grid-cols-3 gap-1.5">
              <button
                v-for="p in (['douyin', 'wechat', 'zhihu'] as const)"
                :key="p"
                @click="targetPlatform = p"
                :class="['py-2 px-2 rounded-xl text-[11px] border transition-all flex flex-col items-center gap-1',
                  targetPlatform === p ? 'bg-blue-600/10 border-blue-500 text-blue-400' : 'bg-gray-950 border-gray-800 text-gray-500 hover:border-gray-700']"
              >
                <span>{{ p === 'douyin' ? '抖音' : p === 'wechat' ? '微信' : '知乎' }}</span>
              </button>
            </div>
          </div>

          <!-- 主题输入 -->
          <div v-if="generationMode === 'new'" class="space-y-2.5">
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">创作主题</label>
            <input
              v-model="topic"
              type="text"
              placeholder="输入你想创作的主题..."
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-purple-500 transition-colors"
            />
          </div>

          <!-- 素材/原文 -->
          <div class="space-y-2.5">
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">
              {{ generationMode === 'new' ? '补充参考素材 (可选)' : '原始内容 (需改造的内容)' }}
            </label>
            <textarea
              v-model="sourceMaterial"
              rows="9"
              :placeholder="generationMode === 'new' ? '粘贴相关链接、评论或大纲...' : '在此粘贴你想进行 GEO 改造或重写的原文...'"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-purple-500 transition-colors resize-none font-sans"
            ></textarea>
          </div>

          <!-- RAG 提示 -->
          <div class="p-3.5 bg-blue-500/5 border border-blue-500/10 rounded-xl">
            <div class="flex items-center gap-2 text-blue-400 mb-1">
              <ShieldCheck class="w-3.5 h-3.5" />
              <span class="text-[11px] font-bold">RAG 知识增强已开启</span>
            </div>
            <p class="text-[10px] text-gray-500 leading-relaxed">系统将自动从企业知识库中检索相关文档作为创作依据，确保内容真实严谨。</p>
          </div>

          <button
            @click="handleGenerate"
            :disabled="isGenerating || (generationMode === 'new' && !topic.trim()) || (generationMode === 'rewrite' && !sourceMaterial.trim())"
            class="w-full bg-gradient-to-r from-purple-600 to-blue-600 hover:from-purple-500 hover:to-blue-500 disabled:opacity-40 disabled:cursor-not-allowed text-white py-3.5 rounded-xl font-bold shadow-xl shadow-purple-900/20 transition-all flex items-center justify-center gap-2"
          >
            <RefreshCw v-if="isGenerating" class="w-4 h-4 animate-spin" />
            <Wand2 v-else class="w-4 h-4" />
            {{ isGenerating ? '正在深度创作与分析...' : '开始智能生成' }}
          </button>
        </div>
      </div>
    </aside>

    <!-- ===== 展开按钮：absolute 定位在容器左边沿，随内容区走 ===== -->
    <button
      v-if="isStudioCollapsed"
      @click="isStudioCollapsed = false"
      class="absolute left-0 top-1/2 -translate-y-1/2 z-40 w-6 h-14 bg-gray-800 hover:bg-purple-600 text-gray-400 hover:text-white rounded-r-xl flex items-center justify-center transition-all shadow-lg border-y border-r border-gray-700/50"
      title="展开侧边栏"
    >
      <PanelLeftOpen class="w-3.5 h-3.5" />
    </button>

    <!-- ===== 左侧拖动条 ===== -->
    <div
      v-if="!isStudioCollapsed"
      class="flex-shrink-0 w-4 cursor-col-resize relative z-20 group"
      @mousedown="startResizing('left', $event)"
    >
      <!-- 细线：hover 或拖动时变紫色 -->
      <div
        class="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px transition-colors"
        :class="activeResizer === 'left' ? 'bg-purple-500' : 'bg-gray-800 group-hover:bg-purple-500/60'"
      ></div>
    </div>

    <!-- ===== 中间：生成预览 ===== -->
    <main class="flex-1 min-w-0 flex flex-col bg-gray-950 overflow-hidden">
      <header class="h-14 border-b border-gray-800 flex-shrink-0 flex items-center justify-between px-6 bg-gray-900/20">
        <div class="flex items-center gap-2 text-sm">
          <FileText class="w-4 h-4 text-gray-400" />
          <span class="text-gray-300 font-medium">生成结果预览</span>
        </div>
        <button
          v-if="generatedContent"
          @click="copyContent"
          class="flex items-center gap-1.5 text-xs text-gray-400 hover:text-white transition-colors bg-gray-800 px-3 py-1.5 rounded-lg border border-gray-700"
        >
          <Copy class="w-3.5 h-3.5" /> 复制正文
        </button>
      </header>

      <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
        <!-- 空态 -->
        <div v-if="!generatedContent && !isGenerating" class="h-full flex flex-col items-center justify-center text-gray-600 space-y-4">
          <div class="w-16 h-16 rounded-3xl bg-gray-900 flex items-center justify-center">
            <Eye class="w-8 h-8 text-gray-800" />
          </div>
          <p class="text-sm">在左侧输入创意，点击生成后在此预览</p>
        </div>

        <!-- 加载中 -->
        <div v-else-if="isGenerating && !generatedContent" class="h-full flex flex-col items-center justify-center space-y-6">
          <div class="relative w-16 h-16">
            <div class="absolute inset-0 border-4 border-purple-500/20 rounded-full"></div>
            <div class="absolute inset-0 border-4 border-purple-500 border-t-transparent rounded-full animate-spin"></div>
          </div>
          <div class="text-center space-y-1.5">
            <p class="text-gray-300 font-medium animate-pulse">正在调动全网 GEO 策略与企业知识库...</p>
            <p class="text-xs text-gray-500">正在生成适应 {{ targetPlatform }} 平台的高价值内容</p>
          </div>
        </div>

        <!-- 内容 -->
        <div v-else class="max-w-3xl mx-auto">
          <div class="prose prose-invert max-w-none">
            <div class="markdown-content text-gray-200 leading-relaxed" v-html="renderedContent"></div>
          </div>
        </div>
      </div>
    </main>

    <!-- ===== 右侧拖动条 ===== -->
    <div
      class="flex-shrink-0 w-4 cursor-col-resize relative z-20 group"
      @mousedown="startResizing('right', $event)"
    >
      <div
        class="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px transition-colors"
        :class="activeResizer === 'right' ? 'bg-purple-500' : 'bg-gray-800 group-hover:bg-purple-500/60'"
      ></div>
    </div>

    <!-- ===== 右侧：AI 审计报告 ===== -->
    <aside
      class="flex-shrink-0 border-l border-gray-800 bg-gray-900/30 flex flex-col overflow-hidden"
      :class="activeResizer === 'right' ? '' : 'transition-[width] duration-75'"
      :style="{ width: auditWidth + 'px' }"
    >
      <header class="h-14 border-b border-gray-800 flex-shrink-0 flex items-center px-5 bg-gray-900/20">
        <div class="flex items-center gap-2 text-sm">
          <BarChart3 class="w-4 h-4 text-purple-400" />
          <span class="text-gray-300 font-medium">AI 舆情及 GEO 评估</span>
        </div>
      </header>

      <div class="flex-1 overflow-y-auto p-5 custom-scrollbar">
        <div v-if="!auditReport" class="h-full flex flex-col items-center justify-center text-gray-700 text-center space-y-3">
          <History class="w-8 h-8 opacity-10" />
          <p class="text-xs">内容生成后<br/>此处将显示多维度评估报告</p>
        </div>
        <div v-else>
          <div class="markdown-content text-xs text-gray-400" v-html="renderedAudit"></div>
        </div>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #1f2937; border-radius: 10px; }

:deep(.markdown-content) h1,
:deep(.markdown-content) h2 {
  color: #fff;
  margin-top: 1.5rem;
  margin-bottom: 0.75rem;
  font-weight: 700;
}
:deep(.markdown-content) h1 {
  font-size: 1.4rem;
  border-bottom: 1px solid #374151;
  padding-bottom: 0.5rem;
}
:deep(.markdown-content) h2 { font-size: 1.15rem; }
:deep(.markdown-content) h3 { font-size: 1rem; color: #d1d5db; margin-top: 1rem; margin-bottom: 0.5rem; }
:deep(.markdown-content) p { margin-bottom: 1rem; line-height: 1.8; }
:deep(.markdown-content) strong { color: #a78bfa; }
:deep(.markdown-content) ul { list-style: disc; margin-left: 1.5rem; margin-bottom: 1rem; }
:deep(.markdown-content) li { margin-bottom: 0.3rem; }
:deep(.markdown-content) blockquote {
  border-left: 3px solid #6366f1;
  padding-left: 1rem;
  color: #9ca3af;
  margin: 1rem 0;
}
</style>
