<script setup lang="ts">
import { MessageSquare, FileText, Sparkles, Radio } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();
</script>

<template>
  <div class="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <!-- 1. 直播弹幕回复 (LIVE REPLY) -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl relative overflow-hidden">
      <div class="absolute top-0 right-0 p-8 opacity-5 pointer-events-none">
        <Radio class="w-32 h-32 text-red-500" />
      </div>

      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <MessageSquare class="w-4 h-4 text-red-500" />
        直播弹幕回复 (LIVE REPLY)
      </h3>

      <div class="grid grid-cols-1 gap-6 relative z-10">
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-[11px] font-bold text-gray-500 uppercase mb-2 tracking-wider">直播主题</label>
            <input
              v-model="config.llm.live_theme"
              type="text"
              placeholder="例如：数码产品测评"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-red-500 transition-all"
            />
          </div>
          <div>
            <label class="block text-[11px] font-bold text-gray-500 uppercase mb-2 tracking-wider">内容关键词</label>
            <input
              v-model="config.llm.live_content"
              type="text"
              placeholder="例如：手机、耳机、爆款参数"
              class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-red-500 transition-all"
            />
          </div>
        </div>

        <div>
          <label class="block text-[11px] font-bold text-gray-500 uppercase mb-2 tracking-wider">AI 自动回复提示词 (Prompt)</label>
          <textarea
            v-model="config.llm.live_reply_prompt"
            rows="5"
            placeholder="描述 AI 应该以什么语气、什么逻辑回复弹幕..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-red-500 transition-all resize-none leading-relaxed"
          ></textarea>
          <div class="mt-3 flex items-start gap-2 text-[11px] text-gray-500 italic">
            <Sparkles class="w-3.5 h-3.5 text-amber-500 flex-shrink-0 mt-0.5" />
            <p>该提示词将结合直播主题和知识库内容，为直播间的每一条弹幕生成针对性的回复建议。</p>
          </div>
        </div>
      </div>
    </div>

    <!-- 2. 脚本生成 · 系统提示词 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 space-y-6 shadow-xl relative overflow-hidden">
      <div class="absolute top-0 right-0 p-8 opacity-5 pointer-events-none">
        <FileText class="w-32 h-32 text-amber-500" />
      </div>

      <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-2">
        <MessageSquare class="w-4 h-4 text-amber-500" />
        脚本生成 · 系统提示词
      </h3>

      <div class="grid grid-cols-1 gap-6 relative z-10">
        <div>
          <p class="text-xs text-gray-500 mb-4 leading-relaxed">
            用于视频创作中心「生成脚本」时的核心创作准则。您可以定义脚本的受众风格、表达调性以及强制性的卖点植入规则。
          </p>
          <textarea
            v-model="config.video.script_system_prompt"
            rows="8"
            placeholder="留空使用默认准则（GEO 答案前置 / 事实密度 / 场景化）..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-amber-500 transition-all font-mono leading-relaxed resize-y"
          ></textarea>
          <div class="mt-4 p-4 bg-amber-500/5 border border-amber-500/10 rounded-xl">
            <h4 class="text-[10px] font-bold text-amber-500 uppercase mb-2">提示</h4>
            <ul class="text-[11px] text-gray-500 space-y-1 list-disc list-inside">
              <li>脚本将以固定 JSON 格式返回，包含文案、时长、关键词等。</li>
              <li>建议在这里专注于描述“品牌人设”和“语言风格”。</li>
              <li>系统会自动注入企业知识库中的相关事实。</li>
            </ul>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. 采集数据分析 (可选补充) -->
    <div class="bg-gray-900/20 border border-gray-800/50 rounded-2xl p-6 space-y-4 shadow-sm grayscale opacity-70 hover:grayscale-0 hover:opacity-100 transition-all">
       <h3 class="text-xs font-bold text-gray-500 uppercase tracking-widest flex items-center gap-2">
        数据分析提示词 (Analysis)
      </h3>
      <textarea
        v-model="config.llm.analysis_prompt"
        rows="3"
        class="w-full bg-gray-950/30 border border-gray-800 rounded-xl px-4 py-2 text-xs text-gray-400 focus:text-white transition-colors"
      ></textarea>
    </div>
  </div>
</template>
