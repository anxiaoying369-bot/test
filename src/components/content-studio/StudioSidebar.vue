<script setup lang="ts">
import { inject } from 'vue';
import {
  Sparkles, BarChart3, RefreshCw, Layout, Zap, ShieldCheck,
  Wand2, Search, PanelLeftClose
} from 'lucide-vue-next';
import { useContentStudio } from '../../composables/useContentStudio';

const {
  workMode, topic, sourceMaterial, generationMode, studioPlatforms, selectedStudioPlatform,
  isGenerating, geoBrand, geoKeyword, isGeoQuerying, geoResults, geoError, selectedGeoModel,
  studioWidth, isStudioCollapsed, activeResizer,
  handleGeoQuery, handleGenerate,
} = useContentStudio();

const navigateTo = inject<(page: string, settingsTab?: string) => void>('navigateTo');
function goToGeoSettings() {
  navigateTo?.('settings', 'geo');
}
</script>

<template>
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
      <div class="flex items-center justify-between mb-5 flex-shrink-0">
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

      <!-- 模式切换 -->
      <div class="grid grid-cols-2 gap-1 p-1 bg-gray-950 rounded-xl border border-gray-800 mb-6 flex-shrink-0">
        <button
          @click="workMode = 'studio'"
          :class="['py-2 px-3 rounded-lg text-xs font-medium transition-all flex items-center justify-center gap-1.5',
            workMode === 'studio' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300']"
        >
          <Wand2 class="w-3 h-3" /> 内容创作
        </button>
        <button
          @click="workMode = 'geo'"
          :class="['py-2 px-3 rounded-lg text-xs font-medium transition-all flex items-center justify-center gap-1.5',
            workMode === 'geo' ? 'bg-purple-600/80 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300']"
        >
          <BarChart3 class="w-3 h-3" /> GEO 监控
        </button>
      </div>

      <!-- ===== GEO 监控表单 ===== -->
      <div v-if="workMode === 'geo'" class="space-y-5">
        <div class="space-y-2.5">
          <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">品牌 / 产品名称</label>
          <input
            v-model="geoBrand"
            type="text"
            placeholder="例如：AutoCast"
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-purple-500 transition-colors"
          />
        </div>

        <div class="space-y-2.5">
          <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">监控关键词 / 查询内容</label>
          <textarea
            v-model="geoKeyword"
            rows="4"
            placeholder="例如：最好的直播设备推荐&#10;水基灭火器哪个牌子好..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-purple-500 transition-colors resize-none font-sans"
          ></textarea>
          <p class="text-[10px] text-gray-600">系统将把此内容作为提问发送给各 AI 平台，检测你的品牌是否被提及</p>
        </div>

        <button
          @click="handleGeoQuery"
          :disabled="isGeoQuerying || !geoBrand.trim() || !geoKeyword.trim()"
          class="w-full bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed text-white py-3.5 rounded-xl font-bold shadow-xl shadow-purple-900/20 transition-all flex items-center justify-center gap-2"
        >
          <RefreshCw v-if="isGeoQuerying" class="w-4 h-4 animate-spin" />
          <Search v-else class="w-4 h-4" />
          {{ isGeoQuerying ? '正在查询各平台...' : '开始 GEO 监控查询' }}
        </button>

        <!-- 结果模型列表 -->
        <div v-if="geoResults.length > 0" class="space-y-2">
          <p class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">查询结果 · 点击查看详情</p>
          <div
            v-for="r in geoResults"
            :key="r.model_name"
            @click="selectedGeoModel = r"
            :class="['flex items-center justify-between p-3 rounded-xl cursor-pointer border transition-all',
              selectedGeoModel?.model_name === r.model_name
                ? 'border-purple-500 bg-purple-500/10'
                : 'border-gray-800 bg-gray-950 hover:border-gray-700']"
          >
            <div class="flex items-center gap-2">
              <div :class="['w-2 h-2 rounded-full flex-shrink-0',
                r.error ? 'bg-gray-600' : r.mentioned ? 'bg-green-500' : 'bg-red-500']"></div>
              <span class="text-sm text-white">{{ r.model_name }}</span>
            </div>
            <div class="text-right">
              <span v-if="r.error" class="text-xs text-gray-600">请求失败</span>
              <span v-else-if="r.mentioned" class="text-xs text-green-400 font-medium">第 {{ r.position }} 位提及</span>
              <span v-else class="text-xs text-red-400">未提及</span>
            </div>
          </div>
        </div>

        <div v-if="geoError" class="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-xs text-red-400">{{ geoError }}</div>
      </div>

      <!-- ===== 创作模式内容（原有） ===== -->
      <div v-else class="space-y-6">
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
          <div v-if="studioPlatforms.length > 0" class="flex flex-wrap gap-1.5">
            <button
              v-for="p in studioPlatforms"
              :key="p.name"
              @click="selectedStudioPlatform = p"
              :class="['py-1.5 px-3 rounded-xl text-[11px] border transition-all',
                selectedStudioPlatform?.name === p.name
                  ? 'bg-blue-600/10 border-blue-500 text-blue-400'
                  : 'bg-gray-950 border-gray-800 text-gray-500 hover:border-gray-700 hover:text-gray-300']"
            >{{ p.name }}</button>
          </div>
          <div v-else class="p-3 bg-gray-950 border border-dashed border-gray-800 rounded-xl text-center">
            <p class="text-[10px] text-gray-600">请先前往<span class="text-purple-400 cursor-pointer hover:underline" @click="goToGeoSettings"> 设置 → GEO 监控 </span>添加内容发布平台</p>
          </div>
          <div v-if="selectedStudioPlatform" class="p-2.5 bg-blue-500/5 border border-blue-500/10 rounded-xl">
            <p class="text-[10px] text-gray-500 leading-relaxed line-clamp-2">{{ selectedStudioPlatform.system_prompt || '使用默认生成策略' }}</p>
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
</template>

<style scoped>
.overflow-y-auto::-webkit-scrollbar { width: 4px; }
.overflow-y-auto::-webkit-scrollbar-track { background: transparent; }
.overflow-y-auto::-webkit-scrollbar-thumb { background: #1f2937; border-radius: 10px; }
</style>
