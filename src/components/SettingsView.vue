<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Save, RefreshCw, CheckCircle, XCircle, ShieldCheck, Globe, Cpu, Wand2, Video, MessageSquare, RotateCcw, Binary, Plus, Trash2, BarChart3, ToggleLeft, ToggleRight } from 'lucide-vue-next';

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
}

interface LLMConfig {
  api_key: string;
  base_url: string;
  model: string;
  kb_api_key: string;
  kb_base_url: string;
  embedding_model: string;
  analysis_prompt: string;
  live_reply_prompt: string;
  im_reply_prompt: string;
  live_theme: string;
  live_content: string;
  geo_models: GeoModelConfig[];
  geo_publish_platforms: GeoPublishPlatform[];
}

interface AppConfig {
  llm: LLMConfig;
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
    im_reply_prompt: '',
    live_theme: '',
    live_content: '',
    geo_models: [],
    geo_publish_platforms: [],
  },
});

const activeTab = ref<'model' | 'prompt' | 'live' | 'kb' | 'geo'>('model');

function addGeoModel() {
  config.value.llm.geo_models.push({ name: '', base_url: '', api_key: '', model_id: '', enabled: true });
}
function removeGeoModel(index: number) {
  config.value.llm.geo_models.splice(index, 1);
}

function addPublishPlatform() {
  config.value.llm.geo_publish_platforms.push({ name: '', url: '', description: '' });
}
function removePublishPlatform(index: number) {
  config.value.llm.geo_publish_platforms.splice(index, 1);
}

