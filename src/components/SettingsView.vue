<script setup lang="ts">
import { ref, onMounted, inject, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Save, RefreshCw, CheckCircle, XCircle, ShieldCheck, Globe, Cpu, Wand2, Video, MessageSquare, RotateCcw, FileText } from 'lucide-vue-next';

interface LLMConfig {
  api_key: string;
  base_url: string;
  model: string;
  kb_api_key: string;
  kb_base_url: string;
  embedding_model: string;
  analysis_prompt: string;
  live_reply_prompt: string;
  live_theme: string;
  live_content: string;
  geo_models: GeoModelConfig[];
  geo_publish_platforms: GeoPublishPlatform[];
}

interface GeoModelConfig {
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
  enabled: boolean;
}

interface GeoPublishPlatform {
  name: string;
  url: string;
  description: string;
  system_prompt: string;
}

interface HermesConfig {
  enabled: boolean;
  gateway_url: string;
  api_key: string;
}

interface VideoConfig {
  fal_key: string;
  volc_key: string;
  openai_api_key: string;
  openai_base_url: string;
  openai_model: string;
  default_provider: string;
}

interface AppConfig {
  llm: LLMConfig;
  hermes: HermesConfig;
  video: VideoConfig;
}

const config = ref<AppConfig>({
  llm: {
    api_key: '',
    base_url: 'https://api.openai.com/v1',
    model: 'gpt-4o',
    kb_api_key: '',
    kb_base_url: 'https://api.openai.com/v1',
    embedding_model: 'text-embedding-3-small',
    analysis_prompt: '',
    live_reply_prompt: '',
    live_theme: '',
    live_content: '',
    geo_models: [],
    geo_publish_platforms: [],
  },
  hermes: {
    enabled: false,
    gateway_url: 'http://127.0.0.1:8642',
    api_key: '',
  },
  video: {
    fal_key: '',
    volc_key: '',
    openai_api_key: '',
    openai_base_url: 'https://api.openai.com/v1',
    openai_model: 'v0',
    default_provider: 'fal',
  },
});

const settingsInitialTab = inject<ReturnType<typeof ref<string>>>('settingsInitialTab');
const activeTab = ref(settingsInitialTab?.value || 'llm');
const isSaving = ref(false);
const saveStatus = ref<'idle' | 'success' | 'error'>('idle');
const statusMsg = ref('');

// 监听 inject 的变化 (如果父组件改变了 tab)
watch(() => settingsInitialTab?.value, (newTab) => {
  if (newTab) activeTab.value = newTab;
});

onMounted(async () => {
  try {
    const loadedConfig = await invoke('get_config') as AppConfig;
    config.value = loadedConfig;
  } catch (err) {
    console.error('Failed to load config:', err);
  }
});

const handleSave = async () => {
  isSaving.value = true;
  saveStatus.value = 'idle';
  try {
    await invoke('save_config', { config: config.value });
    saveStatus.value = 'success';
    statusMsg.value = '设置已保存';
    setTimeout(() => {
      saveStatus.value = 'idle';
    }, 3000);
  } catch (err) {
    saveStatus.value = 'error';
    statusMsg.value = String(err);
  } finally {
    isSaving.value = false;
  }
};

const resetToDefault = async () => {
  if (confirm('确定要恢复默认设置吗？所有当前修改将被覆盖。')) {
    try {
      const defaultConfig = await invoke('get_default_config') as AppConfig;
      config.value = defaultConfig;
    } catch (err) {
      alert('恢复默认失败: ' + err);
    }
  }
};

// --- GEO 辅助 ---
const addGeoModel = () => {
  config.value.llm.geo_models.push({
    name: 'New Model',
    base_url: 'https://api.openai.com/v1',
    api_key: '',
    model_id: '',
    enabled: true
  });
};

const removeGeoModel = (index: number) => {
  config.value.llm.geo_models.splice(index, 1);
};

// --- 发布平台辅助 ---
const addGeoPublishPlatform = () => {
  if (!config.value.llm.geo_publish_platforms) {
    config.value.llm.geo_publish_platforms = [];
  }
  config.value.llm.geo_publish_platforms.push({
    name: '新平台',
    url: '',
    description: '',
    system_prompt: '你是一位资深文案专家...'
  });
};

