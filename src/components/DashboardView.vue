<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { 
  Activity, Cpu, HardDrive, Clock, StopCircle, 
  Search, Radio, Video, Layers, CheckCircle, AlertCircle
} from 'lucide-vue-next';

interface Task {
  id: string;
  name: string;
  task_type: string;
  status: string;
  pid?: number;
  cpu: number;
  memory: number; // Bytes
  created_at: number;
  updated_at: number;
}

const tasks = ref<Task[]>([]);
const isRefreshing = ref(false);
let timer: number | null = null;

const loadTasks = async () => {
  isRefreshing.value = true;
  try {
    tasks.value = await invoke('list_active_tasks');
  } catch (e) {
    console.error('加载任务失败:', e);
  } finally {
    isRefreshing.value = false;
  }
};

const stopTask = async (id: string) => {
  if (!confirm('确定要强行终止该任务吗？')) return;
  try {
    await invoke('kill_task', { taskId: id });
    await loadTasks();
  } catch (e) {
    alert('终止失败: ' + e);
  }
};

const formatMemory = (bytes: number) => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

const formatTime = (ts: number) => {
  return new Date(ts * 1000).toLocaleTimeString();
};

const getTaskIcon = (type: string) => {
  switch (type) {
    case 'scraper': return Search;
    case 'live_monitor': return Radio;
    case 'video_studio': return Video;
    default: return Layers;
  }
};

onMounted(() => {
  loadTasks();
  timer = window.setInterval(loadTasks, 3000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100 overflow-hidden">
    <!-- 顶部状态栏 -->
    <header class="px-8 pt-8 pb-6 border-b border-gray-900 bg-gray-950/50 backdrop-blur-xl">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-2xl font-bold bg-gradient-to-r from-white to-gray-400 bg-clip-text text-transparent flex items-center gap-3">
            <Activity class="w-7 h-7 text-blue-500" />
            任务调度中心
          </h2>
          <p class="text-gray-500 text-sm mt-1">实时监控系统后台 Python 脚本与渲染进程的运行状态</p>
        </div>
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-6 px-6 py-2.5 bg-gray-900/50 border border-gray-800 rounded-2xl shadow-inner">
            <div class="flex flex-col">
              <span class="text-[10px] text-gray-500 uppercase font-bold tracking-wider">运行中</span>
              <span class="text-xl font-mono font-bold text-blue-400">
                {{ tasks.filter(t => t.status === 'running').length }}
              </span>
            </div>
            <div class="w-px h-8 bg-gray-800"></div>
            <div class="flex flex-col">
              <span class="text-[10px] text-gray-500 uppercase font-bold tracking-wider">今日累计</span>
              <span class="text-xl font-mono font-bold text-gray-300">{{ tasks.length }}</span>
            </div>
          </div>
        </div>
      </div>
    </header>

    <!-- 任务列表区域 -->
    <main class="flex-1 overflow-y-auto p-8 custom-scrollbar">
      <div class="max-w-5xl mx-auto space-y-6">
        <div v-if="tasks.length === 0" class="flex flex-col items-center justify-center py-20 text-gray-600 border-2 border-dashed border-gray-900 rounded-3xl">
          <Layers class="w-16 h-16 opacity-10 mb-4" />
          <p class="text-sm">暂无活跃任务，启动采集或监控后此处将显示状态</p>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div 
            v-for="task in tasks" 
            :key="task.id"
            :class="[
              'group p-5 rounded-2xl border transition-all duration-300 relative overflow-hidden',
              task.status === 'running' 
                ? 'bg-gray-900/40 border-gray-800 hover:border-blue-500/50 hover:bg-gray-900/60 shadow-lg' 
                : 'bg-gray-950 border-gray-900 opacity-60 grayscale'
            ]"
          >
            <!-- 运行背景动效 -->
            <div v-if="task.status === 'running'" class="absolute top-0 right-0 p-1">
              <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-pulse shadow-[0_0_8px_rgba(59,130,246,0.8)]"></div>
            </div>

            <div class="flex items-start gap-4">
              <!-- 图标 -->
              <div :class="[
                'w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 transition-transform group-hover:scale-105',
                task.status === 'running' ? 'bg-blue-600/10 text-blue-400' : 'bg-gray-800 text-gray-500'
              ]">
                <component :is="getTaskIcon(task.task_type)" class="w-6 h-6" />
              </div>

              <!-- 信息 -->
              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between mb-1">
                  <h3 class="font-bold text-sm truncate pr-2">{{ task.name }}</h3>
                  <span :class="[
                    'text-[9px] px-2 py-0.5 rounded-full font-bold uppercase tracking-tighter',
                    task.status === 'running' ? 'bg-green-500/10 text-green-500' : 'bg-gray-800 text-gray-400'
                  ]">
                    {{ task.status }}
                  </span>
                </div>
                
                <div class="flex items-center gap-4 mt-3">
                  <!-- CPU -->
                  <div class="flex items-center gap-1.5">
                    <Cpu class="w-3.5 h-3.5 text-gray-500" />
                    <span class="text-xs font-mono text-gray-300">{{ task.cpu.toFixed(1) }}%</span>
                  </div>
                  <!-- Memory -->
                  <div class="flex items-center gap-1.5">
                    <HardDrive class="w-3.5 h-3.5 text-gray-500" />
                    <span class="text-xs font-mono text-gray-300">{{ formatMemory(task.memory) }}</span>
                  </div>
                  <!-- Time -->
                  <div class="flex items-center gap-1.5">
                    <Clock class="w-3.5 h-3.5 text-gray-500" />
                    <span class="text-xs font-mono text-gray-300">{{ formatTime(task.created_at) }}</span>
                  </div>
                </div>

                <!-- 进度条（如果是运行中） -->
                <div v-if="task.status === 'running'" class="mt-4 h-1 w-full bg-gray-800 rounded-full overflow-hidden">
                  <div class="h-full bg-blue-500 animate-progress w-1/3"></div>
                </div>
              </div>
            </div>

            <!-- 操作按钮 -->
            <button 
              v-if="task.status === 'running'"
              @click="stopTask(task.id)"
              class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 p-2 hover:bg-red-500/10 text-red-500 rounded-lg transition-all"
              title="终止任务"
            >
              <StopCircle class="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
@keyframes progress {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(300%); }
}
.animate-progress {
  animation: progress 2s infinite linear;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1e293b;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #334155;
}
</style>
