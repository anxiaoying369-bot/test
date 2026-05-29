<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { marked } from 'marked';
import {
  ShoppingBag, FileText, Film, Download, Plus,
  Loader2, CheckCircle2, Play, XCircle,
  Trash2, RefreshCw, Music, Settings2, Zap,
  Image as ImageIcon, Upload, Copy, Sparkles
} from 'lucide-vue-next';

// ============ 类型定义 ============

interface VideoProject {
  id: string;
  title: string;
  description?: string;
  config?: any;
  status: string;
  final_video_path?: string;
  created_at?: string;
  updated_at?: string;
}

interface VideoMaterial {
  id: string;
  project_id: string;
  material_type: string;
  source?: string;
  local_path?: string;
  remote_url?: string;
  meta?: any;
}

interface VideoTask {
  id: string;
  project_id?: string;
  task_type: string;
  status: string;
  progress: number;
  result_path?: string;
  error_msg?: string;
}

interface FfmpegProgress {
  task_id: string;
  percentage: number;
  speed: string;
  time: string;
  stage: string;
}

// ============ 状态管理 ============

const activeTab = ref<'selection' | 'script' | 'material' | 'export'>('selection');
const projects = ref<VideoProject[]>([]);
const currentProject = ref<VideoProject | null>(null);
const materials = ref<VideoMaterial[]>([]);
const activeTasks = ref<Record<string, VideoTask>>({});
const ffmpegProgress = ref<Record<string, FfmpegProgress>>({});

const audioMaterials = computed(() => materials.value.filter(m => m.material_type === 'audio'));
const imageMaterials = computed(() => materials.value.filter(m => m.material_type === 'image'));
const videoMaterials = computed(() => materials.value.filter(m => m.material_type === 'video'));

// 导出页面：用户选择的素材
const exportSelectedAudio = ref<string | null>(null);
const exportSelectedImages = ref<string[]>([]);
const exportSelectedVideos = ref<string[]>([]);

const isGenerating = ref(false);
const generationPrompt = ref('');
const videoRatio = ref('9:16');
const selectedProvider = ref('fal');

// ============ 脚本创作流程 ============
// 用户的真正工作流：产品 → 知识库 → AI 生成脚本 → 预览 → 反馈重生成 → 确认后才发起视频生成
//
// 持久化策略：把脚本相关字段全部存到 project.config.script，调 video_upsert_project 落盘到 SQLite。
// 切换项目时从 project.config.script 恢复。
const productInfo = ref('');
const referenceScript = ref('');
const generatedScript = ref('');
const scriptFeedback = ref('');
const isGeneratingScript = ref(false);
const scriptConfirmed = ref(false);

// 平台 + 剧本类型选择
const selectedPlatform = ref<string>('douyin');
const selectedScriptType = ref<'voiceover' | 'ai-video'>('voiceover');

// AI 视频：参考图选择
const referenceImageId = ref<string>('');
const referenceImageWarningAck = ref(false);
const showReferencePicker = ref(false);
const showNoReferenceWarning = ref(false);

const referenceImageMaterial = computed(() => {
  if (!referenceImageId.value) return null;
  return materials.value.find(m => m.id === referenceImageId.value) || null;
});
const availableReferenceImages = computed(() =>
  materials.value.filter(m => m.material_type === 'image' && m.local_path)
);

const PLATFORM_OPTIONS = [
  { id: 'douyin',         label: '抖音',   emoji: '🎵', desc: '钩子强、节奏快、口语化' },
  { id: 'kuaishou',       label: '快手',   emoji: '🔥', desc: '接地气、性价比、老铁感' },
  { id: 'wechat-channel', label: '视频号', emoji: '💚', desc: '朋友圈调性、信任背书' },
  { id: 'xiaohongshu',    label: '小红书', emoji: '📕', desc: '精致、种草、关键词' },
];

const SCRIPT_TYPE_OPTIONS = [
  { id: 'voiceover',  label: '口播剧本', desc: '用素材库素材 + TTS 合成视频' },
];

// 切换项目时屏蔽 watcher 的自动保存，防止刚恢复出来的数据被空值覆写
let suppressAutoSave = false;
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

interface ScriptState {
  productInfo?: string;
  referenceScript?: string;
  videoRatio?: string;
  platform?: string;
  scriptType?: 'voiceover' | 'ai-video';
  generatedScript?: string;
  scriptConfirmed?: boolean;
  generationPrompt?: string;
  // AI 视频参考图相关
  referenceImageId?: string;
  referenceImageWarningAck?: boolean;   // 用户已经点过"无参考图也继续"的二次确认
}

async function persistScriptState() {
  if (!currentProject.value) return;
  const updated: VideoProject = {
    ...currentProject.value,
    config: {
      ...(currentProject.value.config || {}),
      script: {
        productInfo: productInfo.value,
        referenceScript: referenceScript.value,
        videoRatio: videoRatio.value,
        platform: selectedPlatform.value,
        scriptType: selectedScriptType.value,
        generatedScript: generatedScript.value,
        scriptConfirmed: scriptConfirmed.value,
        generationPrompt: generationPrompt.value,
        referenceImageId: referenceImageId.value,
        referenceImageWarningAck: referenceImageWarningAck.value,
      } as ScriptState,
    },
  };
  try {
    await invoke('video_upsert_project', { project: updated });
    currentProject.value = updated;
    // 同步更新左侧列表里的引用（保证下次 selectProject 拿到的是最新的）
    const idx = projects.value.findIndex(p => p.id === updated.id);
    if (idx >= 0) projects.value[idx] = updated;
  } catch (e) {
    console.error('保存项目脚本状态失败:', e);
  }
}

function scheduleAutoSave(delay = 600) {
  if (suppressAutoSave) return;
  if (autoSaveTimer) clearTimeout(autoSaveTimer);
  autoSaveTimer = setTimeout(() => {
    persistScriptState();
  }, delay);
}

// 用户输入文本/切比例/换平台/换剧本类型时 debounce 保存
watch(
  [productInfo, referenceScript, videoRatio, selectedPlatform, selectedScriptType],
  () => {
    if (currentProject.value) scheduleAutoSave();
  }
);

function generateScript(isRegenerate: boolean = false) {
  if (!productInfo.value.trim()) {
    alert('请先填写要卖的产品信息');
    return;
  }
  if (isRegenerate && !scriptFeedback.value.trim()) {
    alert('请填写修改意见');
    return;
  }
  isGeneratingScript.value = true;
  try {
    const script = await invoke<string>('video_generate_script', {
      product: productInfo.value.trim(),
      referenceScript: referenceScript.value.trim() || null,
      videoRatio: videoRatio.value,
      platform: selectedPlatform.value,
      scriptType: selectedScriptType.value,
      previousScript: isRegenerate ? generatedScript.value : null,
      feedback: isRegenerate ? scriptFeedback.value.trim() : null,
    });
    generatedScript.value = script;
    scriptConfirmed.value = false;
    if (isRegenerate) {
      scriptFeedback.value = '';
    }
    await persistScriptState();
  } catch (e: any) {
    alert('脚本生成失败: ' + e);
  } finally {
    isGeneratingScript.value = false;
  }
}

async function confirmScript() {
  if (!generatedScript.value.trim()) {
    alert('请先生成脚本');
    return;
  }
  // 把脚本灌进生成视频的 prompt
  generationPrompt.value = generatedScript.value;
  scriptConfirmed.value = true;
  await persistScriptState();
}

async function resetScriptFlow() {
  generatedScript.value = '';
  scriptFeedback.value = '';
  scriptConfirmed.value = false;
  generationPrompt.value = '';
  await persistScriptState();
}

// ============ 素材预览（modal） ============
const previewMaterial = ref<VideoMaterial | null>(null);
const previewModalRef = ref<HTMLElement | null>(null);

// 图片缩放/拖拽状态
const previewZoom = ref(1);
const previewOffset = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0, offsetX: 0, offsetY: 0 });
let previewCloseDebounce = false;

const MIN_ZOOM = 0.2;
const MAX_ZOOM = 8;
const ZOOM_STEP = 0.25;

function openPreview(m: VideoMaterial) {
  if (!m.local_path) return;
  previewMaterial.value = m;
  resetZoom();
}

function downloadFinalVideo() {
  if (!currentProject.value?.final_video_path) return;
  const a = document.createElement('a');
  a.href = convertFileSrc(currentProject.value.final_video_path);
  a.download = currentProject.value.title + '.mp4';
  a.click();
}

async function openInFinder(path: string) {
  await invoke('open_file_in_finder', { path }).catch(() => {});
}

function closePreview() {
  previewMaterial.value = null;
  resetZoom();
  nextTick(() => previewModalRef.value?.focus());
}

function resetZoom() {
  previewZoom.value = 1;
  previewOffset.value = { x: 0, y: 0 };
}

function zoomIn() {
  previewZoom.value = Math.min(MAX_ZOOM, +(previewZoom.value + ZOOM_STEP).toFixed(2));
}

function zoomOut() {
  const next = +(previewZoom.value - ZOOM_STEP).toFixed(2);
  previewZoom.value = Math.max(MIN_ZOOM, next);
  // 缩到 1 以下时把偏移归零，避免越拖越偏
  if (previewZoom.value <= 1) previewOffset.value = { x: 0, y: 0 };
}

// 滚轮缩放（仅对图片）
function handlePreviewWheel(e: WheelEvent) {
  if (!previewMaterial.value || previewMaterial.value.material_type !== 'image') return;
  e.preventDefault();
  const delta = e.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP;
  const next = +(previewZoom.value + delta).toFixed(2);
  previewZoom.value = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, next));
  if (previewZoom.value <= 1) previewOffset.value = { x: 0, y: 0 };
}

// 拖拽平移（缩放 >1 时才生效）
function startDrag(e: MouseEvent) {
  if (!previewMaterial.value || previewMaterial.value.material_type !== 'image') return;
  if (previewZoom.value <= 1) return;
  isDragging.value = true;
  dragStart.value = {
    x: e.clientX,
    y: e.clientY,
    offsetX: previewOffset.value.x,
    offsetY: previewOffset.value.y,
  };
  e.preventDefault();
}

function onDrag(e: MouseEvent) {
  if (!isDragging.value) return;
  previewOffset.value = {
    x: dragStart.value.offsetX + (e.clientX - dragStart.value.x),
    y: dragStart.value.offsetY + (e.clientY - dragStart.value.y),
  };
}

function endDrag() {
  isDragging.value = false;
}

