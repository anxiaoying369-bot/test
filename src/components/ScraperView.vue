<script setup lang="ts">
import {
  Play, Square, RefreshCw,
  CheckCircle, XCircle, Loader2, AlertTriangle, ChevronDown
} from 'lucide-vue-next';
import { useScraper } from '../composables/useScraper';

const {
  selectedAccount, secUid, scrapeType, limit, skipExisting, incremental,
  isRunning, progress, error,
  douyinAccounts, statusLabel, statusColor, progressPercent, elapsedStr,
  startScrape, cancelScrape, resetForm,
} = useScraper();

const typeOptions = [
  { value: 'all', label: '全量（作品+评论+回复）' },
  { value: 'video', label: '仅作品' },
  { value: 'comment', label: '仅评论' },
  { value: 'reply', label: '仅回复' },
  { value: 'follower', label: '仅粉丝 (erma0 集成)' },
  { value: 'like', label: '仅喜欢 (erma0 集成)' },
];
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
    <div class="bg-gray-900 p-5 rounded-xl mb-6">
      <div class="grid grid-cols-2 gap-4">
        <!-- 选择账号 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">使用账号（Cookie 来源）</label>
          <div class="relative">
            <select v-model="selectedAccount" :disabled="isRunning"
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
          <label class="text-xs text-gray-400 block mb-1.5">目标用户 ID 或主页</label>
          <input v-model="secUid" type="text" :disabled="isRunning"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500 disabled:opacity-50"
            placeholder="sec_uid 或主页链接..." />
          <p class="text-[10px] text-gray-500 mt-1 px-0.5">
            支持 sec_uid (MS4wLj...) 或主页链接 (douyin.com/user/...)
          </p>
        </div>


        <!-- 采集类型 -->
        <div>
          <label class="text-xs text-gray-400 block mb-1.5">采集类型</label>
          <div class="relative">
            <select v-model="scrapeType" :disabled="isRunning"
              class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white appearance-none focus:outline-none focus:border-blue-500 pr-8 disabled:opacity-50">
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
          <input v-model.number="limit" type="number" min="0" :disabled="isRunning"
            class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500 disabled:opacity-50" />
        </div>
      </div>

      <!-- 选项行 -->
      <div class="flex items-center gap-6 mt-4">
        <label class="flex items-center gap-2 text-xs text-gray-400 cursor-pointer">
          <input type="checkbox" v-model="skipExisting" :disabled="isRunning" class="rounded bg-gray-950 border-gray-700 disabled:opacity-50" />
          跳过已存在评论
        </label>
        <label class="flex items-center gap-2 text-xs text-gray-400 cursor-pointer">
          <input type="checkbox" v-model="incremental" :disabled="isRunning" class="rounded bg-gray-950 border-gray-700 disabled:opacity-50" />
          增量模式 (遇旧作品停止)
        </label>
        <div class="flex-1"></div>
        <button v-if="!isRunning" @click="startScrape"
          class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-5 py-2 rounded-lg text-sm font-medium transition-colors shadow-lg shadow-blue-900/20">
          <Play class="w-4 h-4" />
          开始采集
        </button>
        <button v-else @click="cancelScrape"
          class="flex items-center gap-2 bg-red-600 hover:bg-red-700 text-white px-5 py-2 rounded-lg text-sm font-medium transition-colors shadow-lg shadow-red-900/20">
          <Square class="w-4 h-4" />
          停止采集
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

      <!-- 错误详情 -->
      <div v-if="progress?.status === 'error' && progress?.stats?.error"
           class="mb-4 bg-red-900/20 border border-red-500/30 rounded-lg p-3">
        <div class="text-xs font-bold text-red-300 mb-1">错误原因</div>
        <div class="text-[11px] text-red-200/90 font-mono break-all whitespace-pre-wrap">{{ progress.stats.error }}</div>
        <div class="text-[10px] text-red-400/60 mt-2">
          完整日志见: ~/Library/Application Support/AutoCastAI/logs/scrape_*.log
        </div>
      </div>

      <!-- 统计数据 -->
      <div v-if="progress?.stats && progress?.status !== 'error'" class="grid grid-cols-3 gap-3 mb-4">
        <div v-for="(stats, type) in progress.stats" :key="type"
          class="bg-gray-950 p-3 rounded-lg border border-gray-800">
          <div class="text-xs text-gray-500 mb-1">
            {{ type === 'video' ? '作品' : type === 'comment' ? '评论' : type === 'reply' ? '回复' : type === 'follower' ? '粉丝' : '喜欢' }}
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
