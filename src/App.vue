<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Users, RefreshCw, Trash2, CheckCircle, XCircle, HelpCircle } from 'lucide-vue-next';

const showAccounts = ref(false);
const accounts = ref<any[]>([]);
const isLoginModalOpen = ref(false);
const currentPlatform = ref('');
const qrcodeSrc = ref('');
const loginStatus = ref('');
const sessionId = ref('');
const loginStep = ref<'init' | 'waiting' | 'success' | 'error'>('init');
const debugMsg = ref('系统就绪');
const verifyingIds = ref<Set<string>>(new Set());

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

// ============ 登录流程 ============

async function startLogin(platform: string) {
  currentPlatform.value = platform;
  isLoginModalOpen.value = true;
  loginStep.value = 'init';
  qrcodeSrc.value = '';
  loginStatus.value = '正在初始化登录...';
  debugMsg.value = `准备登录: ${platform}`;

  try {
    const session: any = await invoke('init_login_session', { platform });
    sessionId.value = session.session_id;
    loginStep.value = 'waiting';
    loginStatus.value = '请前往浏览器进行操作';
    pollLoginStatus();
  } catch (e) {
    console.error('初始化登录失败:', e);
    loginStep.value = 'error';
    loginStatus.value = '登录初始化失败: ' + e;
    debugMsg.value = '初始化失败: ' + e;
  }
}

async function pollLoginStatus() {
  if (!isLoginModalOpen.value || !sessionId.value) return;

  try {
    const status: any = await invoke('get_login_status', { sessionId: sessionId.value });

    if (status.status === 'pending' || status.status === 'qrcode') {
      loginStep.value = 'waiting';
      loginStatus.value = '请前往浏览器进行操作';
    } else if (status.status === 'scanned') {
      loginStep.value = 'waiting';
      loginStatus.value = '已扫码，等待确认...';
    } else if (status.status === 'confirmed') {
      loginStep.value = 'success';
      loginStatus.value = '✓ 登录成功！';
      // 登录成功后不自动关闭，等用户点击确认
      return;
    } else if (status.status === 'expired') {
      loginStep.value = 'error';
      loginStatus.value = '二维码已过期，请关闭后重试';
      return;
    } else if (status.status === 'error') {
      loginStep.value = 'error';
      loginStatus.value = '登录出错: ' + (status.error || '未知错误');
      return;
    }
    setTimeout(pollLoginStatus, 2000);
  } catch (e) {
    console.error('轮询失败:', e);
    setTimeout(pollLoginStatus, 3000);
  }
}

// ============ 保存账号 ============

