<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import {
  FileText, Copy, Eye, Search, CheckCircle, XCircle, AlertCircle,
  ExternalLink, PanelLeftOpen, BarChart3
} from 'lucide-vue-next';
import { useContentStudio } from '../composables/useContentStudio';
import StudioSidebar from './content-studio/StudioSidebar.vue';
import StudioAuditPanel from './content-studio/StudioAuditPanel.vue';

const {
  workMode, isGenerating, generatedContent, renderedContent, selectedStudioPlatform,
  isGeoQuerying, geoResults, geoError, selectedGeoModel, geoBrand, geoPublishPlatforms,
  containerRef, isStudioCollapsed, activeResizer,
  loadStudioPlatforms, copyContent, startResizing, stopResizing,
} = useContentStudio();

onMounted(loadStudioPlatforms);
onUnmounted(stopResizing);
</script>

<template>
  <!-- 容器：relative 让展开按钮的 absolute 定位生效 -->
  <div ref="containerRef" class="flex h-full bg-gray-950 overflow-hidden relative">

    <!-- ===== 左侧：工作区设置 ===== -->
    <StudioSidebar />

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
      <div
        class="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px transition-colors"
        :class="activeResizer === 'left' ? 'bg-purple-500' : 'bg-gray-800 group-hover:bg-purple-500/60'"
      ></div>
    </div>

    <!-- ===== 中间：生成预览 / GEO 响应详情 ===== -->
    <main class="flex-1 min-w-0 flex flex-col bg-gray-950 overflow-hidden">
      <header class="h-14 border-b border-gray-800 flex-shrink-0 flex items-center justify-between px-6 bg-gray-900/20">
        <div class="flex items-center gap-2 text-sm">
          <FileText v-if="workMode === 'studio'" class="w-4 h-4 text-gray-400" />
          <Search v-else class="w-4 h-4 text-purple-400" />
          <span class="text-gray-300 font-medium">
            {{ workMode === 'studio' ? '生成结果预览' : (selectedGeoModel ? selectedGeoModel.model_name + ' · 完整回答' : 'GEO 响应详情') }}
          </span>
        </div>
        <button
          v-if="workMode === 'studio' && generatedContent"
          @click="copyContent"
          class="flex items-center gap-1.5 text-xs text-gray-400 hover:text-white transition-colors bg-gray-800 px-3 py-1.5 rounded-lg border border-gray-700"
        >
          <Copy class="w-3.5 h-3.5" /> 复制正文
        </button>
        <span v-if="workMode === 'geo' && geoPublishPlatforms.length > 0" class="text-[10px] text-gray-600">{{ geoPublishPlatforms.length }} 个发布平台 · 见右侧</span>
      </header>

      <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">

        <!-- ===== 创作模式内容 ===== -->
        <template v-if="workMode === 'studio'">
          <div v-if="!generatedContent && !isGenerating" class="h-full flex flex-col items-center justify-center text-gray-600 space-y-4">
            <div class="w-16 h-16 rounded-3xl bg-gray-900 flex items-center justify-center">
              <Eye class="w-8 h-8 text-gray-800" />
            </div>
            <p class="text-sm">在左侧输入创意，点击生成后在此预览</p>
          </div>
          <div v-else-if="isGenerating && !generatedContent" class="h-full flex flex-col items-center justify-center space-y-6">
            <div class="relative w-16 h-16">
              <div class="absolute inset-0 border-4 border-purple-500/20 rounded-full"></div>
              <div class="absolute inset-0 border-4 border-purple-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
            <div class="text-center space-y-1.5">
              <p class="text-gray-300 font-medium animate-pulse">正在调动全网 GEO 策略与企业知识库...</p>
              <p class="text-xs text-gray-500">正在生成适应「{{ selectedStudioPlatform?.name ?? '默认' }}」平台的高价值内容</p>
            </div>
          </div>
          <div v-else class="max-w-3xl mx-auto">
            <div class="prose prose-invert max-w-none">
              <div class="markdown-content text-gray-200 leading-relaxed" v-html="renderedContent"></div>
            </div>
          </div>
        </template>

        <!-- ===== GEO 监控响应详情 ===== -->
        <template v-else>
          <!-- 空态：未查询 -->
          <div v-if="!isGeoQuerying && geoResults.length === 0 && !geoError" class="h-full flex flex-col items-center justify-center text-gray-600 space-y-4">
            <div class="w-16 h-16 rounded-3xl bg-gray-900 flex items-center justify-center">
              <BarChart3 class="w-8 h-8 text-gray-800" />
            </div>
            <p class="text-sm">在左侧填写品牌和关键词，发起查询后在此查看各模型的完整回答</p>
          </div>

          <!-- 加载中 -->
          <div v-else-if="isGeoQuerying" class="h-full flex flex-col items-center justify-center space-y-6">
            <div class="relative w-16 h-16">
              <div class="absolute inset-0 border-4 border-purple-500/20 rounded-full"></div>
              <div class="absolute inset-0 border-4 border-purple-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
            <p class="text-gray-300 font-medium animate-pulse">正在并行查询各 AI 平台...</p>
          </div>

          <!-- 选中模型的响应 -->
          <div v-else-if="selectedGeoModel" class="max-w-3xl mx-auto space-y-5">
            <!-- 提及状态 -->
            <div :class="['flex items-center gap-3 p-4 rounded-xl border', selectedGeoModel.error ? 'border-gray-700 bg-gray-900/50' : selectedGeoModel.mentioned ? 'border-green-500/30 bg-green-500/5' : 'border-red-500/30 bg-red-500/5']">
              <CheckCircle v-if="!selectedGeoModel.error && selectedGeoModel.mentioned" class="w-5 h-5 text-green-400 flex-shrink-0" />
              <XCircle v-else-if="!selectedGeoModel.error && !selectedGeoModel.mentioned" class="w-5 h-5 text-red-400 flex-shrink-0" />
              <AlertCircle v-else class="w-5 h-5 text-gray-500 flex-shrink-0" />
              <div>
                <p class="text-sm font-medium text-white">
                  {{ selectedGeoModel.error ? '请求失败' : selectedGeoModel.mentioned ? `「${geoBrand}」被提及（排位第 ${selectedGeoModel.position}）` : `「${geoBrand}」未被提及` }}
                </p>
                <p v-if="selectedGeoModel.error" class="text-xs text-gray-500 mt-0.5">{{ selectedGeoModel.error }}</p>
              </div>
            </div>

            <!-- 原文回答 -->
            <div v-if="selectedGeoModel.response" class="bg-gray-900/50 border border-gray-800 rounded-xl p-5">
              <p class="text-[11px] text-gray-500 uppercase tracking-wider font-bold mb-3">{{ selectedGeoModel.model_name }} 原文回答</p>
              <p class="text-sm text-gray-300 leading-relaxed whitespace-pre-wrap">{{ selectedGeoModel.response }}</p>
            </div>

            <!-- 信息来源 -->
            <div v-if="selectedGeoModel.sources.length > 0" class="bg-gray-900/50 border border-gray-800 rounded-xl p-5">
              <p class="text-[11px] text-gray-500 uppercase tracking-wider font-bold mb-3">信息来源</p>
              <ul class="space-y-2">
                <li v-for="(src, i) in selectedGeoModel.sources" :key="i">
                  <a
                    v-if="src.startsWith('http')"
                    :href="src" target="_blank"
                    class="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1.5 break-all"
                  >
                    <ExternalLink class="w-3 h-3 flex-shrink-0" />{{ src }}
                  </a>
                  <span v-else class="text-xs text-gray-400 flex items-center gap-1.5">
                    <span class="text-gray-600">·</span>{{ src }}
                  </span>
                </li>
              </ul>
            </div>
            <div v-else-if="selectedGeoModel.response && !selectedGeoModel.error" class="text-xs text-gray-600 text-center py-2">
              该平台回答中未检测到明确的信息来源引用
            </div>
          </div>
        </template>
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

    <!-- ===== 右侧：AI 审计报告 / GEO 排名概览 ===== -->
    <StudioAuditPanel />
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
