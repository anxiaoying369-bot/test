<script setup lang="ts">
import {
  Radio, StopCircle, Play, Users, MessageSquare,
  Heart, Gift, Trash2, AlertCircle, Monitor, Hash, ExternalLink
} from 'lucide-vue-next';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { LiveRoom } from '../types/live-monitor';
import { useLiveMonitor } from '../composables/useLiveMonitor';
import MessageItem from './live-monitor/MessageItem.vue';

const props = defineProps<{
  globalRooms: Record<string, LiveRoom>
}>();

const {
  selectedRoomId, newRoomId, selectedAccount, messageContainer, copiedId, aiReplies, filters,
  douyinAccounts, activeRoomsCount, selectedRoom, filteredMessages,
  generateAiReply, copyToClipboard, addRoom, stopMonitor, removeRoom,
} = useLiveMonitor(props.globalRooms);

// 把弹幕里带 sec_uid 的观众一键收录到「用户信息查询」用户库
const libraryToast = ref('');
let toastTimer: ReturnType<typeof setTimeout> | null = null;
function flashToast(text: string) {
  libraryToast.value = text;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { libraryToast.value = ''; }, 2500);
}
async function addUserToLibrary(secUid: string) {
  if (!selectedAccount.value) {
    flashToast('请先在左上角选择一个抖音账号');
    return;
  }
  try {
    const card: any = await invoke('query_and_save_user', {
      accountName: selectedAccount.value,
      userId: secUid,
    });
    flashToast(`已收录：${card?.nickname || secUid.slice(0, 12)}`);
  } catch (e: any) {
    flashToast(`收录失败：${String(e)}`);
  }
}
</script>

