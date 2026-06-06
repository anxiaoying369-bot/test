import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import { useVideoProjects, useVideoMaterials } from './useVideoStudio';
import { useSettings } from './useSettings';

// ============ 常量 ============
export const PLATFORM_OPTIONS = [
  { id: 'douyin', label: '抖音', emoji: '🎵', desc: '快节奏，前 3 秒钩子，强互动' },
  { id: 'kuaishou', label: '快手', emoji: '🧡', desc: '接地气，老铁味，高性价比感' },
  { id: 'xiaohongshu', label: '小红书', emoji: '📕', desc: '种草风，美感强，多 emoji 标记' },
  { id: 'wechat-channel', label: '视频号', emoji: '📽️', desc: '朋友圈调性，信任感，叙述稳重' },
];

export const ASPECT_OPTIONS = [
  { id: '9:16', label: '竖屏 9:16', hint: '抖音/快手/视频号' },
  { id: '16:9', label: '横屏 16:9', hint: '西瓜/B站/横版' },
  { id: '1:1', label: '方形 1:1', hint: '朋友圈/部分信息流' },
];

// edge-tts 常用音色（免费，无需 API Key）。命名格式与 MPT 引擎一致：<voice>-<Gender>
export const EDGE_VOICES = [
  { id: 'zh-CN-XiaoxiaoNeural-Female', name: '晓晓（女·温柔）' },
  { id: 'zh-CN-XiaoyiNeural-Female', name: '晓伊（女·亲和）' },
  { id: 'zh-CN-YunxiNeural-Male', name: '云希（男·阳光）' },
  { id: 'zh-CN-YunjianNeural-Male', name: '云健（男·浑厚）' },
  { id: 'zh-CN-YunyangNeural-Male', name: '云扬（男·专业）' },
  { id: 'zh-CN-YunxiaNeural-Male', name: '云夏（男·少年）' },
  { id: 'zh-CN-liaoning-XiaobeiNeural-Female', name: '晓北（东北女声）' },
  { id: 'zh-HK-HiuGaaiNeural-Female', name: '曉佳（粤语女）' },
  { id: 'zh-TW-HsiaoChenNeural-Female', name: '曉臻（台湾女）' },
  { id: 'en-US-AvaNeural-Female', name: 'Ava（英·女）' },
  { id: 'en-US-AndrewNeural-Male', name: 'Andrew（英·男）' },
  { id: 'en-US-EmmaNeural-Female', name: 'Emma（英·女）' },
  { id: 'en-US-BrianNeural-Male', name: 'Brian（英·男）' },
];

export const BGM_OPTIONS = [
  { id: 'random', label: '随机背景音乐' },
  { id: '', label: '不加背景音乐' },
];

export const SUBTITLE_PROVIDER_OPTIONS = [
  { id: 'edge', label: 'Edge（快速·免费·默认）' },
  { id: 'whisper', label: 'Whisper（更精准·需下模型）' },
];

export type StudioStep = 'script' | 'keywords' | 'options' | 'generate';

