<script setup lang="ts">
import { ShoppingBag, FileText, Settings2, Film } from 'lucide-vue-next';

import ProjectSidebar from './video-studio/Sidebar.vue';
import TabScript from './video-studio/TabScript.vue';
import TabMaterial from './video-studio/TabMaterial.vue';
import TabExport from './video-studio/TabExport.vue';
import StudioModals from './video-studio/StudioModals.vue';
import { useVideoStudioView } from '../composables/useVideoStudioView';

const {
  PLATFORM_OPTIONS, SCRIPT_TYPE_OPTIONS, IMAGE_SIZE_PRESETS,
  projects, currentProject, createProject, selectProject, deleteProject,
  isUploadingMaterial, uploadMaterial, deleteMaterial,
  audioMaterials, imageMaterials, videoMaterials, activeTasks,
  activeTab,
  productInfo, referenceScript, scriptFeedback, generatedScript, isGeneratingScript,
  scriptConfirmed, selectedPlatform, selectedScriptType, videoRatio,
  ttsVoiceId, ttsSpeed, isSynthesizingVoice, isLoadingVoices, availableVoices, latestVoiceoverPath,
  exportSelectedAudio, exportSelectedImages, exportSelectedVideos, isExporting, burnSubtitle,
  showImageGenModal, imageGenPrompt, imageGenRefPath, imageGenProvider, imageGenSize, isGeneratingImage,
  showReferencePicker, referenceImageId, showNoReferenceWarning,
  previewMaterial, previewZoom, previewOffset, isDragging,
  generateScript, resetScriptFlow, confirmScript, saveScript, loadVoices, synthesizeVoice,
  startExportRender, pickImageGenReference, generateImageMaterial, openPreview, closePreview, updateResolution,
} = useVideoStudioView();
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
            :PLATFORM_OPTIONS="PLATFORM_OPTIONS"
            :SCRIPT_TYPE_OPTIONS="SCRIPT_TYPE_OPTIONS"
            @generateScript="generateScript"
            @resetScriptFlow="resetScriptFlow"
            @confirmScript="confirmScript"
            @loadVoices="loadVoices"
            @synthesizeVoice="synthesizeVoice"
            @updateResolution="updateResolution"
            @saveScript="saveScript"
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
            :isExporting="isExporting"
            v-model:burnSubtitle="burnSubtitle"
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
      @generateImageMaterial="generateImageMaterial"
      @pickImageGenReference="pickImageGenReference"
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
