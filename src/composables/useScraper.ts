import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface ScraperAccount {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

export interface ScraperProgress {
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

export function useScraper() {
  const accounts = ref<ScraperAccount[]>([]);
  const selectedAccount = ref('');
  const secUid = ref('');
  const scrapeType = ref('all');
  const limit = ref(0);
  const skipExisting = ref(true);
  const incremental = ref(true);
  const isRunning = ref(false);
  const currentTaskId = ref('');
  const progress = ref<ScraperProgress | null>(null);
  const error = ref('');
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));

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

  async function loadAccounts() {
    try {
      const res = await invoke('list_accounts', { platform: null }) as ScraperAccount[];
      accounts.value = res;
    } catch (e) {
      console.error('加载账号失败:', e);
    }
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
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
          await invoke('clear_current_task');
        }
      } catch (e) {
        // ignore
      }
    }, 1500);
  }

  async function startScrape() {
    if (!selectedAccount.value) {
      error.value = '请选择一个账号';
      return;
    }
    const input = secUid.value.trim();
    if (!input) {
      error.value = '请输入目标用户的 sec_uid 或主页链接';
      return;
    }

    error.value = '';
    isRunning.value = true;
    progress.value = null;

    try {
      const resolvedSecUid = await invoke('resolve_user_sec_uid', { input }) as string;
      const task: any = await invoke('start_scrape', {
        accountName: selectedAccount.value,
        platform: 'douyin',
        secUid: resolvedSecUid,
        scrapeType: scrapeType.value,
        limit: limit.value,
        skipExisting: skipExisting.value,
        incremental: incremental.value,
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

  function resetForm() {
    isRunning.value = false;
    currentTaskId.value = '';
    progress.value = null;
    error.value = '';
  }

  onMounted(async () => {
    loadAccounts();
    try {
      const activeTaskId = await invoke('get_current_task') as string | null;
      if (activeTaskId) {
        currentTaskId.value = activeTaskId;
        isRunning.value = true;
        startPolling();
      }
    } catch (e) {
      console.error('任务恢复失败:', e);
    }
  });

  onUnmounted(() => {
    stopPolling();
  });

  return {
    accounts, selectedAccount, secUid, scrapeType, limit, skipExisting, incremental,
    isRunning, currentTaskId, progress, error,
    douyinAccounts, statusLabel, statusColor, progressPercent, elapsedSeconds, elapsedStr,
    loadAccounts, startScrape, cancelScrape, resetForm,
  };
}
