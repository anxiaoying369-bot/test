<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { 
  ShoppingBag, FileText, Film, Download, Plus, 
  Loader2, CheckCircle2, XCircle, Play, Save, 
  Trash2, ExternalLink, RefreshCw, Music, Settings2, Zap
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
  
  try {
    const cfg = await invoke('get_config') as any;
    if (cfg?.video?.default_provider) {
      selectedProvider.value = cfg.video.default_provider;
    }
  } catch (e) {
    console.error('加载视频配置失败:', e);
  }

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
  } catch (e) {
    console.error('Failed to load projects:', e);
  }
}

async function loadMaterials(projectId: string) {
  try {
    materials.value = await invoke('video_list_materials', { projectId });
  } catch (e) {
    console.error('Failed to load materials:', e);
  }
}

async function createProject() {
  const title = `新视频项目 - ${new Date().toLocaleTimeString()}`;

  const newProject: VideoProject = {
    id: crypto.randomUUID(),
    title,
    status: 'draft'
  };

  try {
    await invoke('video_upsert_project', { project: newProject });
    await loadProjects();
    selectProject(newProject);
  } catch (e) {
    alert('创建失败: ' + e);
  }
}

async function deleteProject(projectId: string, event: Event) {
  event.stopPropagation(); // 阻止冒泡到 selectProject
  
  if (!confirm('确定要删除这个项目吗？所有关联素材和记录将被永久清除。')) return;

  try {
    await invoke('video_delete_project', { id: projectId });
    await loadProjects();
    if (currentProject.value?.id === projectId) {
      currentProject.value = null;
      activeTab.value = 'selection';
    }
  } catch (e) {
    alert('删除失败: ' + e);
  }
}

function selectProject(project: VideoProject) {
  currentProject.value = project;
  activeTab.value = 'script';
  loadMaterials(project.id);
}

// ============ AI 生成逻辑 ============

async function startGeneration() {
  if (!currentProject.value || !generationPrompt.value) return;
  
  isGenerating.value = true;
  try {
    const cfg = await invoke('get_config') as any;
    
    // 从 video 配置中获取对应的 Key
    let apiKey = '';
    let baseUrl = '';
    let model = '';

    if (selectedProvider.value === 'fal') {
      apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
    } else if (selectedProvider.value === 'volcengine') {
      apiKey = cfg?.video?.volc_key;
    } else if (selectedProvider.value === 'openai') {
      apiKey = cfg?.video?.openai_api_key;
      baseUrl = cfg?.video?.openai_base_url;
      model = cfg?.video?.openai_model;
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
      ratio: videoRatio.value,
      baseUrl: baseUrl,
      model: model
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
      let baseUrl = '';
      let model = '';

      if (selectedProvider.value === 'fal') {
        apiKey = cfg?.video?.fal_key || cfg?.llm?.api_key;
      } else if (selectedProvider.value === 'volcengine') {
        apiKey = cfg?.video?.volc_key;
      } else if (selectedProvider.value === 'openai') {
        apiKey = cfg?.video?.openai_api_key;
        baseUrl = cfg?.video?.openai_base_url;
        model = cfg?.video?.openai_model;
      } else if (selectedProvider.value === 'mock') {
        apiKey = 'sk-mock-key';
      }

      const res: any = await invoke('video_poll_task_status', {
        taskId,
        provider: selectedProvider.value,
        apiKey: apiKey,
        baseUrl: baseUrl,
        model: model
      });

      if (res.status === 'completed') {
        clearInterval(timer);
        delete activeTasks.value[taskId];
        
        // 如果是生成任务，自动触发下载
        if (res.video_url && currentProject.value) {
          await invoke('video_download_material', {
            projectId: currentProject.value.id,
            url: res.video_url,
            materialType: 'video'
          });
          loadMaterials(currentProject.value.id);
        }
      } else if (res.status === 'error') {
        clearInterval(timer);
        alert('任务失败: ' + res.error);
        delete activeTasks.value[taskId];
      }
    } catch (e) {
      console.error('Polling failed:', e);
      clearInterval(timer);
    }
  }, 3000);
}

// ============ 素材与剪辑 ============