export function useVideoStudioView() {
  const step = ref<StudioStep>('script');
  const { projects, currentProject, loadProjects, createProject, selectProject, deleteProject } = useVideoProjects();
  const { materials, isUploadingMaterial, loadMaterials, uploadMaterial, deleteMaterial } = useVideoMaterials();
  const { config: appConfig, loadSettings } = useSettings();

  const videoMaterials = computed(() => materials.value.filter(m => m.material_type === 'video'));

  // ── 步骤 1：脚本 ──
  const productInfo = ref('');
  const referenceScript = ref('');
  const scriptText = ref('');           // 喂给 MPT 的纯口播文案（可编辑）
  const rawScriptJson = ref('');        // video_generate_script 的原始 JSON（用于反馈迭代）
  const scriptFeedback = ref('');
  const isGeneratingScript = ref(false);
  const selectedPlatform = ref('douyin');
  const videoAspect = ref('9:16');

  // ── 步骤 2：关键词 ──
  const terms = ref<string[]>([]);
  const newTerm = ref('');
  const isGeneratingTerms = ref(false);

  // ── 步骤 3：参数 ──
  const videoSource = ref<'pexels' | 'local'>('pexels');
  const voiceName = ref('zh-CN-XiaoxiaoNeural-Female');
  const voiceRate = ref(1.0);
  const subtitleEnabled = ref(true);
  const subtitleProvider = ref('edge');
  const fontName = ref('STHeitiMedium.ttc');
  const subtitlePosition = ref('bottom');
  const textForeColor = ref('#FFFFFF');
  const strokeColor = ref('#000000');
  const fontSize = ref(60);
  const bgmType = ref('random');
  const bgmVolume = ref(0.2);
  const clipDuration = ref(5);
  const concatMode = ref<'random' | 'sequential'>('random');
  const videoCount = ref(1);
  const selectedLocalMaterialIds = ref<string[]>([]);
  const isPreviewingVoice = ref(false);
  let previewAudio: HTMLAudioElement | null = null;

  // ── 步骤 4：生成 ──
  const isGenerating = ref(false);
  const progress = ref(0);
  const stageLabel = ref('');
  const finalVideoPath = ref<string | null>(null);
  const errorMsg = ref('');

  let unlistenProgress: UnlistenFn | null = null;

  onMounted(async () => {
    await loadProjects();
    await loadSettings();
    // 用设置页的默认值初始化参数
    const v = appConfig.value?.video || {};
    if (v.mpt_voice_name) voiceName.value = v.mpt_voice_name;
    if (v.mpt_subtitle_provider) subtitleProvider.value = v.mpt_subtitle_provider;
    unlistenProgress = await listen<any>('video-mpt-progress', (event) => {
      const p = event.payload || {};
      progress.value = p.progress ?? progress.value;
      if (p.stage) stageLabel.value = p.stage;
    });
  });

  onUnmounted(() => {
    if (unlistenProgress) unlistenProgress();
  });

  watch(currentProject, async (newVal) => {
    // 切项目时重置流程与已恢复的配置
    step.value = 'script';
    finalVideoPath.value = null;
    errorMsg.value = '';
    progress.value = 0;
    if (newVal) {
      await loadMaterials(newVal.id);
      const cfg = newVal.config?.mpt || {};
      productInfo.value = cfg.productInfo || '';
      referenceScript.value = cfg.referenceScript || '';
      scriptText.value = cfg.scriptText || '';
      rawScriptJson.value = cfg.rawScriptJson || '';
      selectedPlatform.value = cfg.selectedPlatform || 'douyin';
      videoAspect.value = cfg.videoAspect || '9:16';
      terms.value = Array.isArray(cfg.terms) ? cfg.terms : [];
      videoSource.value = cfg.videoSource || 'pexels';
      voiceName.value = cfg.voiceName || appConfig.value?.video?.mpt_voice_name || 'zh-CN-XiaoxiaoNeural-Female';
      voiceRate.value = cfg.voiceRate ?? 1.0;
      subtitleEnabled.value = cfg.subtitleEnabled ?? true;
      subtitleProvider.value = cfg.subtitleProvider || appConfig.value?.video?.mpt_subtitle_provider || 'edge';
      bgmType.value = cfg.bgmType ?? 'random';
      clipDuration.value = cfg.clipDuration ?? 5;
      concatMode.value = cfg.concatMode || 'random';
      videoCount.value = cfg.videoCount ?? 1;
      if (newVal.final_video_path) finalVideoPath.value = newVal.final_video_path;
    }
  });

  const saveProjectConfig = async () => {
    if (!currentProject.value) return;
    const mpt = {
      productInfo: productInfo.value,
      referenceScript: referenceScript.value,
      scriptText: scriptText.value,
      rawScriptJson: rawScriptJson.value,
      selectedPlatform: selectedPlatform.value,
      videoAspect: videoAspect.value,
      terms: terms.value,
      videoSource: videoSource.value,
      voiceName: voiceName.value,
      voiceRate: voiceRate.value,
      subtitleEnabled: subtitleEnabled.value,
      subtitleProvider: subtitleProvider.value,
      bgmType: bgmType.value,
      clipDuration: clipDuration.value,
      concatMode: concatMode.value,
      videoCount: videoCount.value,
    };
    currentProject.value.config = { ...currentProject.value.config, mpt };
    await invoke('video_upsert_project', { project: currentProject.value });
  };

  // ── 步骤 1：脚本 ──
  const generateScript = async (isFeedback: boolean) => {
    if (!productInfo.value.trim()) {
      alert('请先填写要做的主题/产品信息');
      return;
    }
    isGeneratingScript.value = true;
    try {
      const json = await invoke<string>('video_generate_script', {
        product: productInfo.value,
        referenceScript: referenceScript.value || null,
        videoRatio: videoAspect.value,
        platform: selectedPlatform.value,
        scriptType: 'voiceover',
        previousScript: isFeedback ? rawScriptJson.value : null,
        feedback: isFeedback ? scriptFeedback.value : null,
      });
      rawScriptJson.value = json;
      // 只取「口播文案」作为纯文案。素材关键词不从这里预填：Pexels 素材库以英文索引，
      // 中文关键词命中率极低，因此关键词统一由「关键词」步骤的英文生成器产出。
      try {
        const data = JSON.parse(json);
        scriptText.value = (data['口播文案'] || data['表演脚本'] || '').toString().trim();
      } catch {
        scriptText.value = json;
      }
      await saveProjectConfig();
    } catch (e) {
      alert('生成脚本失败: ' + e);
    } finally {
      isGeneratingScript.value = false;
    }
  };

  const confirmScriptStep = async () => {
    if (!scriptText.value.trim()) {
      alert('请先生成或填写口播文案');
      return;
    }
    await saveProjectConfig();
    // pexels 模式需要关键词；本地素材模式直接跳到参数步
    if (videoSource.value === 'local') {
      step.value = 'options';
    } else {
      step.value = 'keywords';
      if (terms.value.length === 0) await generateTerms();
    }
  };

  // ── 步骤 2：关键词 ──
  const generateTerms = async () => {
    isGeneratingTerms.value = true;
    try {
      const result = await invoke<string[]>('video_mpt_generate_terms', {
        videoSubject: productInfo.value,
        videoScript: scriptText.value,
        amount: 5,
      });
      terms.value = result;
      await saveProjectConfig();
    } catch (e) {
      alert('关键词生成失败: ' + e);
    } finally {
      isGeneratingTerms.value = false;
    }
  };

  const addTerm = () => {
    const t = newTerm.value.trim();
    if (t && !terms.value.includes(t)) {
      terms.value.push(t);
      newTerm.value = '';
      saveProjectConfig();
    }
  };

  const removeTerm = (t: string) => {
    terms.value = terms.value.filter(x => x !== t);
    saveProjectConfig();
  };

  // ── 步骤 3：参数 / 本地素材 ──
  const uploadLocalMaterial = async () => {
    if (!currentProject.value) return;
    await uploadMaterial(currentProject.value.id, 'video');
  };

  const toggleLocalMaterial = (id: string) => {
    const i = selectedLocalMaterialIds.value.indexOf(id);
    if (i >= 0) selectedLocalMaterialIds.value.splice(i, 1);
    else selectedLocalMaterialIds.value.push(id);
  };

  // 音色试听：用 Edge TTS 合成一小段示例音频并播放（首个音色合成后会缓存）。
  const previewVoice = async () => {
    if (isPreviewingVoice.value) return;
    isPreviewingVoice.value = true;
    try {
      const path = await invoke<string>('video_mpt_preview_voice', { voiceName: voiceName.value });
      if (previewAudio) { previewAudio.pause(); previewAudio = null; }
      previewAudio = new Audio(convertFileSrc(path));
      await previewAudio.play();
    } catch (e) {
      alert('试听失败：' + e);
    } finally {
      isPreviewingVoice.value = false;
    }
  };

  // ── 步骤 4：生成 ──
  const startGenerate = async () => {
    if (!currentProject.value) return;
    if (!scriptText.value.trim()) {
      alert('缺少口播文案，请回到第一步生成脚本');
      return;
    }

    // 组装参数
    const params: Record<string, any> = {
      video_subject: productInfo.value,
      video_script: scriptText.value,
      video_aspect: videoAspect.value,
      video_source: videoSource.value,
      voice_name: voiceName.value,
      voice_rate: voiceRate.value,
      subtitle_enabled: subtitleEnabled.value,
      subtitle_provider: subtitleProvider.value,
      font_name: fontName.value,
      subtitle_position: subtitlePosition.value,
      text_fore_color: textForeColor.value,
      stroke_color: strokeColor.value,
      font_size: fontSize.value,
      bgm_type: bgmType.value,
      bgm_volume: bgmVolume.value,
      video_clip_duration: clipDuration.value,
      video_concat_mode: concatMode.value,
      video_count: videoCount.value,
    };

    if (videoSource.value === 'local') {
      const chosen = videoMaterials.value
        .filter(m => selectedLocalMaterialIds.value.includes(m.id))
        .map(m => m.local_path)
        .filter((p): p is string => !!p);
      if (chosen.length === 0) {
        alert('本地素材模式下，请至少勾选一个视频素材');
        return;
      }
      params.video_materials = chosen;
    } else {
      if (terms.value.length === 0) {
        alert('Pexels 模式需要至少一个素材关键词');
        return;
      }
      params.video_terms = terms.value;
    }

    isGenerating.value = true;
    progress.value = 5;
    stageLabel.value = '准备中';
    errorMsg.value = '';
    finalVideoPath.value = null;
    try {
      const path = await invoke<string>('video_mpt_generate', {
        projectId: currentProject.value.id,
        params,
      });
      finalVideoPath.value = path;
      progress.value = 100;
      stageLabel.value = '完成';
      await loadProjects();
      await loadMaterials(currentProject.value.id);
      // 同步当前项目对象上的成片路径
      const refreshed = projects.value.find(p => p.id === currentProject.value!.id);
      if (refreshed) currentProject.value = refreshed;
    } catch (e) {
      errorMsg.value = String(e);
    } finally {
      isGenerating.value = false;
    }
  };

  const canProceedFromScript = computed(() => !!scriptText.value.trim());
  const canGenerate = computed(() => {
    if (!scriptText.value.trim()) return false;
    if (videoSource.value === 'local') return selectedLocalMaterialIds.value.length > 0;
    return terms.value.length > 0;
  });

  return {
    // constants
    PLATFORM_OPTIONS, ASPECT_OPTIONS, EDGE_VOICES, BGM_OPTIONS, SUBTITLE_PROVIDER_OPTIONS,
    // projects / materials
    projects, currentProject, createProject, selectProject, deleteProject,
    materials, videoMaterials, isUploadingMaterial, deleteMaterial,
    // step
    step,
    // script
    productInfo, referenceScript, scriptText, rawScriptJson, scriptFeedback,
    isGeneratingScript, selectedPlatform, videoAspect,
    // keywords
    terms, newTerm, isGeneratingTerms,
    // options
    videoSource, voiceName, voiceRate, subtitleEnabled, subtitleProvider, fontName,
    subtitlePosition, textForeColor, strokeColor, fontSize, bgmType, bgmVolume,
    clipDuration, concatMode, videoCount, selectedLocalMaterialIds, isPreviewingVoice,
    // generate
    isGenerating, progress, stageLabel, finalVideoPath, errorMsg,
    // computed
    canProceedFromScript, canGenerate,
    // methods
    generateScript, confirmScriptStep, generateTerms, addTerm, removeTerm,
    uploadLocalMaterial, toggleLocalMaterial, previewVoice, startGenerate, saveProjectConfig,
  };
}
