<script setup lang="ts">
import { inject, type Ref } from 'vue';
import { Search, Loader2, RefreshCw, Copy, ChevronDown, UserX, Trash2, Users, MessageSquare } from 'lucide-vue-next';
import { useUserInfo, type UserCard } from '../composables/useUserInfo';

const {
  selectedAccount, input, loading, error, cards, refreshingSecUid,
  douyinAccounts, queryUser, refreshCard, deleteCard,
} = useUserInfo();

// 点用户卡片 → 跳转「评论采集」并预填该用户 sec_uid
const navigateTo = inject<(page: string) => void>('navigateTo');
const scraperPrefill = inject<Ref<{ secUid: string; account?: string } | null>>('scraperPrefill');
function openScraper(card: UserCard) {
  if (scraperPrefill) {
    scraperPrefill.value = { secUid: card.sec_uid, account: selectedAccount.value || undefined };
  }
  navigateTo?.('scraper');
}

function formatCount(n?: number): string {
  const v = n ?? 0;
  if (v >= 100000000) return (v / 100000000).toFixed(1) + ' 亿';
  if (v >= 10000) return (v / 10000).toFixed(1) + ' 万';
  return String(v);
}

async function copyText(text?: string) {
  if (!text) return;
  try { await navigator.clipboard.writeText(text); } catch { /* ignore */ }
}
</script>

<template>
  <div class="flex flex-col h-full overflow-y-auto p-6">
    <!-- 标题 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-bold">用户信息查询</h2>
      <span class="text-xs text-gray-500 bg-gray-900 px-3 py-1 rounded-full border border-gray-800 font-mono">
        已收录 {{ cards.length }} 个用户
      </span>
    </div>

    <!-- 查询栏 -->
    <div class="bg-gray-900 p-5 rounded-xl mb-6">
      <div class="flex flex-wrap items-end gap-3">
        <div class="w-56">
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
        <div class="flex-1 min-w-[240px]">
          <label class="text-xs text-gray-400 block mb-1.5">sec_uid 或主页链接</label>
          <input v-model="input" type="text" :disabled="loading" @keyup.enter="queryUser"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500 disabled:opacity-50"
            placeholder="MS4wLj... 或 douyin.com/user/..." />
        </div>
        <button @click="queryUser" :disabled="loading"
          class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-medium transition-colors shadow-lg shadow-blue-900/20">
          <Loader2 v-if="loading" class="w-4 h-4 animate-spin" />
          <Search v-else class="w-4 h-4" />
          {{ loading ? '查询中...' : '查询并收录' }}
        </button>
      </div>
      <p class="text-[10px] text-gray-500 mt-2 px-0.5">
        支持 sec_uid、主页链接、分享短链。纯数字 uid 暂不支持（抖音 web 无公开接口）——直播监控里的用户可一键收录。
      </p>
      <div v-if="error" class="mt-3 text-sm text-red-400 bg-red-900/20 px-3 py-2 rounded-lg flex items-center gap-2">
        <UserX class="w-4 h-4 flex-shrink-0" />
        <span>{{ error }}</span>
      </div>
    </div>

    <!-- 卡片网格 -->
    <div v-if="cards.length" class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      <div v-for="card in cards" :key="card.sec_uid" @click="openScraper(card)"
        class="group bg-gray-900 rounded-xl border border-gray-800 hover:border-blue-600/60 hover:bg-gray-900/80 overflow-hidden flex flex-col cursor-pointer transition-colors"
        title="点击采集 TA 的评论">
        <!-- 头部 -->
        <div class="flex items-start gap-3 p-4">
          <img v-if="card.avatar_url" :src="card.avatar_url" alt="avatar"
            class="w-14 h-14 rounded-full object-cover border border-gray-700 flex-shrink-0" />
          <div v-else class="w-14 h-14 rounded-full bg-gray-800 flex items-center justify-center flex-shrink-0">
            <UserX class="w-6 h-6 text-gray-600" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="font-bold truncate">{{ card.nickname || '（无昵称）' }}</div>
            <div v-if="card.unique_id" class="text-xs text-gray-400 mt-0.5">抖音号：{{ card.unique_id }}</div>
            <div v-if="card.ip_location" class="text-[11px] text-gray-500 mt-0.5">{{ card.ip_location }}</div>
          </div>
          <div class="flex flex-col gap-1.5 flex-shrink-0">
            <button @click.stop="refreshCard(card.sec_uid)" :disabled="refreshingSecUid === card.sec_uid"
              class="text-gray-500 hover:text-blue-400 disabled:opacity-50" title="刷新">
              <Loader2 v-if="refreshingSecUid === card.sec_uid" class="w-4 h-4 animate-spin" />
              <RefreshCw v-else class="w-4 h-4" />
            </button>
            <button @click.stop="deleteCard(card.sec_uid)" class="text-gray-500 hover:text-red-400" title="移除">
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- 四数据格 -->
        <div class="grid grid-cols-4 gap-2 px-4">
          <div class="text-center">
            <div class="text-sm font-bold">{{ formatCount(card.follower_count) }}</div>
            <div class="text-[10px] text-gray-500">粉丝</div>
          </div>
          <div class="text-center">
            <div class="text-sm font-bold">{{ formatCount(card.following_count) }}</div>
            <div class="text-[10px] text-gray-500">关注</div>
          </div>
          <div class="text-center">
            <div class="text-sm font-bold">{{ formatCount(card.total_favorited) }}</div>
            <div class="text-[10px] text-gray-500">获赞</div>
          </div>
          <div class="text-center">
            <div class="text-sm font-bold">{{ formatCount(card.aweme_count) }}</div>
            <div class="text-[10px] text-gray-500">作品</div>
          </div>
        </div>

        <!-- sec_uid -->
        <div class="mt-3 px-4 pb-4">
          <div class="flex items-center gap-2 bg-gray-950 rounded-lg border border-gray-800 px-2.5 py-1.5">
            <span class="text-[10px] text-gray-500 flex-shrink-0">sec_uid</span>
            <span class="text-[11px] text-gray-300 font-mono truncate flex-1">{{ card.sec_uid }}</span>
            <button @click.stop="copyText(card.sec_uid)" class="text-gray-500 hover:text-blue-400 flex-shrink-0" title="复制">
              <Copy class="w-3 h-3" />
            </button>
          </div>
        </div>

        <!-- 点击提示 -->
        <div class="mt-auto px-4 py-2 border-t border-gray-800/80 flex items-center gap-1.5 text-[11px] text-gray-500 group-hover:text-blue-400 transition-colors">
          <MessageSquare class="w-3 h-3" />
          点击采集 TA 的评论 →
        </div>
      </div>
    </div>

    <!-- 空态 -->
    <div v-else class="flex flex-col items-center justify-center text-center py-16 text-gray-500">
      <Users class="w-10 h-10 mb-3 text-gray-700" />
      <p class="text-sm">还没有收录任何用户</p>
      <p class="text-xs mt-1 text-gray-600">输入 sec_uid / 主页链接查询，或在「直播监控」里把观众一键收录</p>
    </div>
  </div>
</template>
