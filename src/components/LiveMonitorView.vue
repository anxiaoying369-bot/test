<script setup lang="ts">
import { ref, onMounted, computed, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { 
  Radio, StopCircle, Play, Users, MessageSquare, 
  Heart, Gift, Trash2,
  AlertCircle, Monitor, Hash, ExternalLink
} from 'lucide-vue-next';

interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

interface LiveMessage {
  time: string;
  user_name: string;
  user_id: string;
  content?: string;
  gift_name?: string;
  gift_count?: number;
  count?: number; // for likes
  gender?: string; // for member join
}

interface LiveRoom {
  id: string;
  anchor_name: string;
  status: 'connecting' | 'running' | 'stopped' | 'error';
  messages: { type: string; payload: LiveMessage }[];
  error?: string;
}

const props = defineProps<{
  globalRooms: Record<string, LiveRoom>
}>();

const selectedRoomId = ref<string | null>(null);
const newRoomId = ref('');
const accounts = ref<Account[]>([]);
const selectedAccount = ref('');
const messageContainer = ref<HTMLElement | null>(null);

const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));
const activeRoomsCount = computed(() => Object.keys(props.globalRooms).length);
const selectedRoom = computed(() => selectedRoomId.value ? props.globalRooms[selectedRoomId.value] : null);

// 监听房间列表变化，如果有新房间加入且当前没选中，或者只有一个房间，自动选中
watch(() => Object.keys(props.globalRooms).length, (newCount, oldCount) => {
  if (newCount > (oldCount || 0)) {
    const keys = Object.keys(props.globalRooms);
    // 寻找最新添加的 ID (这里简单取最后一个)
    const latestId = keys[keys.length - 1];
    if (latestId && !selectedRoomId.value) {
      selectedRoomId.value = latestId;
    }
  }
}, { immediate: true });

async function loadAccounts() {
  try {
    const res = await invoke('list_accounts', { platform: null }) as Account[];
    accounts.value = res;
    if (res.length > 0) {
      selectedAccount.value = res[0].name;
    }
  } catch (e) {
    console.error('加载账号失败:', e);
  }
}

async function loadHistory(rid: string) {
  try {
    const history = await invoke('get_live_history', { roomId: rid }) as any[];
    if (history.length > 0 && props.globalRooms[rid]) {
      // 避免重复加载
      if (props.globalRooms[rid].messages.length === 0) {
        props.globalRooms[rid].messages = history.map(h => ({
          type: h.data_type,
          payload: h.payload
        }));
        // 如果历史记录里有主播名，也同步一下
        const lastWithAnchor = [...history].reverse().find(h => h.anchor_name);
        if (lastWithAnchor) {
          props.globalRooms[rid].anchor_name = lastWithAnchor.anchor_name;
        }
      }
    }
  } catch (e) {
    console.error('加载历史失败:', e);
  }
}

async function addRoom() {
  const input = newRoomId.value.trim();
  if (!input) return;

  let rid = input;
  
  // 如果不是纯数字，则尝试从 URL 解析
  if (!/^\d+$/.test(input)) {
    try {
      rid = await invoke('resolve_live_url', { url: input });
    } catch (e: any) {
      alert(e);
      return;
    }
  }
  
  if (!/^\d+$/.test(rid)) {
    alert('无法获取有效的直播间 ID');
    return;
  }
  
  if (props.globalRooms[rid] && props.globalRooms[rid].status === 'running') {
    selectedRoomId.value = rid;
    newRoomId.value = '';
    return;
  }
  
  if (activeRoomsCount.value >= 10 && !props.globalRooms[rid]) {
    alert('最多只能同时监控 10 个直播间');
    return;
  }

  // 初始化或重置房间状态
  if (!props.globalRooms[rid]) {
    props.globalRooms[rid] = {
      id: rid,
      anchor_name: '',
      status: 'connecting',
      messages: []
    };
  } else {
    props.globalRooms[rid].status = 'connecting';
    props.globalRooms[rid].error = undefined;
  }
  
  selectedRoomId.value = rid;
  newRoomId.value = '';

  // 先加载历史记录
  await loadHistory(rid);

  try {
    await invoke('start_live_monitor', { 
      roomId: rid, 
      accountName: selectedAccount.value 
    });
  } catch (e: any) {
    if (props.globalRooms[rid]) {
      props.globalRooms[rid].status = 'error';
      props.globalRooms[rid].error = String(e);
    }
  }
}

async function stopMonitor(rid: string) {
  try {
    await invoke('stop_live_monitor', { roomId: rid });
    if (props.globalRooms[rid]) {
      props.globalRooms[rid].status = 'stopped';
    }
  } catch (e) {
    console.error('停止监控失败:', e);
  }
}

function removeRoom(rid: string) {
  stopMonitor(rid);
  delete props.globalRooms[rid];
  if (selectedRoomId.value === rid) {
    selectedRoomId.value = Object.keys(props.globalRooms)[0] || null;
  }
}

function scrollToBottom() {
  if (messageContainer.value) {
    messageContainer.value.scrollTop = messageContainer.value.scrollHeight;
  }
}

