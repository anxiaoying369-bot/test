<script setup lang="ts">
import { Search, Loader2, RefreshCw, Copy, ChevronDown, UserX } from 'lucide-vue-next';
import { useUserInfo } from '../composables/useUserInfo';

const {
  selectedAccount, input, loading, error, user,
  douyinAccounts, fetchUserInfo, reset,
} = useUserInfo();

function formatCount(n?: number): string {
  const v = n ?? 0;
  if (v >= 100000000) return (v / 100000000).toFixed(1) + ' 亿';
  if (v >= 10000) return (v / 10000).toFixed(1) + ' 万';
  return String(v);
}

async function copyText(text?: string) {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // ignore
  }
}
</script>

<template>
  <div class="flex flex-col h-full overflow-y-auto p-6">
    <!-- 标题 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-bold">用户信息查询</h2>
      <span class="text-xs text-gray-500 bg-gray-900 px-3 py-1 rounded-full border border-gray-800 font-mono">
        {{ douyinAccounts.length }} 个抖音账号
      </span>
    </div>

    <!-- 查询区域 -->
    <div class="bg-gray-900 p-5 rounded-xl mb-6">
      <div class="grid grid-cols-2 gap-4">
        <!-- 选择账号 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">使用账号（Cookie 来源）</label>
          <div class="relative">
            <select v-model="selectedAccount" :disabled="loading"
              class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white appearance-none focus:outline-none focus:border-blue-500 pr-8 disabled:opacity-50">
              <option value="" disabled>选择已授权的抖音账号</option>
              <option v-for="acc in douyinAccounts" :key="acc.name" :value="acc.name">
                {{ acc.name }}{{ acc.meta?.nickname ? ` (${acc.meta.nickname})` : '' }}
              </option>
            </select>
            <ChevronDown class="w-4 h-4 text-gray-500 absolute right-2.5 top-2.5 pointer-events-none" />
          </div>
        </div>

        <!-- 目标用户 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">用户 ID / 抖音号 / 链接</label>
          <input v-model="input" type="text" :disabled="loading" @keyup.enter="fetchUserInfo"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500 disabled:opacity-50"
            placeholder="sec_uid / 抖音号 / 分享短链 / 主页链接..." />
          <p class="text-[10px] text-gray-500 mt-1 px-0.5">
            支持 sec_uid、uid、抖音号、分享短链 (v.douyin.com) 或主页链接
          </p>
        </div>
      </div>

      <!-- 操作行 -->
      <div class="flex items-center gap-3 mt-4">
        <div class="flex-1"></div>
        <button v-if="user || error" @click="reset"
          class="flex items-center gap-2 bg-gray-800 hover:bg-gray-700 text-gray-300 px-4 py-2 rounded-lg text-sm transition-colors">
          <RefreshCw class="w-4 h-4" />
          重置
        </button>
        <button @click="fetchUserInfo" :disabled="loading"
          class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-medium transition-colors shadow-lg shadow-blue-900/20">
          <Loader2 v-if="loading" class="w-4 h-4 animate-spin" />
          <Search v-else class="w-4 h-4" />
          {{ loading ? '查询中...' : '查询' }}
        </button>
      </div>

      <!-- 错误提示 -->
      <div v-if="error" class="mt-3 text-sm text-red-400 bg-red-900/20 px-3 py-2 rounded-lg flex items-center gap-2">
        <UserX class="w-4 h-4 flex-shrink-0" />
        <span>{{ error }}</span>
      </div>
    </div>

    <!-- 结果卡片 -->
    <div v-if="user" class="bg-gray-900 rounded-xl mb-6 overflow-hidden">
      <!-- 头部：头像 + 昵称 -->
      <div class="flex items-center gap-4 p-5 border-b border-gray-800">
        <img v-if="user.avatar_url" :src="user.avatar_url" alt="avatar"
          class="w-20 h-20 rounded-full object-cover border-2 border-gray-700 flex-shrink-0" />
        <div v-else class="w-20 h-20 rounded-full bg-gray-800 flex items-center justify-center flex-shrink-0">
          <UserX class="w-8 h-8 text-gray-600" />
        </div>
        <div class="min-w-0">
          <div class="text-lg font-bold truncate">{{ user.nickname || '（无昵称）' }}</div>
          <div v-if="user.unique_id" class="text-sm text-gray-400 mt-0.5">抖音号：{{ user.unique_id }}</div>
          <div class="flex items-center gap-3 mt-1">
            <span v-if="user.location" class="text-xs text-gray-500">IP 属地：{{ user.location }}</span>
            <span v-if="user.custom_verify || user.enterprise_verify_reason" class="text-xs text-amber-400/90 truncate">
              ✓ {{ user.custom_verify || user.enterprise_verify_reason }}
            </span>
          </div>
          <p v-if="user.signature" class="text-xs text-gray-500 mt-1.5 line-clamp-2 whitespace-pre-wrap">{{ user.signature }}</p>
        </div>
      </div>

      <!-- 数据统计 -->
      <div class="grid grid-cols-4 gap-3 p-5">
        <div class="bg-gray-950 p-3 rounded-lg border border-gray-800 text-center">
          <div class="text-lg font-bold">{{ formatCount(user.follower_count) }}</div>
          <div class="text-[11px] text-gray-500 mt-0.5">粉丝</div>
        </div>
        <div class="bg-gray-950 p-3 rounded-lg border border-gray-800 text-center">
          <div class="text-lg font-bold">{{ formatCount(user.following_count) }}</div>
          <div class="text-[11px] text-gray-500 mt-0.5">关注</div>
        </div>
        <div class="bg-gray-950 p-3 rounded-lg border border-gray-800 text-center">
          <div class="text-lg font-bold">{{ formatCount(user.total_favorited) }}</div>
          <div class="text-[11px] text-gray-500 mt-0.5">获赞</div>
        </div>
        <div class="bg-gray-950 p-3 rounded-lg border border-gray-800 text-center">
          <div class="text-lg font-bold">{{ formatCount(user.aweme_count) }}</div>
          <div class="text-[11px] text-gray-500 mt-0.5">作品</div>
        </div>
      </div>

      <!-- 标识符（可复制） -->
      <div class="px-5 pb-5 space-y-2">
        <div v-if="user.sec_uid" class="flex items-center gap-2 bg-gray-950 rounded-lg border border-gray-800 px-3 py-2">
          <span class="text-xs text-gray-500 w-16 flex-shrink-0">sec_uid</span>
          <span class="text-xs text-gray-300 font-mono truncate flex-1">{{ user.sec_uid }}</span>
          <button @click="copyText(user.sec_uid)" class="text-gray-500 hover:text-blue-400 flex-shrink-0" title="复制">
            <Copy class="w-3.5 h-3.5" />
          </button>
        </div>
        <div v-if="user.uid" class="flex items-center gap-2 bg-gray-950 rounded-lg border border-gray-800 px-3 py-2">
          <span class="text-xs text-gray-500 w-16 flex-shrink-0">UID</span>
          <span class="text-xs text-gray-300 font-mono truncate flex-1">{{ user.uid }}</span>
          <button @click="copyText(user.uid)" class="text-gray-500 hover:text-blue-400 flex-shrink-0" title="复制">
            <Copy class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>

    <!-- 使用说明 -->
    <div v-if="!user && !loading" class="p-4 bg-gray-900/50 rounded-xl border border-gray-800">
      <h4 class="text-sm font-medium mb-2 text-gray-300">使用说明</h4>
      <ul class="text-xs text-gray-500 space-y-1">
        <li>1. 选择一个已授权的抖音账号（用于提供 Cookie）</li>
        <li>2. 输入目标用户的 sec_uid / uid / 抖音号 / 分享短链 / 主页链接</li>
        <li>3. 点击「查询」，即可获取头像、昵称、抖音号、sec_uid 等公开信息</li>
        <li class="text-gray-600">注：查询会在后台打开浏览器抓取页面，首次可能稍慢，请确保本机已安装 Chrome。</li>
      </ul>
    </div>
  </div>
</template>