// 全局键盘：modal 打开期间监听 Esc / + / - / 0（重置）
function handlePreviewKey(e: KeyboardEvent) {
  if (!previewMaterial.value) return;
  if (e.key === 'Escape') { closePreview(); return; }
  if (previewMaterial.value.material_type !== 'image') return;
  if (e.key === '+' || e.key === '=') { e.preventDefault(); zoomIn(); }
  else if (e.key === '-' || e.key === '_') { e.preventDefault(); zoomOut(); }
  else if (e.key === '0') { e.preventDefault(); resetZoom(); }
}

watch(previewMaterial, (m) => {
  if (m) {
    window.addEventListener('keydown', handlePreviewKey);
  } else {
    window.removeEventListener('keydown', handlePreviewKey);
  }
});

// 把脚本作为 Markdown 渲染成 HTML（marked 已经在依赖里）
const renderedScript = computed(() => {
  if (!generatedScript.value) return '';
  try {
    return marked.parse(generatedScript.value, { breaks: true, gfm: true }) as string;
  } catch {
    // 渲染失败就回退到纯文本
    return `<pre class="whitespace-pre-wrap text-sm">${
      generatedScript.value
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
    }</pre>`;
  }
});

// ============ 上传素材到项目 ============
const isUploadingMaterial = ref(false);

async function uploadMaterial(kind: 'image' | 'video' | 'audio') {
  if (!currentProject.value) {
    alert('请先选择或新建一个项目');
    return;
  }
  const filters = kind === 'image'
    ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }]
    : kind === 'video'
    ? [{ name: 'Videos', extensions: ['mp4', 'mov', 'webm', 'mkv', 'avi'] }]
    : [{ name: 'Audio', extensions: ['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac'] }];

  try {
    const selected = await open({ multiple: true, filters });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected as string];
    if (paths.length === 0) return;

    isUploadingMaterial.value = true;
    let okCount = 0;
    for (const p of paths) {
      try {
        await invoke('video_upload_material', {
          projectId: currentProject.value.id,
          sourcePath: p,
          materialType: kind,
        });
        okCount++;
      } catch (e) {
        console.error(`上传失败 ${p}:`, e);
      }
    }
    await loadMaterials(currentProject.value.id);
    if (okCount < paths.length) {
      alert(`已上传 ${okCount} / ${paths.length}，部分失败请查看控制台`);
    }
  } catch (e) {
    alert('选择文件失败: ' + e);
  } finally {
    isUploadingMaterial.value = false;
  }
}

// ============ TTS 口播合成 ============
interface VoiceItem { id: string; name: string; gender?: string; language?: string; }
const availableVoices = ref<VoiceItem[]>([]);
const ttsVoiceId = ref('');
const ttsSpeed = ref(1.0);
const isLoadingVoices = ref(false);
const isSynthesizingVoice = ref(false);
const isRenderingVoiceover = ref(false);
const latestVoiceoverPath = ref(''); // 当前项目最近一次合成的音频路径

