<script setup lang="ts">
import { Music, ImageIcon, Film, CheckCircle2 } from 'lucide-vue-next';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { VideoMaterial } from '../../types/video-studio';

const props = defineProps<{
  audioMaterials: VideoMaterial[];
  imageMaterials: VideoMaterial[];
  videoMaterials: VideoMaterial[];
  exportSelectedAudio: string | null;
  exportSelectedImages: string[];
  exportSelectedVideos: string[];
}>();

const emit = defineEmits<{
  (e: 'update:activeTab', val: string): void;
  (e: 'update:exportSelectedAudio', val: string | null): void;
  (e: 'update:exportSelectedImages', val: string[]): void;
  (e: 'update:exportSelectedVideos', val: string[]): void;
  (e: 'startExportRender'): void;
}>();

const toggleExportImage = (id: string) => {
  const next = props.exportSelectedImages.includes(id)
    ? props.exportSelectedImages.filter(x => x !== id)
    : [...props.exportSelectedImages, id];
  emit('update:exportSelectedImages', next);
};

const toggleExportVideo = (id: string) => {
  const next = props.exportSelectedVideos.includes(id)
    ? props.exportSelectedVideos.filter(x => x !== id)
    : [...props.exportSelectedVideos, id];
  emit('update:exportSelectedVideos', next);
};
</script>

<template>
  <div class="max-w-6xl mx-auto animate-in fade-in slide-in-from-bottom-2">
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
            <button @click="emit('update:activeTab', 'material')" class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white text-xs font-bold rounded-lg">去上传音频</button>
          </div>
          <div v-else class="grid grid-cols-2 gap-3">
            <button
              v-for="a in audioMaterials" :key="a.id"
              @click="emit('update:exportSelectedAudio', a.id)"
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
            <button @click="emit('update:activeTab', 'material')" class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold rounded-lg">去上传图片</button>
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
            <button @click="emit('update:exportSelectedImages', [])" class="text-xs text-gray-500 hover:text-red-400">清除选择</button>
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
            <button @click="emit('update:activeTab', 'material')" class="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-xs font-bold rounded-lg">去上传视频</button>
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
            <button @click="emit('update:exportSelectedVideos', [])" class="text-xs text-gray-500 hover:text-red-400">清除选择</button>
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
          @click="emit('startExportRender')"
          :disabled="!exportSelectedAudio || (exportSelectedImages.length === 0 && exportSelectedVideos.length === 0)"
          class="w-full bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white py-4 rounded-xl text-sm font-bold transition-all shadow-lg shadow-purple-900/30 flex items-center justify-center gap-2"
        >
          <Film class="w-5 h-5" />
          开始合成视频
        </button>
      </div>
    </div>
  </div>
</template>
