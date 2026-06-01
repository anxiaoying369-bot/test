<script setup lang="ts">
import { BarChart3, RefreshCw, History, ExternalLink } from 'lucide-vue-next';
import { useContentStudio } from '../../composables/useContentStudio';

const {
  workMode, auditReport, renderedAudit,
  geoResults, isGeoQuerying, selectedGeoModel, geoPublishPlatforms, geoBrand, geoMentionRate,
  auditWidth, activeResizer, openUrl,
} = useContentStudio();
</script>

<template>
  <aside
    class="flex-shrink-0 border-l border-gray-800 bg-gray-900/30 flex flex-col overflow-hidden"
    :class="activeResizer === 'right' ? '' : 'transition-[width] duration-75'"
    :style="{ width: auditWidth + 'px' }"
  >
    <header class="h-14 border-b border-gray-800 flex-shrink-0 flex items-center px-5 bg-gray-900/20">
      <div class="flex items-center gap-2 text-sm">
        <BarChart3 class="w-4 h-4 text-purple-400" />
        <span class="text-gray-300 font-medium">{{ workMode === 'studio' ? 'AI 舆情及 GEO 评估' : 'GEO 排名概览' }}</span>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto p-5 custom-scrollbar">
      <!-- 创作模式：审计报告 -->
      <template v-if="workMode === 'studio'">
        <div v-if="!auditReport" class="h-full flex flex-col items-center justify-center text-gray-700 text-center space-y-3">
          <History class="w-8 h-8 opacity-10" />
          <p class="text-xs">内容生成后<br/>此处将显示多维度评估报告</p>
        </div>
        <div v-else>
          <div class="markdown-content text-xs text-gray-400" v-html="renderedAudit"></div>
        </div>
      </template>

      <!-- GEO 监控模式：排名统计 -->
      <template v-else>
        <div v-if="geoResults.length === 0 && !isGeoQuerying" class="h-full flex flex-col items-center justify-center text-gray-700 text-center space-y-3">
          <BarChart3 class="w-8 h-8 opacity-10" />
          <p class="text-xs">查询完成后<br/>此处将显示各平台排名统计</p>
        </div>
        <div v-else-if="isGeoQuerying" class="h-full flex items-center justify-center">
          <RefreshCw class="w-5 h-5 text-gray-600 animate-spin" />
        </div>
        <div v-else class="space-y-5">
          <!-- 总体提及率 -->
          <div class="text-center py-4">
            <div class="text-4xl font-black mb-1" :class="geoMentionRate >= 50 ? 'text-green-400' : geoMentionRate > 0 ? 'text-yellow-400' : 'text-red-400'">
              {{ geoMentionRate }}%
            </div>
            <p class="text-xs text-gray-500">全平台品牌提及率</p>
            <p class="text-[10px] text-gray-600 mt-1">
              {{ geoResults.filter(r => r.mentioned && !r.error).length }} / {{ geoResults.filter(r => !r.error).length }} 个平台提及了「{{ geoBrand }}」
            </p>
          </div>

          <!-- 每个 LLM 平台结果 -->
          <div class="space-y-2">
            <p class="text-[10px] text-gray-600 uppercase tracking-wider font-bold">各模型结果</p>
            <div
              v-for="r in geoResults" :key="r.model_name"
              @click="selectedGeoModel = r"
              :class="['p-3 rounded-xl border cursor-pointer transition-all',
                selectedGeoModel?.model_name === r.model_name ? 'border-purple-500/50 bg-purple-500/5' : 'border-gray-800 hover:border-gray-700']"
            >
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-white">{{ r.model_name }}</span>
                <span v-if="r.error" class="text-[10px] text-gray-600 bg-gray-800 px-1.5 py-0.5 rounded">失败</span>
                <span v-else-if="r.mentioned" class="text-[10px] text-green-400 bg-green-500/10 px-1.5 py-0.5 rounded">提及 #{{ r.position }}</span>
                <span v-else class="text-[10px] text-red-400 bg-red-500/10 px-1.5 py-0.5 rounded">未提及</span>
              </div>
              <div v-if="!r.error" class="w-full h-1 bg-gray-800 rounded-full overflow-hidden">
                <div
                  :class="['h-full rounded-full transition-all', r.mentioned ? 'bg-green-500' : 'bg-red-500/30']"
                  :style="{ width: r.mentioned ? Math.max(10, 100 - (r.position - 1) * 15) + '%' : '10%' }"
                ></div>
              </div>
            </div>
          </div>

          <!-- 内容发布平台（提升 GEO 的数据来源） -->
          <div v-if="geoPublishPlatforms.length > 0" class="space-y-2">
            <p class="text-[10px] text-gray-600 uppercase tracking-wider font-bold">发布内容 · 提升曝光</p>
            <p class="text-[10px] text-gray-600 leading-relaxed">在以下平台发布内容，可增加被 AI 引用的概率</p>
            <div
              v-for="p in geoPublishPlatforms" :key="p.name"
              @click="openUrl(p.url)"
              class="flex items-center justify-between p-2.5 rounded-lg border border-gray-800 hover:border-blue-500/40 bg-gray-950 cursor-pointer transition-all group"
            >
              <div>
                <span class="text-xs text-white group-hover:text-blue-400 transition-colors">{{ p.name }}</span>
                <p v-if="p.description" class="text-[10px] text-gray-600 mt-0.5">{{ p.description }}</p>
              </div>
              <ExternalLink class="w-3.5 h-3.5 text-gray-600 group-hover:text-blue-400 transition-colors flex-shrink-0" />
            </div>
          </div>
        </div>
      </template>
    </div>
  </aside>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #1f2937; border-radius: 10px; }

:deep(.markdown-content) h1,
:deep(.markdown-content) h2 { color: #fff; margin-top: 1.5rem; margin-bottom: 0.75rem; font-weight: 700; }
:deep(.markdown-content) h1 { font-size: 1.4rem; border-bottom: 1px solid #374151; padding-bottom: 0.5rem; }
:deep(.markdown-content) h2 { font-size: 1.15rem; }
:deep(.markdown-content) h3 { font-size: 1rem; color: #d1d5db; margin-top: 1rem; margin-bottom: 0.5rem; }
:deep(.markdown-content) p { margin-bottom: 1rem; line-height: 1.8; }
:deep(.markdown-content) strong { color: #a78bfa; }
:deep(.markdown-content) ul { list-style: disc; margin-left: 1.5rem; margin-bottom: 1rem; }
:deep(.markdown-content) li { margin-bottom: 0.3rem; }
:deep(.markdown-content) blockquote { border-left: 3px solid #6366f1; padding-left: 1rem; color: #9ca3af; margin: 1rem 0; }
</style>