<template>
  <div class="flex h-full bg-gray-950 text-gray-50 overflow-hidden">
    <!-- 收录用户 toast -->
    <div v-if="libraryToast"
      class="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 bg-gray-800 border border-gray-700 text-sm text-gray-100 px-4 py-2 rounded-lg shadow-xl animate-in fade-in slide-in-from-bottom-2">
      {{ libraryToast }}
    </div>

    <!-- 左侧边栏：房间列表 -->
    <aside class="w-64 border-r border-gray-800 flex flex-col flex-shrink-0 bg-gray-900/50">
      <div class="p-4 border-b border-gray-800">
        <h2 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-4">
          <Monitor class="w-4 h-4" /> 直播监控 ({{ activeRoomsCount }}/10)
        </h2>

        <div class="space-y-3">
          <!-- 账号选择 -->
          <div>
            <label class="text-[10px] text-gray-500 block mb-1">COOKIE 来源</label>
            <select v-model="selectedAccount" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-2 py-1.5 text-xs focus:outline-none focus:border-blue-500">
              <option v-for="acc in douyinAccounts" :key="acc.name" :value="acc.name">
                {{ acc.name }}
              </option>
            </select>
          </div>

          <!-- 添加房间 -->
          <div class="space-y-1.5">
            <div class="flex gap-2">
              <input v-model="newRoomId" @keyup.enter="addRoom" type="text" placeholder="输入 ID 或链接..."
                class="flex-1 bg-gray-950 border border-gray-700 rounded-lg px-2 py-1.5 text-xs focus:outline-none focus:border-blue-500" />
              <button @click="addRoom" class="bg-blue-600 hover:bg-blue-700 p-1.5 rounded-lg transition-colors">
                <Play class="w-4 h-4" />
              </button>
            </div>
            <div class="px-1">
              <p class="text-[10px] text-gray-500 leading-relaxed">
                支持纯数字 ID (如 371206...) 或标准直播链接 (live.douyin.com/...)
              </p>
            </div>
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        <div v-if="activeRoomsCount === 0" class="py-10 text-center text-gray-600">
          <Hash class="w-8 h-8 mx-auto mb-2 opacity-20" />
          <p class="text-xs">暂无监控房间</p>
        </div>

        <div v-for="(room, rid) in globalRooms" :key="rid"
          @click="selectedRoomId = rid"
          :class="['group flex items-center justify-between p-2.5 rounded-xl cursor-pointer transition-all',
                    selectedRoomId === rid ? 'bg-blue-600/10 border border-blue-500/50' : 'hover:bg-gray-800 border border-transparent']">
          <div class="flex items-center gap-3 min-w-0">
            <div class="relative">
              <div :class="['w-2 h-2 rounded-full',
                            room.status === 'running' ? 'bg-green-500 animate-pulse' :
                            room.status === 'connecting' ? 'bg-blue-500 animate-pulse' :
                            room.status === 'error' ? 'bg-red-500' : 'bg-gray-500']"></div>
            </div>
            <div class="min-w-0">
              <div class="text-sm font-medium truncate">{{ room.anchor_name || rid }}</div>
              <div class="text-[10px] text-gray-500 uppercase">{{ room.status }}</div>
            </div>
          </div>
          <button @click.stop="removeRoom(rid)" class="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 transition-all">
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </aside>

    <!-- 右侧：监控面板 -->
    <main class="flex-1 flex flex-col min-w-0 bg-gray-950">
      <template v-if="selectedRoom">
        <header class="p-4 border-b border-gray-800 flex flex-col gap-4 bg-gray-900/20">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-xl bg-gray-800 flex items-center justify-center text-blue-400">
                <Radio class="w-6 h-6" />
              </div>
              <div>
                <h3 class="font-bold flex items-center gap-2">
                  直播间: {{ selectedRoom.anchor_name || selectedRoomId }}
                  <a :href="'https://live.douyin.com/' + selectedRoomId" target="_blank" class="text-gray-500 hover:text-blue-400">
                    <ExternalLink class="w-3.5 h-3.5" />
                  </a>
                </h3>
                <div class="text-xs text-gray-500 flex items-center gap-2">
                  <span :class="{
                    'text-green-500': selectedRoom.status === 'running',
                    'text-yellow-400': selectedRoom.status === 'connecting',
                    'text-red-400': selectedRoom.status === 'error',
                    'text-gray-500': selectedRoom.status === 'stopped',
                  }">
                    ●
                    <template v-if="selectedRoom.status === 'running'">正在监控</template>
                    <template v-else-if="selectedRoom.status === 'connecting'">正在连接...</template>
                    <template v-else-if="selectedRoom.status === 'error'">连接失败</template>
                    <template v-else>已停止</template>
                  </span>
                  <span>• {{ selectedRoom.messages.length }} 条实时数据</span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button v-if="selectedRoom.status === 'running'" @click="stopMonitor(selectedRoomId!)"
                class="flex items-center gap-1.5 px-3 py-1.5 bg-red-900/20 hover:bg-red-900/40 text-red-400 rounded-lg text-xs font-medium transition-all border border-red-800/30">
                <StopCircle class="w-3.5 h-3.5" /> 停止监控
              </button>
              <button v-else-if="selectedRoom.status !== 'connecting'" @click="newRoomId = String(selectedRoomId); addRoom()"
                class="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-medium transition-all">
                <Play class="w-3.5 h-3.5" /> 重新连接
              </button>
            </div>
          </div>

          <!-- 错误详情 -->
          <div v-if="selectedRoom.status === 'error'"
               class="mb-3 bg-red-900/20 border border-red-500/30 rounded-lg p-3">
            <div class="text-xs font-bold text-red-300 mb-1">连接失败</div>
            <div class="text-[11px] text-red-200/90 font-mono break-all whitespace-pre-wrap">{{ selectedRoom.error || '未知原因（Python 子进程退出但未报告错误）' }}</div>
            <div class="text-[10px] text-red-400/60 mt-2 leading-relaxed">
              常见原因：① 未安装 Node.js (直播监控必需，下载 https://nodejs.org) ② Cookie 已过期，请重新登录抖音账号 ③ 直播间已关闭
            </div>
          </div>

          <!-- 过滤器 -->
          <div class="flex items-center gap-2 bg-gray-950/50 p-1 rounded-xl border border-gray-800 w-fit">
            <button
              @click="filters.chat = !filters.chat"
              :class="['flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-bold uppercase tracking-wider transition-all',
                        filters.chat ? 'bg-blue-600 text-white shadow-lg shadow-blue-900/20' : 'text-gray-500 hover:text-gray-300']"
            >
              <MessageSquare class="w-3.5 h-3.5" /> 弹幕
            </button>
            <button
              @click="filters.like = !filters.like"
              :class="['flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-bold uppercase tracking-wider transition-all',
                        filters.like ? 'bg-red-600 text-white shadow-lg shadow-red-900/20' : 'text-gray-500 hover:text-gray-300']"
            >
              <Heart class="w-3.5 h-3.5" /> 点赞
            </button>
            <button
              @click="filters.gift = !filters.gift"
              :class="['flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-bold uppercase tracking-wider transition-all',
                        filters.gift ? 'bg-purple-600 text-white shadow-lg shadow-purple-900/20' : 'text-gray-500 hover:text-gray-300']"
            >
              <Gift class="w-3.5 h-3.5" /> 送礼
            </button>
            <button
              @click="filters.member = !filters.member"
              :class="['flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-bold uppercase tracking-wider transition-all',
                        filters.member ? 'bg-gray-700 text-white shadow-lg shadow-gray-900/20' : 'text-gray-500 hover:text-gray-300']"
            >
              <Users class="w-3.5 h-3.5" /> 进场
            </button>
          </div>
        </header>

        <!-- 异常提示 -->
        <div v-if="selectedRoom.error" class="m-4 p-3 bg-red-900/20 border border-red-800 rounded-lg flex items-center gap-3 text-red-400 text-sm animate-in fade-in slide-in-from-top-2">
          <AlertCircle class="w-5 h-5 flex-shrink-0" />
          <div class="flex-1">
            <div class="font-bold">连接异常</div>
            <div class="opacity-80 font-mono text-xs">{{ selectedRoom.error }}</div>
          </div>
          <button @click="selectedRoom!.error = undefined" class="hover:text-white">&times;</button>
        </div>

        <!-- 消息列表 -->
        <div ref="messageContainer" class="flex-1 overflow-y-auto p-4 space-y-3 custom-scrollbar">
          <div v-if="filteredMessages.length === 0" class="h-full flex flex-col items-center justify-center text-gray-600">
            <div class="w-16 h-16 rounded-full bg-gray-900 flex items-center justify-center mb-4">
              <MessageSquare class="w-8 h-8 opacity-20" />
            </div>
            <p class="text-sm">没有符合当前过滤条件的消息...</p>
            <p class="text-[10px] mt-2 opacity-50 uppercase tracking-widest">Adjust filters to see more data</p>
          </div>

          <MessageItem
            v-for="(msg, i) in filteredMessages" :key="i"
            :msg="msg"
            :copiedId="copiedId"
            :aiReply="aiReplies[`${selectedRoomId}_${i}`]"
            @copy="copyToClipboard"
            @generateReply="generateAiReply(i, msg.payload.user_name, msg.payload.content || '')"
            @addToLibrary="addUserToLibrary"
          />
        </div>
      </template>

      <!-- 未选中房间 -->
      <div v-else class="h-full flex flex-col items-center justify-center text-gray-600 bg-gray-950">
        <div class="w-20 h-20 rounded-2xl bg-gray-900 flex items-center justify-center mb-6 shadow-xl">
          <Monitor class="w-10 h-10 opacity-20" />
        </div>
        <h3 class="text-lg font-bold text-gray-400 mb-2">选择或添加一个直播间</h3>
        <p class="text-sm max-w-xs text-center leading-relaxed">
          在左侧输入直播间的 ID (Web RID) 并选择对应的账号进行实时监控。最多支持 10 路并发。
        </p>
      </div>
    </main>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #374151;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes slide-in-bottom { from { transform: translateY(10px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
@keyframes slide-in-top { from { transform: translateY(-10px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }

.animate-in { animation-duration: 0.2s; animation-fill-mode: forwards; }
.fade-in { animation-name: fade-in; }
.slide-in-from-bottom-1 { animation-name: slide-in-bottom; }
.slide-in-from-top-2 { animation-name: slide-in-top; }
</style>