onMounted(async () => {
  await loadAccounts();
  
  // 恢复已有的监控
  try {
    const activeIds = await invoke('get_active_monitors') as string[];
    for (const id of activeIds) {
      if (!props.globalRooms[id]) {
        props.globalRooms[id] = {
          id,
          anchor_name: '',
          status: 'running',
          messages: []
        };
      }
      await loadHistory(id);
    }
    if (activeIds.length > 0 && !selectedRoomId.value) {
      selectedRoomId.value = activeIds[0];
    }
  } catch (e) {
    console.error('恢复活跃监控失败:', e);
  }
  
  setTimeout(scrollToBottom, 100);
});

watch(selectedRoomId, () => {
  setTimeout(scrollToBottom, 50);
}, { immediate: true });

// 监听消息长度变化，自动滚动
watch(() => selectedRoom.value?.messages.length, () => {
  if (selectedRoomId.value) {
    nextTick(scrollToBottom);
  }
});
</script>

<template>
  <div class="flex h-full bg-gray-950 text-gray-50 overflow-hidden">
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
          <div class="flex gap-2">
            <input v-model="newRoomId" @keyup.enter="addRoom" type="text" placeholder="直播间 ID 或链接"
              class="flex-1 bg-gray-950 border border-gray-700 rounded-lg px-2 py-1.5 text-xs focus:outline-none focus:border-blue-500" />
            <button @click="addRoom" class="bg-blue-600 hover:bg-blue-700 p-1.5 rounded-lg transition-colors">
              <Play class="w-4 h-4" />
            </button>
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
        <header class="p-4 border-b border-gray-800 flex items-center justify-between bg-gray-900/20">
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
                <span :class="selectedRoom.status === 'running' ? 'text-green-500' : 'text-gray-500'">
                  ● {{ selectedRoom.status === 'running' ? '正在监控' : '连接中断' }}
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
          <div v-if="selectedRoom.messages.length === 0" class="h-full flex flex-col items-center justify-center text-gray-600">
            <div class="w-16 h-16 rounded-full bg-gray-900 flex items-center justify-center mb-4">
              <MessageSquare class="w-8 h-8 opacity-20" />
            </div>
            <p class="text-sm">等待直播间消息...</p>
            <p class="text-[10px] mt-2 opacity-50 uppercase tracking-widest">Listening for events...</p>
          </div>

          <div v-for="(msg, i) in selectedRoom.messages" :key="i" 
            class="animate-in fade-in slide-in-from-bottom-1 duration-300">
            <!-- 聊天消息 -->
            <div v-if="msg.type === 'chat' || msg.type === 'emoji'" class="flex gap-3">
              <div class="flex-shrink-0 mt-1">
                <div class="w-2 h-2 rounded-full bg-blue-500 mt-1.5"></div>
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-baseline gap-2 mb-0.5">
                  <span class="text-xs font-bold text-blue-400">{{ msg.payload.user_name }}</span>
                  <span class="text-[10px] text-gray-600">{{ msg.payload.time }}</span>
                </div>
                <div class="text-sm text-gray-300 leading-relaxed bg-gray-900/50 p-2 rounded-lg border border-gray-800 inline-block">
                  {{ msg.payload.content }}
                </div>
              </div>
            </div>

            <!-- 礼物消息 -->
            <div v-else-if="msg.type === 'gift'" class="flex gap-3">
              <div class="flex-shrink-0 mt-1">
                <div class="w-2 h-2 rounded-full bg-purple-500 mt-1.5"></div>
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-baseline gap-2 mb-0.5">
                  <span class="text-xs font-bold text-purple-400">{{ msg.payload.user_name }}</span>
                  <span class="text-[10px] text-gray-600">{{ msg.payload.time }}</span>
                </div>
                <div class="text-sm bg-purple-500/10 border border-purple-500/30 text-purple-200 px-3 py-1.5 rounded-lg flex items-center gap-2">
                  <Gift class="w-4 h-4 text-purple-400" />
                  送出 <span class="font-bold">{{ msg.payload.gift_name }}</span> x{{ msg.payload.gift_count }}
                </div>
              </div>
            </div>

            <!-- 点赞消息 -->
            <div v-else-if="msg.type === 'like'" class="flex gap-3">
              <div class="flex-shrink-0 mt-1 text-red-500">
                <Heart class="w-4 h-4" />
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-xs">
                  <span class="font-bold text-red-400">{{ msg.payload.user_name }}</span>
                  <span class="text-gray-500"> 连点 {{ msg.payload.count }} 个赞</span>
                  <span class="text-[10px] text-gray-600 ml-2">{{ msg.payload.time }}</span>
                </div>
              </div>
            </div>

            <!-- 进场消息 -->
            <div v-else-if="msg.type === 'member'" class="flex gap-3">
              <div class="flex-shrink-0 mt-1 text-gray-600">
                <Users class="w-4 h-4" />
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-xs italic text-gray-500">
                  {{ msg.payload.user_name }} 来了
                  <span class="text-[10px] text-gray-700 ml-2">{{ msg.payload.time }}</span>
                </div>
              </div>
            </div>
          </div>
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
