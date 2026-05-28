<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { 
  ShoppingBag, FileText, Film, Download, Plus, 
  Loader2, CheckCircle2, XCircle, Play, Save, 
  Trash2, ExternalLink, RefreshCw, Music, Settings2
} from 'lucide-vue-next';

// ============ 类型定义 ============

interface VideoProject {
  id: string;
  title: string;
  description?: string;
  config?: any;
  status: string;
  created_at?: string;
  updated_at?: string;
}

interface VideoMaterial {
  id: string;
  project_id: string;
  material_type: string;
  local_path?: string;
  remote_url?: string;
  meta?: any;
}

interface VideoTask {
  id: string;
  project_id?: string;
  task_type: string;
  status: string;
  progress: number;
  result_path?: string;
  error_msg?: string;
}

interface FfmpegProgress {
  task_id: string;
  percentage: number;
  speed: string;
  time: string;
}

// ============ 状态管理 ============

const activeTab = ref<'selection' | 'script' | 'material' | 'export'>('selection');
const projects = ref<VideoProject[]>([]);
const currentProject = ref<VideoProject | null>(null);
const materials = ref<VideoMaterial[]>([]);
const activeTasks = ref<Record<string, VideoTask>>({});
const ffmpegProgress = ref<Record<string, FfmpegProgress>>({});

const isGenerating = ref(false);
const generationPrompt = ref('');
const videoRatio = ref('9:16');
const selectedProvider = ref('fal');

// ============ 渲染配置 ============

const renderConfig = ref({
  width: 1080,
  height: 1920,
  bgm_volume: 0.3,
  transition_type: 'fade',
  ken_burns: true
});

const selectedBgmPath = ref<string | null>(null);

function updateResolution(ratio: string) {
  if (ratio === '9:16') {
    renderConfig.value.width = 1080;
    renderConfig.value.height = 1920;
  } else if (ratio === '16:9') {
    renderConfig.value.width = 1920;
    renderConfig.value.height = 1080;
  } else {
    renderConfig.value.width = 1080;
    renderConfig.value.height = 1080;
  }
}

// ============ 初始化 ============

let unlistenFfmpeg: UnlistenFn | null = null;

onMounted(async () => {
  await loadProjects();
  
  unlistenFfmpeg = await listen<FfmpegProgress>('video-ffmpeg-progress', (event) => {
    ffmpegProgress.value[event.payload.task_id] = event.payload;
  });
});

onUnmounted(() => {
  if (unlistenFfmpeg) unlistenFfmpeg();
});

// ============ 数据加载 ============

async function loadProjects() {
  try {
    projects.value = await invoke('video_list_projects');
    if (projects.value.length > 0 && !currentProject.value) {
      selectProject(projects.value[0]);
    }
  } catch (e) {
    console.error('加载项目失败:', e);
  }
}

async function selectProject(project: VideoProject) {
  currentProject.value = project;
  generationPrompt.value = project.config?.prompt || '';
  await loadMaterials(project.id);
  await loadTasks(project.id);
}

async function loadMaterials(projectId: string) {
  try {
    materials.value = await invoke('video_list_materials', { projectId });
  } catch (e) {
    console.error('加载素材失败:', e);
  }
}

async function loadTasks(projectId: string) {
  try {
    const tasks: VideoTask[] = await invoke('video_list_tasks', { projectId });
    tasks.forEach(t => {
      if (t.status === 'processing') {
        activeTasks.value[t.id] = t;
        startPollingTask(t.id);
      }
    });
  } catch (e) {
    console.error('加载任务失败:', e);
  }
}

// ============ 项目操作 ============

async function createProject() {
  const id = crypto.randomUUID();
  const newProject: VideoProject = {
    id,
    title: `新项目 ${new Date().toLocaleTimeString()}`,
    status: 'draft',
    config: { prompt: '' }
  };
  try {
    await invoke('video_upsert_project', { project: newProject });
    await loadProjects();
    selectProject(newProject);
  } catch (e) {
    alert('创建项目失败: ' + e);
  }
}

