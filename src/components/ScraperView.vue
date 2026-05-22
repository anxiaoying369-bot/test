<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  Play, Square, RefreshCw,
  CheckCircle, XCircle, Loader2, AlertTriangle, ChevronDown
} from 'lucide-vue-next';

// ============ 状态 ============

interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

interface ScraperProgress {
  task_id: string;
  status: string;
  progress: number;
  total: number;
  current_type: string;
  current_user: string;
  stats: Record<string, any>;
  log_lines: string[];
  started_at: number;
  finished_at: number | null;
}

const accounts = ref<Account[]>([]);
const selectedAccount = ref('');
const secUid = ref('');
const scrapeType = ref('all');
const limit = ref(0);
const skipExisting = ref(false);
const isRunning = ref(false);
const currentTaskId = ref('');
const progress = ref<ScraperProgress | null>(null);
const error = ref('');
let pollTimer: ReturnType<typeof setInterval> | null = null;

// ============ 计算属性 ============

const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));

const typeOptions = [
  { value: 'all', label: '全部（作品+评论+回复）' },
  { value: 'video', label: '仅作品' },
  { value: 'comment', label: '仅评论' },
  { value: 'reply', label: '仅回复' },
];

const statusLabel = computed(() => {
  if (!progress.value) return '';
  const s = progress.value.status;
  if (s === 'running') return '采集中';
  if (s === 'completed') return '已完成';
  if (s === 'error') return '出错';
  if (s === 'cookie_expired') return 'Cookie 过期';
  if (s === 'cancelled') return '已取消';
  return s;
});

const statusColor = computed(() => {
  if (!progress.value) return '';
  const s = progress.value.status;
  if (s === 'running') return 'text-blue-400';
  if (s === 'completed') return 'text-green-400';
  if (s === 'error') return 'text-red-400';
  if (s === 'cookie_expired') return 'text-yellow-400';
  if (s === 'cancelled') return 'text-gray-400';
  return 'text-gray-400';
});

const progressPercent = computed(() => {
  if (!progress.value) return 0;
  return Math.min(100, progress.value.progress);
});

const elapsedSeconds = computed(() => {
  if (!progress.value) return 0;
  const end = progress.value.finished_at || Date.now() / 1000;
  return Math.round(end - progress.value.started_at);
});

