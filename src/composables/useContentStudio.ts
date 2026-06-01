import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { marked } from 'marked';

export interface GeoResult {
  model_name: string;
  mentioned: boolean;
  position: number;
  response: string;
  sources: string[];
  error: string | null;
}

export interface GeoPublishPlatform {
  name: string;
  url: string;
  description: string;
  system_prompt: string;
}

// ============ 工作模式 ============
const workMode = ref<'studio' | 'geo'>('studio');

// ============ 创作状态 ============
const topic = ref('');
const sourceMaterial = ref('');
const generationMode = ref<'new' | 'rewrite'>('new');
const studioPlatforms = ref<GeoPublishPlatform[]>([]);
const selectedStudioPlatform = ref<GeoPublishPlatform | null>(null);
const isGenerating = ref(false);
const generatedContent = ref('');
const auditReport = ref('');

// ============ GEO 监控状态 ============
const geoBrand = ref('');
const geoKeyword = ref('');
const isGeoQuerying = ref(false);
const geoResults = ref<GeoResult[]>([]);
const geoError = ref('');
const selectedGeoModel = ref<GeoResult | null>(null);
const geoPublishPlatforms = ref<GeoPublishPlatform[]>([]);

// ============ 布局 ============
const containerRef = ref<HTMLElement | null>(null);
const studioWidth = ref(360);
const auditWidth = ref(340);
const isStudioCollapsed = ref(false);
const activeResizer = ref<'left' | 'right' | null>(null);

// ============ 计算属性 ============
const geoMentionRate = computed(() => {
  if (!geoResults.value.length) return 0;
  const mentioned = geoResults.value.filter(r => r.mentioned && !r.error).length;
  const total = geoResults.value.filter(r => !r.error).length;
  return total > 0 ? Math.round((mentioned / total) * 100) : 0;
});
const renderedContent = computed(() => marked(generatedContent.value));
const renderedAudit = computed(() => marked(auditReport.value));

// ============ 方法 ============
async function loadStudioPlatforms() {
  try {
    const cfg = await invoke('get_config') as any;
    const platforms: GeoPublishPlatform[] = cfg?.llm?.geo_publish_platforms ?? [];
    studioPlatforms.value = platforms;
    if (platforms.length > 0 && !selectedStudioPlatform.value) {
      selectedStudioPlatform.value = platforms[0];
    }
  } catch { /* ignore */ }
}

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
      platform: selectedStudioPlatform.value?.name ?? 'default',
      platformPrompt: selectedStudioPlatform.value?.system_prompt ?? null,
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

// ============ 拖动调整面板宽度 ============
function startResizing(type: 'left' | 'right', e: MouseEvent) {
  e.preventDefault(); // 阻止文字被选中
  activeResizer.value = type;

  // 拖动期间全局禁止文字选择，并锁定光标样式
  document.body.style.userSelect = 'none';
  (document.body.style as any).webkitUserSelect = 'none';
  document.body.style.cursor = 'col-resize';

  document.addEventListener('mousemove', handleResizing, { passive: true });
  document.addEventListener('mouseup', stopResizing, { once: true });
}

function handleResizing(e: MouseEvent) {
  if (!activeResizer.value || !containerRef.value) return;
  const rect = containerRef.value.getBoundingClientRect();

  if (activeResizer.value === 'left') {
    const newWidth = e.clientX - rect.left;
    studioWidth.value = Math.min(Math.max(newWidth, 260), 560);
  } else {
    const newWidth = rect.right - e.clientX;
    auditWidth.value = Math.min(Math.max(newWidth, 220), 560);
  }
}

function stopResizing() {
  activeResizer.value = null;
  document.removeEventListener('mousemove', handleResizing);
  document.body.style.userSelect = '';
  (document.body.style as any).webkitUserSelect = '';
  document.body.style.cursor = '';
}

export function useContentStudio() {
  return {
    workMode,
    topic, sourceMaterial, generationMode, studioPlatforms, selectedStudioPlatform,
    isGenerating, generatedContent, auditReport,
    geoBrand, geoKeyword, isGeoQuerying, geoResults, geoError, selectedGeoModel, geoPublishPlatforms,
    containerRef, studioWidth, auditWidth, isStudioCollapsed, activeResizer,
    geoMentionRate, renderedContent, renderedAudit,
    loadStudioPlatforms, handleGeoQuery, openUrl, handleGenerate, copyContent,
    startResizing, stopResizing,
  };
}
