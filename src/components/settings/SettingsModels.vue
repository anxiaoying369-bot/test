<script setup lang="ts">
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Wand2, Mic, Cpu, Image as ImageIcon, Globe, Plus, Trash2, XCircle, Film, Loader2, CheckCircle2, AlertCircle, Server, Download, Wand } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config, llmTestPassed } = useAppConfig();

// ─── API 中转站 (New API)：一处填地址/密钥 → 拉模型列表 → 应用到下方所有用途 ───
const relayUrl = ref(config.value.llm.base_url || 'https://api.openai.com/v1');
const relayKey = ref(config.value.llm.api_key || '');
const relayModels = ref<string[]>([]);
const fetchingModels = ref(false);
const relayMsg = ref('');
const relayOk = ref(false);

async function fetchRelayModels(silent = false) {
  if (!relayUrl.value.trim() || !relayKey.value.trim()) {
    if (!silent) { relayOk.value = false; relayMsg.value = '请先填写中转站 URL 和 API Key'; }
    return;
  }
  fetchingModels.value = true;
  if (!silent) relayMsg.value = '';
  try {
    relayModels.value = await invoke<string[]>('list_relay_models', {
      baseUrl: relayUrl.value,
      apiKey: relayKey.value,
    });
    relayOk.value = true;
    relayMsg.value = `已获取 ${relayModels.value.length} 个模型，下方各「模型」框可下拉选择`;
  } catch (e) {
    if (!silent) { relayOk.value = false; relayMsg.value = String(e); }
  } finally {
    fetchingModels.value = false;
  }
}

// 把中转站地址/密钥一键灌进下方所有用途（LLM/嵌入/图片/TTS/STT/GEO 节点）
function applyRelayToAll() {
  const u = relayUrl.value.trim();
  const k = relayKey.value.trim();
  if (!u || !k) { relayOk.value = false; relayMsg.value = '请先填写中转站 URL 和 API Key'; return; }
  config.value.llm.base_url = u; config.value.llm.api_key = k;
  config.value.llm.kb_base_url = u; config.value.llm.kb_api_key = k;
  config.value.video.openai_base_url = u; config.value.video.openai_api_key = k;
  config.value.video.tts_base_url = u; config.value.video.tts_api_key = k;
  config.value.stt.base_url = u; config.value.stt.api_key = k;
  (config.value.llm.geo_models || []).forEach((m) => { m.base_url = u; m.api_key = k; });
  relayOk.value = true;
  relayMsg.value = '已将中转站地址与密钥应用到下方所有模型（记得各处选好模型再保存）';
}

// 配置加载完成（api_key 出现）后，自动用已保存的 LLM 地址/密钥种入中转站并静默拉一次模型
let relaySeeded = false;
watch(
  () => config.value.llm.api_key,
  (k) => {
    if (!relaySeeded && k) {
      relaySeeded = true;
      relayUrl.value = config.value.llm.base_url || relayUrl.value;
      relayKey.value = k;
      fetchRelayModels(true);
    }
  },
  { immediate: true },
);

// --- LLM 连接测试：测试通过前不允许保存（gate 在 SettingsView 的保存按钮）---
const testing = ref(false);
const testMsg = ref('');
const testOk = ref(false);

async function testLlm() {
  testing.value = true;
  testMsg.value = '';
  try {
    const msg = await invoke<string>('test_llm_connection', {
      apiKey: config.value.llm.api_key,
      baseUrl: config.value.llm.base_url,
      model: config.value.llm.model,
    });
    testOk.value = true;
    testMsg.value = msg;
    llmTestPassed.value = true;
  } catch (e) {
    testOk.value = false;
    testMsg.value = String(e);
    llmTestPassed.value = false;
  } finally {
    testing.value = false;
  }
}

// 改动任一 LLM 字段后，需重新测试才能保存
watch(
  () => [config.value.llm.api_key, config.value.llm.base_url, config.value.llm.model],
  () => {
    llmTestPassed.value = false;
    testMsg.value = '';
  },
);

