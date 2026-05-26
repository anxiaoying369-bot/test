<script setup lang="ts">
import { ref, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  Sparkles, FileText, BarChart3,
  RefreshCw, Copy,
  Layout, Zap, Eye, ShieldCheck,
  Wand2, History, Search, ExternalLink,
  CheckCircle, XCircle, AlertCircle,
  PanelLeftClose, PanelLeftOpen
} from 'lucide-vue-next';
import { marked } from 'marked';

// ============ 工作模式 ============
const workMode = ref<'studio' | 'geo'>('studio');

// ============ 创作状态 ============

const topic = ref('');
const sourceMaterial = ref('');
const generationMode = ref<'new' | 'rewrite'>('new');
const targetPlatform = ref<'douyin' | 'wechat' | 'zhihu'>('douyin');

const isGenerating = ref(false);
const generatedContent = ref('');
const auditReport = ref('');

// ============ GEO 监控状态 ============

interface GeoResult {
  model_name: string;
  mentioned: boolean;
  position: number;
  response: string;
  sources: string[];
  error: string | null;
}

interface GeoPublishPlatform {
  name: string;
  url: string;
  description: string;
}

const geoBrand = ref('');
const geoKeyword = ref('');
const isGeoQuerying = ref(false);
const geoResults = ref<GeoResult[]>([]);
const geoError = ref('');
const selectedGeoModel = ref<GeoResult | null>(null);
const geoPublishPlatforms = ref<GeoPublishPlatform[]>([]);

const geoMentionRate = computed(() => {
  if (!geoResults.value.length) return 0;
  const mentioned = geoResults.value.filter(r => r.mentioned && !r.error).length;
  const total = geoResults.value.filter(r => !r.error).length;
  return total > 0 ? Math.round((mentioned / total) * 100) : 0;
});

async function handleGeoQuery() {
  if (!geoBrand.value.trim() || !geoKeyword.value.trim()) return;
  isGeoQuerying.value = true;
  geoResults.value = [];
  geoError.value = '';
  selectedGeoModel.value = null;
  try {
    // 同步加载发布平台配置
    const cfg = await invoke('get_config') as any;
    geoPublishPlatforms.value = cfg?.llm?.geo_publish_platforms ?? [];

    const res = await invoke('geo_monitor_query', {
      brand: geoBrand.value,
      keyword: geoKeyword.value,
    }) as GeoResult[];
    geoResults.value = res;
    if (res.length > 0) selectedGeoModel.value = res[0];
  } catch (e: any) {
    geoError.value = String(e);
  } finally {
    isGeoQuerying.value = false;
  }
}

function openUrl(url: string) {
  if (url) window.open(url, '_blank');
}

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
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">品牌 / 账号名称</label>
            <input
              v-model="geoBrand"
              type="text"
              placeholder="例如：某某创作者"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-purple-500 transition-colors"
            />
          </div>

          <div class="space-y-2.5">
            <label class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">监控关键词 / 查询内容</label>
            <textarea
              v-model="geoKeyword"
              rows="4"
              placeholder="例如：历史知识博主推荐&#10;播客创作者有哪些..."
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
              <p class="text-xs text-gray-500">正在生成适应 {{ targetPlatform }} 平台的高价值内容</p>
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
