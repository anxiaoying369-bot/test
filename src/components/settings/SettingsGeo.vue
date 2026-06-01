<script setup lang="ts">
import { Globe, FileText, XCircle } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();

// --- GEO 监控节点辅助 ---
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
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
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
</template>
