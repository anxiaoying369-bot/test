<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { CheckCircle, Download, Loader2, RefreshCw, Sparkles, XCircle } from 'lucide-vue-next';

type JsonMap = Record<string, any>;

type InstallStatus = {
  running: boolean;
  progress: number;
  stage: string;
  message: string;
  ready: boolean;
  cli?: string | null;
  error?: string | null;
  started_at?: number | null;
  finished_at?: number | null;
};

const busy = ref(false);
const status = ref('');
const ready = ref(false);
const cli = ref('');
const installHint = ref('');
const installStatus = ref<InstallStatus | null>(null);
const unlistenInstall = ref<UnlistenFn | null>(null);

const installRunning = computed(() => !!installStatus.value?.running);
const showInstallStatus = computed(() => {
  const s = installStatus.value;
  if (!s) return false;
  return !!(
    s.running ||
    s.error ||
    s.started_at ||
    s.finished_at ||
    s.progress > 0 ||
    (s.stage && s.stage !== 'idle')
  );
});
const installProgress = computed(() => Math.max(0, Math.min(100, installStatus.value?.progress ?? 0)));
const installStage = computed(() => installStatus.value?.stage || 'idle');
const installMessage = computed(() => installStatus.value?.message || status.value || '');
const installError = computed(() => installStatus.value?.error || '');

function applyInstallStatus(next?: InstallStatus | null) {
  if (!next) return;
  installStatus.value = next;
  if (next.ready) {
    ready.value = true;
    cli.value = next.cli || cli.value || '';
    installHint.value = '';
  }
  if (next.running) {
    status.value = next.message || 'F5-TTS 正在下载安装中';
  } else if (next.error) {
    status.value = `下载/安装 F5-TTS：失败 - ${next.error}`;
  } else if (next.ready && next.stage === 'done') {
    status.value = '下载/安装 F5-TTS：完成';
  }
}

async function run(label: string, fn: () => Promise<JsonMap>) {
  busy.value = true;
  status.value = label;
  try {
    const res = await fn();
    ready.value = !!res.ready;
    cli.value = res.cli || '';
    installHint.value = res.install_hint || '';
    status.value = `${label}：完成`;
    return res;
  } catch (e: any) {
    status.value = `${label}：失败 - ${String(e)}`;
    return null;
  } finally {
    busy.value = false;
  }
}

async function checkF5() {
  await run('加载 F5-TTS 状态', () => invoke('voice_clone_check'));
  await refreshInstallStatus();
}

async function refreshInstallStatus() {
  try {
    const res = await invoke<JsonMap>('voice_clone_install_status');
    applyInstallStatus(res.status as InstallStatus);
  } catch (e) {
    console.warn('读取 F5-TTS 安装状态失败:', e);
  }
}

async function installF5() {
  busy.value = true;
  status.value = '启动 F5-TTS 后台安装任务…';
  try {
    const res = await invoke<JsonMap>('voice_clone_install');
    applyInstallStatus(res.status as InstallStatus);
    status.value = res.message || 'F5-TTS 安装任务已在后台启动';
  } catch (e: any) {
    status.value = `下载/安装 F5-TTS：失败 - ${String(e)}`;
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  unlistenInstall.value = await listen<InstallStatus>('voice-clone-install-progress', (event) => {
    applyInstallStatus(event.payload);
  });
  await checkF5();
});

onBeforeUnmount(() => {
  unlistenInstall.value?.();
  unlistenInstall.value = null;
});
</script>