const GEO_MODEL_PRESETS = [
  { name: '豆包', base_url: 'https://ark.cn-beijing.volces.com/api/v3', model_id: '' },
  { name: 'Kimi', base_url: 'https://api.moonshot.cn/v1', model_id: 'moonshot-v1-8k' },
  { name: '通义千问', base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1', model_id: 'qwen-plus' },
  { name: 'ChatGPT', base_url: 'https://api.openai.com/v1', model_id: 'gpt-4o' },
];

const GEO_PUBLISH_PRESETS = [
  { name: '知乎', url: 'https://www.zhihu.com/write', description: '高权重问答平台，AI 引用频率高' },
  { name: '百度百科', url: 'https://baike.baidu.com/', description: '权威百科，国内 AI 核心数据来源' },
  { name: '微信公众号', url: 'https://mp.weixin.qq.com/', description: '内容被搜狗及多家 AI 收录' },
  { name: '小红书', url: 'https://creator.xiaohongshu.com/', description: '生活内容类 AI 引用率上升' },
  { name: 'B站专栏', url: 'https://member.bilibili.com/platform/upload/text/edit', description: '视频知识类内容' },
  { name: '今日头条', url: 'https://mp.toutiao.com/', description: '字节系 AI 数据来源之一' },
];
const isSaving = ref(false);
const saveStatus = ref<'idle' | 'success' | 'error'>('idle');
const statusMsg = ref('');


async function loadConfig() {
  try {
    const res = await invoke('get_config') as AppConfig;
    if (res.llm) {
      config.value = res;
    }
  } catch (e) {
    console.error('加载配置失败:', e);
  }
}

async function restoreDefaultPrompt(type: 'analysis' | 'live' | 'im') {
  try {
    const defaultConfig = await invoke('get_default_config') as AppConfig;
    if (type === 'analysis') {
      config.value.llm.analysis_prompt = defaultConfig.llm.analysis_prompt;
    } else if (type === 'live') {
      config.value.llm.live_reply_prompt = defaultConfig.llm.live_reply_prompt;
    } else if (type === 'im') {
      config.value.llm.im_reply_prompt = defaultConfig.llm.im_reply_prompt;
    }
    statusMsg.value = '已恢复默认提示词';
    saveStatus.value = 'success';
    setTimeout(() => { saveStatus.value = 'idle'; }, 2000);
  } catch (e) {
    console.error('恢复默认失败:', e);
  }
}

async function handleSave() {
  isSaving.value = true;
  saveStatus.value = 'idle';
  try {
    await invoke('save_config', { config: config.value });
    saveStatus.value = 'success';
    statusMsg.value = '配置已保存';
    setTimeout(() => {
      saveStatus.value = 'idle';
    }, 3000);
  } catch (e) {
    console.error('保存配置失败:', e);
    saveStatus.value = 'error';
    statusMsg.value = '保存失败: ' + e;
  } finally {
    isSaving.value = false;
  }
}

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <div class="flex flex-col flex-1 h-full bg-gray-950 p-8 overflow-y-auto">
    <div class="max-w-2xl mx-auto w-full">
      <div class="flex items-center justify-between mb-8">
        <div>
          <h2 class="text-2xl font-bold text-white">系统设置</h2>
          <p class="text-gray-400 text-sm mt-1">配置 LLM 模型及其他系统参数</p>
        </div>
      </div>

      <!-- 选项卡切换 -->
      <div class="flex items-center gap-1 p-1 bg-gray-900 rounded-xl mb-8 border border-gray-800">
        <button
          @click="activeTab = 'model'"
          :class="[
            'flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg text-sm font-medium transition-all',
            activeTab === 'model' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300'
          ]"
        >
          <Cpu class="w-4 h-4" />
          模型配置
        </button>
        <button
          @click="activeTab = 'prompt'"
          :class="[
            'flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg text-sm font-medium transition-all',
            activeTab === 'prompt' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300'
          ]"
        >
          <Wand2 class="w-4 h-4" />
          预置提示词
        </button>
        <button
          @click="activeTab = 'live'"
          :class="[
            'flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg text-sm font-medium transition-all',
            activeTab === 'live' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300'
          ]"
        >
          <Video class="w-4 h-4" />
          直播助手
        </button>
        <button
          @click="activeTab = 'kb'"
          :class="[
            'flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg text-sm font-medium transition-all',
            activeTab === 'kb' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300'
          ]"
        >
          <Database class="w-4 h-4" />
          知识库
        </button>
        <button
          @click="activeTab = 'geo'"
          :class="[
            'flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg text-sm font-medium transition-all',
            activeTab === 'geo' ? 'bg-gray-800 text-white shadow-sm' : 'text-gray-500 hover:text-gray-300'
          ]"
        >
          <BarChart3 class="w-4 h-4" />
          GEO 监控
        </button>
      </div>

      <!-- 配置内容区域 -->
      <div class="min-h-[400px]">
        <!-- 模型配置标签页 -->
        <div v-if="activeTab === 'model'" class="bg-gray-900 border border-gray-800 rounded-2xl p-6 mb-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center gap-2 mb-6">
            <div class="p-2 bg-blue-500/10 rounded-lg">
              <Cpu class="w-5 h-5 text-blue-500" />
            </div>
            <h3 class="text-lg font-medium text-white">LLM 基础参数</h3>
          </div>

          <div class="space-y-6">
            <!-- API Key -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <ShieldCheck class="w-4 h-4 text-gray-400" />
                API Key
              </label>
              <input
                v-model="config.llm.api_key"
                type="password"
                placeholder="sk-..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
              <p class="text-[11px] text-gray-500 mt-2">支持 OpenAI 协议兼容的所有模型服务商（如 DeepSeek, Moonshot 等）</p>
            </div>

            <!-- Base URL -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <Globe class="w-4 h-4 text-gray-400" />
                API 代理地址 (Base URL)
              </label>
              <input
                v-model="config.llm.base_url"
                type="text"
                placeholder="https://api.openai.com/v1"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
            </div>

            <!-- Model -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <Cpu class="w-4 h-4 text-gray-400" />
                聊天模型 (Chat Model)
              </label>
              <input
                v-model="config.llm.model"
                type="text"
                placeholder="gpt-4o"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
            </div>
          </div>
        </div>

        <!-- 预置提示词标签页 -->
        <div v-if="activeTab === 'prompt'" class="bg-gray-900 border border-gray-800 rounded-2xl p-6 mb-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center gap-2 mb-6">
            <div class="p-2 bg-purple-500/10 rounded-lg">
              <Wand2 class="w-5 h-5 text-purple-500" />
            </div>
            <h3 class="text-lg font-medium text-white">AI 分析预置提示词</h3>
          </div>

          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="block text-sm font-medium text-gray-300">系统提示词 (System Prompt)</label>
              <button 
                @click="restoreDefaultPrompt('analysis')" 
                class="flex items-center gap-1 text-[10px] text-gray-500 hover:text-blue-400 transition-colors bg-gray-950 px-2 py-1 rounded border border-gray-800"
              >
                <RotateCcw class="w-3 h-3" /> 恢复默认
              </button>
            </div>
            <textarea
              v-model="config.llm.analysis_prompt"
              rows="12"
              placeholder="输入 AI 分析时的系统提示词..."
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all resize-none font-sans text-sm leading-relaxed"
            ></textarea>
            <div class="mt-4 p-4 bg-gray-950 rounded-xl border border-gray-800/50">
              <p class="text-[11px] text-gray-500 leading-relaxed">
                <strong>💡 提示：</strong>您可以自定义 AI 对评论的分析方向。例如要求它专注于“产品质量反馈”或“竞品对比”。该提示词将在点击“AI分析”按钮时生效。
              </p>
            </div>
          </div>
        </div>

        <!-- 直播助手标签页 -->
        <div v-if="activeTab === 'live'" class="bg-gray-900 border border-gray-800 rounded-2xl p-6 mb-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center gap-2 mb-6">
            <div class="p-2 bg-red-500/10 rounded-lg">
              <Video class="w-5 h-5 text-red-500" />
            </div>
            <h3 class="text-lg font-medium text-white">直播间助手配置</h3>
          </div>

          <div class="space-y-6">
            <!-- Live Theme -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2">直播主题</label>
              <input
                v-model="config.llm.live_theme"
                type="text"
                placeholder="例如: 智能家居好物分享"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
            </div>

            <!-- Live Content -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2">直播核心内容/背景</label>
              <textarea
                v-model="config.llm.live_content"
                rows="4"
                placeholder="输入直播的核心卖点、背景知识或常见问题回答..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all resize-none font-sans text-sm"
              ></textarea>
            </div>

            <!-- Live Prompt -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <label class="block text-sm font-medium text-gray-300 flex items-center gap-2">
                  <MessageSquare class="w-4 h-4 text-gray-400" />
                  回复预置提示词 (System Prompt)
                </label>
                <button 
                  @click="restoreDefaultPrompt('live')" 
                  class="flex items-center gap-1 text-[10px] text-gray-500 hover:text-blue-400 transition-colors bg-gray-950 px-2 py-1 rounded border border-gray-800"
                >
                  <RotateCcw class="w-3 h-3" /> 恢复默认
                </button>
              </div>
              <textarea
                v-model="config.llm.live_reply_prompt"
                rows="5"
                placeholder="设置 AI 主播的回复风格..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all resize-none font-sans text-sm"
              ></textarea>
              <p class="text-[11px] text-gray-500 mt-2">提示 AI 以主播身份简短、亲切地回复用户弹幕</p>
            </div>

            <!-- IM Reply Prompt -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <label class="block text-sm font-medium text-gray-300 flex items-center gap-2">
                  <MessageCircle class="w-4 h-4 text-gray-400" />
                  私信回复预置提示词 (System Prompt)
                </label>
                <button 
                  @click="restoreDefaultPrompt('im')" 
                  class="flex items-center gap-1 text-[10px] text-gray-500 hover:text-blue-400 transition-colors bg-gray-950 px-2 py-1 rounded border border-gray-800"
                >
                  <RotateCcw class="w-3 h-3" /> 恢复默认
                </button>
              </div>
              <textarea
                v-model="config.llm.im_reply_prompt"
                rows="5"
                placeholder="设置 AI 客服/经理的回复风格..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all resize-none font-sans text-sm"
              ></textarea>
              <p class="text-[11px] text-gray-500 mt-2">提示 AI 以专业身份回复用户私信</p>
            </div>
          </div>
        </div>

        <!-- 知识库配置标签页 -->
        <div v-if="activeTab === 'kb'" class="bg-gray-900 border border-gray-800 rounded-2xl p-6 mb-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center gap-2 mb-6">
            <div class="p-2 bg-blue-500/10 rounded-lg">
              <Database class="w-5 h-5 text-blue-500" />
            </div>
            <h3 class="text-lg font-medium text-white">知识库 (RAG) 参数配置</h3>
          </div>

          <div class="space-y-6">
            <!-- KB API Key -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <ShieldCheck class="w-4 h-4 text-gray-400" />
                知识库 API Key
              </label>
              <input
                v-model="config.llm.kb_api_key"
                type="password"
                placeholder="sk-..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
              <p class="text-[11px] text-gray-500 mt-2">选填，如果不填写则默认使用“模型配置”中的主 Key</p>
            </div>

            <!-- KB Base URL -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <Globe class="w-4 h-4 text-gray-400" />
                知识库 API 代理地址 (Base URL)
              </label>
              <input
                v-model="config.llm.kb_base_url"
                type="text"
                placeholder="https://api.openai.com/v1"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
            </div>

            <!-- Embedding Model -->
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-2 flex items-center gap-2">
                <Binary class="w-4 h-4 text-gray-400" />
                向量模型 (Embedding Model)
              </label>
              <input
                v-model="config.llm.embedding_model"
                type="text"
                placeholder="text-embedding-3-small"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all"
              />
              <div class="mt-4 p-4 bg-gray-950 rounded-xl border border-gray-800/50 space-y-2">
                <p class="text-[11px] text-gray-400 leading-relaxed">
                  <strong>什么是向量模型？</strong><br/>
                  该模型用于将您上传的文档（PDF/TXT）转换为 AI 可以理解的数学向量。
                </p>
                <p class="text-[11px] text-yellow-600/80 leading-relaxed">
                  <strong>⚠️ 重要提示：</strong><br/>
                  并非所有 API 供应商都支持此功能。如果您使用 DeepSeek 等不提供向量接口的服务商，您需要为知识库配置一个支持 Embedding 的代理或使用 OpenAI 原生模型。
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- GEO 监控模型配置 -->
        <div v-if="activeTab === 'geo'" class="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300">

          <!-- 1. LLM 查询模型 -->
          <div class="bg-gray-900 border border-gray-800 rounded-2xl p-6">
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center gap-2">
                <div class="p-2 bg-purple-500/10 rounded-lg">
                  <BarChart3 class="w-5 h-5 text-purple-400" />
                </div>
                <h3 class="text-lg font-medium text-white">查询用 LLM 模型</h3>
              </div>
              <button @click="addGeoModel" class="flex items-center gap-1.5 text-xs bg-purple-600/20 hover:bg-purple-600/40 text-purple-400 px-3 py-1.5 rounded-lg border border-purple-600/30 transition-all">
                <Plus class="w-3.5 h-3.5" /> 添加模型
              </button>
            </div>
            <p class="text-xs text-gray-500 mb-5">配置用于发起 GEO 查询的 AI 大模型，系统会向它们发问并检测品牌是否被提及。使用 OpenAI 兼容协议，支持中转站。</p>

            <div class="flex flex-wrap gap-2 mb-5">
              <button
                v-for="preset in GEO_MODEL_PRESETS" :key="preset.name"
                @click="config.llm.geo_models.push({ ...preset, api_key: '', enabled: true })"
                class="text-xs px-3 py-1.5 rounded-lg border border-gray-700 text-gray-400 hover:border-purple-500 hover:text-purple-400 transition-all bg-gray-950"
              >+ {{ preset.name }}</button>
            </div>

            <div v-if="config.llm.geo_models.length === 0" class="text-center py-8 text-gray-600 text-sm border border-dashed border-gray-800 rounded-xl">
              尚未配置查询模型，点击「添加模型」或使用上方预设
            </div>
            <div class="space-y-3">
              <div
                v-for="(model, idx) in config.llm.geo_models" :key="idx"
                class="border rounded-xl p-4 transition-colors"
                :class="model.enabled ? 'border-gray-700 bg-gray-950' : 'border-gray-800 bg-gray-950/50 opacity-60'"
              >
                <div class="flex items-center justify-between mb-3">
                  <input v-model="model.name" placeholder="平台名称（如豆包）" class="bg-transparent text-white font-medium text-sm focus:outline-none placeholder-gray-600 flex-1" />
                  <div class="flex items-center gap-2">
                    <button @click="model.enabled = !model.enabled" class="text-gray-500 hover:text-white transition-colors">
                      <ToggleRight v-if="model.enabled" class="w-5 h-5 text-purple-400" />
                      <ToggleLeft v-else class="w-5 h-5" />
                    </button>
                    <button @click="removeGeoModel(idx)" class="text-gray-600 hover:text-red-400 transition-colors">
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="text-[10px] text-gray-500 uppercase tracking-wider block mb-1">Base URL</label>
                    <input v-model="model.base_url" placeholder="https://api.example.com/v1" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-purple-500 transition-colors" />
                  </div>
                  <div>
                    <label class="text-[10px] text-gray-500 uppercase tracking-wider block mb-1">Model ID</label>
                    <input v-model="model.model_id" placeholder="model-name 或 ep-xxxx" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-purple-500 transition-colors" />
                  </div>
                  <div class="col-span-2">
                    <label class="text-[10px] text-gray-500 uppercase tracking-wider block mb-1">API Key</label>
                    <input v-model="model.api_key" type="password" placeholder="sk-..." class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-purple-500 transition-colors" />
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 2. 内容发布平台（提升 GEO 排名的数据来源） -->
          <div class="bg-gray-900 border border-gray-800 rounded-2xl p-6">
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center gap-2">
                <div class="p-2 bg-blue-500/10 rounded-lg">
                  <Globe class="w-5 h-5 text-blue-400" />
                </div>
                <h3 class="text-lg font-medium text-white">内容发布平台</h3>
              </div>
              <button @click="addPublishPlatform" class="flex items-center gap-1.5 text-xs bg-blue-600/20 hover:bg-blue-600/40 text-blue-400 px-3 py-1.5 rounded-lg border border-blue-600/30 transition-all">
                <Plus class="w-3.5 h-3.5" /> 添加平台
              </button>
            </div>
            <p class="text-xs text-gray-500 mb-5">这些平台是 AI 大模型的数据来源。在此发布内容可提升品牌在 AI 回答中的出现概率。查询结果页会显示快捷跳转链接。</p>

            <div class="flex flex-wrap gap-2 mb-5">
              <button
                v-for="preset in GEO_PUBLISH_PRESETS" :key="preset.name"
                @click="config.llm.geo_publish_platforms.push({ ...preset })"
                class="text-xs px-3 py-1.5 rounded-lg border border-gray-700 text-gray-400 hover:border-blue-500 hover:text-blue-400 transition-all bg-gray-950"
              >+ {{ preset.name }}</button>
            </div>

            <div v-if="config.llm.geo_publish_platforms.length === 0" class="text-center py-8 text-gray-600 text-sm border border-dashed border-gray-800 rounded-xl">
              尚未添加发布平台，点击「添加平台」或使用上方预设
            </div>
            <div class="space-y-2">
              <div v-for="(platform, idx) in config.llm.geo_publish_platforms" :key="idx" class="flex items-center gap-3 p-3 border border-gray-800 bg-gray-950 rounded-xl">
                <div class="flex-1 grid grid-cols-3 gap-2">
                  <input v-model="platform.name" placeholder="平台名称" class="bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-blue-500 transition-colors" />
                  <input v-model="platform.url" placeholder="发布页面 URL" class="bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-blue-500 transition-colors col-span-2" />
                </div>
                <button @click="removePublishPlatform(idx)" class="text-gray-600 hover:text-red-400 transition-colors flex-shrink-0">
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
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
</template>

<style scoped>
.fade-enter-active, .fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}
</style>
