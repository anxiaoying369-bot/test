<script setup lang="ts">
import { ref, onMounted, provide } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Users, RefreshCw, Trash2, CheckCircle, XCircle, HelpCircle, Search, FileText, Radio, MessageCircle, Settings, Database, Sparkles, Terminal } from 'lucide-vue-next';
import ScraperView from './components/ScraperView.vue';
import ResultsView from './components/ResultsView.vue';
import LiveMonitorView from './components/LiveMonitorView.vue';
import DouyinIMView from './components/DouyinIMView.vue';
import SettingsView from './components/SettingsView.vue';
import ChatView from './components/ChatView.vue';
import KnowledgeBaseView from './components/KnowledgeBaseView.vue';
import ContentStudioView from './components/ContentStudioView.vue';
import VideoStudioView from './components/VideoStudioView.vue';
import HermesGatewayView from './components/HermesGatewayView.vue';

type PageKey = 'chat' | 'accounts' | 'scraper' | 'results' | 'live_monitor' | 'douyin_im' | 'settings' | 'kb' | 'studio' | 'video_studio' | 'hermes';
const currentPage = ref<PageKey>('accounts');
const settingsInitialTab = ref<string>('model');

function navigateTo(page: PageKey, settingsTab?: string) {
  currentPage.value = page;
  if (page === 'settings' && settingsTab) {
    settingsInitialTab.value = settingsTab;
  }
}

provide('navigateTo', navigateTo);
provide('settingsInitialTab', settingsInitialTab);
const accounts = ref<any[]>([]);
const isLoginModalOpen = ref(false);
const currentPlatform = ref('');
const loginStatus = ref('');
const sessionId = ref('');
const loginStep = ref<'init' | 'waiting' | 'saving' | 'success' | 'error'>('init');
const accountNameInput = ref('抖音账号');
const debugMsg = ref('系统就绪');
const verifyingIds = ref<Set<string>>(new Set());
const confirmDeleteKey = ref<string | null>(null); // "platform:name" 待确认删除

// ============ 账号列表 ============

async function loadAccounts() {
  debugMsg.value = '正在加载账号...';
  try {
    const res = await invoke('list_accounts', { platform: null }) as any[];
    accounts.value = res;
    debugMsg.value = `账号加载成功 (${res.length})`;
  } catch (e) {
    console.error('加载账号失败:', e);
    debugMsg.value = '加载失败: ' + e;
  }
}

// ============ 登录流程（手动确认模式）============

async function startLogin(platform: string) {
  currentPlatform.value = platform;
  isLoginModalOpen.value = true;
  loginStep.value = 'init';
  loginStatus.value = '正在打开浏览器...';
  accountNameInput.value = '抖音账号';
  debugMsg.value = `准备登录: ${platform}`;

  try {
    const session: any = await invoke('init_login_session', { platform });
    sessionId.value = session.session_id;
    // 浏览器已开启,等用户自己登录,然后手动点"我已登录完成"
    loginStep.value = 'waiting';
    loginStatus.value = '浏览器已打开,请在浏览器中完成抖音登录';
  } catch (e) {
    console.error('初始化登录失败:', e);
    loginStep.value = 'error';
    loginStatus.value = '登录初始化失败: ' + e;
    debugMsg.value = '初始化失败: ' + e;
  }
}

// ============ 验证账号 ============

async function verifyAccount(account: any) {
  const key = `${account.platform}:${account.name}`;
  verifyingIds.value.add(key);
  try {
    const result: any = await invoke('verify_account', {
      platform: account.platform,
      name: account.name,
    });

    // 更新 accounts 中对应账号的状态
    const idx = accounts.value.findIndex(
      (a) => a.platform === account.platform && a.name === account.name
    );
    if (idx >= 0) {
      accounts.value[idx] = {
        ...accounts.value[idx],
        verify_status: result.status,
        verify_method: result.method,
        verify_message: result.message,
      };
    }
    debugMsg.value = `验证完成: ${account.name} → ${result.status}`;
  } catch (e) {
    console.error('验证失败:', e);
    debugMsg.value = `验证失败: ${e}`;
  } finally {
    verifyingIds.value.delete(key);
  }
}