async function saveProject() {
  if (!currentProject.value) return;
  currentProject.value.config = { ...currentProject.value.config, prompt: generationPrompt.value };
  try {
    await invoke('video_upsert_project', { project: currentProject.value });
    await loadProjects();
  } catch (e) {
    alert('保存失败: ' + e);
  }
}

// ============ AI 生成逻辑 ============

async function startGeneration() {
  if (!currentProject.value || !generationPrompt.value) return;
  
  isGenerating.value = true;
  try {
    const cfg = await invoke('get_config') as any;
    
    // 从 video 配置中获取对应的 Key
    let apiKey = '';
    if (selectedProvider.value === 'fal') {
      apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
    } else if (selectedProvider.value === 'volcengine') {
      apiKey = cfg?.video?.volc_key;
    } else if (selectedProvider.value === 'mock') {
      apiKey = 'sk-mock-key';
    }

    if (!apiKey && selectedProvider.value !== 'mock') {
      alert(`请先在设置中配置 ${selectedProvider.value} 的 API Key`);
      return;
    }

    const taskId: string = await invoke('video_start_generation', {
      projectId: currentProject.value.id,
      prompt: generationPrompt.value,
      provider: selectedProvider.value,
      apiKey: apiKey,
      mode: 'text',
      ratio: videoRatio.value
    });

    activeTasks.value[taskId] = {
      id: taskId,
      project_id: currentProject.value.id,
      task_type: 'generation',
      status: 'processing',
      progress: 0
    };

    startPollingTask(taskId);
    activeTab.value = 'material';
  } catch (e) {
    alert('发起生成失败: ' + e);
  } finally {
    isGenerating.value = false;
  }
}

function startPollingTask(taskId: string) {
  const timer = setInterval(async () => {
    try {
      const cfg = await invoke('get_config') as any;
      
      let apiKey = '';
      if (selectedProvider.value === 'fal') {
        apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
      } else if (selectedProvider.value === 'volcengine') {
        apiKey = cfg?.video?.volc_key;
      } else if (selectedProvider.value === 'mock') {
        apiKey = 'sk-mock-key';
      }

      const res: any = await invoke('video_poll_task_status', {
        taskId,
        provider: selectedProvider.value,
        apiKey: apiKey
      });

      if (res.status === 'completed') {
        clearInterval(timer);
        delete activeTasks.value[taskId];
        // 触发下载
        await downloadMaterial(taskId, res.video_url);
      } else if (res.status === 'error') {
        clearInterval(timer);
        activeTasks.value[taskId].status = 'error';
        activeTasks.value[taskId].error_msg = res.error;
      }
    } catch (e) {
      console.error('轮询失败:', e);
    }
  }, 5000);
}

async function downloadMaterial(taskId: string, url: string) {
  if (!currentProject.value) return;
  try {
    await invoke('video_download_material', {
      projectId: currentProject.value.id,
      url,
      materialType: 'video'
    });
    await loadMaterials(currentProject.value.id);
  } catch (e) {
    console.error('下载素材失败:', e);
  }
}

// ============ BGM & 导出逻辑 ============

async function pickBgm() {
  const selected = await open({
    multiple: false,
    filters: [{
      name: 'Audio',
      extensions: ['mp3', 'wav', 'm4a', 'aac']
    }]
  });
  if (selected && !Array.isArray(selected)) {
    selectedBgmPath.value = selected;
  }
}

async function startAdvancedRender() {
  if (!currentProject.value || materials.value.length === 0) {
    alert('请先生成或导入至少一段素材');
    return;
  }

  const paths = materials.value
    .filter(m => m.local_path)
    .map(m => m.local_path!);

  try {
    const taskId: string = await invoke('video_render_advanced', {
      projectId: currentProject.value.id,
      videoPaths: paths,
      bgmPath: selectedBgmPath.value,
      config: renderConfig.value
    });

    activeTasks.value[taskId] = {
      id: taskId,
      project_id: currentProject.value.id,
      task_type: 'export',
      status: 'processing',
      progress: 0
    };
    
    activeTab.value = 'material'; // 切换到素材页看进度
  } catch (e) {
    alert('发起渲染失败: ' + e);
  }
}

