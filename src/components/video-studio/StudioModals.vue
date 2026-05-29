<script setup lang="ts">
import { ref } from 'vue';
import { 
  Sparkles, XCircle, Upload, Loader2, 
  Image as ImageIcon, CheckCircle2, Zap 
} from 'lucide-vue-next';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { VideoMaterial } from '../../types/video-studio';

defineProps<{
  // Image Gen Modal
  showImageGenModal: boolean;
  imageGenPrompt: string;
  imageGenRefPath: string;
  imageGenProvider: string;
  imageGenSize: string;
  isGeneratingImage: boolean;
  IMAGE_SIZE_PRESETS: any[];
  
  // Reference Picker
  showReferencePicker: boolean;
  availableReferenceImages: VideoMaterial[];
  referenceImageId: string;
  
  // No Reference Warning
  showNoReferenceWarning: boolean;
  
  // Preview
  previewMaterial: VideoMaterial | null;
  previewZoom: number;
  previewOffset: { x: number; y: number };
  isDragging: boolean;
  MAX_ZOOM: number;
  MIN_ZOOM: number;
}>();

const emit = defineEmits<{
  (e: 'update:showImageGenModal', val: boolean): void;
  (e: 'update:imageGenPrompt', val: string): void;
  (e: 'update:imageGenRefPath', val: string): void;
  (e: 'update:imageGenProvider', val: string): void;
  (e: 'update:imageGenSize', val: string): void;
  (e: 'generateImageMaterial'): void;
  (e: 'pickImageGenReference'): void;
  
  (e: 'update:showReferencePicker', val: boolean): void;
  (e: 'pickReferenceImage', id: string): void;
  
  (e: 'update:showNoReferenceWarning', val: boolean): void;
  (e: 'ackNoReferenceWarning'): void;
  
  (e: 'closePreview'): void;
  (e: 'handlePreviewWheel', ev: WheelEvent): void;
  (e: 'startDrag', ev: MouseEvent): void;
  (e: 'onDrag', ev: MouseEvent): void;
  (e: 'endDrag'): void;
  (e: 'zoomIn'): void;
  (e: 'zoomOut'): void;
  (e: 'resetZoom'): void;
  (e: 'update:activeTab', val: string): void;
}>();

const previewCloseDebounce = ref(false);

const handleClosePreview = () => {
  if (!previewCloseDebounce.value) {
    previewCloseDebounce.value = true;
    emit('closePreview');
    setTimeout(() => previewCloseDebounce.value = false, 300);
  }
};
</script>