const removeGeoPublishPlatform = (index: number) => {
  config.value.llm.geo_publish_platforms.splice(index, 1);
};
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100">
    <!-- 顶部导航 -->
    <div class="px-8 pt-8 pb-4 border-b border-gray-900 bg-gray-950/50 backdrop-blur-xl sticky top-0 z-10">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h2 class="text-2xl font-bold bg-gradient-to-r from-white to-gray-400 bg-clip-text text-transparent">系统设置</h2>
          <p class="text-gray-500 text-sm mt-1">配置您的 AI 模型、知识库与辅助功能</p>
        </div>
        <button 
          @click="resetToDefault"
          class="flex items-center gap-2 text-xs text-gray-500 hover:text-orange-400 transition-colors px-3 py-1.5 rounded-lg hover:bg-orange-400/5 border border-transparent hover:border-orange-400/20"
        >
          <RotateCcw class="w-3.5 h-3.5" />
          恢复默认
        </button>
      </div>

      <div class="flex gap-2">
        <button
          v-for="tab in [
            { id: 'llm', name: '语言模型', icon: Cpu },
            { id: 'video', name: '视频生成', icon: Video },
            { id: 'hermes', name: 'Hermes 助手', icon: ShieldCheck },
            { id: 'geo', name: 'GEO 监控', icon: Globe }
          ]"
          :key="tab.id"
          @click="activeTab = tab.id"
          :class="[
            'flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-medium transition-all border',
            activeTab === tab.id 
              ? 'bg-blue-600 border-blue-500 text-white shadow-lg shadow-blue-900/20' 
              : 'bg-gray-900/50 border-gray-800 text-gray-400 hover:bg-gray-800 hover:text-gray-200'
          ]"
        >
          <component :is="tab.icon" class="w-4 h-4" />
          {{ tab.name }}
        </button>
      </div>
    </div>

    <!-- 内容区域 -->
    <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
      <div class="max-w-3xl mx-auto space-y-8 pb-12">
        
        <!-- LLM 设置 -->
        <div v-if="activeTab === 'llm'" class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
            <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
              <MessageSquare class="w-4 h-4 text-blue-500" />
              对话与推理配置 (Chat/Reasoning)
            </h3>
            
            <div class="grid grid-cols-1 gap-6">
              <div>
                <label class="block text-sm font-medium text-gray-300 mb-2">API Key</label>
                <input
                  v-model="config.llm.api_key"
                  type="password"
                  placeholder="sk-..."
                  class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
                />
              </div>

              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">Base URL</label>
                  <input
                    v-model="config.llm.base_url"
                    type="text"
                    placeholder="https://api.openai.com/v1"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all"
                  />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">Model ID</label>
                  <input
                    v-model="config.llm.model"
                    type="text"
                    placeholder="gpt-4o"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all"
                  />
                </div>
              </div>
            </div>
          </div>

          <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
            <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
              <Wand2 class="w-4 h-4 text-purple-500" />
              知识库与嵌入 (Knowledge Base & Embedding)
            </h3>
            
            <div class="grid grid-cols-1 gap-6">
              <div class="p-4 bg-purple-500/5 border border-purple-500/10 rounded-xl mb-2">
                <p class="text-xs text-purple-400 leading-relaxed">
                  知识库搜索依赖向量化 (Embedding)。如果留空，将默认尝试使用上方的推理 API Key 访问 OpenAI 接口。
                </p>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-300 mb-2">Embedding API Key (可选)</label>
                <input
                  v-model="config.llm.kb_api_key"
                  type="password"
                  placeholder="留空则沿用主 Key"
                  class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500 transition-all"
                />
              </div>

              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">Embedding Base URL</label>
                  <input
                    v-model="config.llm.kb_base_url"
                    type="text"
                    placeholder="https://api.openai.com/v1"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-purple-500 transition-all"
                  />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">Embedding Model</label>
                  <input
                    v-model="config.llm.embedding_model"
                    type="text"
                    placeholder="text-embedding-3-small"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-purple-500 transition-all"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 视频生成设置 -->
        <div v-if="activeTab === 'video'" class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-8 space-y-8 shadow-xl">
            <!-- fal.ai Key -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                fal.ai (Luma/Kling) API Key
              </label>
              <input
                v-model="config.video.fal_key"
                type="password"
                placeholder="FAL_KEY"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
              <p class="text-[11px] text-gray-500 mt-2">用于 Luma Dream Machine 等高性能模型</p>
            </div>

            <!-- Volcengine Key -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                火山引擎 (Volcengine) API Key
              </label>
              <input
                v-model="config.video.volc_key"
                type="password"
                placeholder="AccessKey:SecretKey"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
              <p class="text-[11px] text-gray-500 mt-2">请输入 "AccessKey:SecretKey" 格式</p>
            </div>

            <!-- OpenAI Compatible -->
            <div class="pt-4 border-t border-gray-800">
              <label class="block text-sm font-medium text-blue-400 mb-4 flex items-center gap-2">
                <Globe class="w-4 h-4" />
                OpenAI 兼容协议 (自定义服务商)
              </label>
              
              <div class="space-y-4">
                <div>
                  <label class="block text-[11px] text-gray-500 uppercase mb-1.5">API Key</label>
                  <input
                    v-model="config.video.openai_api_key"
                    type="password"
                    placeholder="sk-..."
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
                  />
                </div>
                <div>
                  <label class="block text-[11px] text-gray-500 uppercase mb-1.5">Base URL</label>
                  <input
                    v-model="config.video.openai_base_url"
                    type="text"
                    placeholder="https://api.openai.com/v1"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
                  />
                </div>
                <div>
                  <label class="block text-[11px] text-gray-500 uppercase mb-1.5">Model ID</label>
                  <input
                    v-model="config.video.openai_model"
                    type="text"
                    placeholder="v0"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
                  />
                </div>
              </div>
            </div>

            <!-- Default Provider -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2">默认生成服务商</label>
              <select v-model="config.video.default_provider" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all">
                <option value="fal">fal.ai</option>
                <option value="volcengine">火山引擎</option>
                <option value="openai">OpenAI 兼容协议</option>
                <option value="mock">测试模拟</option>
              </select>
            </div>
          </div>
        </div>

        <!-- Hermes 设置 -->
        <div v-if="activeTab === 'hermes'" class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-8 space-y-8 shadow-xl">
             <div class="flex items-center justify-between p-4 bg-blue-600/5 border border-blue-500/10 rounded-xl">
               <div class="flex items-center gap-4">
                 <div class="p-2 bg-blue-600 rounded-lg">
                   <ShieldCheck class="w-5 h-5 text-white" />
                 </div>
                 <div>
                   <h4 class="font-bold">Hermes 安全网关</h4>
                   <p class="text-xs text-gray-500 mt-0.5">多模型统一接入、Tool 调用管控及合规审计</p>
                 </div>
               </div>
               <div class="flex items-center gap-3">
                 <span :class="['text-[10px] px-2 py-0.5 rounded-full font-bold uppercase', config.hermes.enabled ? 'bg-green-500/10 text-green-500' : 'bg-gray-800 text-gray-500']">
                   {{ config.hermes.enabled ? 'Active' : 'Disabled' }}
                 </span>
                 <button 
                  @click="config.hermes.enabled = !config.hermes.enabled"
                  :class="['w-12 h-6 rounded-full relative transition-all duration-300', config.hermes.enabled ? 'bg-blue-600' : 'bg-gray-800']"
                >
                  <div :class="['absolute top-1 w-4 h-4 bg-white rounded-full transition-all duration-300 shadow-sm', config.hermes.enabled ? 'left-7' : 'left-1']"></div>
                </button>
               </div>
             </div>

             <div class="space-y-6" :class="{ 'opacity-50 pointer-events-none': !config.hermes.enabled }">
               <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">网关地址 (Gateway URL)</label>
                  <input
                    v-model="config.hermes.gateway_url"
                    type="text"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all"
                  />
               </div>
               <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">Hermes API Key</label>
                  <input
                    v-model="config.hermes.api_key"
                    type="password"
                    placeholder="hms-..."
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all"
                  />
               </div>
             </div>
          </div>
        </div>

        <!-- GEO 监控设置 -->
        <div v-if="activeTab === 'geo'" class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
           <!-- 多模型监控节点 -->
           <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 shadow-xl">
             <div class="flex items-center justify-between mb-6">
                <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                  <Globe class="w-4 h-4 text-emerald-500" />
                  多模型 GEO 监控节点
                </h3>
                <button 
                  @click="addGeoModel"
                  class="bg-blue-600 hover:bg-blue-500 text-white text-[11px] font-bold px-3 py-1.5 rounded-lg transition-all"
                >
                  添加模型
                </button>
             </div>

             <div class="space-y-4">
               <div 
                 v-for="(model, idx) in config.llm.geo_models" 
                 :key="idx"
                 class="p-4 bg-gray-950 border border-gray-800 rounded-xl space-y-4 group relative"
               >
                 <button 
                   @click="removeGeoModel(idx)"
                   class="absolute top-4 right-4 text-gray-600 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all"
                 >
                   <XCircle class="w-4 h-4" />
                 </button>

                 <div class="grid grid-cols-2 gap-4">
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">节点名称</label>
                      <input v-model="model.name" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white" />
                    </div>
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">Model ID</label>
                      <input v-model="model.model_id" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white font-mono" />
                    </div>
                 </div>

                 <div class="grid grid-cols-1 gap-4 pt-2 border-t border-gray-900">
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">Base URL</label>
                      <input v-model="model.base_url" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-gray-300" />
                    </div>
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">API Key</label>
                      <input v-model="model.api_key" type="password" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white" />
                    </div>
                 </div>
               </div>

               <div v-if="config.llm.geo_models.length === 0" class="text-center py-12 border-2 border-dashed border-gray-900 rounded-2xl">
                 <Globe class="w-10 h-10 text-gray-800 mx-auto mb-3" />
                 <p class="text-xs text-gray-600">暂无监控节点，点击“添加模型”开始</p>
               </div>
             </div>
           </div>

           <!-- 内容发布平台 -->
           <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 shadow-xl">
             <div class="flex items-center justify-between mb-6">
                <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
                  <FileText class="w-4 h-4 text-purple-500" />
                  内容发布平台 (GEO 驱动)
                </h3>
                <button 
                  @click="addGeoPublishPlatform"
                  class="bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-bold px-3 py-1.5 rounded-lg transition-all"
                >
                  添加平台
                </button>
             </div>

             <div class="space-y-4">
               <div 
                 v-for="(platform, idx) in config.llm.geo_publish_platforms" 
                 :key="idx"
                 class="p-4 bg-gray-950 border border-gray-800 rounded-xl space-y-4 group relative"
               >
                 <button 
                   @click="removeGeoPublishPlatform(idx)"
                   class="absolute top-4 right-4 text-gray-600 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all"
                 >
                   <XCircle class="w-4 h-4" />
                 </button>

                 <div class="grid grid-cols-2 gap-4">
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">平台名称</label>
                      <input v-model="platform.name" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white" />
                    </div>
                    <div>
                      <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">基础 URL (可选)</label>
                      <input v-model="platform.url" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white font-mono" />
                    </div>
                 </div>

                 <div>
                    <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">平台描述 / 规则说明</label>
                    <input v-model="platform.description" type="text" placeholder="描述此平台的受众、调性或发布限制..." class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-gray-300" />
                 </div>

                 <div>
                    <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">针对该平台的 System Prompt</label>
                    <textarea 
                      v-model="platform.system_prompt" 
                      rows="3"
                      class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-gray-400 focus:text-white transition-colors"
                    ></textarea>
                 </div>
               </div>

               <div v-if="config.llm.geo_publish_platforms.length === 0" class="text-center py-12 border-2 border-dashed border-gray-900 rounded-2xl">
                 <FileText class="w-10 h-10 text-gray-800 mx-auto mb-3" />
                 <p class="text-xs text-gray-600">暂无发布平台，点击“添加平台”开始</p>
               </div>
             </div>
           </div>
        </div>

        <!-- 保存操作 -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <transition name="fade">
              <div v-if="saveStatus === 'success'" class="flex items-center gap-1.5 text-green-500 text-sm">
                <CheckCircle class="w-4 h-4" />
                {{ statusMsg }}
              </div>
              <div v-else-if="saveStatus === 'error'" class="flex items-center gap-1.5 text-red-500 text-sm">
                <XCircle class="w-4 h-4" />
                {{ statusMsg }}
              </div>
            </transition>
          </div>

          <button
            @click="handleSave"
            :disabled="isSaving"
            class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-white px-8 py-3 rounded-xl font-medium transition-all shadow-lg shadow-blue-900/20"
          >
            <RefreshCw v-if="isSaving" class="w-4 h-4 animate-spin" />
            <Save v-else class="w-4 h-4" />
            {{ isSaving ? '正在保存...' : '保存设置' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
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