// ============ 删除账号 ============

function accountKey(account: any) {
  return `${account.platform}:${account.name}`;
}

function requestDeleteAccount(account: any) {
  confirmDeleteKey.value = accountKey(account);
}

function cancelDeleteAccount() {
  confirmDeleteKey.value = null;
}

async function confirmDeleteAccount(account: any) {
  confirmDeleteKey.value = null;
  try {
    await invoke('delete_account', {
      platform: account.platform,
      name: account.name,
    });
    debugMsg.value = `已删除: ${account.name}`;
    await loadAccounts();
  } catch (e) {
    console.error('删除失败:', e);
    debugMsg.value = '删除失败: ' + e;
  }
}

// ============ 关闭弹窗 ============

async function closeModal() {
  if (sessionId.value) {
    try {
      await invoke('cleanup_login_session', { sessionId: sessionId.value });
    } catch (e) {
      console.error(e);
    }
  }
  isLoginModalOpen.value = false;
  sessionId.value = '';
  loginStep.value = 'init';
  loginStatus.value = '';
}

// ============ 我已登录完成 → 触发保存 ============

async function confirmAndSave() {
  const name = accountNameInput.value.trim();
  if (!name) {
    debugMsg.value = '账号名称不能为空';
    return;
  }

  loginStep.value = 'saving';
  loginStatus.value = '正在抓取 Cookie 并保存...';
  try {
    const account: any = await invoke('finish_login', {
      sessionId: sessionId.value,
      accountName: name,
    });
    console.log('账号已保存:', account);
    debugMsg.value = `账号已保存: ${name}`;
    loginStep.value = 'success';
    loginStatus.value = '✓ 保存成功';
    // 后端 Python 会自动关浏览器,前端 1.5s 后关弹窗
    setTimeout(async () => {
      await closeModal();
      await loadAccounts();
    }, 1500);
  } catch (e) {
    console.error('保存失败:', e);
    loginStep.value = 'error';
    loginStatus.value = String(e);
    debugMsg.value = '保存失败: ' + e;
  }
}

// ============ 重试 ============

async function retryLogin() {
  await closeModal();
  await startLogin(currentPlatform.value);
}

// ============ 生命周期 ============

// ============ 直播监控全局状态 ============
interface LiveMessage {
  time: string;
  user_name: string;
  user_id: string;
  content?: string;
  gift_name?: string;
  gift_count?: number;
  count?: number;
  gender?: string;
}

interface LiveRoom {
  id: string;
  anchor_name: string;
  status: 'connecting' | 'running' | 'stopped' | 'error';
  messages: { type: string; payload: LiveMessage }[];
  error?: string;
}

const liveMonitorRooms = ref<Record<string, LiveRoom>>({});
const activeLiveEventUnlisten = ref<any>(null);

import { listen } from '@tauri-apps/api/event';

async function initLiveEventListener() {
  if (activeLiveEventUnlisten.value) return;
  activeLiveEventUnlisten.value = await listen('live-event', (event: any) => {
    const data = event.payload;
    const rid = data.live_id;
    
    if (!liveMonitorRooms.value[rid]) {
      // 忽略已删除或未初始化的房间的迟到事件
      return;
    }

    const room = liveMonitorRooms.value[rid];

    if (data.type === 'status') {
      if (data.status === 'starting') room.status = 'connecting';
      if (data.status === 'running') room.status = 'running';
      if (data.status === 'stopped') room.status = 'stopped';
      if (data.anchor_name) room.anchor_name = data.anchor_name;
    } else if (data.type === 'init') {
      room.status = 'running';
      if (data.anchor_name) room.anchor_name = data.anchor_name;
    } else if (data.type === 'data') {
      if (data.anchor_name && !room.anchor_name) room.anchor_name = data.anchor_name;
      room.messages.push({ type: data.data_type, payload: data.payload });
      if (room.messages.length > 1000) room.messages.shift();
    } else if (data.type === 'error') {
      room.status = 'error';
      room.error = data.message;
    }
  });
}

onMounted(() => {
  loadAccounts();
  initLiveEventListener();
});

