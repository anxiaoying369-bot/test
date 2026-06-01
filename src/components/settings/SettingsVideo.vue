<script setup lang="ts">
import { MessageSquare, Video, Globe, Cpu, Plus, Trash2 } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();

// ── 音色组增删 ──
function addTtsVoice() {
  if (!config.value.video.tts_voices) config.value.video.tts_voices = [];
  config.value.video.tts_voices.push({ voice_id: '', name: '' });
}
function removeTtsVoice(index: number) {
  config.value.video.tts_voices?.splice(index, 1);
}
</script>

<template>
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <!-- ── 脚本生成系统提示词 ── -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-3 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <MessageSquare class="w-4 h-4 text-amber-500" />
        脚本生成 · 系统提示词
      </h3>
      <p class="text-xs text-gray-500">
        视频创作中心「生成脚本」时使用的系统提示词，控制脚本风格与结构。脚本生成页不显示此设置。
        留空则使用内置默认提示词。
      </p>
      <textarea
        v-model="config.video.script_system_prompt"
        rows="6"
        placeholder="留空使用内置默认提示词（GEO 答案前置 / 事实密度 / 场景化）"
        class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-amber-500 transition-all leading-relaxed resize-y font-mono"
      ></textarea>
      <p class="text-[11px] text-gray-600">
        提示：脚本会以固定 JSON 格式返回（标题/时长/语速/受众/口播文案/卖点关键词/素材关键词），
        JSON 结构由系统强制约束，你只需在这里描述"风格与创作准则"即可。
      </p>
    </div>

    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl">
      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <Video class="w-4 h-4 text-blue-500" />
        图片 / 视频生成服务
      </h3>

      <!-- 图片生成（OpenAI 兼容协议） -->
      <div>
        <label class="block text-sm font-medium text-blue-400 mb-1.5 flex items-center gap-2">
          <Globe class="w-4 h-4" />
          图片生成 · OpenAI 兼容协议 (自定义服务商)
        </label>
        <p class="text-xs text-gray-500 mb-4">
          素材库「AI 生图」选择 “OpenAI 兼容协议” 时使用的凭证。
        </p>

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

    <!-- ── TTS（语音合成）配置 ── -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-5 shadow-xl">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
          <Cpu class="w-4 h-4 text-purple-500" />
          语音合成（TTS）
        </h3>
        <span class="text-[10px] text-gray-600">用于「口播剧本」工作流</span>
      </div>

      <div>
        <label class="block text-sm font-medium text-gray-300 mb-2">TTS Provider</label>
        <select v-model="config.video.tts_provider"
                class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-purple-500 transition-all">
          <option value="mock">测试模拟（静音占位）</option>
          <option value="openai">OpenAI 兼容协议</option>
          <option value="minimax">MiniMax 语音合成</option>
          <option value="volcengine">火山引擎（待接入）</option>
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
                 :placeholder="config.video.tts_provider === 'minimax' ? 'http://pan.ptyxlm.com:3000/v1' : 'https://api.openai.com/v1'"
                 class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">模型</label>
          <input v-model="config.video.tts_model" type="text"
                 :placeholder="config.video.tts_provider === 'minimax' ? 'speech-2.8-hd' : 'tts-1'"
                 class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-purple-500" />
        </div>
      </div>

      <!-- ── 自定义音色组 ── -->
      <div class="border-t border-gray-800 pt-5">
        <div class="flex items-center justify-between mb-3">
          <div>
            <label class="block text-sm font-medium text-gray-300">音色列表</label>
            <p class="text-xs text-gray-500 mt-1">合成音频时从这里选择音色；「音色 ID」传给 TTS 接口，界面显示「音色名称」</p>
          </div>
          <button @click="addTtsVoice"
                  class="px-3 py-1.5 bg-purple-600/20 hover:bg-purple-600/40 text-purple-300 border border-purple-500/30 rounded-lg text-xs flex items-center gap-1.5 transition-colors">
            <Plus class="w-3.5 h-3.5" /> 添加音色
          </button>
        </div>

        <div v-if="!config.video.tts_voices || config.video.tts_voices.length === 0"
             class="text-xs text-gray-600 italic py-4 text-center border border-dashed border-gray-800 rounded-xl">
          还没有音色，点「添加音色」新增。视频创作中心合成口播时从这里选择。
        </div>

        <div v-else class="space-y-2">
          <div v-for="(v, i) in config.video.tts_voices" :key="i"
               class="flex items-center gap-2">
            <input v-model="v.name" type="text" placeholder="音色名称（如：温柔女声）"
                   class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-purple-500" />
            <input v-model="v.voice_id" type="text" placeholder="音色 ID（如：nova）"
                   class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-purple-500 font-mono" />
            <button @click="removeTtsVoice(i)"
                    class="p-2 text-gray-600 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors flex-shrink-0">
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
