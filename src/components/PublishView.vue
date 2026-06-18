<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { Send, RefreshCw, Trash2, X, Calendar, Clock, CheckCircle2, AlertCircle, Loader2, Film, Users, Upload } from 'lucide-vue-next';
import { open } from '@tauri-apps/plugin-dialog';
import { usePublish, type PublishTask, type PublishableVideo } from '../composables/usePublish';

const {
  tasks, videos, accounts, lastError,
  init, refreshVideos, refreshAccounts,
  createTasks, cancelTask, retryTask, deleteTask,
} = usePublish();

// ─── 表单状态 ───
const selectedVideo = ref<string>('');
/** 用户自主上传（从磁盘选）的本地视频，与工作室成片合并展示 */
const localVideos = ref<PublishableVideo[]>([]);
const allVideos = computed<PublishableVideo[]>(() => [...localVideos.value, ...videos.value]);

async function pickLocalVideo() {
  const picked = await open({
    multiple: false,
    filters: [{ name: 'Videos', extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm'] }],
  });
  if (!picked || Array.isArray(picked)) return;
  const path = picked as string;
  // 去重：已在列表里（成片或之前选过的本地）就直接选中
  if (!allVideos.value.some((v) => v.path === path)) {
    const name = path.split(/[\\/]/).pop() || path;
    localVideos.value.unshift({ path, name, size: 0, modified: Date.now() / 1000, local: true });
  }
  selectedVideo.value = path;
}
const selectedAccounts = ref<string[]>([]);
const title = ref('');
const tagsInput = ref('');
const description = ref('');
const location = ref('');
const scheduleMode = ref<'now' | 'scheduled'>('now');
const scheduleTime = ref(''); // datetime-local
const submitting = ref(false);
const formError = ref('');

const tags = computed(() =>
  tagsInput.value.split(/[,，#]/).map((t) => t.trim()).filter(Boolean),
);

function toggleAccount(name: string) {
  const i = selectedAccounts.value.indexOf(name);
  if (i >= 0) selectedAccounts.value.splice(i, 1);
  else selectedAccounts.value.push(name);
}

function fmtSize(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes > 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${bytes} B`;
}

function fmtTime(secs: number | null): string {
  if (!secs) return '';
  return new Date(secs * 1000).toLocaleString('zh-CN', { hour12: false });
}

async function submit() {
  formError.value = '';
  if (!selectedVideo.value) { formError.value = '请选择要发布的视频'; return; }
  if (selectedAccounts.value.length === 0) { formError.value = '请至少选择一个发布账号'; return; }
  if (!title.value.trim()) { formError.value = '请填写标题'; return; }

  let scheduledAt: number | null = null;
  if (scheduleMode.value === 'scheduled') {
    if (!scheduleTime.value) { formError.value = '请选择定时发布的时间'; return; }
    scheduledAt = Math.floor(new Date(scheduleTime.value).getTime() / 1000);
    if (scheduledAt <= Math.floor(Date.now() / 1000)) {
      formError.value = '定时时间必须晚于当前时间'; return;
    }
  }

  submitting.value = true;
  try {
    await createTasks({
      account_names: selectedAccounts.value.slice(),
      video_path: selectedVideo.value,
      title: title.value.trim(),
      tags: tags.value,
      description: description.value.trim(),
      location: location.value.trim() || null,
      scheduled_at: scheduledAt,
    });
    // 成功后重置表单（保留选中的账号便于继续矩阵发布不同视频）
    title.value = '';
    tagsInput.value = '';
    description.value = '';
  } catch (e) {
    formError.value = String(e);
  } finally {
    submitting.value = false;
  }
}

// ─── 任务状态展示 ───
const statusMeta: Record<PublishTask['status'], { label: string; cls: string }> = {
  pending: { label: '排队中', cls: 'text-amber-400 bg-amber-400/10' },
  publishing: { label: '发布中', cls: 'text-blue-400 bg-blue-400/10' },
  success: { label: '已发布', cls: 'text-green-400 bg-green-400/10' },
  failed: { label: '失败', cls: 'text-red-400 bg-red-400/10' },
  cancelled: { label: '已取消', cls: 'text-gray-400 bg-gray-400/10' },
};

function accountLabel(name: string): string {
  const a = accounts.value.find((x) => x.name === name);
  return a?.meta.nickname || name;
}

onMounted(init);
</script>

<template>
  <div class="flex flex-col h-full bg-gray-950 text-gray-200">
    <header class="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
      <div>
        <h1 class="text-lg font-semibold flex items-center gap-2">
          <Send :size="18" class="text-purple-400" /> 发布排期
        </h1>
        <p class="text-xs text-gray-500 mt-0.5">一键发布、定时排期、多账号矩阵分发到抖音创作者中心</p>
      </div>
      <button @click="() => { refreshVideos(); refreshAccounts(); }"
        class="text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1">
        <RefreshCw :size="14" /> 刷新
      </button>
    </header>

    <div class="flex-1 min-h-0 grid grid-cols-2 gap-0">
      <!-- 左：新建发布任务 -->
      <section class="overflow-y-auto p-6 border-r border-gray-800 space-y-5">
        <!-- 选视频 -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <label class="text-xs font-medium text-gray-400 flex items-center gap-1.5">
              <Film :size="14" /> 选择视频
            </label>
            <button type="button" @click="pickLocalVideo"
              class="text-xs text-purple-400 hover:text-purple-300 flex items-center gap-1">
              <Upload :size="13" /> 上传本地视频
            </button>
          </div>
          <div v-if="allVideos.length === 0" class="text-xs text-gray-600 py-3">
            还没有视频。可点右上「上传本地视频」选一个，或在「视频工作室」生成成片。
          </div>
          <div v-else class="space-y-1.5 max-h-44 overflow-y-auto pr-1">
            <label v-for="v in allVideos" :key="v.path"
              :class="['flex items-center gap-2.5 px-3 py-2 rounded-lg cursor-pointer text-sm border',
                selectedVideo === v.path ? 'border-purple-500 bg-purple-500/10' : 'border-gray-800 hover:border-gray-700']">
              <input type="radio" name="video" :value="v.path" v-model="selectedVideo" class="accent-purple-500" />
              <span class="truncate flex-1">{{ v.name }}</span>
              <span v-if="v.local" class="text-[10px] px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 shrink-0">本地</span>
              <span v-else class="text-[10px] text-gray-500 shrink-0">{{ fmtSize(v.size) }}</span>
            </label>
          </div>
        </div>

        <!-- 选账号（矩阵） -->
        <div>
          <label class="text-xs font-medium text-gray-400 flex items-center gap-1.5 mb-2">
            <Users :size="14" /> 发布账号（多选=矩阵分发，共 {{ selectedAccounts.length }} 个）
          </label>
          <div v-if="accounts.length === 0" class="text-xs text-gray-600 py-3">
            还没有抖音账号，请先到「账号管理」扫码登录。
          </div>
          <div v-else class="flex flex-wrap gap-2">
            <button v-for="a in accounts" :key="a.id" type="button" @click="toggleAccount(a.name)"
              :class="['flex items-center gap-2 px-3 py-1.5 rounded-full text-xs border transition',
                selectedAccounts.includes(a.name) ? 'border-purple-500 bg-purple-500/15 text-purple-200' : 'border-gray-700 text-gray-400 hover:border-gray-600']">
              <img v-if="a.meta.avatar" :src="a.meta.avatar" class="w-4 h-4 rounded-full" />
              {{ a.meta.nickname || a.name }}
            </button>
          </div>
        </div>

        <!-- 文案 -->
        <div>
          <label class="text-xs font-medium text-gray-400 mb-2 block">标题（30 字内）</label>
          <input v-model="title" maxlength="30" placeholder="抓眼球的标题"
            class="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-800 text-sm focus:border-purple-500 outline-none" />
        </div>
        <div>
          <label class="text-xs font-medium text-gray-400 mb-2 block">话题（逗号分隔，自动加 #）</label>
          <input v-model="tagsInput" placeholder="例如：好物推荐, 干货分享"
            class="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-800 text-sm focus:border-purple-500 outline-none" />
          <div v-if="tags.length" class="flex flex-wrap gap-1 mt-1.5">
            <span v-for="t in tags" :key="t" class="text-[10px] px-1.5 py-0.5 rounded bg-gray-800 text-purple-300">#{{ t }}</span>
          </div>
        </div>
        <div>
          <label class="text-xs font-medium text-gray-400 mb-2 block">作品描述（可选）</label>
          <textarea v-model="description" rows="2" placeholder="补充描述，留空则用标题"
            class="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-800 text-sm focus:border-purple-500 outline-none resize-none" />
        </div>
        <div>
          <label class="text-xs font-medium text-gray-400 mb-2 block">地理位置（可选）</label>
          <input v-model="location" placeholder="例如：上海"
            class="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-800 text-sm focus:border-purple-500 outline-none" />
        </div>

        <!-- 发布时机 -->
        <div>
          <label class="text-xs font-medium text-gray-400 mb-2 block">发布时机</label>
          <div class="flex gap-2">
            <button type="button" @click="scheduleMode = 'now'"
              :class="['flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-sm border',
                scheduleMode === 'now' ? 'border-purple-500 bg-purple-500/10 text-purple-200' : 'border-gray-800 text-gray-400']">
              <Clock :size="14" /> 立即发布
            </button>
            <button type="button" @click="scheduleMode = 'scheduled'"
              :class="['flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-sm border',
                scheduleMode === 'scheduled' ? 'border-purple-500 bg-purple-500/10 text-purple-200' : 'border-gray-800 text-gray-400']">
              <Calendar :size="14" /> 定时发布
            </button>
          </div>
          <input v-if="scheduleMode === 'scheduled'" v-model="scheduleTime" type="datetime-local"
            class="w-full mt-2 px-3 py-2 rounded-lg bg-gray-900 border border-gray-800 text-sm focus:border-purple-500 outline-none" />
        </div>

        <p v-if="formError" class="text-xs text-red-400">{{ formError }}</p>
        <button @click="submit" :disabled="submitting"
          class="w-full py-2.5 rounded-lg bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white text-sm font-medium flex items-center justify-center gap-2">
          <Loader2 v-if="submitting" :size="16" class="animate-spin" />
          <Send v-else :size="16" />
          {{ scheduleMode === 'now' ? '立即发布' : '加入排期' }}
          <span v-if="selectedAccounts.length > 1">（{{ selectedAccounts.length }} 个账号）</span>
        </button>
      </section>

      <!-- 右：任务列表 -->
      <section class="overflow-y-auto p-6">
        <h2 class="text-sm font-medium text-gray-300 mb-3">发布任务</h2>
        <p v-if="lastError" class="text-xs text-red-400 mb-2">{{ lastError }}</p>
        <div v-if="tasks.length === 0" class="text-xs text-gray-600 py-8 text-center">还没有发布任务。</div>
        <div v-else class="space-y-2.5">
          <div v-for="t in tasks" :key="t.id" class="p-3 rounded-lg bg-gray-900 border border-gray-800">
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0">
                <div class="text-sm font-medium truncate">{{ t.title }}</div>
                <div class="text-[11px] text-gray-500 mt-0.5">
                  @{{ accountLabel(t.account_name) }}
                  <span v-if="t.scheduled_at"> · 排期 {{ fmtTime(t.scheduled_at) }}</span>
                </div>
              </div>
              <span :class="['text-[10px] px-2 py-0.5 rounded-full shrink-0 flex items-center gap-1', statusMeta[t.status].cls]">
                <CheckCircle2 v-if="t.status === 'success'" :size="11" />
                <AlertCircle v-else-if="t.status === 'failed'" :size="11" />
                <Loader2 v-else-if="t.status === 'publishing'" :size="11" class="animate-spin" />
                {{ statusMeta[t.status].label }}
              </span>
            </div>

            <!-- 进度条 -->
            <div v-if="t.status === 'publishing'" class="mt-2">
              <div class="h-1 rounded-full bg-gray-800 overflow-hidden">
                <div class="h-full bg-blue-500 transition-all" :style="{ width: `${t.progress}%` }" />
              </div>
              <div class="text-[10px] text-gray-500 mt-1">{{ t.stage || '处理中' }}…</div>
            </div>

            <div v-if="t.status === 'failed' && t.error_msg" class="mt-1.5 text-[11px] text-red-400/90">{{ t.error_msg }}</div>
            <a v-if="t.status === 'success' && t.result_url" :href="t.result_url" target="_blank"
              class="mt-1.5 inline-block text-[11px] text-purple-400 hover:underline">查看作品 ↗</a>

            <!-- 操作 -->
            <div class="flex items-center gap-3 mt-2 text-[11px]">
              <button v-if="t.status === 'pending' || t.status === 'publishing'" @click="cancelTask(t.id)"
                class="text-gray-400 hover:text-red-400 flex items-center gap-1"><X :size="12" /> 取消</button>
              <button v-if="t.status === 'failed' || t.status === 'cancelled'" @click="retryTask(t.id)"
                class="text-gray-400 hover:text-blue-400 flex items-center gap-1"><RefreshCw :size="12" /> 重试</button>
              <button @click="deleteTask(t.id)"
                class="text-gray-400 hover:text-red-400 flex items-center gap-1 ml-auto"><Trash2 :size="12" /> 删除</button>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
