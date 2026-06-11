<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import {
  Smartphone, RefreshCw, Pencil, Check, X, Trash2, Play, Square,
  ArrowLeft, Home, LayoutGrid, Bell, Camera, Phone,
} from 'lucide-vue-next';
import { useMobileControl } from '../composables/useMobileControl';
import type { MobileDevice } from '../types/mobile';

const {
  devices, serverInfo, frames, frameAt, recordings, streamingId, lastError,
  init, refreshDevices, refreshRecordings, recordingUrl, deleteRecording,
  startStream, stopStream, requestScreenshot,
  tap, swipe, pressKey, setRemark, deleteDevice,
} = useMobileControl();

onMounted(init);

// ─── 右侧面板：实时控制 / 通话记录 ───
const viewMode = ref<'control' | 'recordings'>('control');

// ─── 设备选择 ───
const selectedId = ref<string | null>(null);
const selected = computed<MobileDevice | null>(
  () => devices.value.find((d) => d.device_id === selectedId.value) ?? null,
);

function selectDevice(d: MobileDevice) {
  if (selectedId.value === d.device_id) return;
  stopStream();
  selectedId.value = d.device_id;
  if (d.online) requestScreenshot(d.device_id);
}

function displayName(d: MobileDevice) {
  return d.remark?.trim() ? d.remark : d.model;
}

// ─── 备注编辑 ───
const editingId = ref<string | null>(null);
const editingRemark = ref('');

function beginEdit(d: MobileDevice) {
  editingId.value = d.device_id;
  editingRemark.value = d.remark ?? '';
}

async function commitEdit() {
  if (editingId.value !== null) {
    await setRemark(editingId.value, editingRemark.value.trim());
  }
  editingId.value = null;
}

// ─── 实时画面 ───
const intervalMs = ref(800);

function toggleStream() {
  if (!selected.value) return;
  if (streamingId.value === selected.value.device_id) {
    stopStream();
  } else {
    startStream(selected.value.device_id, intervalMs.value);
  }
}

function onIntervalChange() {
  if (selected.value && streamingId.value === selected.value.device_id) {
    startStream(selected.value.device_id, intervalMs.value);
  }
}

const currentFrame = computed(() =>
  selectedId.value ? frames.value[selectedId.value] ?? null : null,
);
const currentFrameAge = computed(() => {
  if (!selectedId.value) return null;
  const t = frameAt.value[selectedId.value];
  return t ? new Date(t).toLocaleTimeString() : null;
});

// ─── 触控：在截图上点击 / 拖拽映射为 tap / swipe ───
const screenImg = ref<HTMLImageElement | null>(null);
let pressStart: { x: number; y: number; t: number } | null = null;

/** 把鼠标在 <img> 上的位置换算成手机屏幕像素坐标 */
function toDeviceCoords(e: PointerEvent): { x: number; y: number } | null {
  const img = screenImg.value;
  if (!img) return null;
  const rect = img.getBoundingClientRect();
  // 必须用截图自身的像素尺寸：截图与 dispatchGesture 共用同一全屏坐标系。
  // device_info 的 displayMetrics 在带虚拟导航键的机型上不含导航栏高度，
  // 用它换算会导致 Y 轴整体偏移（底部按钮机型点不准）。
  const w = img.naturalWidth;
  const h = img.naturalHeight;
  if (!w || !h || !rect.width || !rect.height) return null;
  const x = ((e.clientX - rect.left) / rect.width) * w;
  const y = ((e.clientY - rect.top) / rect.height) * h;
  return { x: Math.round(x), y: Math.round(y) };
}

function onPointerDown(e: PointerEvent) {
  const p = toDeviceCoords(e);
  if (!p) return;
  pressStart = { ...p, t: Date.now() };
  (e.target as HTMLElement).setPointerCapture(e.pointerId);
}

