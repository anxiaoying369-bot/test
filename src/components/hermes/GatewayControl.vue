<script setup lang="ts">
import { 
  Zap, Loader2, RefreshCw, Square, Play 
} from 'lucide-vue-next';

defineProps<{
  isConnected: boolean;
  isChecking: boolean;
  isStarting: boolean;
  isStopping: boolean;
  isEnablingApi: boolean;
  gatewayUrl: string;
  apiKey: string;
}>();

const emit = defineEmits<{
  (e: 'update:gatewayUrl', val: string): void;
  (e: 'update:apiKey', val: string): void;
  (e: 'checkHealth'): void;
  (e: 'start'): void;
  (e: 'stop'): void;
  (e: 'enableApi'): void;
}>();
</script>

<template>
  <div class="h-16 px-6 bg-gray-900 border-b border-gray-800 flex items-center justify-between">
    <div class="flex items-center gap-4">
      <div :class="['flex items-center gap-2 px-3 py-1.5 rounded-full text-[10px] font-bold uppercase tracking-widest border transition-all', isConnected ? 'bg-green-500/10 border-green-500/30 text-green-400' : 'bg-red-500/10 border-red-500/30 text-red-400']">
        <div :class="['w-2 h-2 rounded-full animate-pulse', isConnected ? 'bg-green-500' : 'bg-red-500']" />
        {{ isConnected ? 'Gateway Connected' : 'Gateway Offline' }}
      </div>
      
      <div class="flex items-center gap-1">
        <button @click="emit('checkHealth')" :disabled="isChecking" class="p-2 text-gray-500 hover:text-white transition-colors" title="刷新状态">
          <RefreshCw :class="['w-4 h-4', isChecking ? 'animate-spin' : '']" />
        </button>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <div class="flex bg-gray-950 border border-gray-800 rounded-xl px-3 py-1.5 gap-3">
        <div class="flex items-center gap-2 border-r border-gray-800 pr-3">
          <span class="text-[10px] text-gray-600 font-bold uppercase">Gateway</span>
          <input
            :value="gatewayUrl"
            @input="e => emit('update:gatewayUrl', (e.target as HTMLInputElement).value)"
            type="text"
            class="bg-transparent border-none text-xs text-gray-300 focus:outline-none w-48"
          />
        </div>
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-gray-600 font-bold uppercase">API Key</span>
          <input
            :value="apiKey"
            @input="e => emit('update:apiKey', (e.target as HTMLInputElement).value)"
            type="password"
            placeholder="sk-..."
            class="bg-transparent border-none text-xs text-gray-300 focus:outline-none w-32"
          />
        </div>
      </div>

      <button
        v-if="!isConnected"
        @click="emit('start')"
        :disabled="isStarting"
        class="bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white px-4 py-2 rounded-xl text-xs font-bold flex items-center gap-2 transition-all shadow-lg shadow-blue-900/20"
      >
        <Loader2 v-if="isStarting" class="w-3.5 h-3.5 animate-spin" />
        <Play v-else class="w-3.5 h-3.5" />
        启动网关
      </button>
      <button
        v-else
        @click="emit('stop')"
        :disabled="isStopping"
        class="bg-gray-800 hover:bg-red-600 text-gray-300 hover:text-white px-4 py-2 rounded-xl text-xs font-bold flex items-center gap-2 transition-all border border-gray-700 hover:border-red-500"
      >
        <Loader2 v-if="isStopping" class="w-3.5 h-3.5 animate-spin" />
        <Square v-else class="w-3.5 h-3.5" />
        停止网关
      </button>

      <button
        v-if="isConnected && !apiKey"
        @click="emit('enableApi')"
        :disabled="isEnablingApi"
        class="bg-amber-600 hover:bg-amber-500 disabled:opacity-50 text-white px-4 py-2 rounded-xl text-xs font-bold flex items-center gap-2 transition-all"
      >
        <Zap v-if="!isEnablingApi" class="w-3.5 h-3.5" />
        <Loader2 v-else class="w-3.5 h-3.5 animate-spin" />
        一键开启 API
      </button>
    </div>
  </div>
</template>