<template>
  <div>
    <!-- ===== AI 图片生成 Modal ===== -->
    <div v-if="showImageGenModal"
         @click.self="!isGeneratingImage && emit('update:showImageGenModal', false)"
         class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200">
      <div class="bg-gray-900 border border-purple-500/30 rounded-2xl w-full max-w-2xl flex flex-col">
        <div class="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
          <h3 class="text-sm font-bold text-gray-100 flex items-center gap-2">
            <Sparkles class="w-4 h-4 text-purple-400" /> AI 生成图片素材
          </h3>
          <button @click="!isGeneratingImage && emit('update:showImageGenModal', false)" class="text-gray-500 hover:text-white">
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
              :value="imageGenPrompt"
              @input="e => emit('update:imageGenPrompt', (e.target as HTMLTextAreaElement).value)"
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
              <button @click="emit('update:imageGenRefPath', '')" :disabled="isGeneratingImage" class="text-gray-500 hover:text-red-400 text-xs">
                移除
              </button>
            </div>
            <button v-else
                    @click="emit('pickImageGenReference')"
                    :disabled="isGeneratingImage"
                    class="w-full py-3 bg-gray-950 hover:bg-gray-800 disabled:opacity-50 text-gray-400 border border-dashed border-gray-700 hover:border-gray-600 rounded-xl text-sm flex items-center justify-center gap-2">
              <Upload class="w-4 h-4" /> 选择本地参考图
            </button>
          </div>

          <!-- Provider + Size -->
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-medium text-gray-400 mb-2">Provider</label>
              <select :value="imageGenProvider" @change="e => emit('update:imageGenProvider', (e.target as HTMLSelectElement).value)" :disabled="isGeneratingImage"
                      class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
                <option value="mock">测试模拟 (Mock)</option>
                <option value="fal">fal.ai FLUX</option>
                <option value="openai">OpenAI 兼容</option>
                <option value="volcengine">火山引擎（待接入）</option>
              </select>
            </div>
            <div>
              <label class="block text-xs font-medium text-gray-400 mb-2">尺寸</label>
              <select :value="imageGenSize" @change="e => emit('update:imageGenSize', (e.target as HTMLSelectElement).value)" :disabled="isGeneratingImage"
                      class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
                <option v-for="s in IMAGE_SIZE_PRESETS" :key="s.id" :value="s.id">{{ s.label }}</option>
              </select>
            </div>
          </div>
        </div>

        <div class="p-6 border-t border-gray-800 flex justify-end gap-2 bg-gray-950/40">
          <button
            @click="emit('update:showImageGenModal', false)"
            :disabled="isGeneratingImage"
            class="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded-lg border border-gray-700"
          >取消</button>
          <button
            @click="emit('generateImageMaterial')"
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
         @click.self="emit('update:showReferencePicker', false)"
         class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200">
      <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-3xl max-h-[80vh] flex flex-col">
        <div class="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
          <h3 class="text-sm font-bold text-gray-200 flex items-center gap-2">
            <ImageIcon class="w-4 h-4 text-blue-400" /> 选择产品参考图
          </h3>
          <button @click="emit('update:showReferencePicker', false)" class="text-gray-500 hover:text-white">
            <XCircle class="w-5 h-5" />
          </button>
        </div>
        <div class="flex-1 overflow-y-auto p-6 custom-scrollbar">
          <div v-if="availableReferenceImages.length === 0" class="py-16 text-center">
            <ImageIcon class="w-12 h-12 text-gray-700 mx-auto mb-3" />
            <p class="text-sm text-gray-500 mb-3">当前项目还没有图片素材</p>
            <button @click="emit('update:showReferencePicker', false); emit('update:activeTab', 'material')"
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs rounded-lg">
              去素材库上传
            </button>
          </div>
          <div v-else class="grid grid-cols-3 md:grid-cols-4 gap-3">
            <button
              v-for="m in availableReferenceImages"
              :key="m.id"
              @click="emit('pickReferenceImage', m.id)"
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
         @click.self="emit('update:showNoReferenceWarning', false)"
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
            <button @click="emit('update:showNoReferenceWarning', false); emit('update:showReferencePicker', true)"
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold rounded-lg">
              去选参考图
            </button>
            <button @click="emit('ackNoReferenceWarning')"
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
      @click.self="handleClosePreview"
      @wheel="emit('handlePreviewWheel', $event)"
      @mousemove="emit('onDrag', $event)"
      @mouseup="emit('endDrag')"
      @mouseleave="emit('endDrag')"
      tabindex="0"
      class="fixed inset-0 z-50 bg-black/90 backdrop-blur-md flex items-center justify-center p-8 animate-in fade-in duration-200 overflow-hidden select-none"
    >
      <div class="absolute top-6 left-6 text-xs text-white/60 font-mono">
        {{ previewMaterial.material_type?.toUpperCase() }} · {{ previewMaterial.id.slice(0, 8) }}
      </div>

      <button
        @click="handleClosePreview"
        class="absolute top-6 right-6 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 text-white flex items-center justify-center transition-colors z-10"
        title="关闭 (Esc)"
      >
        <XCircle class="w-6 h-6" />
      </button>

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
        ></audio>
        <img
          v-else-if="previewMaterial.material_type === 'image' && previewMaterial.local_path"
          :src="convertFileSrc(previewMaterial.local_path)"
          class="max-w-full max-h-[85vh] object-contain rounded-xl shadow-2xl"
          :style="{
            transform: `translate(${previewOffset.x}px, ${previewOffset.y}px) scale(${previewZoom})`,
            cursor: previewZoom > 1 ? (isDragging ? 'grabbing' : 'grab') : 'default',
            transition: isDragging ? 'none' : 'transform 0.12s ease-out',
          }"
          @mousedown="emit('startDrag', $event)"
          @dblclick="previewZoom === 1 ? emit('zoomIn') : emit('resetZoom')"
          draggable="false"
          alt="预览"
        />
      </div>

      <div
        v-if="previewMaterial.material_type === 'image'"
        class="absolute bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-1 bg-black/60 backdrop-blur-md border border-white/10 rounded-full px-2 py-1.5 z-10"
      >
        <button
          @click="emit('zoomOut')"
          :disabled="previewZoom <= MIN_ZOOM"
          class="w-9 h-9 rounded-full hover:bg-white/10 disabled:opacity-30 disabled:cursor-not-allowed text-white flex items-center justify-center text-xl font-bold"
          title="缩小 (滚轮 / -)"
        >−</button>
        <button
          @click="emit('resetZoom')"
          class="px-3 h-9 rounded-full hover:bg-white/10 text-white text-xs font-mono tabular-nums min-w-[70px]"
          :title="`重置缩放（双击图片也可重置）`"
        >{{ Math.round(previewZoom * 100) }}%</button>
        <button
          @click="emit('zoomIn')"
          :disabled="previewZoom >= MAX_ZOOM"
          class="w-9 h-9 rounded-full hover:bg-white/10 disabled:opacity-30 disabled:cursor-not-allowed text-white flex items-center justify-center text-xl font-bold"
          title="放大 (滚轮 / +)"
        >+</button>
      </div>
    </div>
  </div>
</template>
