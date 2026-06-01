<script setup lang="ts">
import { ShieldCheck } from 'lucide-vue-next';
import { useAppConfig } from '../../composables/useAppConfig';

const { config } = useAppConfig();
</script>

<template>
  <div class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl p-8 space-y-8 shadow-xl">
      <div class="flex items-center justify-between p-4 bg-blue-600/5 border border-blue-500/10 rounded-xl">
        <div class="flex items-center gap-4">
          <div class="p-2 bg-blue-600 rounded-lg">
            <ShieldCheck class="w-5 h-5 text-white" />
          </div>
          <div>
            <h4 class="font-bold">Hermes 安全网关</h4>
            <p class="text-xs text-gray-500 mt-0.5">多模型统一接入、Tool 调用管控及合规审计</p>
          </div>
        </div>
        <div class="flex items-center gap-3">
          <span :class="['text-[10px] px-2 py-0.5 rounded-full font-bold uppercase', config.hermes.enabled ? 'bg-green-500/10 text-green-500' : 'bg-gray-800 text-gray-500']">
            {{ config.hermes.enabled ? 'Active' : 'Disabled' }}
          </span>
          <button
            @click="config.hermes.enabled = !config.hermes.enabled"
            :class="['w-12 h-6 rounded-full relative transition-all duration-300', config.hermes.enabled ? 'bg-blue-600' : 'bg-gray-800']"
          >
            <div :class="['absolute top-1 w-4 h-4 bg-white rounded-full transition-all duration-300 shadow-sm', config.hermes.enabled ? 'left-7' : 'left-1']"></div>
          </button>
        </div>
      </div>

      <div class="space-y-6" :class="{ 'opacity-50 pointer-events-none': !config.hermes.enabled }">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">网关地址 (Gateway URL)</label>
          <input
            v-model="config.hermes.gateway_url"
            type="text"
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Hermes API Key</label>
          <input
            v-model="config.hermes.api_key"
            type="password"
            placeholder="hms-..."
            class="w-full bg-gray-950 border border-gray-800 rounded-xl px-4 py-3 text-white focus:outline-none focus:border-blue-500 transition-all"
          />
        </div>
      </div>
    </div>
  </div>
</template>
