<script setup lang="ts">
import { MessageSquare, Wand2, Mic, Cpu, Image as ImageIcon, Globe, Plus, Trash2, XCircle, Film } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();

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
              type="text"
              placeholder="gpt-4o"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-blue-500 transition-all"
            />
          </div>
        </div>
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
            <input v-model="config.video.tts_model" type="text"
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
            <input v-model="config.llm.embedding_model" type="text" placeholder="text-embedding-3-small" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-amber-500 transition-all" />
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
            <input v-model="config.stt.model" type="text" placeholder="whisper-1" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-green-500 transition-all" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