// ============ 视频拼接逻辑 (废弃，改用高级渲染) ============

async function startConcat() {
  await startAdvancedRender();
}

// ============ 辅助函数 ============

function getTaskProgress(taskId: string) {
  return ffmpegProgress.value[taskId]?.time || '处理中...';
}

</script>

<template>
  <div class="flex h-full bg-gray-950 overflow-hidden">
    <!-- 左侧项目列表 -->
    <aside class="w-64 border-r border-gray-900 flex flex-col bg-gray-950">
      <div class="p-4 border-b border-gray-900 flex justify-between items-center">
        <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest">项目列表</h3>
        <button @click="createProject" class="p-1 hover:bg-gray-800 rounded text-blue-400">
          <Plus class="w-5 h-5" />
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        <div 
          v-for="p in projects" 
          :key="p.id"
          @click="selectProject(p)"
          :class="[
            'p-3 rounded-lg cursor-pointer transition-colors group',
            currentProject?.id === p.id ? 'bg-gray-900 border border-gray-800' : 'hover:bg-gray-900/50'
          ]"
        >
          <div class="text-sm font-medium truncate" :class="currentProject?.id === p.id ? 'text-white' : 'text-gray-400'">
            {{ p.title }}
          </div>
          <div class="text-[10px] text-gray-600 mt-1 flex justify-between items-center">
            <span>{{ p.updated_at ? new Date(p.updated_at).toLocaleDateString() : '刚刚' }}</span>
            <span class="opacity-0 group-hover:opacity-100"><Trash2 class="w-3 h-3 hover:text-red-500" /></span>
          </div>
        </div>
      </div>
    </aside>

    <!-- 主工作区 -->
    <div v-if="currentProject" class="flex-1 flex flex-col min-w-0">
      <!-- Header -->
      <header class="flex items-center justify-between px-6 py-4 border-b border-gray-900 bg-gray-950/50 backdrop-blur-md">
        <div class="flex items-center gap-6">
          <div class="flex bg-gray-900 p-1 rounded-lg border border-gray-800">
            <button
              v-for="tab in [
                { key: 'selection', label: '选品', icon: ShoppingBag },
                { key: 'script', label: '脚本', icon: FileText },
                { key: 'material', label: '素材', icon: Film },
                { key: 'export', label: '导出', icon: Download },
              ]"
              :key="tab.key"
              @click="activeTab = tab.key as any"
              :class="[
                'flex items-center gap-2 px-4 py-1.5 rounded-md text-sm transition-all',
                activeTab === tab.key ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'
              ]"
            >
              <component :is="tab.icon" class="w-4 h-4" />
              {{ tab.label }}
            </button>
          </div>
        </div>
        
        <div class="flex items-center gap-3">
          <button @click="saveProject" class="flex items-center gap-2 text-gray-400 hover:text-white px-3 py-1.5 rounded-lg text-sm transition-colors">
            <Save class="w-4 h-4" />
            保存
          </button>
          <button @click="startConcat" class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors shadow-lg shadow-blue-900/20">
            <Download class="w-4 h-4" />
            合成并导出
          </button>
        </div>
      </header>

      <!-- Content -->
      <div class="flex-1 overflow-hidden relative">
        <!-- 选品库 -->
        <div v-if="activeTab === 'selection'" class="h-full p-8 overflow-y-auto">
          <div class="max-w-4xl mx-auto space-y-8">
            <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-12 flex flex-col items-center justify-center text-center space-y-4">
              <div class="w-16 h-16 bg-gray-800 rounded-2xl flex items-center justify-center">
                <ShoppingBag class="w-8 h-8 text-blue-500" />
              </div>
              <div>
                <h3 class="text-lg font-bold text-white">暂无选品素材</h3>
                <p class="text-sm text-gray-500 mt-1">您可以从采集结果中右键导入商品，或手动添加卖点</p>
              </div>
              <button class="bg-gray-800 hover:bg-gray-700 text-white px-6 py-2 rounded-xl text-sm font-medium transition-all">
                浏览采集库
              </button>
            </div>
          </div>
        </div>

        <!-- 脚本工坊 -->
        <div v-if="activeTab === 'script'" class="h-full p-8 overflow-y-auto">
          <div class="max-w-4xl mx-auto space-y-6">
            <div class="bg-gray-900 rounded-2xl p-6 border border-gray-800 shadow-xl">
              <div class="flex items-center justify-between mb-4">
                <h3 class="text-lg font-bold flex items-center gap-2">
                  <FileText class="w-5 h-5 text-purple-400" />
                  AI 脚本编辑器
                </h3>
                <select v-model="selectedProvider" class="bg-gray-800 border-none text-xs rounded-lg px-2 py-1">
                  <option value="fal">fal.ai (Luma)</option>
                  <option value="volcengine">火山引擎 (ByteDance)</option>
                  <option value="mock">测试模拟 (Mock)</option>
                </select>
              </div>
              <textarea 
                v-model="generationPrompt"
                class="w-full h-48 bg-gray-950 border border-gray-800 rounded-xl p-6 text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all mb-4 leading-relaxed"
                placeholder="在此输入您的创意脚本或视频描述..."
              ></textarea>
              <div class="flex gap-4 items-center">
                <div class="flex-1 flex gap-2">
                  <button 
                    v-for="r in ['9:16', '16:9', '1:1']" 
                    :key="r"
                    @click="videoRatio = r; updateResolution(r)"
                    :class="['px-3 py-1 rounded-lg text-xs font-mono transition-colors', videoRatio === r ? 'bg-purple-600 text-white' : 'bg-gray-800 text-gray-400']"
                  >
                    {{ r }}
                  </button>
                </div>
                <button 
                  @click="startGeneration"
                  :disabled="isGenerating || !generationPrompt"
                  class="flex items-center gap-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-white px-8 py-3 rounded-xl text-sm font-bold transition-all transform active:scale-95 shadow-lg shadow-purple-900/30"
                >
                  <Loader2 v-if="isGenerating" class="w-4 h-4 animate-spin" />
                  <Play v-else class="w-4 h-4" />
                  立即生成视频
                </button>
              </div>
            </div>

            <!-- 提示词建议 -->
            <div class="grid grid-cols-2 gap-4">
              <div v-for="hint in ['赛博朋克风格, 街道霓虹灯', '写实风格, 特写镜头, 展现产品质感']" :key="hint" 
                @click="generationPrompt = hint"
                class="p-4 bg-gray-900/30 border border-gray-800 rounded-xl text-xs text-gray-500 hover:border-purple-500/50 hover:bg-gray-900 cursor-pointer transition-all"
              >
                {{ hint }}
              </div>
            </div>
          </div>
        </div>

        <!-- 素材生成 -->
        <div v-if="activeTab === 'material'" class="h-full p-8 overflow-y-auto">
          <!-- 正在处理的任务 -->
          <div v-if="Object.keys(activeTasks).length > 0" class="mb-8 space-y-3">
            <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">进行中的任务</h4>
            <div v-for="task in activeTasks" :key="task.id" class="bg-blue-900/20 border border-blue-500/30 p-4 rounded-xl flex items-center justify-between">
              <div class="flex items-center gap-4">
                <div class="w-10 h-10 bg-blue-500/20 rounded-lg flex items-center justify-center">
                  <Loader2 class="w-5 h-5 text-blue-400 animate-spin" />
                </div>
                <div>
                  <div class="text-sm font-bold text-blue-100">{{ task.task_type === 'generation' ? 'AI 视频生成' : '视频合并' }}</div>
                  <div class="text-[10px] text-blue-400 font-mono mt-0.5">{{ task.id }}</div>
                </div>
              </div>
              <div class="text-right">
                <div class="text-sm font-mono text-blue-300">{{ getTaskProgress(task.id) }}</div>
                <div class="text-[10px] text-blue-500 uppercase mt-0.5 tracking-tighter">{{ task.status }}</div>
              </div>
            </div>
          </div>

          <!-- 已完成素材 -->
          <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">素材库</h4>
          <div v-if="materials.length === 0" class="flex flex-col items-center justify-center py-20 text-gray-600">
            <Film class="w-12 h-12 mb-2 opacity-10" />
            <p class="text-sm">暂无生成记录</p>
          </div>
          <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4">
            <div v-for="m in materials" :key="m.id" class="group relative aspect-[9/16] bg-gray-900 rounded-xl overflow-hidden border border-gray-800 hover:border-blue-500/50 transition-all shadow-xl">
              <video v-if="m.local_path" :src="`https://asset.localhost/${m.local_path}`" class="w-full h-full object-cover" muted loop onmouseover="this.play()" onmouseout="this.pause()"></video>
              <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity p-3 flex flex-col justify-end">
                <div class="flex justify-between items-center">
                  <span class="text-[10px] font-mono text-gray-400">{{ new Date(m.created_at || '').toLocaleTimeString() }}</span>
                  <div class="flex gap-2">
                    <button class="p-1.5 bg-gray-800 rounded-lg hover:bg-blue-600 text-white transition-colors">
                      <ExternalLink class="w-3.5 h-3.5" />
                    </button>
                    <button class="p-1.5 bg-gray-800 rounded-lg hover:bg-red-600 text-white transition-colors">
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
              <div v-if="!m.local_path" class="absolute inset-0 flex items-center justify-center bg-gray-900/80">
                <Loader2 class="w-6 h-6 text-blue-500 animate-spin" />
              </div>
            </div>
          </div>
        </div>

        <!-- 合成导出 -->
        <div v-if="activeTab === 'export'" class="h-full p-8 overflow-y-auto bg-gray-950/20">
           <div class="max-w-4xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-8">
             <!-- 左侧设置 -->
             <div class="lg:col-span-2 space-y-6">
               <div class="bg-gray-900 rounded-2xl border border-gray-800 p-6 shadow-xl">
                 <h4 class="text-sm font-bold text-white flex items-center gap-2 mb-6">
                   <Music class="w-4 h-4 text-pink-500" />
                   背景音乐 (BGM)
                 </h4>
                 
                 <div class="space-y-6">
                   <div class="flex items-center gap-4">
                     <button @click="pickBgm" class="flex-1 bg-gray-800 hover:bg-gray-700 border border-gray-700 text-gray-300 py-3 rounded-xl text-sm transition-all flex items-center justify-center gap-2">
                       <Music class="w-4 h-4" />
                       {{ selectedBgmPath ? selectedBgmPath.split('/').pop() : '选择本地音乐文件...' }}
                     </button>
                     <button v-if="selectedBgmPath" @click="selectedBgmPath = null" class="p-3 text-red-500 hover:bg-red-500/10 rounded-xl transition-colors">
                       <Trash2 class="w-5 h-5" />
                     </button>
                   </div>

                   <div class="space-y-3">
                     <div class="flex justify-between text-xs text-gray-500">
                       <span>BGM 初始音量</span>
                       <span class="font-mono">{{ Math.round(renderConfig.bgm_volume * 100) }}%</span>
                     </div>
                     <input type="range" min="0" max="1" step="0.05" v-model.number="renderConfig.bgm_volume" class="w-full h-1.5 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-pink-500" />
                     <p class="text-[10px] text-gray-600">提示：开启 Audio Ducking 后，旁白出现时 BGM 会自动降音。</p>
                   </div>
                 </div>
               </div>

               <div class="bg-gray-900 rounded-2xl border border-gray-800 p-6 shadow-xl">
                 <h4 class="text-sm font-bold text-white flex items-center gap-2 mb-6">
                   <Settings2 class="w-4 h-4 text-blue-500" />
                   后期效果
                 </h4>
                 
                 <div class="grid grid-cols-2 gap-6">
                   <div class="space-y-4">
                     <label class="text-xs text-gray-500 block">转场效果</label>
                     <div class="grid grid-cols-2 gap-2">
                       <button 
                         v-for="t in [{id:'none', n:'无'}, {id:'fade', n:'淡入淡出'}]" 
                         :key="t.id"
                         @click="renderConfig.transition_type = t.id"
                         :class="['py-2 rounded-lg text-xs transition-all border', renderConfig.transition_type === t.id ? 'bg-blue-600/10 border-blue-500 text-blue-400' : 'bg-gray-800 border-gray-700 text-gray-400']"
                       >
                         {{ t.n }}
                       </button>
                     </div>
                   </div>

                   <div class="space-y-4">
                     <label class="text-xs text-gray-500 block">动态运镜</label>
                     <button 
                       @click="renderConfig.ken_burns = !renderConfig.ken_burns"
                       :class="['w-full py-2 rounded-lg text-xs transition-all border flex items-center justify-center gap-2', renderConfig.ken_burns ? 'bg-orange-600/10 border-orange-500 text-orange-400' : 'bg-gray-800 border-gray-700 text-gray-400']"
                     >
                       <Zap class="w-3 h-3" />
                       Ken Burns (平滑缩放)
                     </button>
                   </div>
                 </div>
               </div>
             </div>

             <!-- 右侧预览与确认 -->
             <div class="space-y-6">
               <div class="bg-gray-900 rounded-2xl border border-gray-800 p-6 shadow-xl">
                 <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">导出详情</h4>
                 <div class="space-y-3">
                   <div class="flex justify-between py-2 border-b border-gray-800/50">
                     <span class="text-xs text-gray-500">分辨率</span>
                     <span class="text-xs text-white font-mono">{{ renderConfig.width }}x{{ renderConfig.height }}</span>
                   </div>
                   <div class="flex justify-between py-2 border-b border-gray-800/50">
                     <span class="text-xs text-gray-500">片段总数</span>
                     <span class="text-xs text-white">{{ materials.length }} 段</span>
                   </div>
                   <div class="flex justify-between py-2 border-b border-gray-800/50">
                     <span class="text-xs text-gray-500">音频模式</span>
                     <span class="text-xs text-white">{{ selectedBgmPath ? '双轨混音' : '原始音频' }}</span>
                   </div>
                 </div>

                 <button @click="startAdvancedRender" :disabled="materials.length === 0" class="w-full mt-8 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 disabled:opacity-50 text-white py-4 rounded-xl text-sm font-bold transition-all shadow-lg shadow-blue-900/20 flex items-center justify-center gap-2">
                   <Download class="w-5 h-5" />
                   开始合成导出
                 </button>
               </div>

               <div class="p-4 bg-blue-950/20 border border-blue-900/30 rounded-xl">
                 <p class="text-[10px] text-blue-400 leading-relaxed">
                   <strong>💡 提示：</strong> 渲染过程将在后台执行。完成后，最终视频将出现在“素材库”中并标记为“成片”。
                 </p>
               </div>
             </div>
           </div>
        </div>
      </div>
    </div>

    <!-- 无项目展示 -->
    <div v-else class="flex-1 flex flex-col items-center justify-center bg-gray-950">
      <div class="p-8 rounded-full bg-gray-900/50 mb-6">
        <Film class="w-16 h-16 text-gray-700" />
      </div>
      <h3 class="text-xl font-bold text-gray-300">选择或创建一个项目开始</h3>
      <p class="text-gray-500 mt-2">在这里，您可以一键生成 AI 视频并进行专业剪辑</p>
      <button @click="createProject" class="mt-8 bg-blue-600 hover:bg-blue-700 text-white px-8 py-3 rounded-xl font-bold transition-all shadow-xl shadow-blue-900/30">
        创建第一个项目
      </button>
    </div>
  </div>
</template>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
</style>