// 从脚本 Markdown 中抽出纯口播文字（去掉标题/分镜标记，只保留 "口播：「...」" 或正文段落）
function extractVoiceoverText(markdown: string): string {
  if (!markdown) return '';
  // 抽取所有「」内的文字
  const quoted: string[] = [];
  const re = /[「""]([^」""]+)[」""]/g;
  let m;
  while ((m = re.exec(markdown)) !== null) quoted.push(m[1].trim());
  if (quoted.length > 0) return quoted.join('\n');
  // 没有引号 → 把 markdown 清掉标记，按段返回
  return markdown
    .replace(/^#+\s.*$/gm, '')
    .replace(/^>.*$/gm, '')
    .replace(/^\*\*[^*]+\*\*[::]\s*/gm, '')
    .replace(/^---+$/gm, '')
    .replace(/`[^`\n]*`/g, '')
    .split(/\n+/).map(s => s.trim()).filter(Boolean).join('\n');
}

async function loadVoices() {
  if (!currentProject.value) return;
  isLoadingVoices.value = true;
  try {
    const cfg = await invoke('get_config') as any;
    const provider = cfg?.video?.tts_provider || 'mock';
    const apiKey = cfg?.video?.tts_api_key || '';
    const baseUrl = cfg?.video?.tts_base_url || null;
    const model = cfg?.video?.tts_model || null;
    const res = await invoke<any>('tts_list_voices', { provider, apiKey, baseUrl, model });
    availableVoices.value = (res?.voices || []) as VoiceItem[];
    if (!ttsVoiceId.value && availableVoices.value.length > 0) {
      ttsVoiceId.value = (cfg?.video?.default_tts_voice || availableVoices.value[0].id) as string;
    }
    if (cfg?.video?.default_tts_speed && !ttsSpeed.value) {
      ttsSpeed.value = Number(cfg.video.default_tts_speed) || 1.0;
    }
  } catch (e) {
    alert('加载音色失败: ' + e);
  } finally {
    isLoadingVoices.value = false;
  }
}

async function synthesizeVoice() {
  if (!currentProject.value) return;
  if (!ttsVoiceId.value) {
    alert('请先选择音色');
    return;
  }
  const text = extractVoiceoverText(generatedScript.value);
  if (!text.trim()) {
    alert('脚本里没找到可用的口播文本');
    return;
  }

  isSynthesizingVoice.value = true;
  try {
    const cfg = await invoke('get_config') as any;
    const provider = cfg?.video?.tts_provider || 'mock';
    const apiKey = cfg?.video?.tts_api_key || '';
    const baseUrl = cfg?.video?.tts_base_url || null;
    const model = cfg?.video?.tts_model || null;

    const audioPath = await invoke<string>('tts_synthesize', {
      projectId: currentProject.value.id,
      text,
      voiceId: ttsVoiceId.value,
      speed: ttsSpeed.value,
      provider,
      apiKey,
      baseUrl,
      model,
    });
    latestVoiceoverPath.value = audioPath;
    await loadMaterials(currentProject.value.id);
  } catch (e) {
    alert('合成失败: ' + e);
  } finally {
    isSynthesizingVoice.value = false;
  }
}

async function startVoiceoverRender() {
  if (!currentProject.value || !latestVoiceoverPath.value) return;

  // 找音频素材 ID
  const audioMaterialId = materials.value.find(m => m.local_path === latestVoiceoverPath.value)?.id;
  if (!audioMaterialId) {
    alert('未找到音频素材 ID，请先完成音频合成');
    return;
  }
  
  // 找视觉素材 ID
  const visualMaterialIds = materials.value
    .filter(m => (m.material_type === 'image' || m.material_type === 'video') && m.local_path)
    .map(m => m.id);

  if (visualMaterialIds.length === 0) {
    alert('请先在素材库添加视觉素材（图片或视频）');
    activeTab.value = 'material';
    return;
  }

  isRenderingVoiceover.value = true;
  try {
    const taskId = await invoke<string>('video_render_voiceover', {
      projectId: currentProject.value.id,
      audioMaterialId,
      visualMaterialIds,
      bgmPath: selectedBgmPath.value,
      config: renderConfig.value
    });

    activeTasks.value[taskId] = {
      id: taskId,
      project_id: currentProject.value.id,
      task_type: 'voiceover',
      status: 'processing',
      progress: 0
    };
    
    // 轮询等待合成完成
    const checkTimer = setInterval(async () => {
        try {
            const tasks = await invoke<VideoTask[]>('video_list_tasks', { projectId: currentProject.value!.id });
            const task = tasks.find(t => t.id === taskId);
            if (task) {
                if (task.status === 'completed') {
                    clearInterval(checkTimer);
                    await loadProjects();
                    const fresh = projects.value.find(p => p.id === currentProject.value!.id);
                    if (fresh) currentProject.value = fresh;
                    delete activeTasks.value[taskId];
                    alert('口播视频合成成功！项目已自动锁定。');
                    activeTab.value = 'material';
                } else if (task.status === 'error') {
                    clearInterval(checkTimer);
                    alert('合成失败: ' + task.error_msg);
                    delete activeTasks.value[taskId];
                }
            }
        } catch (e) {
            console.error('轮询合成任务失败:', e);
        }
    }, 2500);

  } catch (e) {
    alert('发起合成失败: ' + e);
  } finally {
    isRenderingVoiceover.value = false;
  }
}


// ============ AI 图片生成 ============
const showImageGenModal = ref(false);
const imageGenPrompt = ref('');
const imageGenSize = ref('1024x1024');
const imageGenProvider = ref<'fal' | 'volcengine' | 'openai' | 'mock'>('mock');
const imageGenRefPath = ref('');         // 本地参考图绝对路径（可选）
const isGeneratingImage = ref(false);

const IMAGE_SIZE_PRESETS = [
  { id: '1024x1024', label: '1:1 · 1024' },
  { id: '720x1280',  label: '9:16 · 720' },
  { id: '1280x720',  label: '16:9 · 720' },
  { id: '768x1024',  label: '3:4 · 768' },
];

function openImageGenModal() {
  if (!currentProject.value) {
    alert('请先选择或新建一个项目');
    return;
  }
  imageGenPrompt.value = '';
  imageGenRefPath.value = '';
  // 根据当前视频比例预设个合理 size
  if (videoRatio.value === '9:16') imageGenSize.value = '720x1280';
  else if (videoRatio.value === '16:9') imageGenSize.value = '1280x720';
  else imageGenSize.value = '1024x1024';
  showImageGenModal.value = true;
}

async function pickImageGenReference() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    if (selected && typeof selected === 'string') {
      imageGenRefPath.value = selected;
    }
  } catch (e) {
    console.error(e);
  }
}

async function generateImageMaterial() {
  if (!currentProject.value) return;
  if (!imageGenPrompt.value.trim()) {
    alert('请填写提示词');
    return;
  }
  isGeneratingImage.value = true;
  try {
    const cfg = await invoke('get_config') as any;
    let apiKey = '';
    let baseUrl = '';
    let model = '';

    if (imageGenProvider.value === 'fal') {
      apiKey = cfg?.video?.fal_key || '';
    } else if (imageGenProvider.value === 'volcengine') {
      apiKey = cfg?.video?.volc_key || '';
    } else if (imageGenProvider.value === 'openai') {
      apiKey = cfg?.video?.openai_api_key || '';
      baseUrl = cfg?.video?.openai_base_url || '';
      model = cfg?.video?.openai_model || '';
    } else {
      apiKey = 'sk-mock-key';
    }

    if (!apiKey && imageGenProvider.value !== 'mock') {
      alert(`请先在设置中配置 ${imageGenProvider.value} 的 API Key`);
      return;
    }

    await invoke('video_generate_image', {
      projectId: currentProject.value.id,
      prompt: imageGenPrompt.value.trim(),
      size: imageGenSize.value,
      provider: imageGenProvider.value,
      apiKey,
      referenceImagePath: imageGenRefPath.value || null,
      baseUrl: baseUrl || null,
      model: model || null,
    });
    await loadMaterials(currentProject.value.id);
    showImageGenModal.value = false;
  } catch (e) {
    alert('AI 图片生成失败: ' + e);
  } finally {
    isGeneratingImage.value = false;
  }
}

async function deleteMaterial(materialId: string) {
  if (!currentProject.value) return;
  if (!confirm('确定删除这条素材？文件会同时从项目目录里清除。')) return;
  // 同步清理导出选中状态
  if (exportSelectedAudio.value === materialId) exportSelectedAudio.value = null;
  exportSelectedImages.value = exportSelectedImages.value.filter(id => id !== materialId);
  exportSelectedVideos.value = exportSelectedVideos.value.filter(id => id !== materialId);
  try {
    await invoke('video_delete_material', { id: materialId });
    await loadMaterials(currentProject.value.id);
  } catch (e) {
    alert('删除失败: ' + e);
  }
}

function toggleExportImage(id: string) {
  const idx = exportSelectedImages.value.indexOf(id);
  if (idx === -1) exportSelectedImages.value.push(id);
  else exportSelectedImages.value.splice(idx, 1);
}

function toggleExportVideo(id: string) {
  const idx = exportSelectedVideos.value.indexOf(id);
  if (idx === -1) exportSelectedVideos.value.push(id);
  else exportSelectedVideos.value.splice(idx, 1);
}

async function startExportRender() {
  if (!currentProject.value || !exportSelectedAudio.value) return;
  if (exportSelectedImages.value.length === 0 && exportSelectedVideos.value.length === 0) {
    alert('请至少选择图片或视频素材');
    return;
  }
  const audioMat = materials.value.find(m => m.id === exportSelectedAudio.value);
  if (!audioMat?.local_path) { alert('音频素材未本地化'); return; }

  isRenderingVoiceover.value = true;
  try {
    const imagePaths = exportSelectedImages.value
      .map(id => materials.value.find(m => m.id === id)?.local_path)
      .filter(Boolean) as string[];
    const videoPaths = exportSelectedVideos.value
      .map(id => materials.value.find(m => m.id === id)?.local_path)
      .filter(Boolean) as string[];

    const taskId: string = await invoke('video_render_voiceover', {
      projectId: currentProject.value.id,
      audioPath: audioMat.local_path,
      imagePaths,
      videoPaths,
    });
    activeTasks.value = { ...activeTasks.value, [taskId]: { id: taskId, task_type: 'voiceover', status: 'running' } };
  } catch (e) {
    alert('合成失败: ' + e);
  } finally {
    isRenderingVoiceover.value = false;
  }
}

// ============ 渲染配置 ============

const renderConfig = ref({
  width: 1080,
  height: 1920,
  bgm_volume: 0.3,
  transition_type: 'fade',
  ken_burns: true
});

const selectedBgmPath = ref<string | null>(null);

function updateResolution(ratio: string) {
  if (ratio === '9:16') {
    renderConfig.value.width = 1080;
    renderConfig.value.height = 1920;
  } else if (ratio === '16:9') {
    renderConfig.value.width = 1920;
    renderConfig.value.height = 1080;
  } else {
    renderConfig.value.width = 1080;
    renderConfig.value.height = 1080;
  }
}

// ============ 初始化 ============

let unlistenFfmpeg: UnlistenFn | null = null;

onMounted(async () => {
  await loadProjects();
  
  try {
    const cfg = await invoke('get_config') as any;
    if (cfg?.video?.default_provider) {
      selectedProvider.value = cfg.video.default_provider;
    }
  } catch (e) {
    console.error('加载视频配置失败:', e);
  }

  unlistenFfmpeg = await listen<FfmpegProgress>('video-ffmpeg-progress', (event) => {
    ffmpegProgress.value[event.payload.task_id] = event.payload;
  });
});

onUnmounted(() => {
  if (unlistenFfmpeg) unlistenFfmpeg();
  window.removeEventListener('keydown', handlePreviewKey);
});

// ============ 数据加载 ============

async function loadProjects() {
  try {
    projects.value = await invoke('video_list_projects');
  } catch (e) {
    console.error('Failed to load projects:', e);
  }
}

async function loadMaterials(projectId: string) {
  try {
    materials.value = await invoke('video_list_materials', { projectId });
  } catch (e) {
    console.error('Failed to load materials:', e);
  }
}

async function createProject() {
  const title = `新视频项目 - ${new Date().toLocaleTimeString()}`;

  const newProject: VideoProject = {
    id: crypto.randomUUID(),
    title,
    status: 'draft'
  };

  try {
    await invoke('video_upsert_project', { project: newProject });
    await loadProjects();
    selectProject(newProject);
  } catch (e) {
    alert('创建失败: ' + e);
  }
}

async function deleteProject(projectId: string, event: Event) {
  event.stopPropagation(); // 阻止冒泡到 selectProject
  
  if (!confirm('确定要删除这个项目吗？所有关联素材和记录将被永久清除。')) return;

  try {
    await invoke('video_delete_project', { id: projectId });
    await loadProjects();
    if (currentProject.value?.id === projectId) {
      currentProject.value = null;
      activeTab.value = 'selection';
    }
  } catch (e) {
    alert('删除失败: ' + e);
  }
}

// ============ AI 视频参考图 ============
function pickReferenceImage(materialId: string) {
  referenceImageId.value = materialId;
  referenceImageWarningAck.value = false;  // 选了图就不需要 ack 警告了
  showReferencePicker.value = false;
  persistScriptState();
}

function clearReferenceImage() {
  referenceImageId.value = '';
  persistScriptState();
}

function ackNoReferenceWarning() {
  referenceImageWarningAck.value = true;
  showNoReferenceWarning.value = false;
  persistScriptState();
  // 用户确认后立即继续走视频生成
  startGeneration();
}

// ============ 项目克隆 ============
async function cloneCurrentProject() {
  if (!currentProject.value) return;
  try {
    const cloned = await invoke<VideoProject>('video_clone_project', { id: currentProject.value.id });
    await loadProjects();
    const fresh = projects.value.find(p => p.id === cloned.id);
    if (fresh) selectProject(fresh);
  } catch (e) {
    alert('克隆失败: ' + e);
  }
}

function selectProject(project: VideoProject) {
  // 关键：恢复期间屏蔽 watcher 的自动保存，否则空值会覆盖刚读出来的内容
  suppressAutoSave = true;
  if (autoSaveTimer) {
    clearTimeout(autoSaveTimer);
    autoSaveTimer = null;
  }

  currentProject.value = project;
  activeTab.value = 'script';
  loadMaterials(project.id);

  // 从 project.config.script 恢复脚本流程的状态
  const s: ScriptState = (project.config && project.config.script) || {};
  productInfo.value = s.productInfo || '';
  referenceScript.value = s.referenceScript || '';
  videoRatio.value = s.videoRatio || '9:16';
  updateResolution(videoRatio.value);
  selectedPlatform.value = s.platform || 'douyin';
  selectedScriptType.value = s.scriptType || 'ai-video';
  generatedScript.value = s.generatedScript || '';
  scriptConfirmed.value = !!s.scriptConfirmed;
  generationPrompt.value = s.generationPrompt || '';
  scriptFeedback.value = ''; // 反馈框不持久化，每次进入清空
  referenceImageId.value = s.referenceImageId || '';
  referenceImageWarningAck.value = !!s.referenceImageWarningAck;
  latestVoiceoverPath.value = '';   // 从素材里找最新音频
  ttsVoiceId.value = '';

  // 下一帧再放开自动保存（让上面的 ref 赋值不触发 watcher）
  setTimeout(() => { suppressAutoSave = false; }, 0);
}

// 切换到口播流程时自动加载音色 + 找最近一次的合成音频
watch(
  [() => currentProject.value?.id, selectedScriptType, scriptConfirmed],
  ([pid, type, confirmed]) => {
    if (!pid || type !== 'voiceover' || !confirmed) return;
    // 找当前项目最近的 audio 类型素材
    const latest = materials.value.find(m => m.material_type === 'audio' && m.local_path);
    if (latest?.local_path) latestVoiceoverPath.value = latest.local_path;
    if (availableVoices.value.length === 0) loadVoices();
  }
);

// ============ AI 生成逻辑 ============

async function startGeneration() {
  if (!currentProject.value || !generationPrompt.value) return;

  // ── AI 视频类型 & 没选参考图 & 没确认过警告 → 弹强制警告 ──
  if (
    selectedScriptType.value === 'ai-video' &&
    !referenceImageId.value &&
    !referenceImageWarningAck.value
  ) {
    showNoReferenceWarning.value = true;
    return;
  }

  isGenerating.value = true;
  try {
    const cfg = await invoke('get_config') as any;

    // 从 video 配置中获取对应的 Key
    let apiKey = '';
    let baseUrl = '';
    let model = '';

    if (selectedProvider.value === 'fal') {
      apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
    } else if (selectedProvider.value === 'volcengine') {
      apiKey = cfg?.video?.volc_key;
    } else if (selectedProvider.value === 'openai') {
      apiKey = cfg?.video?.openai_api_key;
      baseUrl = cfg?.video?.openai_base_url;
      model = cfg?.video?.openai_model;
    } else if (selectedProvider.value === 'mock') {
      apiKey = 'sk-mock-key';
    }

    if (!apiKey && selectedProvider.value !== 'mock') {
      alert(`请先在设置中配置 ${selectedProvider.value} 的 API Key`);
      return;
    }

    const taskId: string = await invoke('video_start_generation', {
      projectId: currentProject.value.id,
      prompt: generationPrompt.value,
      provider: selectedProvider.value,
      apiKey: apiKey,
      mode: referenceImageId.value ? 'image' : 'text',
      ratio: videoRatio.value,
      baseUrl: baseUrl,
      model: model,
      referenceMaterialId: referenceImageId.value || null,
    });

    activeTasks.value[taskId] = {
      id: taskId,
      project_id: currentProject.value.id,
      task_type: 'generation',
      status: 'processing',
      progress: 0
    };

    startPollingTask(taskId);
    activeTab.value = 'material';
  } catch (e) {
    alert('发起生成失败: ' + e);
  } finally {
    isGenerating.value = false;
  }
}

function startPollingTask(taskId: string) {
  const timer = setInterval(async () => {
    try {
      const cfg = await invoke('get_config') as any;
      
      let apiKey = '';
      let baseUrl = '';
      let model = '';

      if (selectedProvider.value === 'fal') {
        apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
      } else if (selectedProvider.value === 'volcengine') {
        apiKey = cfg?.video?.volc_key;
      } else if (selectedProvider.value === 'openai') {
        apiKey = cfg?.video?.openai_api_key;
        baseUrl = cfg?.video?.openai_base_url;
        model = cfg?.video?.openai_model;
      } else if (selectedProvider.value === 'mock') {
        apiKey = 'sk-mock-key';
      }

      const res: any = await invoke('video_poll_task_status', {
        taskId,
        provider: selectedProvider.value,
        apiKey: apiKey,
        baseUrl: baseUrl,
        model: model
      });

      if (res.status === 'completed') {
        clearInterval(timer);
        delete activeTasks.value[taskId];
        
        // 如果是生成任务，自动触发下载
        if (res.video_url && currentProject.value) {
          await invoke('video_download_material', {
            projectId: currentProject.value.id,
            url: res.video_url,
            materialType: 'video'
          });
          loadMaterials(currentProject.value.id);
        }
      } else if (res.status === 'error') {
        clearInterval(timer);
        alert('任务失败: ' + res.error);
        delete activeTasks.value[taskId];
      }
    } catch (e) {
      console.error('Polling failed:', e);
      clearInterval(timer);
    }
  }, 3000);
}

// ============ 素材与剪辑 ============

async function selectBgm() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'm4a'] }]
  });
  if (selected) {
    selectedBgmPath.value = selected as string;
  }
}

async function startAdvancedRender() {
  if (!currentProject.value || materials.value.length === 0) return;

  try {
    const videoPaths = materials.value
      .filter(m => m.material_type === 'video' && m.local_path)
      .map(m => m.local_path as string);
    
    if (videoPaths.length === 0) {
      alert('没有可用的视频素材');
      return;
    }

    const taskId: string = await invoke('video_render_advanced', {
      projectId: currentProject.value.id,
      videoPaths,
      bgmPath: selectedBgmPath.value,
      config: renderConfig.value
    });

    activeTasks.value[taskId] = {
      id: taskId,
      project_id: currentProject.value.id,
      task_type: 'export',
      status: 'processing',
      progress: 0
    };
  } catch (e) {
    alert('发起合成失败: ' + e);
  }
}
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100 overflow-hidden">
    <!-- 侧边栏：项目列表 -->
    <div class="flex h-full">
      <div class="w-72 bg-gray-900 border-r border-gray-800 flex flex-col">
        <div class="p-6">
          <button @click="createProject" class="w-full bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 rounded-xl flex items-center justify-center gap-2 transition-all shadow-lg shadow-blue-900/20">
            <Plus class="w-5 h-5" />
            新建创作项目
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar px-4 pb-6 space-y-2">
          <div 
            v-for="p in projects" 
            :key="p.id"
            @click="selectProject(p)"
            :class="['p-4 rounded-xl cursor-pointer transition-all border group relative', currentProject?.id === p.id ? 'bg-blue-600/10 border-blue-500/50 shadow-inner' : 'hover:bg-gray-800 border-transparent text-gray-400']"
          >
            <div class="flex items-center gap-2">
              <Film :class="['w-4 h-4', currentProject?.id === p.id ? 'text-blue-400' : 'text-gray-600']" />
              <div class="flex-1 truncate text-sm font-medium">{{ p.title }}</div>
              <button
                @click="deleteProject(p.id, $event)"
                class="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/20 hover:text-red-500 rounded-lg transition-all"
                title="删除项目"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 主内容区 -->
      <div v-if="currentProject" class="flex-1 flex flex-col relative">
        <!-- 头部 -->
        <div class="h-20 px-8 flex items-center justify-between border-b border-gray-800 bg-gray-950/50 backdrop-blur-md">
          <div class="flex items-center gap-3">
            <h2 class="text-xl font-bold text-white">{{ currentProject.title }}</h2>
            <span class="text-[10px] px-2 py-0.5 rounded-full bg-gray-800 text-gray-500 font-mono uppercase tracking-tighter">{{ currentProject.id.slice(0, 8) }}</span>
          </div>

          <div class="flex gap-2 items-center">
            <div class="h-6 w-px bg-gray-800 mx-1" />
            <button
              v-for="tab in [{id:'script', n:'脚本/生成', i:FileText}, {id:'material', n:'素材库', i:ShoppingBag}, {id:'export', n:'后期/导出', i:Settings2}]"
              :key="tab.id"
              @click="activeTab = tab.id as any"
              :class="['px-5 py-2 rounded-xl text-sm font-medium transition-all flex items-center gap-2 border', activeTab === tab.id ? 'bg-gray-800 border-gray-700 text-white' : 'text-gray-500 hover:text-gray-300 border-transparent']"
            >
              <component :is="tab.i" class="w-4 h-4" />
              {{ tab.n }}
            </button>
          </div>
        </div>

        <!-- 各 Tab 内容 -->
        <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
          
          <!-- Tab 1: 脚本（产品 → AI 生成脚本 → 预览 → 反馈重生成 → 确认进入视频生成） -->
          <div v-if="activeTab === 'script'" class="max-w-4xl mx-auto space-y-6 animate-in fade-in slide-in-from-bottom-2">

            <!-- Step 1: 输入产品 + 参考脚本 + 视频比例 -->
            <div class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-2xl">
              <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center gap-2">
                <div class="w-6 h-6 rounded-full bg-blue-600 text-white text-xs font-bold flex items-center justify-center">1</div>
                <h3 class="text-sm font-bold text-gray-200">填写产品信息</h3>
              </div>

              <div class="p-6 space-y-5">
                <div>
                  <label class="block text-xs font-medium text-gray-400 mb-2">要卖的产品 <span class="text-red-400">*</span></label>
                  <textarea
                    v-model="productInfo"
                    :disabled="!!generatedScript"
                    placeholder="例如：3 代花西子空气蜜粉，含珍珠粉成分，定妆 8 小时不脱妆，0.04mm 微细粉质，遮瑕力强..."
                    class="w-full h-28 p-4 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none disabled:opacity-60"
                  ></textarea>
                  <p class="text-[11px] text-gray-600 mt-1">填得越详细，AI 越能写出有信息密度的脚本（卖点/规格/差异化）。</p>
                </div>

                <div>
                  <label class="block text-xs font-medium text-gray-400 mb-2">参考脚本 <span class="text-gray-600">(可选)</span></label>
                  <textarea
                    v-model="referenceScript"
                    :disabled="!!generatedScript"
                    placeholder="如果有同类爆款脚本可粘贴在这里，AI 会借鉴节奏与表达，但不会照抄..."
                    class="w-full h-24 p-4 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none disabled:opacity-60"
                  ></textarea>
                </div>

                <!-- 目标平台 -->
                <div>
                  <label class="block text-xs font-medium text-gray-400 mb-2">
                    目标平台 <span class="text-[10px] text-gray-600">（影响剧本的语气、节奏、CTA 风格）</span>
                  </label>
                  <div class="grid grid-cols-2 md:grid-cols-4 gap-2">
                    <button
                      v-for="p in PLATFORM_OPTIONS"
                      :key="p.id"
                      @click="selectedPlatform = p.id"
                      :disabled="!!generatedScript"
                      :class="[
                        'p-3 rounded-xl border text-left transition-all disabled:opacity-60 disabled:cursor-not-allowed',
                        selectedPlatform === p.id
                          ? 'bg-blue-600/15 border-blue-500/50 text-white'
                          : 'bg-gray-950 border-gray-800 text-gray-400 hover:border-gray-700'
                      ]"
                    >
                      <div class="text-sm font-bold flex items-center gap-1.5">
                        <span>{{ p.emoji }}</span> {{ p.label }}
                      </div>
                      <div class="text-[10px] text-gray-500 mt-0.5 leading-snug">{{ p.desc }}</div>
                    </button>
                  </div>
                </div>

                <!-- 剧本类型 -->
                <div>
                  <label class="block text-xs font-medium text-gray-400 mb-2">
                    剧本类型 <span class="text-[10px] text-gray-600">（决定后续走口播合成还是 AI 视频生成）</span>
                  </label>
                  <div class="grid grid-cols-2 gap-2">
                    <button
                      v-for="t in SCRIPT_TYPE_OPTIONS"
                      :key="t.id"
                      @click="selectedScriptType = t.id as any"
                      :disabled="!!generatedScript"
                      :class="[
                        'p-3 rounded-xl border text-left transition-all disabled:opacity-60 disabled:cursor-not-allowed',
                        selectedScriptType === t.id
                          ? 'bg-purple-600/15 border-purple-500/50 text-white'
                          : 'bg-gray-950 border-gray-800 text-gray-400 hover:border-gray-700'
                      ]"
                    >
                      <div class="text-sm font-bold">{{ t.label }}</div>
                      <div class="text-[10px] text-gray-500 mt-0.5 leading-snug">{{ t.desc }}</div>
                    </button>
                  </div>
                </div>

                <div class="flex items-center justify-between gap-4">
                  <div class="flex flex-col gap-1.5">
                    <span class="text-[10px] text-gray-600 font-bold uppercase tracking-wider">视频比例</span>
                    <div class="flex bg-gray-950 border border-gray-800 p-1 rounded-lg">
                      <button
                        v-for="r in ['9:16', '16:9', '1:1']"
                        :key="r"
                        @click="videoRatio = r; updateResolution(r)"
                        :disabled="!!generatedScript"
                        :class="['px-4 py-1.5 rounded-md text-xs font-medium transition-all disabled:cursor-not-allowed', videoRatio === r ? 'bg-blue-600 text-white' : 'text-gray-500 hover:text-gray-300']"
                      >{{ r }}</button>
                    </div>
                  </div>

                  <button
                    v-if="!generatedScript"
                    @click="generateScript(false)"
                    :disabled="isGeneratingScript || !productInfo.trim()"
                    class="bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white px-8 py-3 rounded-xl font-bold transition-all shadow-lg shadow-blue-900/30 flex items-center gap-2"
                  >
                    <Loader2 v-if="isGeneratingScript" class="w-4 h-4 animate-spin" />
                    <FileText v-else class="w-4 h-4" />
                    {{ isGeneratingScript ? 'AI 检索知识库并生成中...' : '生成脚本' }}
                  </button>
                  <button
                    v-else
                    @click="resetScriptFlow"
                    class="text-xs text-gray-500 hover:text-gray-300 px-3 py-2 border border-gray-800 rounded-lg flex items-center gap-1"
                  >
                    <RefreshCw class="w-3.5 h-3.5" /> 重新输入
                  </button>
                </div>
              </div>
            </div>

            <!-- Step 2: 预览 + 反馈重生成 -->
            <div v-if="generatedScript" class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-2xl">
              <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-6 h-6 rounded-full bg-blue-600 text-white text-xs font-bold flex items-center justify-center">2</div>
                  <h3 class="text-sm font-bold text-gray-200">脚本预览</h3>
                  <span v-if="scriptConfirmed" class="text-[10px] px-2 py-0.5 bg-green-500/20 text-green-400 rounded-full border border-green-500/30">已确认</span>
                </div>
                <span class="text-[10px] text-gray-600">{{ generatedScript.length }} 字</span>
              </div>

              <div class="p-6 max-h-[400px] overflow-y-auto custom-scrollbar">
                <div
                  class="script-markdown prose prose-invert prose-sm max-w-none text-gray-200 leading-relaxed"
                  v-html="renderedScript"
                />
              </div>

              <!-- 反馈 + 重生成 -->
              <div v-if="!scriptConfirmed" class="border-t border-gray-800 p-6 bg-gray-950/40 space-y-3">
                <label class="block text-xs font-medium text-gray-400">不满意？告诉 AI 怎么改</label>
                <textarea
                  v-model="scriptFeedback"
                  placeholder="例如：开头钩子改成提问式；中段加上 30 天无理由退换；结尾去掉过于硬广的语气..."
                  class="w-full h-20 p-3 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none"
                ></textarea>
                <div class="flex items-center justify-end gap-3">
                  <button
                    @click="generateScript(true)"
                    :disabled="isGeneratingScript || !scriptFeedback.trim()"
                    class="bg-gray-800 hover:bg-gray-700 disabled:opacity-50 text-white px-5 py-2.5 rounded-lg font-medium text-sm flex items-center gap-2 border border-gray-700"
                  >
                    <Loader2 v-if="isGeneratingScript" class="w-4 h-4 animate-spin" />
                    <RefreshCw v-else class="w-4 h-4" />
                    根据意见重新生成
                  </button>
                  <button
                    @click="confirmScript"
                    class="bg-green-600 hover:bg-green-500 text-white px-5 py-2.5 rounded-lg font-bold text-sm flex items-center gap-2 shadow-lg shadow-green-900/30"
                  >
                    <CheckCircle2 class="w-4 h-4" />
                    确认脚本，进入下一步
                  </button>
                </div>
              </div>
            </div>

            <!-- 口播剧本：TTS 合成 -->
            <div v-if="scriptConfirmed"
                 class="bg-gray-900 border border-purple-500/30 rounded-2xl overflow-hidden shadow-2xl">
              <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-6 h-6 rounded-full bg-purple-600 text-white text-xs font-bold flex items-center justify-center">3</div>
                  <h3 class="text-sm font-bold text-gray-200">合成口播音频（TTS）</h3>
                </div>
              </div>

              <div class="p-6 space-y-4">
                <!-- 音色 + 语速 -->
                <div class="grid grid-cols-3 gap-3">
                  <div class="col-span-2">
                    <label class="block text-xs font-medium text-gray-400 mb-2">音色</label>
                    <div class="flex gap-2">
                      <select v-model="ttsVoiceId" :disabled="isSynthesizingVoice"
                              class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
                        <option value="">— 请选择音色 —</option>
                        <option v-for="v in availableVoices" :key="v.id" :value="v.id">
                          {{ v.name }} ({{ v.id }})
                        </option>
                      </select>
                      <button @click="loadVoices" :disabled="isLoadingVoices"
                              class="px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded-lg border border-gray-700 flex items-center gap-1"
                              title="刷新音色列表">
                        <Loader2 v-if="isLoadingVoices" class="w-3.5 h-3.5 animate-spin" />
                        <RefreshCw v-else class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                  <div>
                    <label class="block text-xs font-medium text-gray-400 mb-2">语速</label>
                    <input v-model.number="ttsSpeed" type="number" step="0.05" min="0.5" max="2.0"
                           :disabled="isSynthesizingVoice"
                           class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200" />
                  </div>
                </div>

                <!-- 已合成的音频 -->
                <div v-if="latestVoiceoverPath"
                     class="p-3 bg-purple-950/20 border border-purple-500/30 rounded-xl">
                  <div class="flex items-center gap-3 mb-2">
                    <Music class="w-4 h-4 text-purple-400" />
                    <span class="text-xs font-bold text-purple-300">已合成口播音频</span>
                    <span class="text-[10px] text-gray-600 font-mono ml-auto">{{ latestVoiceoverPath.split('/').pop() }}</span>
                  </div>
                  <audio :src="convertFileSrc(latestVoiceoverPath)" controls
                         class="w-full"></audio>
                </div>

                <!-- 合成按钮 -->
                <div class="flex items-center justify-between gap-4 pt-2">
                  <p class="text-[11px] text-gray-500 flex-1 leading-relaxed">
                    将口播脚本里的文本送入 TTS Provider 合成为旁白音频。后续会用此音频时长决定素材轮播节奏（P5 完成后启用全自动合成）。
                  </p>
                  <button @click="synthesizeVoice"
                          :disabled="isSynthesizingVoice || !ttsVoiceId || !generatedScript"
                          class="bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white px-6 py-2.5 rounded-xl font-bold text-sm flex items-center gap-2 shadow-lg shadow-purple-900/30 flex-shrink-0">
                    <Loader2 v-if="isSynthesizingVoice" class="w-4 h-4 animate-spin" />
                    <Music v-else class="w-4 h-4" />
                    {{ isSynthesizingVoice ? '合成中...' : (latestVoiceoverPath ? '重新合成' : '合成口播音频') }}
                  </button>
                </div>

                <!-- 最终合成按钮 (P5) -->
                <div v-if="latestVoiceoverPath" class="pt-4 mt-4 border-t border-purple-500/20 flex items-center justify-between gap-4">
                  <div class="flex-1">
                    <h4 class="text-xs font-bold text-purple-300 mb-1">最后一步：合成视频</h4>
                    <p class="text-[10px] text-gray-500">将上方音频与素材库中所有图片/视频按比例自动对齐拼接。</p>
                  </div>
                  <button @click="startVoiceoverRender"
                          :disabled="isRenderingVoiceover"
                          class="bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 disabled:opacity-50 text-white px-6 py-3 rounded-xl font-bold text-sm flex items-center gap-2 shadow-lg shadow-indigo-900/30">
                    <Loader2 v-if="isRenderingVoiceover" class="w-4 h-4 animate-spin" />
                    <Film v-else class="w-4 h-4" />
                    {{ isRenderingVoiceover ? '正在合成成片...' : '合成最终视频' }}
                  </button>
                </div>
              </div>
            </div>

            <!-- 提示 -->
            <div v-if="!generatedScript" class="p-5 bg-blue-950/10 border border-blue-900/20 rounded-2xl flex gap-4">
              <div class="p-3 bg-blue-600/20 rounded-xl h-fit"><Zap class="w-5 h-5 text-blue-400" /></div>
              <div>
                <h4 class="font-bold text-blue-200 text-sm">工作流说明</h4>
                <p class="text-xs text-blue-400/80 mt-1 leading-relaxed">
                  填写产品 → AI 会两次检索企业知识库（一次品牌资料、一次综合检索）→ 用 AI 助理同一个 LLM 生成口播脚本 →
                  你可以多次反馈重生成 → 最后确认进入视频生成。
                </p>
              </div>
            </div>
          </div>

          <!-- Tab 2: 素材库 -->
          <div v-if="activeTab === 'material'" class="space-y-6 animate-in fade-in slide-in-from-bottom-2">
            <!-- 上传操作栏 -->
            <div class="bg-gray-900 border border-gray-800 rounded-2xl p-5 flex items-center justify-between flex-wrap gap-3">
              <div class="flex items-center gap-3">
                <div class="p-2.5 bg-blue-600/10 rounded-xl border border-blue-500/20">
                  <Upload class="w-5 h-5 text-blue-400" />
                </div>
                <div>
                  <h3 class="text-sm font-bold text-gray-200">素材库</h3>
                  <p class="text-[11px] text-gray-500 mt-0.5">
                    上传本地文件或用 AI 生成；素材用于后期合成或作为视频参考图
                  </p>
                </div>
              </div>
              <div class="flex gap-2 flex-wrap">
                <button
                  @click="openImageGenModal"
                  class="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2 shadow-lg shadow-purple-900/30"
                >
                  <Sparkles class="w-3.5 h-3.5" />
                  AI 生成图片
                </button>
                <button
                  @click="uploadMaterial('image')"
                  :disabled="isUploadingMaterial"
                  class="px-4 py-2 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 text-white text-xs font-medium rounded-lg border border-gray-700 flex items-center gap-2"
                >
                  <Loader2 v-if="isUploadingMaterial" class="w-3.5 h-3.5 animate-spin" />
                  <ImageIcon v-else class="w-3.5 h-3.5" />
                  上传图片
                </button>
                <button
                  @click="uploadMaterial('video')"
                  :disabled="isUploadingMaterial"
                  class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2"
                >
                  <Loader2 v-if="isUploadingMaterial" class="w-3.5 h-3.5 animate-spin" />
                  <Film v-else class="w-3.5 h-3.5" />
                  上传视频
                </button>
              <!-- 素材库：图片 / 视频 / 音频 三区分离 -->
                          <div class="space-y-10">

                            <!-- 图片素材 -->
                            <div>
                              <div class="flex items-center justify-between mb-4">
                                <h3 class="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                                  <ImageIcon class="w-4 h-4" /> 图片 ({{ imageMaterials.length }})
                                </h3>
                                <button @click="uploadMaterial('image')" :disabled="isUploadingMaterial"
                                        class="px-3 py-1.5 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 text-white text-xs font-medium rounded-lg border border-gray-700 flex items-center gap-2">
                                  <Loader2 v-if="isUploadingMaterial" class="w-3 h-3 animate-spin" />
                                  <Upload v-else class="w-3 h-3" /> 上传图片
                                </button>
                              </div>
                              <div v-if="imageMaterials.length > 0" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
                                <div v-for="m in imageMaterials" :key="m.id" class="group bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden hover:border-gray-600 transition-all shadow-xl">
                                  <div class="aspect-[9/16] bg-black relative">
                                    <img v-if="m.local_path" :src="convertFileSrc(m.local_path)" class="w-full h-full object-cover" alt="素材" />
                                    <div v-else class="w-full h-full flex flex-col items-center justify-center gap-3">
                                      <Loader2 class="w-8 h-8 text-gray-800 animate-spin" />
                                      <span class="text-[10px] text-gray-700 uppercase">等待本地化</span>
                                    </div>
                                    <div class="absolute top-2 left-2 px-2 py-0.5 bg-black/60 backdrop-blur-md rounded text-[9px] uppercase tracking-wider text-white font-bold">图片</div>
                                    <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-all p-3 flex flex-col justify-end">
                                      <div class="flex gap-2">
                                        <button @click="openPreview(m)" class="flex-1 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-2">
                                          <Play class="w-3 h-3" /> 预览
                                        </button>
                                        <button @click="deleteMaterial(m.id)" class="bg-red-500/80 hover:bg-red-500 text-white py-2 px-2 rounded-lg text-xs font-bold flex items-center justify-center" title="删除">
                                          <Trash2 class="w-3 h-3" />
                                        </button>
                                      </div>
                                    </div>
                                  </div>
                                </div>
                              </div>
                              <div v-else class="py-12 flex flex-col items-center justify-center border-2 border-dashed border-gray-900 rounded-2xl">
                                <ImageIcon class="w-10 h-10 text-gray-800 mb-3" />
                                <p class="text-xs text-gray-600">暂无图片素材</p>
                              </div>
                            </div>

                            <!-- 视频素材 -->
                            <div>
                              <div class="flex items-center justify-between mb-4">
                                <h3 class="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                                  <Film class="w-4 h-4" /> 视频 ({{ videoMaterials.length }})
                                </h3>
                                <button @click="uploadMaterial('video')" :disabled="isUploadingMaterial"
                                        class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2">
                                  <Loader2 v-if="isUploadingMaterial" class="w-3 h-3 animate-spin" />
                                  <Upload v-else class="w-3 h-3" /> 上传视频
                                </button>
                              </div>
                              <div v-if="videoMaterials.length > 0" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
                                <div v-for="m in videoMaterials" :key="m.id" class="group bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden hover:border-gray-600 transition-all shadow-xl">
                                  <div class="aspect-[9/16] bg-black relative">
                                    <video v-if="m.local_path" :src="convertFileSrc(m.local_path)" class="w-full h-full object-cover" muted loop onmouseover="this.play()" onmouseout="this.pause()"></video>
                                    <div v-else class="w-full h-full flex flex-col items-center justify-center gap-3">
                                      <Loader2 class="w-8 h-8 text-gray-800 animate-spin" />
                                      <span class="text-[10px] text-gray-700 uppercase">等待本地化</span>
                                    </div>
                                    <div class="absolute top-2 left-2 px-2 py-0.5 bg-black/60 backdrop-blur-md rounded text-[9px] uppercase tracking-wider text-white font-bold">视频</div>
                                    <div v-if="m.source === 'ai-generated'" class="absolute top-2 right-2 px-1.5 py-0.5 bg-gradient-to-r from-purple-600 to-pink-600 rounded text-[9px] uppercase tracking-wider text-white font-bold flex items-center gap-1 shadow-lg">
                                      <Sparkles class="w-2.5 h-2.5" /> AI
                                    </div>
                                    <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-all p-3 flex flex-col justify-end">
                                      <div class="flex gap-2">
                                        <button @click="openPreview(m)" class="flex-1 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-2">
                                          <Play class="w-3 h-3" /> 预览
                                        </button>
                                        <button @click="deleteMaterial(m.id)" class="bg-red-500/80 hover:bg-red-500 text-white py-2 px-2 rounded-lg text-xs font-bold flex items-center justify-center" title="删除">
                                          <Trash2 class="w-3 h-3" />
                                        </button>
                                      </div>
                                    </div>
                                  </div>
                                </div>
                              </div>
                              <div v-else class="py-12 flex flex-col items-center justify-center border-2 border-dashed border-gray-900 rounded-2xl">
                                <Film class="w-10 h-10 text-gray-800 mb-3" />
                                <p class="text-xs text-gray-600">暂无视频素材</p>
                              </div>
                            </div>

                            <!-- 音频素材 -->
                            <div>
                              <div class="flex items-center justify-between mb-4">
                                <h3 class="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                                  <Music class="w-4 h-4" /> 音频 ({{ audioMaterials.length }})
                                </h3>
                                <button @click="uploadMaterial('audio')" :disabled="isUploadingMaterial"
                                        class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2">
                                  <Loader2 v-if="isUploadingMaterial" class="w-3 h-3 animate-spin" />
                                  <Upload v-else class="w-3 h-3" /> 上传音频
                                </button>
                              </div>
                              <div v-if="audioMaterials.length > 0" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                <div v-for="m in audioMaterials" :key="m.id" class="group bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden hover:border-gray-600 transition-all shadow-xl">
                                  <div class="p-4 flex items-center gap-4">
                                    <div class="w-12 h-12 rounded-xl bg-purple-600/20 flex items-center justify-center flex-shrink-0">
                                      <Music class="w-6 h-6 text-purple-400" />
                                    </div>
                                    <div class="flex-1 min-w-0">
                                      <div class="text-xs text-gray-300 font-medium truncate">{{ m.id.slice(0, 8) }}.{{ m.meta?.format || 'audio' }}</div>
                                      <div class="text-[10px] text-gray-600 mt-0.5">音频素材</div>
                                    </div>
                                    <div class="flex gap-2 flex-shrink-0">
                                      <button @click="openPreview(m)" class="w-8 h-8 bg-white/10 hover:bg-white/20 rounded-lg flex items-center justify-center text-white" title="预览">
                                        <Play class="w-3.5 h-3.5" />
                                      </button>
                                      <button @click="deleteMaterial(m.id)" class="w-8 h-8 bg-red-500/80 hover:bg-red-500 rounded-lg flex items-center justify-center text-white" title="删除">
                                        <Trash2 class="w-3.5 h-3.5" />
                                      </button>
                                    </div>
                                  </div>
                                </div>
                              </div>
                              <div v-else class="py-12 flex flex-col items-center justify-center border-2 border-dashed border-gray-900 rounded-2xl">
                                <Music class="w-10 h-10 text-gray-800 mb-3" />
                                <p class="text-xs text-gray-600">暂无音频素材</p>
                              </div>
                            </div>

                          </div>

                          <!-- 任务列表（保留） -->
                          <div v-if="Object.keys(activeTasks).length > 0" class="mt-10">
                            <h3 class="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">进行中的任务</h3>
                            <div class="space-y-3">
                              <div v-for="t in activeTasks" :key="t.id" class="bg-gray-900 border border-blue-800/30 rounded-xl p-4 flex items-center gap-4">
                                <div class="w-10 h-10 rounded-full bg-blue-600/20 flex items-center justify-center flex-shrink-0">
                                  <Loader2 class="w-6 h-6 text-blue-500 animate-spin" />
                                </div>
                                <div class="flex-1">
                                  <div class="flex justify-between items-center mb-2">
                                    <span class="text-xs font-bold text-gray-400 uppercase tracking-widest">{{ t.task_type === 'generation' ? 'AI 视频生成' : (t.task_type === 'voiceover' ? '口播合成' : '导出合成') }}</span>
                                    <span class="text-[10px] text-blue-500 font-mono">{{ t.id.slice(0, 8) }}</span>
                                  </div>
                                  <div class="h-1.5 bg-gray-800 rounded-full overflow-hidden">
                                    <div class="h-full bg-blue-500 animate-pulse transition-all duration-500" style="width: 100%"></div>
                                  </div>
                                  <p class="text-[11px] text-gray-500 mt-2">
                                    {{ t.task_type === 'generation' ? '正在调用 AI 引擎生成画面...' : 'FFmpeg 任务排队中...' }}
                                  </p>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
          <!-- Tab 3: 后期导出 -->
          <div v-if="activeTab === 'export'" class="max-w-6xl mx-auto animate-in fade-in slide-in-from-bottom-2">
            <!-- 页面标题 -->
            <div class="mb-8">
              <h2 class="text-lg font-bold text-white mb-1">视频合成导出</h2>
              <p class="text-xs text-gray-500">选择一个音频 + 多张图片/视频，按照音频时长自动拼接轮播</p>
            </div>

            <div class="grid grid-cols-3 gap-10">
              <!-- 左侧：素材选择 -->
              <div class="col-span-2 space-y-8">
                <!-- 音频选择（必选） -->
                <div class="bg-gray-900 rounded-3xl p-8 border border-gray-800 shadow-2xl">
                  <h3 class="text-sm font-bold text-purple-400 uppercase tracking-widest flex items-center gap-2 mb-6">
                    <Music class="w-5 h-5" /> 主音频 <span class="text-red-400 text-xs">* 必选</span>
                  </h3>
                  <div v-if="audioMaterials.length === 0" class="py-8 flex flex-col items-center justify-center border-2 border-dashed border-gray-800 rounded-2xl">
                    <Music class="w-10 h-10 text-gray-800 mb-3" />
                    <p class="text-xs text-gray-600 mb-3">素材库中暂无音频</p>
                    <button @click="activeTab = 'material'" class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white text-xs font-bold rounded-lg">去上传音频</button>
                  </div>
                  <div v-else class="grid grid-cols-2 gap-3">
                    <button
                      v-for="a in audioMaterials" :key="a.id"
                      @click="exportSelectedAudio = a.id"
                      :class="['p-4 rounded-2xl border text-left transition-all flex items-center gap-3', exportSelectedAudio === a.id ? 'bg-purple-950/40 border-purple-500' : 'bg-gray-950 border-gray-800 hover:border-purple-500/50']"
                    >
                      <div class="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0"
                           :class="exportSelectedAudio === a.id ? 'bg-purple-600/30' : 'bg-gray-800'">
                        <Music :class="['w-5 h-5', exportSelectedAudio === a.id ? 'text-purple-400' : 'text-gray-500']" />
                      </div>
                      <div class="flex-1 min-w-0">
                        <div class="text-sm font-medium text-gray-200 truncate">音频 {{ a.id.slice(0, 8) }}</div>
                        <div class="text-[10px] text-gray-500">{{ a.meta?.format || 'audio' }}</div>
                      </div>
                      <CheckCircle2 v-if="exportSelectedAudio === a.id" class="w-5 h-5 text-purple-400 flex-shrink-0" />
                    </button>
                  </div>
                </div>

                <!-- 图片选择（多选） -->
                <div class="bg-gray-900 rounded-3xl p-8 border border-gray-800 shadow-2xl">
                  <h3 class="text-sm font-bold text-blue-400 uppercase tracking-widest flex items-center gap-2 mb-6">
                    <ImageIcon class="w-5 h-5" /> 图片素材 <span class="text-gray-500 text-xs normal-case font-normal">可多选，图片超过3秒自动切换</span>
                  </h3>
                  <div v-if="imageMaterials.length === 0" class="py-8 flex flex-col items-center justify-center border-2 border-dashed border-gray-800 rounded-2xl">
                    <ImageIcon class="w-10 h-10 text-gray-800 mb-3" />
                    <p class="text-xs text-gray-600 mb-3">素材库中暂无图片</p>
                    <button @click="activeTab = 'material'" class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold rounded-lg">去上传图片</button>
                  </div>
                  <div v-else class="grid grid-cols-3 gap-3">
                    <button
                      v-for="img in imageMaterials" :key="img.id"
                      @click="toggleExportImage(img.id)"
                      :class="['relative rounded-xl overflow-hidden border-2 transition-all aspect-[9/16]', exportSelectedImages.includes(img.id) ? 'border-blue-500 ring-2 ring-blue-500/30' : 'border-gray-800 hover:border-gray-600']"
                    >
                      <img v-if="img.local_path" :src="convertFileSrc(img.local_path)" class="w-full h-full object-cover" alt="" />
                      <div v-if="exportSelectedImages.includes(img.id)" class="absolute top-2 right-2 w-6 h-6 rounded-full bg-blue-500 flex items-center justify-center">
                        <CheckCircle2 class="w-4 h-4 text-white" />
                      </div>
                    </button>
                  </div>
                  <div v-if="exportSelectedImages.length > 0" class="mt-3 flex justify-between items-center">
                    <span class="text-xs text-gray-500">已选 {{ exportSelectedImages.length }} 张图片</span>
                    <button @click="exportSelectedImages = []" class="text-xs text-gray-500 hover:text-red-400">清除选择</button>
                  </div>
                </div>

                <!-- 视频选择（多选） -->
                <div class="bg-gray-900 rounded-3xl p-8 border border-gray-800 shadow-2xl">
                  <h3 class="text-sm font-bold text-green-400 uppercase tracking-widest flex items-center gap-2 mb-6">
                    <Film class="w-5 h-5" /> 视频素材 <span class="text-gray-500 text-xs normal-case font-normal">可多选，循环播放</span>
                  </h3>
                  <div v-if="videoMaterials.length === 0" class="py-8 flex flex-col items-center justify-center border-2 border-dashed border-gray-800 rounded-2xl">
                    <Film class="w-10 h-10 text-gray-800 mb-3" />
                    <p class="text-xs text-gray-600 mb-3">素材库中暂无视频</p>
                    <button @click="activeTab = 'material'" class="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-xs font-bold rounded-lg">去上传视频</button>
                  </div>
                  <div v-else class="grid grid-cols-3 gap-3">
                    <button
                      v-for="vid in videoMaterials" :key="vid.id"
                      @click="toggleExportVideo(vid.id)"
                      :class="['relative rounded-xl overflow-hidden border-2 transition-all aspect-[9/16]', exportSelectedVideos.includes(vid.id) ? 'border-green-500 ring-2 ring-green-500/30' : 'border-gray-800 hover:border-gray-600']"
                    >
                      <video v-if="vid.local_path" :src="convertFileSrc(vid.local_path)" class="w-full h-full object-cover" muted loop />
                      <div v-if="exportSelectedVideos.includes(vid.id)" class="absolute top-2 right-2 w-6 h-6 rounded-full bg-green-500 flex items-center justify-center">
                        <CheckCircle2 class="w-4 h-4 text-white" />
                      </div>
                    </button>
                  </div>
                  <div v-if="exportSelectedVideos.length > 0" class="mt-3 flex justify-between items-center">
                    <span class="text-xs text-gray-500">已选 {{ exportSelectedVideos.length }} 个视频片段</span>
                    <button @click="exportSelectedVideos = []" class="text-xs text-gray-500 hover:text-red-400">清除选择</button>
                  </div>
                </div>
              </div>

              <!-- 右侧：预览与确认 -->
              <div class="space-y-6">
                <!-- 已选素材汇总 -->
                <div class="bg-gray-900 rounded-2xl border border-gray-800 p-6 shadow-xl">
                  <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">已选素材</h4>
                  <div class="space-y-3">
                    <div class="flex justify-between py-2 border-b border-gray-800/50">
                      <span class="text-xs text-gray-500 flex items-center gap-1.5"><Music class="w-3 h-3 text-purple-400" /> 主音频</span>
                      <span class="text-xs" :class="exportSelectedAudio ? 'text-white' : 'text-gray-600'">{{ exportSelectedAudio ? '已选' : '未选' }}</span>
                    </div>
                    <div class="flex justify-between py-2 border-b border-gray-800/50">
                      <span class="text-xs text-gray-500 flex items-center gap-1.5"><ImageIcon class="w-3 h-3 text-blue-400" /> 图片</span>
                      <span class="text-xs text-white">{{ exportSelectedImages.length }} 张</span>
                    </div>
                    <div class="flex justify-between py-2 border-b border-gray-800/50">
                      <span class="text-xs text-gray-500 flex items-center gap-1.5"><Film class="w-3 h-3 text-green-400" /> 视频</span>
                      <span class="text-xs text-white">{{ exportSelectedVideos.length }} 段</span>
                    </div>
                  </div>
                </div>

                <!-- 提示 -->
                <div class="p-4 bg-purple-950/20 border border-purple-900/30 rounded-xl">
                  <p class="text-[10px] text-purple-400 leading-relaxed">
                    <strong>合成规则：</strong>音频时长达视频总时长；图片每张最长展示3秒，超时自动切到下一张；素材总时长不够则循环播放。
                  </p>
                </div>

                <!-- 合成按钮 -->
                <button
                  @click="startExportRender"
                  :disabled="!exportSelectedAudio || (exportSelectedImages.length === 0 && exportSelectedVideos.length === 0)"
                  class="w-full bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white py-4 rounded-xl text-sm font-bold transition-all shadow-lg shadow-purple-900/30 flex items-center justify-center gap-2"
                >
                  <Film class="w-5 h-5" />
                  开始合成视频
                </button>
              </div>
            </div>
          </div>
      </div>
    </div>

    <!-- 无项目展示 -->
    <div v-else class="flex-1 flex flex-col items-center justify-center bg-gray-950">
      <div class="p-8 rounded-full bg-gray-900/50 mb-6">
        <Film class="w-16 h-16 text-gray-700" />
      </div>
      <h3 class="text-xl font-bold text-gray-300">选择或创建一个项目开始</h3>
      <p class="text-gray-500 mt-2">在这里，您可以一键生成 AI 视频并进行专业剪辑</p>
      <button @click="createProject" class="mt-8 bg-blue-600 hover:bg-blue-700 text-white px-8 py-3 rounded-xl font-bold transition-all shadow-xl shadow-blue-900/30">
        创建第一个项目
      </button>
    </div>
  </div>

  <!-- ===== AI 图片生成 Modal ===== -->
  <div v-if="showImageGenModal"
       @click.self="!isGeneratingImage && (showImageGenModal = false)"
       class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200">
    <div class="bg-gray-900 border border-purple-500/30 rounded-2xl w-full max-w-2xl flex flex-col">
      <div class="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-100 flex items-center gap-2">
          <Sparkles class="w-4 h-4 text-purple-400" /> AI 生成图片素材
        </h3>
        <button @click="!isGeneratingImage && (showImageGenModal = false)" class="text-gray-500 hover:text-white">
          <XCircle class="w-5 h-5" />
        </button>
      </div>

      <div class="p-6 space-y-5 max-h-[70vh] overflow-y-auto custom-scrollbar">
        <!-- 提示词 -->
        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">
            提示词 <span class="text-red-400">*</span>
          </label>
          <textarea
            v-model="imageGenPrompt"
            :disabled="isGeneratingImage"
            placeholder="例如：极简白色背景下，一支花西子空气蜜粉气垫斜放，柔光，电商主图风格..."
            class="w-full h-28 p-3 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-purple-500 resize-none disabled:opacity-60"
          ></textarea>
        </div>

        <!-- 参考图 -->
        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">
            参考图 <span class="text-gray-600">（可选，用于图生图）</span>
          </label>
          <div v-if="imageGenRefPath" class="flex items-center gap-3 p-3 bg-gray-950 border border-gray-800 rounded-xl">
            <img :src="convertFileSrc(imageGenRefPath)" class="w-16 h-16 object-cover rounded border border-gray-700" alt="" />
            <div class="flex-1 text-[10px] text-gray-500 truncate font-mono">{{ imageGenRefPath }}</div>
            <button @click="imageGenRefPath = ''" :disabled="isGeneratingImage" class="text-gray-500 hover:text-red-400 text-xs">
              移除
            </button>
          </div>
          <button v-else
                  @click="pickImageGenReference"
                  :disabled="isGeneratingImage"
                  class="w-full py-3 bg-gray-950 hover:bg-gray-800 disabled:opacity-50 text-gray-400 border border-dashed border-gray-700 hover:border-gray-600 rounded-xl text-sm flex items-center justify-center gap-2">
            <Upload class="w-4 h-4" /> 选择本地参考图
          </button>
        </div>

        <!-- Provider + Size -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs font-medium text-gray-400 mb-2">Provider</label>
            <select v-model="imageGenProvider" :disabled="isGeneratingImage"
                    class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
              <option value="mock">测试模拟 (Mock)</option>
              <option value="fal">fal.ai FLUX</option>
              <option value="openai">OpenAI 兼容</option>
              <option value="volcengine">火山引擎（待接入）</option>
            </select>
          </div>
          <div>
            <label class="block text-xs font-medium text-gray-400 mb-2">尺寸</label>
            <select v-model="imageGenSize" :disabled="isGeneratingImage"
                    class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
              <option v-for="s in IMAGE_SIZE_PRESETS" :key="s.id" :value="s.id">{{ s.label }}</option>
            </select>
          </div>
        </div>
      </div>

      <div class="p-6 border-t border-gray-800 flex justify-end gap-2 bg-gray-950/40">
        <button
          @click="showImageGenModal = false"
          :disabled="isGeneratingImage"
          class="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded-lg border border-gray-700"
        >取消</button>
        <button
          @click="generateImageMaterial"
          :disabled="isGeneratingImage || !imageGenPrompt.trim()"
          class="px-5 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 disabled:opacity-50 text-white text-xs font-bold rounded-lg flex items-center gap-2 shadow-lg shadow-purple-900/30"
        >
          <Loader2 v-if="isGeneratingImage" class="w-3.5 h-3.5 animate-spin" />
          <Sparkles v-else class="w-3.5 h-3.5" />
          {{ isGeneratingImage ? 'AI 生成中...' : '生成' }}
        </button>
      </div>
    </div>
  </div>

  <!-- ===== 参考图选择 Modal ===== -->
  <div v-if="showReferencePicker"
       @click.self="showReferencePicker = false"
       class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200">
    <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-3xl max-h-[80vh] flex flex-col">
      <div class="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-200 flex items-center gap-2">
          <ImageIcon class="w-4 h-4 text-blue-400" /> 选择产品参考图
        </h3>
        <button @click="showReferencePicker = false" class="text-gray-500 hover:text-white">
          <XCircle class="w-5 h-5" />
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-6 custom-scrollbar">
        <div v-if="availableReferenceImages.length === 0" class="py-16 text-center">
          <ImageIcon class="w-12 h-12 text-gray-700 mx-auto mb-3" />
          <p class="text-sm text-gray-500 mb-3">当前项目还没有图片素材</p>
          <button @click="showReferencePicker = false; activeTab = 'material'"
                  class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs rounded-lg">
            去素材库上传
          </button>
        </div>
        <div v-else class="grid grid-cols-3 md:grid-cols-4 gap-3">
          <button
            v-for="m in availableReferenceImages"
            :key="m.id"
            @click="pickReferenceImage(m.id)"
            :class="[
              'group relative aspect-square rounded-xl overflow-hidden border-2 transition-all',
              referenceImageId === m.id ? 'border-blue-500 shadow-lg shadow-blue-900/40' : 'border-gray-800 hover:border-gray-600'
            ]"
          >
            <img v-if="m.local_path" :src="convertFileSrc(m.local_path)" class="w-full h-full object-cover" alt="" />
            <div class="absolute inset-0 bg-black/0 group-hover:bg-black/30 transition-colors flex items-end p-2">
              <span class="text-[9px] text-white/80 font-mono">{{ m.id.slice(0, 8) }}</span>
            </div>
            <div v-if="referenceImageId === m.id"
                 class="absolute top-2 right-2 w-6 h-6 rounded-full bg-blue-500 text-white flex items-center justify-center">
              <CheckCircle2 class="w-4 h-4" />
            </div>
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- ===== 无参考图警告 Modal ===== -->
  <div v-if="showNoReferenceWarning"
       @click.self="showNoReferenceWarning = false"
       class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200">
    <div class="bg-gray-900 border border-amber-500/30 rounded-2xl max-w-md w-full">
      <div class="p-6">
        <div class="flex items-start gap-3 mb-4">
          <div class="p-2.5 bg-amber-500/15 rounded-xl border border-amber-500/30 flex-shrink-0">
            <Zap class="w-5 h-5 text-amber-400" />
          </div>
          <div>
            <h3 class="text-base font-bold text-white">未提供产品参考图</h3>
            <p class="text-xs text-gray-400 mt-1">AI 视频生成将仅依赖文字描述</p>
          </div>
        </div>
        <div class="text-sm text-gray-300 leading-relaxed bg-amber-950/20 border border-amber-900/30 rounded-xl p-4 mb-5">
          没有参考图时，AI 模型只能根据脚本里的文字生成画面，<span class="text-amber-300 font-bold">很可能与你的实际产品长相不符</span>（颜色/形状/包装/Logo 都会随机）。
          强烈建议先去素材库上传一张产品图。
        </div>
        <div class="flex justify-end gap-2">
          <button @click="showNoReferenceWarning = false; showReferencePicker = true"
                  class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold rounded-lg">
            去选参考图
          </button>
          <button @click="ackNoReferenceWarning"
                  class="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-400 text-xs rounded-lg border border-gray-700">
            我了解，继续生成
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- ===== 素材预览 Modal ===== -->
  <div
    v-if="previewMaterial"
    @click.self="() => { if (!previewCloseDebounce) { previewCloseDebounce = true; closePreview(); setTimeout(() => previewCloseDebounce = false, 300); } }"
    @keydown.esc="() => { if (!previewCloseDebounce) { previewCloseDebounce = true; closePreview(); setTimeout(() => previewCloseDebounce = false, 300); } }"
    @wheel="handlePreviewWheel"
    @mousemove="onDrag"
    @mouseup="endDrag"
    @mouseleave="endDrag"
    tabindex="0"
    class="fixed inset-0 z-50 bg-black/90 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200 overflow-hidden select-none"
    ref="previewModalRef"
  >
    <!-- 顶部信息条 -->
    <div class="absolute top-6 left-6 text-xs text-white/60 font-mono">
      {{ previewMaterial.material_type?.toUpperCase() }} · {{ previewMaterial.id.slice(0, 8) }}
    </div>

    <!-- 关闭 -->
    <button
      @click="() => { if (!previewCloseDebounce) { previewCloseDebounce = true; closePreview(); setTimeout(() => previewCloseDebounce = false, 300); } }"
      class="absolute top-6 right-6 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 text-white flex items-center justify-center transition-colors z-10"
      title="关闭 (Esc)"
    >
      <XCircle class="w-6 h-6" />
    </button>

    <!-- 媒体内容 -->
    <div
      class="relative max-w-[90vw] max-h-[85vh] flex items-center justify-center"
      @click.stop
    >
      <video
        v-if="previewMaterial.material_type === 'video' && previewMaterial.local_path"
        :src="convertFileSrc(previewMaterial.local_path)"
        controls
        autoplay
        class="max-w-full max-h-[85vh] rounded-xl shadow-2xl"
      ></video>
      <audio
        v-else-if="previewMaterial.material_type === 'audio' && previewMaterial.local_path"
        :src="convertFileSrc(previewMaterial.local_path)"
        controls
        autoplay
        class="max-w-full rounded-xl shadow-2xl"
      /></audio>
      <img
        v-else-if="previewMaterial.material_type === 'image' && previewMaterial.local_path"
        :src="convertFileSrc(previewMaterial.local_path)"
        class="max-w-full max-h-[85vh] object-contain rounded-xl shadow-2xl"
        :style="{
          transform: `translate(${previewOffset.x}px, ${previewOffset.y}px) scale(${previewZoom})`,
          cursor: previewZoom > 1 ? (isDragging ? 'grabbing' : 'grab') : 'default',
          transition: isDragging ? 'none' : 'transform 0.12s ease-out',
        }"
        @mousedown="startDrag"
        @dblclick="previewZoom === 1 ? (previewZoom = 2) : resetZoom()"
        draggable="false"
        alt="预览"
      />
    </div>

    <!-- 图片缩放工具栏（仅图片显示） -->
    <div
      v-if="previewMaterial.material_type === 'image'"
      class="absolute bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-1 bg-black/60 backdrop-blur-md border border-white/10 rounded-full px-2 py-1.5 z-10"
    >
      <button
        @click="zoomOut"
        :disabled="previewZoom <= MIN_ZOOM"
        class="w-9 h-9 rounded-full hover:bg-white/10 disabled:opacity-30 disabled:cursor-not-allowed text-white flex items-center justify-center text-xl font-bold"
        title="缩小 (滚轮 / -)"
      >−</button>
      <button
        @click="resetZoom"
        class="px-3 h-9 rounded-full hover:bg-white/10 text-white text-xs font-mono tabular-nums min-w-[70px]"
        :title="`重置缩放（双击图片也可重置）`"
      >{{ Math.round(previewZoom * 100) }}%</button>
      <button
        @click="zoomIn"
        :disabled="previewZoom >= MAX_ZOOM"
        class="w-9 h-9 rounded-full hover:bg-white/10 disabled:opacity-30 disabled:cursor-not-allowed text-white flex items-center justify-center text-xl font-bold"
        title="放大 (滚轮 / +)"
      >+</button>
    </div>
  </div>
</div>
</template>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1e293b;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #334155;
}

/* ── Markdown 脚本预览样式（v-html 渲染的内容用 :deep 透传） ── */
:deep(.script-markdown) {
  font-size: 0.875rem;
  line-height: 1.7;
}
:deep(.script-markdown h1),
:deep(.script-markdown h2),
:deep(.script-markdown h3) {
  color: #e5e7eb;
  font-weight: 700;
  margin: 1.2em 0 0.5em;
}
:deep(.script-markdown h1) { font-size: 1.25rem; }
:deep(.script-markdown h2) { font-size: 1.125rem; }
:deep(.script-markdown h3) { font-size: 1rem; }
:deep(.script-markdown h1:first-child),
:deep(.script-markdown h2:first-child),
:deep(.script-markdown h3:first-child) { margin-top: 0; }
:deep(.script-markdown p) {
  margin: 0.6em 0;
  color: #d1d5db;
}
:deep(.script-markdown strong) { color: #fbbf24; font-weight: 600; }
:deep(.script-markdown em) { color: #93c5fd; }
:deep(.script-markdown code) {
  background: #1f2937;
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-size: 0.85em;
  color: #fbbf24;
}
:deep(.script-markdown pre) {
  background: #0f172a;
  border: 1px solid #1e293b;
  border-radius: 8px;
  padding: 1em;
  overflow-x: auto;
  margin: 0.8em 0;
}
:deep(.script-markdown pre code) {
  background: transparent;
  padding: 0;
}
:deep(.script-markdown ul),
:deep(.script-markdown ol) {
  padding-left: 1.5em;
  margin: 0.5em 0;
}
:deep(.script-markdown li) {
  margin: 0.3em 0;
  color: #d1d5db;
}
:deep(.script-markdown blockquote) {
  border-left: 3px solid #3b82f6;
  padding-left: 1em;
  margin: 0.8em 0;
  color: #9ca3af;
  font-style: italic;
}
:deep(.script-markdown table) {
  border-collapse: collapse;
  margin: 0.8em 0;
  width: 100%;
}
:deep(.script-markdown th),
:deep(.script-markdown td) {
  border: 1px solid #374151;
  padding: 0.4em 0.8em;
  text-align: left;
}
:deep(.script-markdown th) {
  background: #1f2937;
  font-weight: 600;
}
:deep(.script-markdown hr) {
  border: none;
  border-top: 1px solid #374151;
  margin: 1em 0;
}
</style>
