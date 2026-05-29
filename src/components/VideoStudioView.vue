<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { 
  ShoppingBag, FileText, Settings2, Film
} from 'lucide-vue-next';

import type { VideoProject, VideoMaterial } from '../types/video-studio';
import { useVideoProjects, useVideoMaterials } from '../composables/useVideoStudio';
import { useVideoTasks } from '../composables/useVideoTasks';
import { useSettings } from '../composables/useSettings';

import ProjectSidebar from './video-studio/Sidebar.vue';
import TabScript from './video-studio/TabScript.vue';
import TabMaterial from './video-studio/TabMaterial.vue';
import TabExport from './video-studio/TabExport.vue';
import StudioModals from './video-studio/StudioModals.vue';

// ============ Constants ============
const PLATFORM_OPTIONS = [
  { id: 'douyin', label: '抖音', emoji: '🎵', desc: '快节奏，前 3 秒钩子，强互动' },
  { id: 'kuaishou', label: '快手', emoji: '🧡', desc: '接地气，老铁味，高性价比感' },
  { id: 'xiaohongshu', label: '小红书', emoji: '📕', desc: '种草风，美感强，多 emoji 标记' },
  { id: 'wechat-channel', label: '视频号', emoji: '📽️', desc: '朋友圈调性，信任感，叙述稳重' },
];

const SCRIPT_TYPE_OPTIONS = [
  { id: 'voiceover', label: '口播带货', desc: '生成专业脚本 + TTS 合成旁白 + 素材拼接' },
  { id: 'ai-video', label: 'AI 视频流', desc: '生成脚本 + 分镜提示词 + AI 引擎生成画面' },
];

const IMAGE_SIZE_PRESETS = [
  { id: '720x1280', label: '竖屏 (9:16)' },
  { id: '1280x720', label: '横屏 (16:9)' },
  { id: '1024x1024', label: '方形 (1:1)' },
];

// ============ State ============
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
const isRenderingVoiceover = ref(false);

// Export State
const exportSelectedAudio = ref<string | null>(null);
const exportSelectedImages = ref<string[]>([]);
const exportSelectedVideos = ref<string[]>([]);

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
};

const loadVoices = async () => {
  isLoadingVoices.value = true;
  try {
    const res = await invoke<any>('tts_list_voices', {
      provider: appConfig.value.video.tts_provider,
      apiKey: appConfig.value.video.tts_api_key,
      baseUrl: appConfig.value.video.tts_base_url,
      model: appConfig.value.video.tts_model,
    });
    availableVoices.value = res.voices || [];
    if (availableVoices.value.length > 0 && !ttsVoiceId.value) {
      ttsVoiceId.value = availableVoices.value[0].id;
    }
  } catch (e) {
    alert('获取音色列表失败: ' + e);
  } finally {
    isLoadingVoices.value = false;
  }
};

const synthesizeVoice = async () => {
  if (!currentProject.value) return;
  isSynthesizingVoice.value = true;
  try {
    const path = await invoke<string>('tts_synthesize', {
      projectId: currentProject.value.id,
      text: generatedScript.value,
      voiceId: ttsVoiceId.value,
      speed: ttsSpeed.value,
      provider: appConfig.value.video.tts_provider,
      apiKey: appConfig.value.video.tts_api_key,
      baseUrl: appConfig.value.video.tts_base_url,
      model: appConfig.value.video.tts_model,
    });
    latestVoiceoverPath.value = path;
    await loadMaterials(currentProject.value.id);
  } catch (e) {
    alert('合成失败: ' + e);
  } finally {
    isSynthesizingVoice.value = false;
  }
};

const startVoiceoverRender = async () => {
  alert('功能开发中 (Phase 5)...');
};