// --- GEO 监控节点辅助 ---
const addGeoModel = () => {
  if (!config.value.llm.geo_models) config.value.llm.geo_models = [];
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

// --- 音色组增删 ---
function addTtsVoice() {
  if (!config.value.video.tts_voices) config.value.video.tts_voices = [];
  config.value.video.tts_voices.push({ voice_id: '', name: '' });
}
function removeTtsVoice(index: number) {
  config.value.video.tts_voices?.splice(index, 1);
}
</script>

<template>
  <div class="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <!-- 共享：中转站拉到的模型列表，供下方各「模型」框下拉选择 -->
    <datalist id="relay-models">
      <option v-for="m in relayModels" :key="m" :value="m" />
    </datalist>

    <!-- 0. API 中转站 (New API) -->
    <div class="bg-gradient-to-br from-blue-950/40 to-gray-900/50 border border-blue-800/40 rounded-2xl p-6 space-y-4 shadow-xl">
      <h3 class="text-sm font-bold text-blue-300 uppercase tracking-widest flex items-center gap-2">
        <Server class="w-4 h-4 text-blue-400" />
        API 中转站 (New API)
      </h3>
      <p class="text-xs text-gray-400 leading-relaxed">
        填中转站地址和密钥 → 点「获取模型列表」拉取可用模型 → 点「应用到所有模型」把地址/密钥一键填入下方全部用途，
        再在各处「模型」框下拉选择对应模型即可。无需逐项重复填地址和密钥。
      </p>
      <div class="grid grid-cols-1 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">中转站 URL（填到 /v1 这一层）</label>
          <input v-model="relayUrl" type="text" placeholder="https://your-newapi.com/v1"
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">中转站 API Key</label>
          <input v-model="relayKey" type="password" placeholder="sk-..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all font-mono" />
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-3 pt-1">
        <button @click="fetchRelayModels()" :disabled="fetchingModels"
          class="text-sm px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white flex items-center gap-2 transition-colors">
          <Loader2 v-if="fetchingModels" class="w-4 h-4 animate-spin" />
          <Download v-else class="w-4 h-4" />
          {{ fetchingModels ? '获取中…' : '获取模型列表' }}
        </button>
        <button @click="applyRelayToAll"
          class="text-sm px-4 py-2 rounded-xl bg-gray-800 hover:bg-gray-700 text-gray-200 flex items-center gap-2 transition-colors">
          <Wand class="w-4 h-4" /> 应用到所有模型
        </button>
        <span v-if="relayMsg" :class="['text-xs flex items-center gap-1', relayOk ? 'text-green-400' : 'text-red-400']">
          <CheckCircle2 v-if="relayOk" class="w-3.5 h-3.5 shrink-0" />
          <AlertCircle v-else class="w-3.5 h-3.5 shrink-0" />
          {{ relayMsg }}
        </span>
      </div>
    </div>

    <!-- 1. LLM 对话模型 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <MessageSquare class="w-4 h-4 text-blue-500" />
        LLM 对话模型 (Chat/Reasoning)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">API Key</label>
          <input
            v-model="config.llm.api_key"
            type="password"
            placeholder="sk-..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all font-mono"
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
              list="relay-models"
              type="text"
              placeholder="gpt-4o"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all"
            />
          </div>
        </div>
      </div>

      <!-- 测试连接：通过后才能保存 -->
      <div class="flex items-center gap-3 pt-1">
        <button
          @click="testLlm"
          :disabled="testing"
          class="text-sm px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white flex items-center gap-2 transition-colors"
        >
          <Loader2 v-if="testing" class="w-4 h-4 animate-spin" />
          <Cpu v-else class="w-4 h-4" />
          {{ testing ? '测试中…' : '测试连接' }}
        </button>
        <span
          v-if="testMsg"
          :class="['text-xs flex items-center gap-1', testOk ? 'text-green-400' : 'text-red-400']"
        >
          <CheckCircle2 v-if="testOk" class="w-3.5 h-3.5 shrink-0" />
          <AlertCircle v-else class="w-3.5 h-3.5 shrink-0" />
          {{ testMsg }}
        </span>
        <span v-else-if="!llmTestPassed" class="text-xs text-amber-400/90 flex items-center gap-1">
          <AlertCircle class="w-3.5 h-3.5 shrink-0" /> 测试通过后才能保存设置
        </span>
      </div>
    </div>

    <!-- 2. 图片生成模型 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <ImageIcon class="w-4 h-4 text-cyan-500" />
        图片生成模型 (Image Generation)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-cyan-400 mb-1.5 flex items-center gap-2">
            <Globe class="w-4 h-4" />
            OpenAI 兼容协议凭证 (图片生成)
          </label>
          <div class="space-y-4 bg-gray-950/50 p-4 rounded-xl border border-gray-800">
            <div>
              <label class="block text-[11px] text-gray-500 uppercase mb-1.5">API Key</label>
              <input
                v-model="config.video.openai_api_key"
                type="password"
                placeholder="sk-..."
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-cyan-500 transition-all font-mono"
              />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-[11px] text-gray-500 uppercase mb-1.5">Base URL</label>
                <input
                  v-model="config.video.openai_base_url"
                  type="text"
                  placeholder="https://api.openai.com/v1"
                  class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-cyan-500 transition-all"
                />
              </div>
              <div>
                <label class="block text-[11px] text-gray-500 uppercase mb-1.5">Model ID</label>
                <input
                  v-model="config.video.openai_model"
                  list="relay-models"
                  type="text"
                  placeholder="v0"
                  class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-cyan-500 transition-all"
                />
              </div>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">默认图片生成服务商</label>
            <select v-model="config.video.default_provider" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-cyan-500 transition-all">
              <option value="fal">fal.ai</option>
              <option value="volcengine">火山引擎</option>
              <option value="openai">OpenAI 兼容协议</option>
              <option value="mock">测试模拟</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. 声音合成 (TTS) -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <Cpu class="w-4 h-4 text-purple-500" />
        声音合成模型 (TTS)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">TTS Provider</label>
          <select v-model="config.video.tts_provider"
                  class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-purple-500 transition-all">
            <option value="mock">测试模拟 (静音占位)</option>
            <option value="openai">OpenAI 兼容协议</option>
            <option value="minimax">MiniMax 语音合成</option>
            <option value="volcengine">火山引擎</option>
          </select>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">TTS API Key</label>
          <input v-model="config.video.tts_api_key" type="password"
                 :placeholder="config.video.tts_provider === 'volcengine' ? 'appid:access_token' : 'sk-...'"
                 class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500 font-mono text-sm" />
        </div>

        <div v-if="config.video.tts_provider === 'openai' || config.video.tts_provider === 'minimax'" class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Base URL</label>
            <input v-model="config.video.tts_base_url" type="text"
                   placeholder="https://api.openai.com/v1"
                   class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Model ID</label>
            <input v-model="config.video.tts_model" list="relay-models" type="text"
                   placeholder="tts-1"
                   class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500" />
          </div>
        </div>

        <div class="border-t border-gray-800 pt-5">
          <div class="flex items-center justify-between mb-4">
            <label class="block text-sm font-medium text-gray-300">音色库列表</label>
            <button @click="addTtsVoice"
                    class="px-3 py-1.5 bg-purple-600/20 hover:bg-purple-600/40 text-purple-300 border border-purple-500/30 rounded-lg text-xs flex items-center gap-1.5 transition-colors font-bold uppercase">
              <Plus class="w-3.5 h-3.5" /> 添加音色
            </button>
          </div>
          <div class="space-y-2">
            <div v-for="(v, i) in config.video.tts_voices" :key="i" class="flex items-center gap-2">
              <input v-model="v.name" type="text" placeholder="友好名称" class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white" />
              <input v-model="v.voice_id" type="text" placeholder="音色 ID" class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white font-mono" />
              <button @click="removeTtsVoice(i)" class="p-2 text-gray-600 hover:text-red-400 transition-colors"><Trash2 class="w-4 h-4" /></button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 3.5 视频生成引擎 (MoneyPrinterTurbo) -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <Film class="w-4 h-4 text-blue-500" />
        视频生成引擎 (素材拼接成片)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Pexels API Key</label>
          <input v-model="config.video.pexels_api_keys" type="password"
                 placeholder="在线素材库检索用，多个可用英文逗号分隔；本地素材模式无需填写"
                 class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 font-mono text-sm" />
          <p class="text-[11px] text-gray-600 mt-1.5">免费申请：https://www.pexels.com/api/ —— 用于按关键词下载免版权高清视频素材。</p>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">默认配音音色 (Edge TTS)</label>
            <select v-model="config.video.mpt_voice_name"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all">
              <option value="zh-CN-XiaoxiaoNeural-Female">晓晓（女·温柔）</option>
              <option value="zh-CN-XiaoyiNeural-Female">晓伊（女·亲和）</option>
              <option value="zh-CN-YunxiNeural-Male">云希（男·阳光）</option>
              <option value="zh-CN-YunjianNeural-Male">云健（男·浑厚）</option>
              <option value="zh-CN-YunyangNeural-Male">云扬（男·专业）</option>
              <option value="en-US-AvaNeural-Female">Ava（英·女）</option>
              <option value="en-US-AndrewNeural-Male">Andrew（英·男）</option>
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">字幕生成方式</label>
            <select v-model="config.video.mpt_subtitle_provider"
                    class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all">
              <option value="edge">Edge（快速·免费·默认）</option>
              <option value="whisper">Whisper（更精准·需下模型）</option>
            </select>
          </div>
        </div>
        <p class="text-[11px] text-gray-600 -mt-2">Edge TTS 免费、无需 API Key。Whisper 字幕更精准，但首次使用会下载模型（约 250MB+），需在 requirements.txt 启用 faster-whisper 并重跑依赖安装。</p>
      </div>
    </div>

    <!-- 4. 多模型 GEO 监控节点 -->
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
          添加节点
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
              <input v-model="model.model_id" list="relay-models" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white font-mono" />
            </div>
          </div>

          <div class="grid grid-cols-1 gap-4 pt-2 border-t border-gray-900">
            <div>
              <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">Base URL</label>
              <input v-model="model.base_url" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-gray-300" />
            </div>
            <div>
              <label class="block text-[10px] text-gray-500 uppercase mb-1 font-bold">API Key</label>
              <input v-model="model.api_key" type="password" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2 text-xs text-white font-mono" />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 5. 知识库与嵌入 (Embedding) -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl opacity-80 hover:opacity-100 transition-opacity">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <Wand2 class="w-4 h-4 text-amber-500" />
        知识库与嵌入 (Embedding)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Embedding API Key (可选)</label>
          <input
            v-model="config.llm.kb_api_key"
            type="password"
            placeholder="留空则沿用主对话 Key"
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-amber-500 transition-all font-mono"
          />
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Base URL</label>
            <input v-model="config.llm.kb_base_url" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-amber-500 transition-all" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Model ID</label>
            <input v-model="config.llm.embedding_model" list="relay-models" type="text" placeholder="text-embedding-3-small" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-amber-500 transition-all" />
          </div>
        </div>
      </div>
    </div>

    <!-- 6. AI 助理行为 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl opacity-80 hover:opacity-100 transition-opacity">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <Wand2 class="w-4 h-4 text-purple-500" />
        AI 助理行为
      </h3>
      <label class="flex items-start justify-between gap-4 cursor-pointer select-none">
        <div>
          <div class="text-sm font-medium text-gray-300">动作执行后用 AI 总结结果</div>
          <p class="text-xs text-gray-500 mt-1 leading-relaxed">
            开启后，AI 助理确认执行采集 / 合成 / 删除等动作后，会额外调用一次大模型，把执行结果总结成自然语言反馈。
            体验更好，但每次执行会多消耗一次 API 配额。关闭则只展示结构化要点（默认）。
          </p>
        </div>
        <button
          type="button"
          @click="config.llm.ai_summarize_actions = !config.llm.ai_summarize_actions"
          :class="['relative inline-flex h-6 w-11 flex-shrink-0 items-center rounded-full transition-colors mt-1',
                   config.llm.ai_summarize_actions ? 'bg-purple-600' : 'bg-gray-700']"
        >
          <span :class="['inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                         config.llm.ai_summarize_actions ? 'translate-x-6' : 'translate-x-1']"></span>
        </button>
      </label>
    </div>

    <!-- 7. 语音转文字 (STT) -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl opacity-80 hover:opacity-100 transition-opacity">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <Mic class="w-4 h-4 text-green-500" />
        语音转文字 (Speech-to-Text)
      </h3>

      <div class="grid grid-cols-1 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">STT API Key</label>
          <input
            v-model="config.stt.api_key"
            type="password"
            placeholder="sk-..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-green-500 transition-all font-mono"
          />
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Base URL</label>
            <input v-model="config.stt.base_url" type="text" placeholder="https://api.openai.com/v1" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-green-500 transition-all" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Model ID</label>
            <input v-model="config.stt.model" list="relay-models" type="text" placeholder="whisper-1" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-green-500 transition-all" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
