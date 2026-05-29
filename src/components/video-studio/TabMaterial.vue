<script setup lang="ts">
import { 
  Upload, Sparkles, Loader2, Image as ImageIcon, 
  Film, Music, Play, Trash2 
} from 'lucide-vue-next';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { VideoMaterial, VideoTask } from '../../types/video-studio';

defineProps<{
  imageMaterials: VideoMaterial[];
  videoMaterials: VideoMaterial[];
  audioMaterials: VideoMaterial[];
  isUploadingMaterial: boolean;
  activeTasks: Record<string, VideoTask>;
}>();

const emit = defineEmits<{
  (e: 'openImageGenModal'): void;
  (e: 'uploadMaterial', type: string): void;
  (e: 'openPreview', m: VideoMaterial): void;
  (e: 'deleteMaterial', id: string): void;
}>();
</script>

<template>
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2">
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
          @click="emit('openImageGenModal')"
          class="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2 shadow-lg shadow-purple-900/30"
        >
          <Sparkles class="w-3.5 h-3.5" />
          AI 生成图片
        </button>
        <button
          @click="emit('uploadMaterial', 'image')"
          :disabled="isUploadingMaterial"
          class="px-4 py-2 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 text-white text-xs font-medium rounded-lg border border-gray-700 flex items-center gap-2"
        >
          <Loader2 v-if="isUploadingMaterial" class="w-3.5 h-3.5 animate-spin" />
          <ImageIcon v-else class="w-3.5 h-3.5" />
          上传图片
        </button>
        <button
          @click="emit('uploadMaterial', 'video')"
          :disabled="isUploadingMaterial"
          class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-xs font-medium rounded-lg flex items-center gap-2"
        >
          <Loader2 v-if="isUploadingMaterial" class="w-3.5 h-3.5 animate-spin" />
          <Film v-else class="w-3.5 h-3.5" />
          上传视频
        </button>
      </div>
    </div>

    <!-- 素材库：图片 / 视频 / 音频 三区分离 -->
    <div class="space-y-10">
      <!-- 图片素材 -->
      <div>
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
            <ImageIcon class="w-4 h-4" /> 图片 ({{ imageMaterials.length }})
          </h3>
          <button @click="emit('uploadMaterial', 'image')" :disabled="isUploadingMaterial"
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
                  <button @click="emit('openPreview', m)" class="flex-1 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-2">
                    <Play class="w-3 h-3" /> 预览
                  </button>
                  <button @click="emit('deleteMaterial', m.id)" class="bg-red-500/80 hover:bg-red-500 text-white py-2 px-2 rounded-lg text-xs font-bold flex items-center justify-center" title="删除">
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
          <button @click="emit('uploadMaterial', 'video')" :disabled="isUploadingMaterial"
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
                  <button @click="emit('openPreview', m)" class="flex-1 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-2">
                    <Play class="w-3 h-3" /> 预览
                  </button>
                  <button @click="emit('deleteMaterial', m.id)" class="bg-red-500/80 hover:bg-red-500 text-white py-2 px-2 rounded-lg text-xs font-bold flex items-center justify-center" title="删除">
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
          <button @click="emit('uploadMaterial', 'audio')" :disabled="isUploadingMaterial"
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
                <button @click="emit('openPreview', m)" class="w-8 h-8 bg-white/10 hover:bg-white/20 rounded-lg flex items-center justify-center text-white" title="预览">
                  <Play class="w-3.5 h-3.5" />
                </button>
                <button @click="emit('deleteMaterial', m.id)" class="w-8 h-8 bg-red-500/80 hover:bg-red-500 rounded-lg flex items-center justify-center text-white" title="删除">
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

    <!-- 任务列表 -->
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
</template>
