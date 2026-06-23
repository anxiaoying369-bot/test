<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Brush, Eraser, ImagePlus, Loader2, RotateCcw, Sparkles } from 'lucide-vue-next';

type ToolMode = 'brush' | 'eraser';

const imagePath = ref('');
const prompt = ref('');
const status = ref('');
const busy = ref(false);
const tool = ref<ToolMode>('brush');
const brushSize = ref(36);
const resultUrl = ref('');

const imageCanvas = ref<HTMLCanvasElement | null>(null);
const overlayCanvas = ref<HTMLCanvasElement | null>(null);
const maskCanvas = ref<HTMLCanvasElement | null>(null);
const isDrawing = ref(false);
const lastPoint = ref<{ x: number; y: number } | null>(null);

const displayImageUrl = computed(() => imagePath.value ? convertFileSrc(imagePath.value) : '');
const resultImageUrl = computed(() => {
  if (!resultUrl.value) return '';
  return resultUrl.value.startsWith('http') || resultUrl.value.startsWith('data:')
    ? resultUrl.value
    : convertFileSrc(resultUrl.value);
});

async function pickImage() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
  });
  if (typeof selected !== 'string') return;
  imagePath.value = selected;
  resultUrl.value = '';
  await nextTick();
  await loadImageToCanvas();
}

async function loadImageToCanvas() {
  if (!displayImageUrl.value || !imageCanvas.value || !overlayCanvas.value || !maskCanvas.value) return;
  const img = new Image();
  img.crossOrigin = 'anonymous';
  img.onload = () => {
    const baseCanvas = imageCanvas.value;
    const paintCanvas = overlayCanvas.value;
    const hiddenMaskCanvas = maskCanvas.value;
    if (!baseCanvas || !paintCanvas || !hiddenMaskCanvas) return;
    const maxW = 900;
    const maxH = 620;
    const scale = Math.min(maxW / img.naturalWidth, maxH / img.naturalHeight, 1);
    const w = Math.round(img.naturalWidth * scale);
    const h = Math.round(img.naturalHeight * scale);
    for (const canvas of [baseCanvas, paintCanvas, hiddenMaskCanvas]) {
      canvas.width = w;
      canvas.height = h;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
    }
    const imageCtx = baseCanvas.getContext('2d')!;
    imageCtx.clearRect(0, 0, w, h);
    imageCtx.drawImage(img, 0, 0, w, h);
    clearMask();
  };
  img.onerror = () => {
    status.value = '图片加载失败';
  };
  img.src = displayImageUrl.value;
}

function clearMask() {
  const overlay = overlayCanvas.value?.getContext('2d');
  const mask = maskCanvas.value?.getContext('2d');
  if (!overlayCanvas.value || !maskCanvas.value || !overlay || !mask) return;
  overlay.clearRect(0, 0, overlayCanvas.value.width, overlayCanvas.value.height);
  mask.fillStyle = 'black';
  mask.fillRect(0, 0, maskCanvas.value.width, maskCanvas.value.height);
}

function canvasPoint(event: PointerEvent) {
  const canvas = overlayCanvas.value!;
  const rect = canvas.getBoundingClientRect();
  const x = (event.clientX - rect.left) * (canvas.width / rect.width);
  const y = (event.clientY - rect.top) * (canvas.height / rect.height);
  return { x, y };
}

function drawStroke(from: { x: number; y: number }, to: { x: number; y: number }) {
  const overlay = overlayCanvas.value?.getContext('2d');
  const mask = maskCanvas.value?.getContext('2d');
  if (!overlay || !mask) return;
  for (const ctx of [overlay, mask]) {
    ctx.lineWidth = brushSize.value;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    if (tool.value === 'eraser') {
      ctx.globalCompositeOperation = 'destination-out';
      ctx.strokeStyle = 'rgba(0,0,0,1)';
    } else {
      ctx.globalCompositeOperation = 'source-over';
      ctx.strokeStyle = ctx === overlay ? 'rgba(59,130,246,0.42)' : 'white';
    }
    ctx.stroke();
    ctx.globalCompositeOperation = 'source-over';
  }
}

function pointerDown(event: PointerEvent) {
  if (!overlayCanvas.value) return;
  overlayCanvas.value.setPointerCapture(event.pointerId);
  isDrawing.value = true;
  lastPoint.value = canvasPoint(event);
  drawStroke(lastPoint.value, lastPoint.value);
}

function pointerMove(event: PointerEvent) {
  if (!isDrawing.value || !lastPoint.value) return;
  const next = canvasPoint(event);
  drawStroke(lastPoint.value, next);
  lastPoint.value = next;
}

function pointerUp(event: PointerEvent) {
  overlayCanvas.value?.releasePointerCapture(event.pointerId);
  isDrawing.value = false;
  lastPoint.value = null;
}

function maskDataUrl() {
  if (!maskCanvas.value) throw new Error('请先涂抹要修改的区域');
  return maskCanvas.value.toDataURL('image/png');
}

