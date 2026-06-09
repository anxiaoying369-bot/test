<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { MessageCircle, KeyRound, Plug, Search, Bell, Radio, Loader2, Play, Pause, Square, Volume2 } from 'lucide-vue-next';
import { useWeChat, type WeChatMessage } from '../composables/useWeChat';

const wx = useWeChat();
const {
  connected, monitoring, accountDir, hexKey, statusText, busy,
  contacts, friendCount, groupCount, currentSessionId, messages, watched, newMessages,
  transcribeVoice, sttReady, sttDownloading, checkSTTModel, downloadSTTModel,
} = wx;

// 媒体懒加载：localId → data URL（视频缩略图）
const mediaUrls = ref<Record<number, string>>({});
const transcribing = ref<Record<number, boolean>>({}); // localId -> loading
const transcriptions = ref<Record<number, string>>({}); // localId -> text

const loadingVoice = ref<number | null>(null);
const playingId = ref<number | null>(null);   // 当前加载到播放器的语音 localId
const isPaused = ref(false);                   // 当前语音是否处于暂停
let currentAudio: HTMLAudioElement | null = null;

function stopVoice() {
  if (currentAudio) { currentAudio.pause(); currentAudio.currentTime = 0; currentAudio = null; }
  playingId.value = null;
  isPaused.value = false;
}

// 点击语音气泡：未播放→加载播放；正在播放本条→暂停；本条已暂停→继续
async function toggleVoice(m: WeChatMessage) {
  const id = m.localId ?? null;
  if (loadingVoice.value === id) return;

  if (playingId.value === id && currentAudio) {
    if (isPaused.value) { await currentAudio.play(); isPaused.value = false; }
    else { currentAudio.pause(); isPaused.value = true; }
    return;
  }

  // 切换到另一条：先停掉旧的
  stopVoice();
  loadingVoice.value = id;
  try {
    const url = await wx.getVoiceUrl(m);
    if (!url) { statusText.value = '未取到语音数据'; return; }
    const audio = new Audio(url);
    audio.onended = () => { if (playingId.value === id) stopVoice(); };
    currentAudio = audio;
    playingId.value = id;
    isPaused.value = false;
    await audio.play();
  } finally {
    loadingVoice.value = null;
  }
}

async function doTranscribe(m: WeChatMessage, sessionId?: string) {
  const id = m.localId;
  if (id == null || transcribing.value[id]) return;

  // 检查模型是否就绪
  if (!sttReady.value) {
    const ok = await checkSTTModel();
    if (!ok) {
      if (confirm('语音转文字需要下载 SenseVoiceSmall 模型 (约 900MB)，是否立即下载？\n建议在 Wi-Fi 环境下进行。')) {
        const success = await downloadSTTModel();
        if (!success) {
          statusText.value = '模型下载失败，请检查网络';
          return;
        }
      } else {
        return;
      }
    }
  }

  transcribing.value[id] = true;
  try {
    const text = await transcribeVoice(m, sessionId);
    if (text) transcriptions.value[id] = text;
    else statusText.value = '语音识别未返回结果';
  } finally {
    transcribing.value[id] = false;
  }
}

async function loadThumb(m: WeChatMessage) {
  const id = m.localId;
  if (id == null || mediaUrls.value[id] !== undefined) return;
  mediaUrls.value[id] = '';  // 占位，避免重复请求
  // 图片走 get_image（缩略图），视频走 get_media（明文 jpg 缩略图）
  const url = m.localType === 3 ? await wx.getImageUrl(m, false) : await wx.getMediaUrl(m);
  if (url) mediaUrls.value[id] = url;
}

// 大图查看 modal
const bigImage = ref<string | null>(null);
const bigLoading = ref(false);
async function viewLarge(m: WeChatMessage) {
  bigLoading.value = true;
  bigImage.value = null;
  try {
    const url = await wx.getImageUrl(m, true);
    if (url) bigImage.value = url;
    else statusText.value = '大图加载失败';
  } finally {
    bigLoading.value = false;
  }
}
function closeLarge() { bigImage.value = null; bigLoading.value = false; }

// 切换会话后，预取图片/视频缩略图
watch(messages, (ms) => {
  stopVoice();
  mediaUrls.value = {};
  for (const m of ms) {
    if (m.localType === 3 || m.localType === 43) loadThumb(m);
  }
});

// 自动转文字：新消息到达时
watch(newMessages, (evts) => {
  // 如果模型没好，自动转文字暂不触发，避免频繁弹窗（手动点转文字才会弹窗）
  if (!sttReady.value) return;
  for (const evt of evts) {
    for (const m of evt.messages) {
      if (m.localType === 34 && m.localId && !transcriptions.value[m.localId] && !transcribing.value[m.localId]) {
        doTranscribe(m, evt.sessionId);
      }
    }
  }
}, { deep: true });