const startExportRender = async () => {
  alert('功能开发中 (Phase 5)...');
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
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100 overflow-hidden">
    <div class="flex h-full">
      <!-- 侧边栏 -->
      <ProjectSidebar 
        :projects="projects" 
        :currentProject="currentProject"
        @create="createProject"
        @select="selectProject"
        @delete="deleteProject"
      />

      <!-- 主内容区 -->
      <div v-if="currentProject" class="flex-1 flex flex-col relative">
        <!-- 头部 -->
        <div class="h-20 px-8 flex items-center justify-between border-b border-gray-800 bg-gray-950/50 backdrop-blur-md">
          <div class="flex items-center gap-3">
            <div class="p-2.5 bg-blue-600/10 rounded-xl border border-blue-500/20">
              <Film class="w-5 h-5 text-blue-400" />
            </div>
            <div>
              <h2 class="text-sm font-bold text-white">{{ currentProject.title }}</h2>
              <p class="text-[10px] text-gray-500 uppercase tracking-widest mt-0.5">Project ID: {{ currentProject.id.slice(0, 8) }}</p>
            </div>
          </div>

          <div class="flex items-center gap-6">
            <div class="flex bg-gray-900 border border-gray-800 p-1 rounded-xl">
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
        </div>

        <!-- 页面区域 -->
        <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
          <TabScript
            v-if="activeTab === 'script'"
            v-model:productInfo="productInfo"
            v-model:referenceScript="referenceScript"
            v-model:scriptFeedback="scriptFeedback"
            v-model:selectedPlatform="selectedPlatform"
            v-model:selectedScriptType="selectedScriptType"
            v-model:videoRatio="videoRatio"
            v-model:ttsVoiceId="ttsVoiceId"
            v-model:ttsSpeed="ttsSpeed"
            :generatedScript="generatedScript"
            :isGeneratingScript="isGeneratingScript"
            :scriptConfirmed="scriptConfirmed"
            :isSynthesizingVoice="isSynthesizingVoice"
            :isLoadingVoices="isLoadingVoices"
            :availableVoices="availableVoices"
            :latestVoiceoverPath="latestVoiceoverPath"
            :isRenderingVoiceover="isRenderingVoiceover"
            :PLATFORM_OPTIONS="PLATFORM_OPTIONS"
            :SCRIPT_TYPE_OPTIONS="SCRIPT_TYPE_OPTIONS"
            @generateScript="generateScript"
            @resetScriptFlow="resetScriptFlow"
            @confirmScript="confirmScript"
            @loadVoices="loadVoices"
            @synthesizeVoice="synthesizeVoice"
            @startVoiceoverRender="startVoiceoverRender"
            @updateResolution="updateResolution"
          />

          <TabMaterial
            v-else-if="activeTab === 'material'"
            :imageMaterials="imageMaterials"
            :videoMaterials="videoMaterials"
            :audioMaterials="audioMaterials"
            :isUploadingMaterial="isUploadingMaterial"
            :activeTasks="activeTasks"
            @openImageGenModal="showImageGenModal = true"
            @uploadMaterial="t => uploadMaterial(currentProject!.id, t)"
            @openPreview="openPreview"
            @deleteMaterial="id => deleteMaterial(currentProject!.id, id)"
          />

          <TabExport
            v-else-if="activeTab === 'export'"
            v-model:exportSelectedAudio="exportSelectedAudio"
            v-model:exportSelectedImages="exportSelectedImages"
            v-model:exportSelectedVideos="exportSelectedVideos"
            :audioMaterials="audioMaterials"
            :imageMaterials="imageMaterials"
            :videoMaterials="videoMaterials"
            @update:activeTab="t => activeTab = t as any"
            @startExportRender="startExportRender"
          />
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

    <!-- Modals -->
    <StudioModals
      v-model:showImageGenModal="showImageGenModal"
      v-model:imageGenPrompt="imageGenPrompt"
      v-model:imageGenRefPath="imageGenRefPath"
      v-model:imageGenProvider="imageGenProvider"
      v-model:imageGenSize="imageGenSize"
      v-model:showReferencePicker="showReferencePicker"
      v-model:showNoReferenceWarning="showNoReferenceWarning"
      :isGeneratingImage="isGeneratingImage"
      :IMAGE_SIZE_PRESETS="IMAGE_SIZE_PRESETS"
      :availableReferenceImages="imageMaterials"
      :referenceImageId="referenceImageId"
      :previewMaterial="previewMaterial"
      :previewZoom="previewZoom"
      :previewOffset="previewOffset"
      :isDragging="isDragging"
      :MAX_ZOOM="5"
      :MIN_ZOOM="0.5"
      @closePreview="closePreview"
      @pickReferenceImage="id => referenceImageId = id"
    />
  </div>
</template>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #374151;
}
</style>
