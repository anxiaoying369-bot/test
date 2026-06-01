<script setup lang="ts">
import { ref, onMounted, inject, watch } from 'vue';
import { Save, RefreshCw, CheckCircle, XCircle, ShieldCheck, Globe, Cpu, Video, RotateCcw } from 'lucide-vue-next';
import { useAppConfig } from '../composables/useAppConfig';
import SettingsLLM from './settings/SettingsLLM.vue';
import SettingsVideo from './settings/SettingsVideo.vue';
import SettingsHermes from './settings/SettingsHermes.vue';
import SettingsGeo from './settings/SettingsGeo.vue';

const { loadConfig, saveConfig, resetConfig } = useAppConfig();

const TABS = [
  { id: 'llm', name: '语言模型', icon: Cpu },
  { id: 'video', name: '视频生成', icon: Video },
  { id: 'hermes', name: 'Hermes 助手', icon: ShieldCheck },
  { id: 'geo', name: 'GEO 监控', icon: Globe },
];
const VALID_TABS = TABS.map(t => t.id);
const normalizeTab = (t?: string) => (t && VALID_TABS.includes(t) ? t : 'llm');

const settingsInitialTab = inject<ReturnType<typeof ref<string>>>('settingsInitialTab');
// 始终落到一个有效标签，避免传入未知值时页面空白、无任何标签选中
const activeTab = ref(normalizeTab(settingsInitialTab?.value));
const isSaving = ref(false);
const saveStatus = ref<'idle' | 'success' | 'error'>('idle');
const statusMsg = ref('');

// 监听 inject 的变化 (如果父组件改变了 tab)
watch(() => settingsInitialTab?.value, (newTab) => {
  if (newTab) activeTab.value = normalizeTab(newTab);
});

onMounted(loadConfig);

const handleSave = async () => {
  isSaving.value = true;
  saveStatus.value = 'idle';
  try {
    await saveConfig();
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
      await resetConfig();
    } catch (err) {
      alert('恢复默认失败: ' + err);
    }
  }
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
          v-for="tab in TABS"
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
        <SettingsLLM v-if="activeTab === 'llm'" />
        <SettingsVideo v-else-if="activeTab === 'video'" />
        <SettingsHermes v-else-if="activeTab === 'hermes'" />
        <SettingsGeo v-else-if="activeTab === 'geo'" />

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