const sessionFilter = ref('');
const intervalSecs = ref(5);
const activeFilter = ref<'all' | 'friend' | 'group'>('all');

const filteredContacts = computed(() => {
  const kw = sessionFilter.value.trim().toLowerCase();
  return contacts.value.filter(c => {
    if (activeFilter.value !== 'all' && c.category !== activeFilter.value) return false;
    if (!kw) return true;
    return (c.displayName || '').toLowerCase().includes(kw) ||
           (c.username || '').toLowerCase().includes(kw);
  });
});

const currentSessionName = computed(() => {
  const c = contacts.value.find(c => c.username === currentSessionId.value);
  return c ? c.displayName : currentSessionId.value;
});

const currentIsGroup = computed(() => {
  const c = contacts.value.find(c => c.username === currentSessionId.value);
  return !!(c && c.isGroup);
});

function fmtTime(sec: number) {
  if (!sec) return '';
  return new Date(sec * 1000).toLocaleString();
}

function msgText(m: WeChatMessage) {
  return m.parsedContent || m.content || '';
}

onMounted(async () => {
  await wx.initListener();
  await wx.loadCredentials();
  await wx.refreshStatus();
  if (connected.value) await wx.loadContacts();
  await checkSTTModel(); // 启动时异步检查模型状态
});
</script>