async function runInpaint() {
  if (!imagePath.value) { status.value = '请先上传图片'; return; }
  if (!prompt.value.trim()) { status.value = '请输入修改描述'; return; }
  busy.value = true;
  status.value = '正在提交局部重绘任务';
  resultUrl.value = '';
  try {
    const res = await invoke<Record<string, any>>('image_inpaint', {
      imagePath: imagePath.value,
      maskDataUrl: maskDataUrl(),
      prompt: prompt.value,
      size: `${imageCanvas.value?.width || 1024}x${imageCanvas.value?.height || 1024}`,
    });
    resultUrl.value = res.image_url || '';
    status.value = resultUrl.value ? '局部重绘完成' : '任务完成，但没有返回图片';
  } catch (e: any) {
    status.value = `局部重绘失败：${String(e)}`;
  } finally {
    busy.value = false;
  }
}

watch(displayImageUrl, () => {
  nextTick(loadImageToCanvas);
});
</script>

<template>
  <div class="flex-1 overflow-hidden bg-gray-950 text-gray-100">
    <div class="h-full flex">
      <aside class="w-72 shrink-0 border-r border-gray-800 bg-gray-950 p-5 space-y-5 overflow-y-auto">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <ImagePlus class="w-5 h-5 text-cyan-400" />AI 图像工作台
          </h1>
          <p class="text-xs text-gray-500 mt-2">上传图片，涂抹不规则区域，用文字描述局部修改。</p>
        </div>

        <button @click="pickImage" class="w-full px-4 py-2.5 rounded-xl bg-cyan-600 hover:bg-cyan-500 flex items-center justify-center gap-2">
          <ImagePlus class="w-4 h-4" />上传图片
        </button>

        <div class="space-y-2">
          <label class="text-xs text-gray-500">工具</label>
          <div class="grid grid-cols-2 gap-2">
            <button @click="tool = 'brush'" :class="['px-3 py-2 rounded-lg border flex items-center justify-center gap-2', tool === 'brush' ? 'border-cyan-500 bg-cyan-500/10 text-cyan-200' : 'border-gray-800 text-gray-400']">
              <Brush class="w-4 h-4" />涂抹
            </button>
            <button @click="tool = 'eraser'" :class="['px-3 py-2 rounded-lg border flex items-center justify-center gap-2', tool === 'eraser' ? 'border-cyan-500 bg-cyan-500/10 text-cyan-200' : 'border-gray-800 text-gray-400']">
              <Eraser class="w-4 h-4" />擦除
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <label class="text-xs text-gray-500">笔刷大小：{{ brushSize }}px</label>
          <input v-model="brushSize" type="range" min="8" max="120" class="w-full accent-cyan-500" />
        </div>

        <button @click="clearMask" :disabled="!imagePath" class="w-full px-4 py-2.5 rounded-xl border border-gray-800 hover:border-gray-600 disabled:opacity-50 flex items-center justify-center gap-2">
          <RotateCcw class="w-4 h-4" />清空涂抹
        </button>

        <div class="space-y-2">
          <label class="text-xs text-gray-500">修改描述</label>
          <textarea v-model="prompt" rows="6" class="w-full bg-gray-950 border border-gray-800 rounded-xl p-3 text-sm focus:outline-none focus:border-cyan-500" placeholder="例如：把涂抹区域改成一束暖黄色灯光" />
        </div>

        <button @click="runInpaint" :disabled="busy || !imagePath" class="w-full px-4 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 disabled:opacity-50 flex items-center justify-center gap-2">
          <Loader2 v-if="busy" class="w-4 h-4 animate-spin" />
          <Sparkles v-else class="w-4 h-4" />局部重绘
        </button>

        <div v-if="status" class="rounded-xl border border-gray-800 bg-gray-900/70 p-3 text-xs whitespace-pre-wrap" :class="status.includes('失败') ? 'text-red-300' : 'text-gray-300'">
          {{ status }}
        </div>
      </aside>

      <main class="flex-1 min-w-0 h-full overflow-auto p-6">
        <div class="min-h-full grid grid-cols-1 xl:grid-cols-2 gap-6 content-start">
          <section class="space-y-3">
            <h2 class="text-sm font-medium text-gray-400">原图与涂抹区域</h2>
            <div class="min-h-[420px] rounded-xl border border-gray-800 bg-gray-900/40 flex items-center justify-center overflow-auto p-4">
              <div v-if="imagePath" class="relative inline-block select-none">
                <canvas ref="imageCanvas" class="block max-w-full rounded-lg" />
                <canvas
                  ref="overlayCanvas"
                  class="absolute inset-0 touch-none cursor-crosshair rounded-lg"
                  @pointerdown="pointerDown"
                  @pointermove="pointerMove"
                  @pointerup="pointerUp"
                  @pointercancel="pointerUp"
                  @pointerleave="pointerUp"
                />
                <canvas ref="maskCanvas" class="hidden" />
              </div>
              <div v-else class="text-sm text-gray-500">先上传一张图片</div>
            </div>
          </section>

          <section class="space-y-3">
            <h2 class="text-sm font-medium text-gray-400">生成结果</h2>
            <div class="min-h-[420px] rounded-xl border border-gray-800 bg-gray-900/40 flex items-center justify-center overflow-auto p-4">
              <img v-if="resultImageUrl" :src="resultImageUrl" class="max-w-full rounded-lg" />
              <div v-else class="text-sm text-gray-500">局部重绘后的图片会显示在这里</div>
            </div>
          </section>
        </div>
      </main>
    </div>
  </div>
</template>
