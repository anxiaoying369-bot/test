<script setup lang="ts">
import { MessageSquare, Wand2 } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();
</script>

<template>
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
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
</template>