<template>
  <div class="flex flex-col h-full bg-gray-950 text-gray-50">
    <!-- 顶部：连接区 -->
    <div class="p-4 border-b border-gray-800 space-y-3">
      <div class="flex items-center gap-2 text-lg font-semibold">
        <MessageCircle class="w-5 h-5 text-green-500" />
        微信聊天监控
        <span v-if="connected" class="text-[11px] bg-green-700 text-white px-2 py-0.5 rounded-full">已连接</span>
        <span v-else class="text-[11px] bg-gray-700 text-gray-300 px-2 py-0.5 rounded-full">未连接</span>
        <span v-if="monitoring" class="text-[11px] bg-red-600 text-white px-2 py-0.5 rounded-full flex items-center gap-1">
          <Radio class="w-3 h-3" /> 监控中
        </span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-[1fr_1fr_auto] gap-2 items-end">
        <label class="text-xs text-gray-400 flex flex-col gap-1">
          账号目录 (accountDir)
          <input v-model="accountDir" placeholder="自动获取或手动粘贴 .../xwechat_files/wxid_xxx"
            class="bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-sm text-gray-100 focus:outline-none focus:border-green-600" />
        </label>
        <label class="text-xs text-gray-400 flex flex-col gap-1">
          数据库密钥 (hexKey)
          <input v-model="hexKey" placeholder="64 位十六进制密钥" spellcheck="false"
            class="bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-sm text-gray-100 font-mono focus:outline-none focus:border-green-600" />
        </label>
        <div class="flex gap-2">
          <button @click="wx.autoGetKey()" :disabled="busy"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded bg-gray-800 hover:bg-gray-700 text-sm disabled:opacity-50">
            <Loader2 v-if="busy" class="w-4 h-4 animate-spin" />
            <KeyRound v-else class="w-4 h-4 text-amber-400" />
            自动获取密钥
          </button>
          <button @click="wx.connect()" :disabled="busy"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded bg-green-700 hover:bg-green-600 text-sm disabled:opacity-50">
            <Plug class="w-4 h-4" /> 连接
          </button>
        </div>
      </div>

      <p v-if="statusText" class="text-xs text-gray-400">{{ statusText }}</p>
    </div>

    <!-- 主体：会话列表 / 聊天 / 新消息 -->
    <div class="flex flex-1 min-h-0">
      <!-- 左：会话列表 -->
      <div class="w-72 flex-shrink-0 border-r border-gray-800 flex flex-col">
        <div class="p-2 border-b border-gray-800 space-y-2">
          <div class="flex items-center gap-2 bg-gray-900 rounded px-2">
            <Search class="w-4 h-4 text-gray-500" />
            <input v-model="sessionFilter" placeholder="搜索联系人/群"
              class="flex-1 bg-transparent py-1.5 text-sm focus:outline-none" />
          </div>
          <!-- 筛选 tab：全部 / 联系人 / 群聊 -->
          <div class="flex gap-1 text-xs">
            <button @click="activeFilter = 'all'"
              :class="['flex-1 py-1 rounded', activeFilter === 'all' ? 'bg-green-700 text-white' : 'bg-gray-900 text-gray-400 hover:bg-gray-800']">
              全部 {{ friendCount + groupCount }}
            </button>
            <button @click="activeFilter = 'friend'"
              :class="['flex-1 py-1 rounded', activeFilter === 'friend' ? 'bg-green-700 text-white' : 'bg-gray-900 text-gray-400 hover:bg-gray-800']">
              联系人 {{ friendCount }}
            </button>
            <button @click="activeFilter = 'group'"
              :class="['flex-1 py-1 rounded', activeFilter === 'group' ? 'bg-green-700 text-white' : 'bg-gray-900 text-gray-400 hover:bg-gray-800']">
              群聊 {{ groupCount }}
            </button>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto">
          <div v-if="!connected" class="p-4 text-xs text-gray-500">连接后显示通讯录</div>
          <div v-else-if="!filteredContacts.length" class="p-4 text-xs text-gray-500">无匹配的联系人</div>
          <div v-for="c in filteredContacts" :key="c.username"
            @click="wx.openSession(c.username)"
            :class="['flex items-center gap-2 px-3 py-2 cursor-pointer border-b border-gray-900',
                     currentSessionId === c.username ? 'bg-gray-900' : 'hover:bg-gray-900/50']">
            <input type="checkbox" :checked="!!watched[c.username]" @click.stop="wx.toggleWatch(c)"
              class="accent-green-600" title="监控此会话" />
            <div class="flex-1 min-w-0">
              <div class="text-sm truncate flex items-center gap-1.5">
                <span v-if="c.isGroup" class="text-[9px] bg-gray-700 text-gray-300 px-1 rounded shrink-0">群</span>
                {{ c.displayName || c.username }}
              </div>
              <div class="text-[10px] text-gray-500 truncate">{{ c.username }}</div>
            </div>
            <Bell v-if="watched[c.username]" class="w-3.5 h-3.5 text-amber-400" />
          </div>
        </div>

        <!-- 监控控制 -->
        <div class="p-2 border-t border-gray-800 space-y-2">
          <div class="flex items-center gap-2 text-xs text-gray-400">
            轮询间隔
            <input v-model.number="intervalSecs" type="number" min="2" max="120"
              class="w-16 bg-gray-900 border border-gray-700 rounded px-1.5 py-1 text-gray-100" /> 秒
          </div>
          <button v-if="!monitoring" @click="wx.startMonitor(intervalSecs)" :disabled="!connected"
            class="w-full px-3 py-1.5 rounded bg-red-700 hover:bg-red-600 text-sm disabled:opacity-50">
            开始监控（已选 {{ Object.keys(watched).length }}）
          </button>
          <button v-else @click="wx.stopMonitor()"
            class="w-full px-3 py-1.5 rounded bg-gray-700 hover:bg-gray-600 text-sm">
            停止监控
          </button>
        </div>
      </div>

      <!-- 中：聊天内容 -->
      <div class="flex-1 min-w-0 flex flex-col">
        <div class="px-4 py-2 border-b border-gray-800 text-sm font-medium">
          {{ currentSessionName || '选择左侧会话查看聊天记录' }}
        </div>
        <div class="flex-1 overflow-y-auto p-4 space-y-3">
          <div v-if="!messages.length" class="text-xs text-gray-500">暂无消息</div>
          <div v-for="(m, i) in messages" :key="m.localId || m.svrId || i"
            :class="['flex flex-col max-w-[75%]', (m.isSender ? 'items-end ml-auto' : 'items-start')]">
            <div class="text-[10px] text-gray-500 mb-0.5">
              <span v-if="currentIsGroup && !m.isSender && m.senderName" class="text-cyan-400 mr-1">{{ m.senderName }}</span>
              {{ fmtTime(m.createTime) }}
            </div>
            <!-- 语音：点击播放/暂停，活动时可停止 -->
            <div v-if="m.localType === 34" class="space-y-2 max-w-full">
              <div :class="['flex items-center gap-1.5 px-2 py-2 rounded-lg w-fit',
                            m.isSender ? 'bg-green-700 text-white ml-auto' : 'bg-gray-800 text-gray-100']">
                <button @click="toggleVoice(m)" class="flex items-center gap-1.5 hover:opacity-90 min-w-[64px]">
                  <Loader2 v-if="loadingVoice === m.localId" class="w-4 h-4 animate-spin" />
                  <Pause v-else-if="playingId === m.localId && !isPaused" class="w-4 h-4" />
                  <Play v-else class="w-4 h-4" />
                  <Volume2 class="w-4 h-4" />
                  <span class="text-xs">语音</span>
                </button>
                <button v-if="playingId === m.localId" @click="stopVoice" title="停止"
                  class="p-0.5 rounded hover:bg-black/20">
                  <Square class="w-3.5 h-3.5" />
                </button>
                <div class="w-px h-3 bg-white/20 mx-1"></div>
                <button @click="doTranscribe(m)" :disabled="transcribing[m.localId as number]"
                  class="text-[10px] opacity-70 hover:opacity-100 flex items-center gap-1">
                  <Loader2 v-if="transcribing[m.localId as number]" class="w-3 h-3 animate-spin" />
                  {{ transcriptions[m.localId as number] ? '重新识别' : '转文字' }}
                </button>
              </div>
              <div v-if="transcriptions[m.localId as number]"
                :class="['px-3 py-2 rounded-lg text-xs italic break-words',
                         m.isSender ? 'bg-green-900/50 text-gray-300 ml-auto' : 'bg-gray-900/50 text-gray-400']">
                {{ transcriptions[m.localId as number] }}
              </div>
            </div>

            <!-- 图片：缩略图，点击看大图 -->
            <div v-else-if="m.localType === 3">
              <img v-if="mediaUrls[m.localId as number]" :src="mediaUrls[m.localId as number]"
                @click="viewLarge(m)"
                class="max-w-[220px] max-h-[260px] rounded-lg object-contain cursor-zoom-in hover:opacity-90" />
              <div v-else class="w-32 h-32 rounded-lg bg-gray-800 flex items-center justify-center text-xs text-gray-500">
                [图片] 加载中…
              </div>
            </div>

            <!-- 视频：缩略图 + 播放标记，点击用系统播放器播放 -->
            <div v-else-if="m.localType === 43" class="relative cursor-pointer" @click="wx.openVideo(m)">
              <img v-if="mediaUrls[m.localId as number]" :src="mediaUrls[m.localId as number]"
                class="max-w-[220px] max-h-[220px] rounded-lg object-cover" />
              <div v-else class="w-40 h-28 rounded-lg bg-gray-800 flex items-center justify-center text-xs text-gray-500">
                视频缩略图加载中…
              </div>
              <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
                <div class="w-10 h-10 rounded-full bg-black/50 flex items-center justify-center">
                  <Play class="w-5 h-5 text-white" />
                </div>
              </div>
            </div>

            <!-- 其它（文本 / 图片占位 / 表情占位等）-->
            <div v-else :class="['px-3 py-2 rounded-lg text-sm whitespace-pre-wrap break-words',
                          m.isSender ? 'bg-green-700 text-white' : 'bg-gray-800 text-gray-100']">
              {{ msgText(m) }}
            </div>
          </div>
        </div>
      </div>

      <!-- 右：新消息提示 -->
      <div class="w-80 flex-shrink-0 border-l border-gray-800 flex flex-col">
        <div class="px-3 py-2 border-b border-gray-800 flex items-center justify-between">
          <span class="text-sm font-medium flex items-center gap-1.5">
            <Bell class="w-4 h-4 text-amber-400" /> 新消息
          </span>
          <button @click="wx.clearNewMessages()" class="text-[11px] text-gray-500 hover:text-gray-300">清空</button>
        </div>
        <div class="flex-1 overflow-y-auto p-2 space-y-2">
          <div v-if="!newMessages.length" class="p-3 text-xs text-gray-500">
            勾选会话并开始监控后，新消息会实时出现在这里。
          </div>
          <div v-for="(evt, i) in newMessages" :key="i"
            @click="wx.openSession(evt.sessionId)"
            class="bg-gray-900 rounded-lg p-2.5 cursor-pointer hover:bg-gray-800 border border-gray-800">
            <div class="flex items-center justify-between mb-1">
              <span class="text-sm font-medium text-green-400 truncate">{{ evt.displayName || evt.sessionId }}</span>
              <span class="text-[10px] bg-red-600 text-white px-1.5 rounded-full">{{ evt.messages.length }}</span>
            </div>
            <div v-for="(m, j) in evt.messages.slice(0, 3)" :key="j" class="text-xs text-gray-300 truncate">
              {{ msgText(m) }}
            </div>
            <div v-if="evt.messages.length > 3" class="text-[10px] text-gray-500">…等 {{ evt.messages.length }} 条</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 大图查看 modal -->
    <div v-if="bigImage || bigLoading" @click="closeLarge"
      class="fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-8 cursor-zoom-out">
      <Loader2 v-if="bigLoading" class="w-8 h-8 text-white animate-spin" />
      <img v-else-if="bigImage" :src="bigImage" @click.stop
        class="max-w-[92vw] max-h-[92vh] object-contain rounded shadow-2xl" />
    </div>

    <!-- STT 模型下载提示 -->
    <div v-if="sttDownloading" class="fixed inset-0 z-[60] bg-black/70 flex items-center justify-center p-6">
      <div class="bg-gray-900 border border-gray-700 rounded-xl p-6 w-full max-w-md shadow-2xl flex flex-col items-center justify-center space-y-4">
        <Loader2 class="w-8 h-8 text-green-500 animate-spin" />
        <div class="text-green-400 font-medium text-center">
          正在下载语音识别模型...<br>
          <span class="text-xs text-gray-400 font-normal mt-2 block">模型约 900MB，仅首次使用需下载。<br>请保持网络连接，这可能需要几分钟时间。</span>
        </div>
      </div>
    </div>
  </div>
</template>
