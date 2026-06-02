<script setup lang="ts">
import { FileText, XCircle } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();

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
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-6 shadow-xl">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-sm font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2">
          <FileText class="w-4 h-4 text-purple-500" />
          GEO 内容发布平台
        </h3>
        <button
          @click="addGeoPublishPlatform"
          class="bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-bold px-4 py-2 rounded-lg transition-all shadow-lg shadow-purple-900/20"
        >
          添加平台
        </button>
      </div>

      <div class="space-y-6">
        <div
          v-for="(platform, idx) in config.llm.geo_publish_platforms"
          :key="idx"
          class="p-6 bg-gray-950 border border-gray-800 rounded-xl space-y-4 group relative"
        >
          <button
            @click="removeGeoPublishPlatform(idx)"
            class="absolute top-4 right-4 text-gray-600 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all p-2 rounded-lg hover:bg-red-500/5"
          >
            <XCircle class="w-4 h-4" />
          </button>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-[10px] text-gray-500 uppercase mb-1.5 font-bold tracking-wider">平台名称</label>
              <input v-model="platform.name" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2.5 text-xs text-white focus:outline-none focus:border-purple-500" />
            </div>
            <div>
              <label class="block text-[10px] text-gray-500 uppercase mb-1.5 font-bold tracking-wider">基础 URL (可选)</label>
              <input v-model="platform.url" type="text" class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2.5 text-xs text-white font-mono focus:outline-none focus:border-purple-500" />
            </div>
          </div>

          <div>
            <label class="block text-[10px] text-gray-500 uppercase mb-1.5 font-bold tracking-wider">平台描述 / 规则说明</label>
            <input v-model="platform.description" type="text" placeholder="描述此平台的受众、调性或发布限制..." class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2.5 text-xs text-gray-300 focus:outline-none focus:border-purple-500" />
          </div>

          <div>
            <label class="block text-[10px] text-gray-500 uppercase mb-1.5 font-bold tracking-wider">针对该平台的 System Prompt</label>
            <textarea
              v-model="platform.system_prompt"
              rows="4"
              class="w-full bg-gray-900 border border-gray-800 rounded-lg px-3 py-2.5 text-xs text-gray-400 focus:text-white focus:outline-none focus:border-purple-500 transition-colors leading-relaxed"
            ></textarea>
          </div>
        </div>

        <div v-if="!config.llm.geo_publish_platforms || config.llm.geo_publish_platforms.length === 0" class="text-center py-16 border-2 border-dashed border-gray-900 rounded-3xl">
          <FileText class="w-12 h-12 text-gray-800 mx-auto mb-4 opacity-50" />
          <p class="text-sm text-gray-600">暂无发布平台，点击右上方“添加平台”开始配置</p>
        </div>
      </div>
    </div>
  </div>
</template>
