<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Save, RefreshCw, CheckCircle, XCircle, ShieldCheck, Globe, Cpu, Wand2, Video, MessageSquare, RotateCcw, Binary } from 'lucide-vue-next';

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
    live_theme: '',
    live_content: '',
  },
});

const activeTab = ref<'model' | 'prompt' | 'live' | 'kb'>('model');
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

async function restoreDefaultPrompt(type: 'analysis' | 'live') {
  try {
    const defaultConfig = await invoke('get_default_config') as AppConfig;
    if (type === 'analysis') {
      config.value.llm.analysis_prompt = defaultConfig.llm.analysis_prompt;
    } else {
      config.value.llm.live_reply_prompt = defaultConfig.llm.live_reply_prompt;
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