// ============ 辅助 ============

function statusIcon(status: string) {
  if (status === 'valid') return CheckCircle;
  if (status === 'invalid') return XCircle;
  return HelpCircle;
}

function statusBg(status: string) {
  if (status === 'valid') return 'bg-green-900/30 text-green-400';
  if (status === 'invalid') return 'bg-red-900/30 text-red-400';
  return 'bg-gray-800 text-gray-400';
}

function isVerifying(platform: string, name: string) {
  return verifyingIds.value.has(`${platform}:${name}`);
}
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-gray-950 text-gray-50 font-sans">
    <!-- 左侧导航（固定宽度，不随内容区压缩） -->
    <aside class="flex flex-col w-56 flex-shrink-0 h-full bg-gray-950 border-r border-gray-800">
      <div class="p-6 font-bold tracking-tight">AutoCast AI</div>
      <nav class="flex-1 px-3 space-y-1">
        <a href="#" @click="currentPage = 'chat'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'chat' ? 'bg-gray-900' : 'text-gray-400']">
          <MessageSquare class="w-5 h-5 text-blue-500" />
          <span>AI 助理对话</span>
        </a>
        <a href="#" @click="currentPage = 'studio'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'studio' ? 'bg-gray-900' : 'text-gray-400']">
          <Sparkles class="w-5 h-5 text-purple-400" />
          <span>AI 创作中心</span>
        </a>
        <a href="#" @click="currentPage = 'video_studio'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'video_studio' ? 'bg-gray-900' : 'text-gray-400']">
          <Film class="w-5 h-5 text-orange-400" />
          <span>视频创作中心</span>
        </a>
        <a href="#" @click="currentPage = 'accounts'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'accounts' ? 'bg-gray-900' : 'text-gray-400']">
          <Users class="w-5 h-5" />
          <span>账号管理</span>
        </a>
        <a href="#" @click="currentPage = 'scraper'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'scraper' ? 'bg-gray-900' : 'text-gray-400']">
          <Search class="w-5 h-5 text-purple-500" />
          <span>评论采集</span>
        </a>
        <a href="#" @click="currentPage = 'results'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'results' ? 'bg-gray-900' : 'text-gray-400']">
          <FileText class="w-5 h-5 text-green-500" />
          <span>采集结果</span>
        </a>
        <a href="#" @click="currentPage = 'kb'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'kb' ? 'bg-gray-900' : 'text-gray-400']">
          <Database class="w-5 h-5 text-blue-500" />
          <span>企业知识库</span>
        </a>
        <a href="#" @click="currentPage = 'live_monitor'" :class="['flex items-center justify-between px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'live_monitor' ? 'bg-gray-900' : 'text-gray-400']">
          <div class="flex items-center gap-3">
            <Radio class="w-5 h-5 text-red-500" />
            <span>直播监控</span>
          </div>
          <span v-if="Object.keys(liveMonitorRooms).length > 0" class="text-[10px] bg-red-600 text-white px-1.5 py-0.5 rounded-full font-bold">
            {{ Object.values(liveMonitorRooms).filter(r => r.status === 'running').length }}
          </span>
        </a>
        <div class="pt-4 mt-4 border-t border-gray-900 space-y-1">
          <a href="#" @click="currentPage = 'settings'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'settings' ? 'bg-gray-900' : 'text-gray-400']">
            <Settings class="w-5 h-5" />
            <span>系统设置</span>
          </a>
          <a href="#" @click="currentPage = 'hermes'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'hermes' ? 'bg-gray-900' : 'text-gray-400']">
            <Terminal class="w-5 h-5 text-indigo-400" />
            <span>Hermes Agent</span>
          </a>
        </div>
      </nav>
    </aside>

    <!-- 主内容：AI 助理 -->
    <main v-if="currentPage === 'chat'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ChatView />
    </main>

    <!-- 主内容：AI 创作中心 -->
    <main v-if="currentPage === 'studio'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ContentStudioView />
    </main>

    <!-- 主内容：视频创作中心 -->
    <main v-if="currentPage === 'video_studio'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <VideoStudioView />
    </main>

    <!-- 主内容：Hermes 网关 -->
    <main v-if="currentPage === 'hermes'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <HermesGatewayView />
    </main>

    <!-- 主内容：账号管理 -->
    <main v-if="currentPage === 'accounts'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950 p-6 overflow-y-auto">
      <div class="flex justify-between items-center mb-8">
        <h2 class="text-xl font-bold">账号管理</h2>
        <div class="text-xs text-gray-500 bg-gray-900 px-3 py-1 rounded-full border border-gray-800 font-mono">
          {{ debugMsg }}
        </div>
      </div>

      <!-- 账号列表 -->
      <div class="grid grid-cols-1 gap-6">
        <!-- 抖音 -->
        <div class="bg-gray-900 p-5 rounded-xl">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg">♪ 抖音</h3>
            <button @click="startLogin('douyin')" class="text-xs bg-blue-600 px-3 py-1.5 rounded-lg hover:bg-blue-700 transition-colors">+ 新增授权</button>
          </div>
          <div v-if="accounts.filter(a => a.platform === 'douyin').length === 0" class="text-gray-500 text-sm py-8 text-center border border-dashed border-gray-800 rounded-lg">
            暂无授权账号
          </div>
          <div v-for="acc in accounts.filter(a => a.platform === 'douyin')" :key="`${acc.platform}:${acc.name}`" class="p-3 bg-gray-950 rounded-lg border border-gray-800 mb-2">
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center gap-3">
                <div v-if="acc.meta?.avatar" class="w-8 h-8 rounded-full bg-gray-800 overflow-hidden flex-shrink-0">
                  <img :src="acc.meta.avatar" class="w-full h-full object-cover" />
                </div>
                <div v-else class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center text-sm flex-shrink-0">♪</div>
                <div>
                  <div class="text-sm font-medium flex items-center gap-2">
                    {{ acc.name }}
                    <span v-if="acc.verify_status" :class="['inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded font-mono', statusBg(acc.verify_status)]">
                      <component :is="statusIcon(acc.verify_status)" class="w-3 h-3" />
                      {{ acc.verify_status }}
                    </span>
                  </div>
                  <div class="text-xs text-gray-500">{{ acc.meta?.nickname || acc.meta?.user_id || '—' }}</div>
                </div>
              </div>
              <div class="flex items-center gap-1">
                <template v-if="confirmDeleteKey === accountKey(acc)">
                  <span class="text-xs text-red-400 mr-1">确认删除?</span>
                  <button @click.stop="confirmDeleteAccount(acc)" class="text-xs bg-red-700 hover:bg-red-600 text-white px-2 py-1 rounded transition-colors">确认</button>
                  <button @click.stop="cancelDeleteAccount()" class="text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 px-2 py-1 rounded transition-colors">取消</button>
                </template>
                <template v-else>
                  <button @click="verifyAccount(acc)" :disabled="isVerifying(acc.platform, acc.name)" class="text-xs bg-gray-800 hover:bg-gray-700 px-2 py-1 rounded flex items-center gap-1 transition-colors disabled:opacity-50">
                    <RefreshCw class="w-3 h-3" :class="isVerifying(acc.platform, acc.name) ? 'animate-spin' : ''" />
                    验证
                  </button>
                  <button @click.stop="requestDeleteAccount(acc)" class="text-xs bg-red-900/30 hover:bg-red-800 text-red-400 px-2 py-1 rounded transition-colors">
                    <Trash2 class="w-3 h-3" />
                  </button>
                </template>
              </div>
            </div>
            <div v-if="acc.verify_message" class="text-[10px] text-gray-500 font-mono mt-1 px-2">
              {{ acc.verify_message }}
            </div>
          </div>
        </div>
      </div>

      <!-- 使用说明 -->
      <div class="mt-8 p-4 bg-gray-900/50 rounded-xl border border-gray-800">
        <h4 class="text-sm font-medium mb-2 text-gray-300">💡 使用说明</h4>
        <ul class="text-xs text-gray-500 space-y-1">
          <li>• 点击「新增授权」后会自动打开 Chrome 浏览器</li>
          <li>• 在浏览器中手动完成抖音登录(扫码或账号密码均可)</li>
          <li>• 登录完成后回到本软件,点击「我已登录完成」</li>
          <li>• 输入账号名称(用于区分多账号)即可保存</li>
          <li>• 点击「验证」可检查 Cookie 是否有效</li>
        </ul>
      </div>
    </main>

    <!-- 主内容：评论采集 -->
    <main v-if="currentPage === 'scraper'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ScraperView />
    </main>

    <!-- 主内容：采集结果 -->
    <main v-if="currentPage === 'results'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ResultsView />
    </main>

    <!-- 主内容：企业知识库 -->
    <main v-if="currentPage === 'kb'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <KnowledgeBaseView />
    </main>

    <!-- 主内容：直播监控 -->
    <main v-if="currentPage === 'live_monitor'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <LiveMonitorView :globalRooms="liveMonitorRooms" />
    </main>

    <!-- 主内容：系统设置 -->
    <main v-if="currentPage === 'settings'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <SettingsView />
    </main>

    <!-- 登录弹窗 -->
    <div v-if="isLoginModalOpen" class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center">
      <div class="bg-gray-900 border border-gray-800 rounded-xl p-6 w-[360px] shadow-2xl relative">
        <button @click="closeModal" class="absolute top-4 right-4 text-gray-400 hover:text-white text-xl leading-none">&times;</button>

        <h3 class="text-lg font-bold mb-4 text-center">
          ♪ 抖音登录
        </h3>

        <!-- 状态区域 -->
        <div class="bg-gray-800 rounded-xl p-6 flex flex-col items-center justify-center min-h-[180px] mb-4">
          <div v-if="loginStep === 'init'" class="flex flex-col items-center gap-3">
            <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            <span class="text-gray-400 text-sm">{{ loginStatus }}</span>
          </div>
          <div v-else-if="loginStep === 'waiting'" class="flex flex-col items-stretch gap-3 w-full">
            <div class="flex flex-col items-center gap-2 text-center">
              <div class="text-3xl">🌐</div>
              <span class="text-gray-300 text-sm font-medium">{{ loginStatus }}</span>
              <span class="text-gray-500 text-xs">完成登录后,填好下方账号名再点按钮</span>
            </div>
            <div class="mt-2">
              <label class="text-xs text-gray-400 block mb-1">账号名称(区分多账号)</label>
              <input
                v-model="accountNameInput"
                type="text"
                class="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                placeholder="如:我的工作号"
                @keydown.enter="confirmAndSave"
              />
            </div>
          </div>
          <div v-else-if="loginStep === 'saving'" class="flex flex-col items-center gap-3">
            <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            <span class="text-gray-400 text-sm">{{ loginStatus }}</span>
          </div>
          <div v-else-if="loginStep === 'success'" class="flex flex-col items-center gap-3">
            <CheckCircle class="w-12 h-12 text-green-500" />
            <span class="text-green-400 text-sm font-medium">{{ loginStatus }}</span>
          </div>
          <div v-else-if="loginStep === 'error'" class="flex flex-col items-center gap-3 text-center">
            <XCircle class="w-12 h-12 text-red-500" />
            <span class="text-red-400 text-sm">{{ loginStatus }}</span>
          </div>
        </div>

        <!-- 底部操作 -->
        <div class="flex flex-col gap-2">
          <!-- 等待中:用户登录完毕主动点 -->
          <button v-if="loginStep === 'waiting'" @click="confirmAndSave" class="w-full bg-green-600 hover:bg-green-700 text-white py-2.5 rounded-lg text-sm font-medium transition-colors">
            我已登录完成
          </button>
          <!-- 取消 -->
          <button v-if="loginStep === 'waiting' || loginStep === 'error'" @click="closeModal" class="w-full bg-gray-800 hover:bg-gray-700 text-white py-2.5 rounded-lg text-sm font-medium transition-colors">
            取消
          </button>
          <!-- 重试 -->
          <button v-if="loginStep === 'error'" @click="retryLogin" class="w-full bg-blue-600 hover:bg-blue-700 text-white py-2.5 rounded-lg text-sm font-medium transition-colors">
            重试
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