<template>
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-8 space-y-8 shadow-xl">
      <div class="flex items-center justify-between p-4 bg-purple-600/5 border border-purple-500/10 rounded-xl">
        <div class="flex items-center gap-4">
          <div class="p-2 bg-purple-600 rounded-lg">
            <Sparkles class="w-5 h-5 text-white" />
          </div>
          <div>
            <h4 class="font-bold">F5-TTS 本地语音克隆</h4>
            <p class="text-xs text-gray-500 mt-0.5">用于「音频实验室」的本地音色克隆与合成。模型/依赖按需下载，不进入默认安装包。</p>
          </div>
        </div>
        <span :class="['text-[10px] px-2 py-0.5 rounded-full font-bold uppercase', ready ? 'bg-green-500/10 text-green-500' : installRunning ? 'bg-purple-500/10 text-purple-300' : 'bg-gray-800 text-gray-500']">
          {{ ready ? 'Ready' : installRunning ? 'Installing' : 'Missing' }}
        </span>
      </div>

      <div class="space-y-4">
        <div class="rounded-xl bg-gray-950 border border-gray-800 p-4 space-y-2">
          <div class="flex items-center gap-2 text-sm font-medium">
            <CheckCircle v-if="ready" class="w-4 h-4 text-green-400" />
            <Loader2 v-else-if="installRunning" class="w-4 h-4 text-purple-300 animate-spin" />
            <XCircle v-else class="w-4 h-4 text-yellow-400" />
            <span>{{ ready ? 'F5-TTS 已加载' : installRunning ? 'F5-TTS 正在下载安装' : '未检测到 F5-TTS' }}</span>
          </div>
          <div class="text-xs text-gray-500 break-all">CLI：{{ cli || '未找到 f5-tts_infer-cli' }}</div>
          <div v-if="installHint" class="text-xs text-yellow-300/90 break-all">{{ installHint }}</div>
          <div class="text-xs text-gray-500 leading-relaxed">
            下载按钮会在 AutoCast 用户数据目录创建独立 Python venv 并安装 f5-tts，避免污染系统 Python。
            安装体积较大，首次下载取决于网络和 PyTorch 镜像速度。
          </div>
        </div>

        <div v-if="showInstallStatus" class="rounded-xl border border-purple-500/20 bg-purple-950/10 p-4 space-y-3">
          <div class="flex items-center justify-between text-xs">
            <span class="text-purple-200 font-medium">安装进度：{{ installStage }}</span>
            <span class="text-purple-300 font-mono">{{ installProgress }}%</span>
          </div>
          <div class="h-2 rounded-full bg-gray-800 overflow-hidden">
            <div class="h-full bg-purple-500 transition-all duration-300" :style="{ width: `${installProgress}%` }"></div>
          </div>
          <div class="text-xs text-gray-300 break-all">
            <Loader2 v-if="installRunning" class="w-3.5 h-3.5 inline mr-1 animate-spin text-purple-300" />
            {{ installMessage }}
          </div>
          <div v-if="installError" class="text-xs text-red-300 whitespace-pre-wrap break-all">{{ installError }}</div>
          <p class="text-[11px] text-gray-500">安装任务在 Tauri 后台执行，切换设置标签或离开本页面不会中断；回到本页会自动恢复当前进度。</p>
        </div>

        <div v-if="status && !showInstallStatus" class="rounded-xl border border-gray-800 bg-gray-950 px-4 py-3 text-sm" :class="status.includes('失败') ? 'text-red-300' : 'text-gray-300'">
          <Loader2 v-if="busy" class="w-4 h-4 inline mr-2 animate-spin" />{{ status }}
        </div>

        <div class="flex gap-3">
          <button
            @click="installF5"
            :disabled="busy || installRunning"
            class="flex items-center gap-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-white px-5 py-3 rounded-xl font-medium transition-all shadow-lg shadow-purple-900/20"
          >
            <Loader2 v-if="installRunning" class="w-4 h-4 animate-spin" />
            <Download v-else class="w-4 h-4" />
            {{ installRunning ? '安装中…' : '下载/安装 F5-TTS' }}
          </button>
          <button
            @click="checkF5"
            :disabled="busy"
            class="flex items-center gap-2 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 px-5 py-3 rounded-xl font-medium transition-all"
          >
            <RefreshCw class="w-4 h-4" />
            重新加载状态
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
