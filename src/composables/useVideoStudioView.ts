import { ref, onMounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

import type { VideoMaterial } from '../types/video-studio';
import { useVideoProjects, useVideoMaterials } from './useVideoStudio';
import { useVideoTasks } from './useVideoTasks';
import { useSettings } from './useSettings';

// ============ Constants ============
export const PLATFORM_OPTIONS = [
  { id: 'douyin', label: '抖音', emoji: '🎵', desc: '快节奏，前 3 秒钩子，强互动' },
  { id: 'kuaishou', label: '快手', emoji: '🧡', desc: '接地气，老铁味，高性价比感' },
  { id: 'xiaohongshu', label: '小红书', emoji: '📕', desc: '种草风，美感强，多 emoji 标记' },
  { id: 'wechat-channel', label: '视频号', emoji: '📽️', desc: '朋友圈调性，信任感，叙述稳重' },
];

export const SCRIPT_TYPE_OPTIONS = [
  { id: 'voiceover', label: '口播带货', desc: '生成专业脚本 + TTS 合成旁白 + 素材拼接' },
  { id: 'ai-video', label: 'AI 视频流', desc: '生成脚本 + 分镜提示词 + AI 引擎生成画面' },
];

export const IMAGE_SIZE_PRESETS = [
  { id: '720x1280', label: '竖屏 (9:16)' },
  { id: '1280x720', label: '横屏 (16:9)' },
  { id: '1024x1024', label: '方形 (1:1)' },
];

export function useVideoStudioView() {
  const activeTab = ref<'script' | 'material' | 'export'>('script');
  const { projects, currentProject, loadProjects, createProject, selectProject, deleteProject } = useVideoProjects();
  const { materials, isUploadingMaterial, loadMaterials, uploadMaterial, deleteMaterial } = useVideoMaterials();
  const { activeTasks } = useVideoTasks();
  const { config: appConfig, loadSettings } = useSettings();

  // Material lists
  const audioMaterials = computed(() => materials.value.filter(m => m.material_type === 'audio'));
  const imageMaterials = computed(() => materials.value.filter(m => m.material_type === 'image'));
  const videoMaterials = computed(() => materials.value.filter(m => m.material_type === 'video'));

  // Script State
  const productInfo = ref('');
  const referenceScript = ref('');
  const generatedScript = ref('');
  const scriptFeedback = ref('');
  const isGeneratingScript = ref(false);
  const scriptConfirmed = ref(false);
  const selectedPlatform = ref('douyin');
  const selectedScriptType = ref<'voiceover' | 'ai-video'>('voiceover');
  const videoRatio = ref('9:16');

  // TTS State
  const ttsVoiceId = ref('');
  const ttsSpeed = ref(1.0);
  const isSynthesizingVoice = ref(false);
  const isLoadingVoices = ref(false);
  const availableVoices = ref<any[]>([]);
  const latestVoiceoverPath = ref<string | null>(null);

  // Export State
  const exportSelectedAudio = ref<string | null>(null);
  const exportSelectedImages = ref<string[]>([]);
  const exportSelectedVideos = ref<string[]>([]);
  const isExporting = ref(false);
  const burnSubtitle = ref(false);

  // Modal States
  const showImageGenModal = ref(false);
  const imageGenPrompt = ref('');
  const imageGenRefPath = ref('');
  const imageGenProvider = ref('fal');
  const imageGenSize = ref('720x1280');
  const isGeneratingImage = ref(false);

  const showReferencePicker = ref(false);
  const referenceImageId = ref('');
  const showNoReferenceWarning = ref(false);

  const previewMaterial = ref<VideoMaterial | null>(null);
  const previewZoom = ref(1);
  const previewOffset = ref({ x: 0, y: 0 });
  const isDragging = ref(false);

  // ============ Logic ============
  onMounted(async () => {
    await loadProjects();
    await loadSettings();
  });

  watch(currentProject, async (newVal) => {
    if (newVal) {
      await loadMaterials(newVal.id);
      // Restore script from config
      const cfg = newVal.config?.script || {};
      productInfo.value = cfg.productInfo || '';
      referenceScript.value = cfg.referenceScript || '';
      generatedScript.value = cfg.generatedScript || '';
      scriptConfirmed.value = cfg.scriptConfirmed || false;
      selectedPlatform.value = cfg.selectedPlatform || 'douyin';
      selectedScriptType.value = cfg.selectedScriptType || 'voiceover';
      videoRatio.value = cfg.videoRatio || '9:16';
      // 恢复已确认脚本的项目时，自动加载音色（无需手动刷新）
      if (scriptConfirmed.value) loadVoices();
    }
  });

  const saveProjectConfig = async () => {
    if (!currentProject.value) return;
    const scriptCfg = {
      productInfo: productInfo.value,
      referenceScript: referenceScript.value,
      generatedScript: generatedScript.value,
      scriptConfirmed: scriptConfirmed.value,
      selectedPlatform: selectedPlatform.value,
      selectedScriptType: selectedScriptType.value,
      videoRatio: videoRatio.value,
    };
    currentProject.value.config = { ...currentProject.value.config, script: scriptCfg };
    await invoke('video_upsert_project', { project: currentProject.value });
  };

  const generateScript = async (isFeedback: boolean) => {
    isGeneratingScript.value = true;
    try {
      const script = await invoke<string>('video_generate_script', {
        product: productInfo.value,
        referenceScript: referenceScript.value,
        videoRatio: videoRatio.value,
        platform: selectedPlatform.value,
        scriptType: selectedScriptType.value,
        previousScript: isFeedback ? generatedScript.value : null,
        feedback: isFeedback ? scriptFeedback.value : null,
      });
      generatedScript.value = script;
      scriptConfirmed.value = false;
      await saveProjectConfig();
    } catch (e) {
      alert('生成脚本失败: ' + e);
    } finally {
      isGeneratingScript.value = false;
    }
  };

  const resetScriptFlow = () => {
    if (!confirm('确定要清除当前脚本并重新输入吗？')) return;
    generatedScript.value = '';
    scriptConfirmed.value = false;
    scriptFeedback.value = '';
    saveProjectConfig();
  };

  const confirmScript = () => {
    scriptConfirmed.value = true;
    saveProjectConfig();
    // 进入 TTS 步骤时自动加载音色列表，无需手动刷新
    loadVoices();
  };

  // 用户手动编辑脚本后保存（JSON 字符串）
  const saveScript = (json: string) => {
    generatedScript.value = json;
    saveProjectConfig();
  };

  const loadVoices = async () => {
    isLoadingVoices.value = true;
    try {
      // 先拉最新配置，避免用户在设置页改了音色/Provider 后这里还是旧值
      await loadSettings();
      // 只显示用户在设置页自定义的音色组（不再合并 Provider 内置音色）
      availableVoices.value = (appConfig.value.video.tts_voices || [])
        .filter((v: any) => v.voice_id)
        .map((v: any) => ({ id: v.voice_id, name: v.name || v.voice_id }));

      if (availableVoices.value.length > 0 && !ttsVoiceId.value) {
        const def = appConfig.value.video.default_tts_voice;
        ttsVoiceId.value = (def && availableVoices.value.some((v: any) => v.id === def))
          ? def
          : availableVoices.value[0].id;
      }
    } finally {
      isLoadingVoices.value = false;
    }
  };

  const synthesizeVoice = async () => {
    if (!currentProject.value) return;

    // 优先取「表演脚本」字段用于 TTS，该字段包含语气/声调标注
    // 如果没有，则退而求其次取「口播文案」
    let voiceText = '';
    try {
      const data = JSON.parse(generatedScript.value);
      voiceText = (data['表演脚本'] || data['口播文案'] || '').toString().trim();
    } catch {
      voiceText = '';
    }
    if (!voiceText) {
      alert('脚本中没有可用的口播内容，无法合成。请重新生成脚本。');
      return;
    }

    isSynthesizingVoice.value = true;
    try {
      // 合成前刷新配置，确保用的是设置页最新的 Provider / Base URL / 模型
      await loadSettings();
      const path = await invoke<string>('tts_synthesize', {
        projectId: currentProject.value.id,
        text: voiceText,
        voiceId: ttsVoiceId.value,
        speed: ttsSpeed.value,
        provider: appConfig.value.video.tts_provider,
        apiKey: appConfig.value.video.tts_api_key,
        baseUrl: appConfig.value.video.tts_base_url,
        model: appConfig.value.video.tts_model,
      });
      latestVoiceoverPath.value = path;
      // 重新加载素材库，确保音频出现在素材列表
      await loadMaterials(currentProject.value.id);
    } catch (e) {
      alert('合成失败: ' + e);
    } finally {
      isSynthesizingVoice.value = false;
    }
  };

  const startExportRender = async () => {
    if (!currentProject.value) return;

    const audio = audioMaterials.value.find(m => m.id === exportSelectedAudio.value);
    if (!audio?.local_path) {
      alert('请先选择主音频');
      return;
    }
    const visuals = [
      ...imageMaterials.value.filter(m => exportSelectedImages.value.includes(m.id)),
      ...videoMaterials.value.filter(m => exportSelectedVideos.value.includes(m.id)),
    ];
    const visualPaths = visuals.map(m => m.local_path).filter((p): p is string => !!p);
    if (visualPaths.length === 0) {
      alert('请至少选择一个图片或视频素材');
      return;
    }

    // 字幕文本：从脚本 JSON 取「口播文案」（与 TTS 配音一致）
    let subtitleText = '';
    if (burnSubtitle.value) {
      try {
        const data = JSON.parse(generatedScript.value);
        subtitleText = (data['口播文案'] || '').toString().trim();
      } catch {
        subtitleText = '';
      }
      if (!subtitleText) {
        alert('勾选了字幕，但脚本里没有「口播文案」。请先生成脚本，或取消字幕勾选。');
        return;
      }
    }

    isExporting.value = true;
    try {
      // video_export_render 内部 await ffmpeg 完成后才返回，返回成片绝对路径
      const outputPath = await invoke<string>('video_export_render', {
        projectId: currentProject.value.id,
        audioPath: audio.local_path,
        visualPaths,
        burnSubtitle: burnSubtitle.value,
        subtitleText: subtitleText || null,
      });
      // 刷新素材库（成片已作为 video 素材入库）
      await loadMaterials(currentProject.value.id);
      // 切到素材库并预览成片
      const newMat = materials.value.find(m => m.local_path === outputPath);
      if (newMat) {
        activeTab.value = 'material';
        openPreview(newMat);
      } else {
        alert('合成完成！成片已保存到素材库。');
      }
    } catch (e) {
      alert('合成失败: ' + e);
    } finally {
      isExporting.value = false;
    }
  };

  // 选择本地参考图（图生图用）
  const pickImageGenReference = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    if (selected && !Array.isArray(selected)) {
      imageGenRefPath.value = selected;
    }
  };

  // AI 生成图片素材
  const generateImageMaterial = async () => {
    if (!currentProject.value) return;
    if (!imageGenPrompt.value.trim()) {
      alert('请先填写图片描述（提示词）');
      return;
    }
    // 用最新配置（避免前端缓存旧 provider/key）
    await loadSettings();
    const v = appConfig.value.video;

    // 图片生成的 Provider 凭证：复用 OpenAI 兼容协议的 key/base_url
    let apiKey = '';
    let baseUrl = '';
    let model = '';
    if (imageGenProvider.value === 'openai') {
      apiKey = v.openai_api_key || '';
      baseUrl = v.openai_base_url || '';
      model = v.openai_model || '';
    } else if (imageGenProvider.value === 'fal') {
      apiKey = v.fal_key || '';
    } else if (imageGenProvider.value === 'volcengine') {
      apiKey = v.volc_key || '';
    }
    // mock 不需要 key

    isGeneratingImage.value = true;
    try {
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
      // 关闭弹窗、清空输入
      showImageGenModal.value = false;
      imageGenPrompt.value = '';
      imageGenRefPath.value = '';
    } catch (e) {
      alert('图片生成失败: ' + e);
    } finally {
      isGeneratingImage.value = false;
    }
  };

  const openPreview = (m: VideoMaterial) => {
    previewMaterial.value = m;
    previewZoom.value = 1;
    previewOffset.value = { x: 0, y: 0 };
  };

  const closePreview = () => {
    previewMaterial.value = null;
  };

  const updateResolution = (r: string) => {
    videoRatio.value = r;
    if (r === '9:16') {
      imageGenSize.value = '720x1280';
    } else if (r === '16:9') {
      imageGenSize.value = '1280x720';
    } else {
      imageGenSize.value = '1024x1024';
    }
  };

  return {
    // constants
    PLATFORM_OPTIONS, SCRIPT_TYPE_OPTIONS, IMAGE_SIZE_PRESETS,
    // projects / materials / tasks
    projects, currentProject, createProject, selectProject, deleteProject,
    isUploadingMaterial, uploadMaterial, deleteMaterial,
    audioMaterials, imageMaterials, videoMaterials, activeTasks,
    // tabs
    activeTab,
    // script
    productInfo, referenceScript, scriptFeedback, generatedScript, isGeneratingScript,
    scriptConfirmed, selectedPlatform, selectedScriptType, videoRatio,
    // tts
    ttsVoiceId, ttsSpeed, isSynthesizingVoice, isLoadingVoices, availableVoices, latestVoiceoverPath,
    // export
    exportSelectedAudio, exportSelectedImages, exportSelectedVideos, isExporting, burnSubtitle,
    // modals
    showImageGenModal, imageGenPrompt, imageGenRefPath, imageGenProvider, imageGenSize, isGeneratingImage,
    showReferencePicker, referenceImageId, showNoReferenceWarning,
    // preview
    previewMaterial, previewZoom, previewOffset, isDragging,
    // methods
    generateScript, resetScriptFlow, confirmScript, saveScript, loadVoices, synthesizeVoice,
    startExportRender, pickImageGenReference, generateImageMaterial, openPreview, closePreview, updateResolution,
  };
}