const elapsedStr = computed(() => {
  const s = elapsedSeconds.value;
  if (s < 60) return `${s}秒`;
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}分${r}秒`;
});

// ============ 方法 ============

async function loadAccounts() {
  try {
    const res = await invoke('list_accounts', { platform: null }) as Account[];
    accounts.value = res;
  } catch (e) {
    console.error('加载账号失败:', e);
  }
}

async function startScrape() {
  if (!selectedAccount.value) {
    error.value = '请选择一个账号';
    return;
  }
  if (!secUid.value.trim()) {
    error.value = '请输入目标用户的 sec_uid';
    return;
  }

  error.value = '';
  isRunning.value = true;
  progress.value = null;

  try {
    const task: any = await invoke('start_scrape', {
      accountName: selectedAccount.value,
      platform: 'douyin',
      secUid: secUid.value.trim(),
      scrapeType: scrapeType.value,
      limit: limit.value,
      skipExisting: skipExisting.value,
    });
    currentTaskId.value = task.task_id;
    startPolling();
  } catch (e: any) {
    error.value = String(e);
    isRunning.value = false;
  }
}

async function cancelScrape() {
  if (!currentTaskId.value) return;
  try {
    await invoke('cancel_scrape', { taskId: currentTaskId.value });
    stopPolling();
    isRunning.value = false;
  } catch (e) {
    console.error('取消失败:', e);
  }
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(async () => {
    if (!currentTaskId.value) return;
    try {
      const p = await invoke('get_scrape_progress', { taskId: currentTaskId.value }) as ScraperProgress;
      progress.value = p;
      if (p.status !== 'running') {
        stopPolling();
        isRunning.value = false;
      }
    } catch (e) {
      // 进度文件可能还没创建，忽略
    }
  }, 1500);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function resetForm() {
  isRunning.value = false;
  currentTaskId.value = '';
  progress.value = null;
  error.value = '';
}

// ============ 生命周期 ============

onMounted(() => {
  loadAccounts();
});

onUnmounted(() => {
  stopPolling();
});
</script>

<template>
  <div class="flex flex-col h-full overflow-y-auto p-6">
    <!-- 标题 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-bold">评论采集</h2>
      <span class="text-xs text-gray-500 bg-gray-900 px-3 py-1 rounded-full border border-gray-800 font-mono">
        {{ douyinAccounts.length }} 个抖音账号
      </span>
    </div>

    <!-- 配置区域 -->
    <div class="bg-gray-900 p-5 rounded-xl mb-6" v-if="!isRunning && progress?.status !== 'completed'">
      <div class="grid grid-cols-2 gap-4">
        <!-- 选择账号 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">使用账号（Cookie 来源）</label>
          <div class="relative">
            <select v-model="selectedAccount"
              class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white appearance-none focus:outline-none focus:border-blue-500 pr-8">
              <option value="" disabled>选择已授权的抖音账号</option>
              <option v-for="acc in douyinAccounts" :key="acc.name" :value="acc.name">
                {{ acc.name }}{{ acc.meta?.nickname ? ` (${acc.meta.nickname})` : '' }}
              </option>
            </select>
            <ChevronDown class="w-4 h-4 text-gray-500 absolute right-2.5 top-2.5 pointer-events-none" />
          </div>
        </div>

        <!-- 目标 sec_uid -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">目标用户 sec_uid</label>
          <input v-model="secUid" type="text"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
            placeholder="从抖音主页 URL 获取，如 MS4wLjAB..." />
        </div>

        <!-- 采集类型 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">采集类型</label>
          <div class="relative">
            <select v-model="scrapeType"
              class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white appearance-none focus:outline-none focus:border-blue-500 pr-8">
              <option v-for="opt in typeOptions" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
            <ChevronDown class="w-4 h-4 text-gray-500 absolute right-2.5 top-2.5 pointer-events-none" />
          </div>
        </div>

        <!-- 限制数量 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">采集数量限制（0=不限）</label>
          <input v-model.number="limit" type="number" min="0"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500" />
        </div>
      </div>

      <!-- 选项行 -->
      <div class="flex items-center justify-between mt-4">
        <label class="flex items-center gap-2 text-sm text-gray-400 cursor-pointer">
          <input type="checkbox" v-model="skipExisting" class="rounded bg-gray-950 border-gray-700" />
          跳过已采集的数据
        </label>
        <button @click="startScrape"
          class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-5 py-2 rounded-lg text-sm font-medium transition-colors">
          <Play class="w-4 h-4" />
          开始采集
        </button>
      </div>

      <!-- 错误提示 -->
      <div v-if="error" class="mt-3 text-sm text-red-400 bg-red-900/20 px-3 py-2 rounded-lg">
        {{ error }}
      </div>
    </div>

    <!-- 进度区域 -->
    <div v-if="isRunning || progress" class="bg-gray-900 p-5 rounded-xl mb-6">
      <!-- 头部状态 -->
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-3">
          <Loader2 v-if="isRunning" class="w-5 h-5 text-blue-400 animate-spin" />
          <CheckCircle v-else-if="progress?.status === 'completed'" class="w-5 h-5 text-green-400" />
          <XCircle v-else-if="progress?.status === 'error'" class="w-5 h-5 text-red-400" />
          <AlertTriangle v-else-if="progress?.status === 'cookie_expired'" class="w-5 h-5 text-yellow-400" />
          <span class="text-sm font-medium" :class="statusColor">{{ statusLabel }}</span>
          <span v-if="progress?.current_type && isRunning" class="text-xs text-gray-500">
            正在采集: {{ progress.current_type }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 font-mono">{{ elapsedStr }}</span>
          <button v-if="isRunning" @click="cancelScrape"
            class="flex items-center gap-1 bg-red-900/30 hover:bg-red-800 text-red-400 px-3 py-1 rounded text-xs transition-colors">
            <Square class="w-3 h-3" />
            取消
          </button>
          <button v-if="!isRunning" @click="resetForm"
            class="flex items-center gap-1 bg-gray-800 hover:bg-gray-700 px-3 py-1 rounded text-xs transition-colors">
            <RefreshCw class="w-3 h-3" />
            重新采集
          </button>
        </div>
      </div>

      <!-- 进度条 -->
      <div class="w-full bg-gray-800 rounded-full h-2 mb-4">
        <div class="h-2 rounded-full transition-all duration-500"
          :class="{
            'bg-blue-500': progress?.status === 'running',
            'bg-green-500': progress?.status === 'completed',
            'bg-red-500': progress?.status === 'error',
            'bg-yellow-500': progress?.status === 'cookie_expired',
            'bg-gray-600': progress?.status === 'cancelled',
          }"
          :style="{ width: progressPercent + '%' }">
        </div>
      </div>

      <!-- 统计数据 -->
      <div v-if="progress?.stats" class="grid grid-cols-3 gap-3 mb-4">
        <div v-for="(stats, type) in progress.stats" :key="type"
          class="bg-gray-950 p-3 rounded-lg border border-gray-800">
          <div class="text-xs text-gray-500 mb-1">
            {{ type === 'video' ? '作品' : type === 'comment' ? '评论' : '回复' }}
          </div>
          <div class="text-lg font-bold">{{ stats.new || stats.total || 0 }}</div>
          <div class="text-[10px] text-gray-600">
            共 {{ stats.total || 0 }} 条 / 新增 {{ stats.new || 0 }}
            <span v-if="stats.duration"> / {{ stats.duration }}</span>
          </div>
        </div>
      </div>

      <!-- 实时日志 -->
      <div v-if="progress?.log_lines?.length" class="mt-4">
        <div class="text-xs text-gray-500 mb-1.5">实时日志</div>
        <div class="bg-gray-950 rounded-lg border border-gray-800 p-3 max-h-[200px] overflow-y-auto font-mono text-[11px] text-gray-500 leading-relaxed">
          <div v-for="(line, i) in progress.log_lines.slice(-50)" :key="i">{{ line }}</div>
        </div>
      </div>
    </div>

    <!-- Cookie 过期提示 -->
    <div v-if="progress?.status === 'cookie_expired'" class="bg-yellow-900/20 border border-yellow-800 p-4 rounded-xl mb-6">
      <div class="flex items-center gap-2 mb-2">
        <AlertTriangle class="w-4 h-4 text-yellow-400" />
        <span class="text-sm text-yellow-400 font-medium">Cookie 已过期</span>
      </div>
      <p class="text-xs text-gray-400">请前往「账号管理」页面重新授权该账号，然后再尝试采集。</p>
    </div>

    <!-- 使用说明 -->
    <div v-if="!isRunning && !progress" class="p-4 bg-gray-900/50 rounded-xl border border-gray-800">
      <h4 class="text-sm font-medium mb-2 text-gray-300">使用说明</h4>
      <ul class="text-xs text-gray-500 space-y-1">
        <li>1. 选择一个已授权的抖音账号（用于提供 Cookie）</li>
        <li>2. 输入目标用户的 sec_uid（从抖音主页 URL 中获取）</li>
        <li>3. 选择采集类型：作品、评论、回复，或全量采集</li>
        <li>4. 点击「开始采集」，实时查看进度和日志</li>
        <li>5. 采集数据保存在 ~/Library/Application Support/AutoCastAI/scraper_data/</li>
      </ul>
    </div>
  </div>
</template>