function onPointerUp(e: PointerEvent) {
  const dev = selected.value;
  const end = toDeviceCoords(e);
  if (!dev || !end || !pressStart) { pressStart = null; return; }
  const start = pressStart;
  pressStart = null;

  const dist = Math.hypot(end.x - start.x, end.y - start.y);
  if (dist < 10) {
    tap(dev.device_id, end.x, end.y);
  } else {
    const duration = Math.min(Math.max(Date.now() - start.t, 100), 1000);
    swipe(dev.device_id, start.x, start.y, end.x, end.y, duration);
  }
  // 操作后稍等界面响应再补一帧（串流时由定时器负责）
  if (streamingId.value !== dev.device_id) {
    setTimeout(() => requestScreenshot(dev.device_id), 500);
  }
}

// 当前所选设备的录音；未选设备时显示全部
const visibleRecordings = computed(() =>
  selectedId.value
    ? recordings.value.filter((r) => r.device_id === selectedId.value)
    : recordings.value,
);

function fmtTime(ts: number) {
  return ts ? new Date(ts * 1000).toLocaleString() : '—';
}

function fmtSize(bytes: number) {
  if (bytes > 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB';
  if (bytes > 1024) return (bytes / 1024).toFixed(0) + ' KB';
  return bytes + ' B';
}
</script>

<template>
  <div class="flex flex-col h-full bg-gray-950 text-gray-50">
    <!-- 顶部 -->
    <div class="p-4 border-b border-gray-800 space-y-2">
      <div class="flex items-center gap-2 text-lg font-semibold">
        <Smartphone class="w-5 h-5 text-emerald-400" />
        手机控制
        <span class="text-[11px] bg-gray-800 text-gray-300 px-2 py-0.5 rounded-full">
          在线 {{ devices.filter(d => d.online).length }} / {{ devices.length }}
        </span>
        <button @click="refreshDevices" class="ml-auto flex items-center gap-1 text-xs text-gray-400 hover:text-gray-200">
          <RefreshCw class="w-3.5 h-3.5" /> 刷新
        </button>
      </div>
      <div v-if="serverInfo" class="text-xs text-gray-500">
        手机端 App 连接地址：
        <span v-for="ip in serverInfo.ips" :key="ip" class="font-mono bg-gray-900 border border-gray-800 rounded px-1.5 py-0.5 mr-1.5 text-gray-300">
          {{ ip }}:{{ serverInfo.port }}
        </span>
        <span class="text-gray-600">（手机需与电脑在同一局域网，并开启 AutoCast Mobile 的无障碍服务）</span>
      </div>
      <div v-if="lastError" class="text-xs text-red-400">{{ lastError }}</div>
    </div>

    <div class="flex flex-1 min-h-0">
      <!-- 左侧：设备列表 -->
      <div class="w-80 flex-shrink-0 border-r border-gray-800 overflow-y-auto p-3 space-y-2">
        <div v-if="devices.length === 0" class="text-sm text-gray-500 text-center pt-10 px-4 leading-relaxed">
          暂无设备。<br />在手机上安装 AutoCast Mobile，填入上方地址即可自动连接。
        </div>
        <div
          v-for="d in devices" :key="d.device_id"
          @click="selectDevice(d)"
          :class="['rounded-lg border p-3 cursor-pointer transition-colors',
            selectedId === d.device_id ? 'border-emerald-600 bg-gray-900' : 'border-gray-800 bg-gray-900/40 hover:bg-gray-900']"
        >
          <div class="flex items-center gap-2">
            <span :class="['w-2 h-2 rounded-full flex-shrink-0', d.online ? 'bg-emerald-500' : 'bg-gray-600']" />
            <!-- 备注（显示名）行 -->
            <template v-if="editingId === d.device_id">
              <input
                v-model="editingRemark" placeholder="备注，如：张三的手机"
                @keyup.enter="commitEdit" @keyup.esc="editingId = null" @click.stop
                class="flex-1 min-w-0 bg-gray-950 border border-gray-700 rounded px-2 py-0.5 text-sm focus:outline-none focus:border-emerald-600"
                autofocus
              />
              <button @click.stop="commitEdit" class="text-emerald-400 hover:text-emerald-300"><Check class="w-4 h-4" /></button>
              <button @click.stop="editingId = null" class="text-gray-500 hover:text-gray-300"><X class="w-4 h-4" /></button>
            </template>
            <template v-else>
              <span class="font-medium text-sm truncate">{{ displayName(d) }}</span>
              <button @click.stop="beginEdit(d)" class="text-gray-500 hover:text-gray-200" title="编辑备注">
                <Pencil class="w-3.5 h-3.5" />
              </button>
              <button
                v-if="!d.online" @click.stop="deleteDevice(d.device_id)"
                class="ml-auto text-gray-600 hover:text-red-400" title="删除离线设备记录"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </template>
          </div>
          <div class="mt-1.5 text-[11px] text-gray-500 space-y-0.5">
            <div>型号：{{ d.model }} <span v-if="d.width">· {{ d.width }}×{{ d.height }}</span></div>
            <div class="font-mono truncate">{{ d.device_id }}</div>
            <div>{{ d.online ? '在线' : '最后在线：' + fmtTime(d.last_seen) }}</div>
          </div>
        </div>

      </div>

      <!-- 右侧：屏幕画面 + 控制 / 通话记录 -->
      <div class="flex-1 min-w-0 flex flex-col">
        <!-- 模式切换 -->
        <div class="flex items-center gap-1 px-3 pt-3 border-b border-gray-800">
          <button
            @click="viewMode = 'control'"
            :class="['flex items-center gap-1.5 text-xs px-3 py-2 rounded-t-lg border-b-2 -mb-px',
              viewMode === 'control' ? 'border-emerald-500 text-gray-100' : 'border-transparent text-gray-500 hover:text-gray-300']"
          >
            <Smartphone class="w-3.5 h-3.5" /> 实时控制
          </button>
          <button
            @click="viewMode = 'recordings'; refreshRecordings()"
            :class="['flex items-center gap-1.5 text-xs px-3 py-2 rounded-t-lg border-b-2 -mb-px',
              viewMode === 'recordings' ? 'border-emerald-500 text-gray-100' : 'border-transparent text-gray-500 hover:text-gray-300']"
          >
            <Phone class="w-3.5 h-3.5" /> 通话记录
            <span v-if="visibleRecordings.length" class="text-[10px] bg-gray-700 text-gray-200 px-1.5 rounded-full">{{ visibleRecordings.length }}</span>
          </button>
        </div>

        <!-- 实时控制 -->
        <div v-if="viewMode === 'control'" class="flex-1 min-h-0 flex flex-col">
        <div v-if="!selected" class="flex-1 flex items-center justify-center text-gray-600 text-sm">
          从左侧选择一台手机开始控制
        </div>
        <template v-else>
          <!-- 控制条 -->
          <div class="flex items-center gap-2 px-4 py-2.5 border-b border-gray-800 flex-wrap">
            <span class="text-sm font-medium">{{ displayName(selected) }}</span>
            <span :class="['text-[11px] px-2 py-0.5 rounded-full', selected.online ? 'bg-emerald-700 text-white' : 'bg-gray-700 text-gray-300']">
              {{ selected.online ? '在线' : '离线' }}
            </span>

            <div class="ml-auto flex items-center gap-2">
              <select
                v-model.number="intervalMs" @change="onIntervalChange"
                class="bg-gray-900 border border-gray-700 rounded px-2 py-1 text-xs text-gray-300 focus:outline-none"
              >
                <option :value="500">0.5 秒/帧</option>
                <option :value="800">0.8 秒/帧</option>
                <option :value="1500">1.5 秒/帧</option>
                <option :value="3000">3 秒/帧</option>
              </select>
              <button
                @click="toggleStream" :disabled="!selected.online"
                :class="['flex items-center gap-1.5 text-xs px-3 py-1.5 rounded disabled:opacity-40',
                  streamingId === selected.device_id ? 'bg-red-700 hover:bg-red-600 text-white' : 'bg-emerald-700 hover:bg-emerald-600 text-white']"
              >
                <Square v-if="streamingId === selected.device_id" class="w-3.5 h-3.5" />
                <Play v-else class="w-3.5 h-3.5" />
                {{ streamingId === selected.device_id ? '停止实时画面' : '实时画面' }}
              </button>
              <button
                @click="requestScreenshot(selected.device_id)" :disabled="!selected.online"
                class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 text-gray-200 disabled:opacity-40"
              >
                <Camera class="w-3.5 h-3.5" /> 截图
              </button>
            </div>
          </div>

          <!-- 屏幕区域 -->
          <div class="flex-1 min-h-0 flex items-center justify-center bg-gray-900/40 p-4">
            <div v-if="!currentFrame" class="text-gray-600 text-sm text-center leading-relaxed">
              {{ selected.online ? '等待画面… 点击「实时画面」或「截图」获取手机屏幕' : '设备离线，无法获取画面' }}
            </div>
            <img
              v-else
              ref="screenImg"
              :src="currentFrame"
              draggable="false"
              @pointerdown="onPointerDown"
              @pointerup="onPointerUp"
              class="max-h-full max-w-full object-contain rounded-lg border border-gray-800 cursor-crosshair select-none touch-none shadow-lg"
              :title="selected.online ? '单击 = 点按手机；按住拖动 = 滑动' : ''"
            />
          </div>

          <!-- 底部按键 + 状态 -->
          <div class="flex items-center gap-2 px-4 py-2.5 border-t border-gray-800">
            <button @click="pressKey(selected.device_id, 'back')" :disabled="!selected.online"
              class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 disabled:opacity-40">
              <ArrowLeft class="w-3.5 h-3.5" /> 返回
            </button>
            <button @click="pressKey(selected.device_id, 'home')" :disabled="!selected.online"
              class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 disabled:opacity-40">
              <Home class="w-3.5 h-3.5" /> 主页
            </button>
            <button @click="pressKey(selected.device_id, 'recents')" :disabled="!selected.online"
              class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 disabled:opacity-40">
              <LayoutGrid class="w-3.5 h-3.5" /> 多任务
            </button>
            <button @click="pressKey(selected.device_id, 'notifications')" :disabled="!selected.online"
              class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 disabled:opacity-40">
              <Bell class="w-3.5 h-3.5" /> 通知栏
            </button>
            <span class="ml-auto text-[11px] text-gray-600">
              <template v-if="currentFrameAge">画面更新于 {{ currentFrameAge }} · </template>
              在画面上单击=点按，拖动=滑动
            </span>
          </div>
        </template>
        </div>

        <!-- 通话记录 -->
        <div v-else class="flex-1 min-h-0 flex flex-col">
          <div class="flex items-center gap-2 px-4 py-2.5 border-b border-gray-800">
            <span class="text-sm font-medium">
              通话录音
              <span v-if="selected" class="text-gray-500 text-xs">· {{ displayName(selected) }}</span>
              <span v-else class="text-gray-500 text-xs">· 全部设备</span>
            </span>
            <button @click="refreshRecordings()" class="ml-auto flex items-center gap-1 text-xs text-gray-400 hover:text-gray-200">
              <RefreshCw class="w-3.5 h-3.5" /> 刷新
            </button>
          </div>

          <div class="flex-1 overflow-y-auto p-4 space-y-2">
            <div v-if="visibleRecordings.length === 0" class="text-sm text-gray-500 text-center pt-12 leading-relaxed">
              暂无通话录音。<br />
              通话结束后手机会自动回传录音（需在手机系统设置中开启「通话自动录音」）。
            </div>
            <div
              v-for="r in visibleRecordings" :key="r.path"
              class="rounded-lg border border-gray-800 bg-gray-900/40 p-3"
            >
              <div class="flex items-center gap-2 mb-2">
                <Phone class="w-4 h-4 text-emerald-400 flex-shrink-0" />
                <span class="text-sm text-gray-200 truncate">{{ r.name }}</span>
                <button
                  @click="deleteRecording(r.path)"
                  class="ml-auto text-gray-600 hover:text-red-400" title="删除录音"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
              <div class="text-[11px] text-gray-500 mb-2 flex flex-wrap gap-x-3">
                <span>{{ fmtTime(r.modified) }}</span>
                <span>{{ fmtSize(r.size) }}</span>
                <span v-if="!selectedId" class="truncate">
                  {{ devices.find(d => d.device_id === r.device_id) ? displayName(devices.find(d => d.device_id === r.device_id)!) : r.device_id }}
                </span>
              </div>
              <audio :src="recordingUrl(r.path)" controls preload="none" class="w-full h-9" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