async function saveAccount(userInfo: any, cookieData: any) {
  const platformLabel = currentPlatform.value === 'xiaohongshu' ? '小红书' : '抖音';
  const suggestedName = userInfo?.name || userInfo?.user_id || `${platformLabel}账号`;

  const name = prompt('登录成功！请输入账号名称（如：我的工作号）:', suggestedName);
  if (!name || !name.trim()) return;

  try {
    const account: any = await invoke('save_account', {
      platform: currentPlatform.value,
      name: name.trim(),
      userInfo: userInfo,
      cookieData: cookieData,
    });
    console.log('账号已保存:', account);
    debugMsg.value = `账号已保存: ${name}`;
  } catch (e) {
    console.error('保存账号失败:', e);
    debugMsg.value = '保存失败: ' + e;
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

async function removeAccount(account: any) {
  const key = `${account.platform}:${account.name}`;
  const confirmed = confirm(`确认删除账号「${account.name}」？`);
  if (!confirmed) return;

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

// ============ 确认保存 ============

async function confirmAndSave() {
  try {
    const cookieData: any = await invoke('get_cookies', { sessionId: sessionId.value });
    const userInfo: any = cookieData.user_info || { user_id: '', name: '', avatar: null };
    await saveAccount(userInfo, cookieData);
  } catch (e) {
    console.error('获取 Cookie 失败:', e);
    debugMsg.value = '保存失败: ' + e;
  }
  closeModal();
  await loadAccounts();
}

// ============ 重试 ============

async function retryLogin() {
  closeModal();
  await startLogin(currentPlatform.value);
}

// ============ 生命周期 ============

onMounted(() => {
  loadAccounts();
});

// ============ 辅助 ============

function statusIcon(status: string) {
  if (status === 'valid') return CheckCircle;
  if (status === 'invalid') return XCircle;
  return HelpCircle;
}

function statusColor(status: string) {
  if (status === 'valid') return 'text-green-400';
  if (status === 'invalid') return 'text-red-400';
  return 'text-gray-400';
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
    <!-- 左侧导航 -->
    <aside class="flex flex-col w-[20%] h-full bg-gray-950 border-r border-gray-800">
      <div class="p-6 font-bold tracking-tight">AutoCast AI</div>
      <nav class="flex-1 px-3 space-y-1">
        <a href="#" @click="showAccounts = false" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', !showAccounts ? 'bg-gray-900' : 'text-gray-400']">
          <MessageSquare class="w-5 h-5 text-blue-500" />
          <span>AI 助理对话</span>
        </a>
        <a href="#" @click="showAccounts = true" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', showAccounts ? 'bg-gray-900' : 'text-gray-400']">
          <Users class="w-5 h-5" />
          <span>账号管理</span>
        </a>
      </nav>
    </aside>

    <!-- 主内容 -->
    <main v-if="!showAccounts" class="flex flex-col flex-1 h-full bg-gray-950 p-4">
      <div class="flex-1 flex items-center justify-center text-gray-500">
        <p>AI 助理对话区域（开发中）</p>
      </div>
    </main>

    <!-- 账号管理面板 -->
    <main v-if="showAccounts" class="flex flex-col flex-1 h-full bg-gray-950 p-6 overflow-y-auto">
      <div class="flex justify-between items-center mb-8">
        <h2 class="text-xl font-bold">账号管理</h2>
        <div class="text-xs text-gray-500 bg-gray-900 px-3 py-1 rounded-full border border-gray-800 font-mono">
          {{ debugMsg }}
        </div>
      </div>

      <!-- 账号列表 -->
      <div class="grid grid-cols-2 gap-6">
        <!-- 小红书 -->
        <div class="bg-gray-900 p-5 rounded-xl">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg">📕 小红书</h3>
            <button @click="startLogin('xiaohongshu')" class="text-xs bg-blue-600 px-3 py-1.5 rounded-lg hover:bg-blue-700 transition-colors">+ 新增授权</button>
          </div>
          <div v-if="accounts.filter(a => a.platform === 'xiaohongshu').length === 0" class="text-gray-500 text-sm py-8 text-center border border-dashed border-gray-800 rounded-lg">
            暂无授权账号
          </div>
          <div v-for="acc in accounts.filter(a => a.platform === 'xiaohongshu')" :key="`${acc.platform}:${acc.name}`" class="p-3 bg-gray-950 rounded-lg border border-gray-800 mb-2">
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center gap-3">
                <div v-if="acc.meta?.avatar" class="w-8 h-8 rounded-full bg-gray-800 overflow-hidden flex-shrink-0">
                  <img :src="acc.meta.avatar" class="w-full h-full object-cover" />
                </div>
                <div v-else class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center text-sm flex-shrink-0">📕</div>
                <div>
                  <div class="text-sm font-medium flex items-center gap-2">
                    {{ acc.name }}
                    <!-- 验证状态徽章 -->
                    <span v-if="acc.verify_status" :class="['inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded font-mono', statusBg(acc.verify_status)]">
                      <component :is="statusIcon(acc.verify_status)" class="w-3 h-3" />
                      {{ acc.verify_status }}
                    </span>
                  </div>
                  <div class="text-xs text-gray-500">{{ acc.meta?.nickname || acc.meta?.user_id || '—' }}</div>
                </div>
              </div>
              <div class="flex items-center gap-1">
                <button @click="verifyAccount(acc)" :disabled="isVerifying(acc.platform, acc.name)" class="text-xs bg-gray-800 hover:bg-gray-700 px-2 py-1 rounded flex items-center gap-1 transition-colors disabled:opacity-50">
                  <RefreshCw class="w-3 h-3" :class="isVerifying(acc.platform, acc.name) ? 'animate-spin' : ''" />
                  验证
                </button>
                <button @click="removeAccount(acc)" class="text-xs bg-red-900/30 hover:bg-red-800 text-red-400 px-2 py-1 rounded transition-colors">
                  <Trash2 class="w-3 h-3" />
                </button>
              </div>
            </div>
            <!-- 验证详情 -->
            <div v-if="acc.verify_message" class="text-[10px] text-gray-500 font-mono mt-1 px-2">
              {{ acc.verify_message }}
            </div>
          </div>
        </div>

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
                <button @click="verifyAccount(acc)" :disabled="isVerifying(acc.platform, acc.name)" class="text-xs bg-gray-800 hover:bg-gray-700 px-2 py-1 rounded flex items-center gap-1 transition-colors disabled:opacity-50">
                  <RefreshCw class="w-3 h-3" :class="isVerifying(acc.platform, acc.name) ? 'animate-spin' : ''" />
                  验证
                </button>
                <button @click="removeAccount(acc)" class="text-xs bg-red-900/30 hover:bg-red-800 text-red-400 px-2 py-1 rounded transition-colors">
                  <Trash2 class="w-3 h-3" />
                </button>
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
          <li>• 点击「新增授权」后会显示登录二维码</li>
          <li>• 请使用对应 App 扫码登录</li>
          <li>• 登录成功后请输入账号名称（用于区分多账号）</li>
          <li>• 点击「验证」可检查 Cookie 是否有效（三层检测）</li>
          <li>• 支持同一平台绑定多个账号</li>
        </ul>
      </div>
    </main>

    <!-- 登录弹窗 -->
    <div v-if="isLoginModalOpen" class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center">
      <div class="bg-gray-900 border border-gray-800 rounded-xl p-6 w-[360px] shadow-2xl relative">
        <button @click="closeModal" class="absolute top-4 right-4 text-gray-400 hover:text-white text-xl leading-none">&times;</button>

        <h3 class="text-lg font-bold mb-4 text-center">
          {{ currentPlatform === 'xiaohongshu' ? '📕 小红书' : '♪ 抖音' }} 扫码登录
        </h3>

        <!-- 状态区域 -->
        <div class="bg-gray-800 rounded-xl p-6 flex flex-col items-center justify-center min-h-[180px] mb-4">
          <!-- 等待操作状态 -->
          <div v-if="loginStep === 'init' || loginStep === 'waiting'" class="flex flex-col items-center gap-3">
            <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            <span class="text-gray-400 text-sm">{{ loginStatus }}</span>
          </div>
          <!-- 成功状态 -->
          <div v-else-if="loginStep === 'success'" class="flex flex-col items-center gap-3">
            <CheckCircle class="w-12 h-12 text-green-500" />
            <span class="text-green-400 text-sm font-medium">登录成功</span>
          </div>
          <!-- 错误状态 -->
          <div v-else-if="loginStep === 'error'" class="flex flex-col items-center gap-3">
            <XCircle class="w-12 h-12 text-red-500" />
            <span class="text-red-400 text-sm">{{ loginStatus }}</span>
          </div>
        </div>

        <!-- 底部操作 -->
        <div class="flex flex-col gap-2">
          <!-- 成功：确认保存 -->
          <button v-if="loginStep === 'success'" @click="confirmAndSave" class="w-full bg-green-600 hover:bg-green-700 text-white py-2.5 rounded-lg text-sm font-medium transition-colors">
            确认并保存账号
          </button>
          <!-- 等待/错误：关闭 -->
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