async function selectBgm() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'm4a'] }]
  });
  if (selected) {
    selectedBgmPath.value = selected as string;
  }
}

async function startAdvancedRender() {
  if (!currentProject.value || materials.value.length === 0) return;

  try {
    const videoPaths = materials.value
      .filter(m => m.material_type === 'video' && m.local_path)
      .map(m => m.local_path as string);
    
    if (videoPaths.length === 0) {
      alert('没有可用的视频素材');
      return;
    }

    const taskId: string = await invoke('video_render_advanced', {
      projectId: currentProject.value.id,
      videoPaths,
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
  } catch (e) {
    alert('发起合成失败: ' + e);
  }
}
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100 overflow-hidden">
    <!-- 侧边栏：项目列表 -->
    <div class="flex h-full">
      <div class="w-72 bg-gray-900 border-r border-gray-800 flex flex-col">
        <div class="p-6">
          <button @click="createProject" class="w-full bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 rounded-xl flex items-center justify-center gap-2 transition-all shadow-lg shadow-blue-900/20">
            <Plus class="w-5 h-5" />
            新建创作项目
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar px-4 pb-6 space-y-2">
          <div 
            v-for="p in projects" 
            :key="p.id"
            @click="selectProject(p)"
            :class="['p-4 rounded-xl cursor-pointer transition-all border group relative', currentProject?.id === p.id ? 'bg-blue-600/10 border-blue-500/50 shadow-inner' : 'hover:bg-gray-800 border-transparent text-gray-400']"
          >
            <div class="flex items-center gap-3">
              <Film :class="['w-4 h-4', currentProject?.id === p.id ? 'text-blue-400' : 'text-gray-600']" />
              <div class="flex-1 truncate text-sm font-medium">{{ p.title }}</div>
              <button 
                @click="deleteProject(p.id, $event)"
                class="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/20 hover:text-red-500 rounded-lg transition-all"
                title="删除项目"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 主内容区 -->
      <div v-if="currentProject" class="flex-1 flex flex-col relative">
        <!-- 头部 -->
        <div class="h-20 px-8 flex items-center justify-between border-b border-gray-800 bg-gray-950/50 backdrop-blur-md">
          <div class="flex items-center gap-4">
            <h2 class="text-xl font-bold text-white">{{ currentProject.title }}</h2>
            <span class="text-[10px] px-2 py-0.5 rounded-full bg-gray-800 text-gray-500 font-mono uppercase tracking-tighter">{{ currentProject.id.slice(0, 8) }}</span>
          </div>

          <div class="flex gap-2">
            <button
              v-for="tab in [{id:'script', n:'脚本/生成', i:FileText}, {id:'material', n:'素材库', i:ShoppingBag}, {id:'export', n:'后期/导出', i:Settings2}]"
              :key="tab.id"
              @click="activeTab = tab.id as any"
              :class="['px-5 py-2 rounded-xl text-sm font-medium transition-all flex items-center gap-2 border', activeTab === tab.id ? 'bg-gray-800 border-gray-700 text-white' : 'text-gray-500 hover:text-gray-300 border-transparent']"
            >
              <component :is="tab.i" class="w-4 h-4" />
              {{ tab.n }}
            </button>
          </div>
        </div>

        <!-- 各 Tab 内容 -->
        <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
          
          <!-- Tab 1: 脚本与生成 -->
          <div v-if="activeTab === 'script'" class="max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-2">
            <div class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-2xl">
              <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex justify-between items-center">
                <h3 class="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                  <FileText class="w-4 h-4 text-blue-500" />
                  AI 脚本编辑器
                </h3>
                <select v-model="selectedProvider" class="bg-gray-800 border-none text-xs rounded-lg px-2 py-1">
                  <option value="fal">fal.ai (Luma)</option>
                  <option value="volcengine">火山引擎 (ByteDance)</option>
                  <option value="openai">OpenAI 兼容协议</option>
                  <option value="mock">测试模拟 (Mock)</option>
                </select>
              </div>
              <textarea 
                v-model="generationPrompt"
                placeholder="在此输入您的视频创意脚本，例如：'一个赛博朋克风格的未来城市，霓虹灯闪烁，飞行汽车穿梭其中'..."
                class="w-full h-64 p-8 bg-transparent text-lg text-gray-200 leading-relaxed placeholder-gray-700 resize-none focus:outline-none"
              ></textarea>
              
              <div class="p-6 bg-gray-950/50 border-t border-gray-800 flex items-center justify-between">
                <div class="flex gap-4">
                  <div class="flex flex-col gap-1">
                    <span class="text-[10px] text-gray-600 font-bold uppercase">视频比例</span>
                    <div class="flex bg-gray-900 p-1 rounded-lg">
                      <button 
                        v-for="r in ['9:16', '16:9', '1:1']" 
                        :key="r"
                        @click="videoRatio = r; updateResolution(r)"
                        :class="['px-3 py-1 rounded-md text-xs font-medium transition-all', videoRatio === r ? 'bg-blue-600 text-white' : 'text-gray-500 hover:text-gray-300']"
                      >
                        {{ r }}
                      </button>
                    </div>
                  </div>
                </div>

                <button 
                  @click="startGeneration" 
                  :disabled="isGenerating || !generationPrompt"
                  class="bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white px-10 py-3 rounded-xl font-bold transition-all shadow-lg shadow-blue-900/30 flex items-center gap-3"
                >
                  <Loader2 v-if="isGenerating" class="w-5 h-5 animate-spin" />
                  <Play v-else class="w-5 h-5" />
                  {{ isGenerating ? 'AI 正在思考中...' : '开始生成视频' }}
                </button>
              </div>
            </div>

            <div class="p-6 bg-blue-950/10 border border-blue-900/20 rounded-2xl flex gap-4">
               <div class="p-3 bg-blue-600/20 rounded-xl h-fit">
                 <Zap class="w-6 h-6 text-blue-400" />
               </div>
               <div>
                 <h4 class="font-bold text-blue-200">GEO 驱动的内容生成</h4>
                 <p class="text-sm text-blue-400/80 mt-1 leading-relaxed">
                   建议在脚本中包含品牌词和核心卖点。AI 将自动针对生成式引擎进行优化，确视频视觉冲击力的同时，符合搜索引擎的可视化抓取逻辑。
                 </p>
               </div>
            </div>
          </div>

          <!-- Tab 2: 素材库 -->
          <div v-if="activeTab === 'material'" class="space-y-8 animate-in fade-in slide-in-from-bottom-2">
            <!-- 正在进行的任务 -->
            <div v-if="Object.keys(activeTasks).length > 0" class="grid grid-cols-2 gap-4">
              <div v-for="t in activeTasks" :key="t.id" class="bg-gray-900 border border-blue-900/30 rounded-2xl p-6 flex items-center gap-6 shadow-xl">
                <div class="relative">
                  <div class="w-12 h-12 rounded-xl bg-blue-600/20 flex items-center justify-center">
                    <Loader2 class="w-6 h-6 text-blue-500 animate-spin" />
                  </div>
                </div>
                <div class="flex-1">
                  <div class="flex justify-between items-center mb-2">
                    <span class="text-xs font-bold text-gray-400 uppercase tracking-widest">{{ t.task_type === 'generation' ? 'AI 视频生成' : '导出合成' }}</span>
                    <span class="text-[10px] text-blue-500 font-mono">{{ t.id.slice(0, 8) }}</span>
                  </div>
                  <div class="h-1.5 bg-gray-800 rounded-full overflow-hidden">
                    <div class="h-full bg-blue-500 animate-pulse transition-all duration-500" style="width: 100%"></div>
                  </div>
                  <p class="text-[11px] text-gray-500 mt-2">云端任务处理中，请稍候...</p>
                </div>
              </div>
            </div>

            <!-- 素材列表 -->
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
              <div v-for="m in materials" :key="m.id" class="group bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden hover:border-gray-600 transition-all shadow-xl">
                <div class="aspect-[9/16] bg-black relative">
                  <video 
                    v-if="m.local_path" 
                    :src="`https://asset.localhost/${m.local_path}`" 
                    class="w-full h-full object-cover"
                    muted
                    loop
                    onmouseover="this.play()"
                    onmouseout="this.pause()"
                  ></video>
                  <div v-else class="w-full h-full flex flex-col items-center justify-center gap-3">
                    <Loader2 class="w-8 h-8 text-gray-800 animate-spin" />
                    <span class="text-[10px] text-gray-700 uppercase">等待本地化</span>
                  </div>

                  <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-all p-4 flex flex-col justify-end">
                    <div class="flex gap-2">
                      <button @click="m.local_path && window.open(`https://asset.localhost/${m.local_path}`)" class="flex-1 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-2">
                        <ExternalLink class="w-3.5 h-3.5" />
                        全屏
                      </button>
                    </div>
                  </div>
                </div>
                <div class="p-4 border-t border-gray-800/50">
                   <div class="flex justify-between items-center">
                     <span class="text-[10px] font-bold text-gray-500 uppercase font-mono">{{ m.id.slice(0, 8) }}</span>
                     <CheckCircle2 class="w-4 h-4 text-green-500" />
                   </div>
                </div>
              </div>

              <!-- 无素材提示 -->
              <div v-if="materials.length === 0 && Object.keys(activeTasks).length === 0" class="col-span-full py-32 flex flex-col items-center justify-center border-2 border-dashed border-gray-900 rounded-3xl">
                <ShoppingBag class="w-12 h-12 text-gray-800 mb-4" />
                <p class="text-sm text-gray-600">该项目暂无素材，去“脚本”中生成一段吧</p>
              </div>
            </div>
          </div>

          <!-- Tab 3: 后期导出 -->
          <div v-if="activeTab === 'export'" class="max-w-6xl mx-auto animate-in fade-in slide-in-from-bottom-2">
           <div class="grid grid-cols-3 gap-10">
             <!-- 左侧：渲染参数 -->
             <div class="col-span-2 space-y-8">
               <div class="bg-gray-900 rounded-3xl p-8 border border-gray-800 shadow-2xl">
                 <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-8">
                   <Settings2 class="w-5 h-5 text-indigo-500" />
                   高级渲染引擎配置
                 </h3>

                 <div class="space-y-8">
                   <!-- 背景音乐 -->
                   <div class="space-y-4">
                     <label class="block text-xs text-gray-500 uppercase font-bold tracking-tighter">背景音乐 (BGM)</label>
                     <div class="flex gap-4">
                       <button @click="selectBgm" class="flex-1 bg-gray-950 border border-gray-800 rounded-2xl p-4 flex items-center gap-4 hover:border-blue-500/50 transition-all text-left">
                         <div class="p-3 bg-blue-600/10 rounded-xl">
                           <Music class="w-5 h-5 text-blue-500" />
                         </div>
                         <div class="flex-1 truncate">
                           <div class="text-sm font-bold text-gray-200">{{ selectedBgmPath ? selectedBgmPath.split('/').pop() : '选择本地音频' }}</div>
                           <div class="text-[10px] text-gray-600 mt-0.5">{{ selectedBgmPath ? '已加载本地音轨' : '支持 mp3, wav, m4a' }}</div>
                         </div>
                       </button>
                       <button v-if="selectedBgmPath" @click="selectedBgmPath = null" class="p-4 border border-gray-800 rounded-2xl hover:bg-red-500/10 hover:border-red-500/30 transition-all">
                         <Trash2 class="w-5 h-5 text-gray-600" />
                       </button>
                     </div>
                   </div>

                   <!-- 音量平衡 -->
                   <div class="space-y-4">
                     <div class="flex justify-between items-center">
                       <label class="text-xs text-gray-500 uppercase font-bold">BGM 混音比例</label>
                       <span class="text-xs font-mono text-blue-500">{{ Math.round(renderConfig.bgm_volume * 100) }}%</span>
                     </div>
                     <input 
                       type="range" v-model.number="renderConfig.bgm_volume" 
                       min="0" max="1" step="0.05"
                       class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-blue-600"
                     />
                   </div>
                 </div>
               </div>

               <div class="bg-gray-900 rounded-3xl p-8 border border-gray-800 shadow-2xl">
                 <h4 class="text-xs font-bold text-gray-400 uppercase tracking-widest mb-6 flex items-center gap-2">
                   <Zap class="w-4 h-4 text-orange-500" />
                   转场与视觉增强
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
</div>
</template>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
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
