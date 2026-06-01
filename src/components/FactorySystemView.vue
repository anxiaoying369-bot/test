<script setup lang="ts">
import { ref } from 'vue';
import { Factory, UsersRound, Boxes } from 'lucide-vue-next';
import FactoryPersonnel from './factory/FactoryPersonnel.vue';
import FactoryInventory from './factory/FactoryInventory.vue';

const TABS = [
  { id: 'personnel', name: '人员管理', icon: UsersRound },
  { id: 'inventory', name: '库存管理', icon: Boxes },
];

const activeTab = ref<'personnel' | 'inventory'>('personnel');
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100">
    <!-- 顶部：标题 + 子页面切换 tabbar -->
    <div class="px-8 pt-8 pb-4 border-b border-gray-900 bg-gray-950/50 backdrop-blur-xl sticky top-0 z-10">
      <div class="flex items-center gap-3 mb-6">
        <div class="p-2.5 bg-orange-600/10 rounded-xl border border-orange-500/20">
          <Factory class="w-5 h-5 text-orange-400" />
        </div>
        <div>
          <h2 class="text-2xl font-bold bg-gradient-to-r from-white to-gray-400 bg-clip-text text-transparent">工厂系统</h2>
          <p class="text-gray-500 text-sm mt-0.5">人员、库存等工厂运营模块的统一管理中心</p>
        </div>
      </div>

      <div class="flex gap-2">
        <button
          v-for="tab in TABS"
          :key="tab.id"
          @click="activeTab = tab.id as any"
          :class="[
            'flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-medium transition-all border',
            activeTab === tab.id
              ? 'bg-orange-600 border-orange-500 text-white shadow-lg shadow-orange-900/20'
              : 'bg-gray-900/50 border-gray-800 text-gray-400 hover:bg-gray-800 hover:text-gray-200'
          ]"
        >
          <component :is="tab.icon" class="w-4 h-4" />
          {{ tab.name }}
        </button>
      </div>
    </div>

    <!-- 内容区域：子页面 -->
    <div class="flex-1 overflow-y-auto">
      <FactoryPersonnel v-if="activeTab === 'personnel'" />
      <FactoryInventory v-else-if="activeTab === 'inventory'" />
    </div>
  </div>
</template>
