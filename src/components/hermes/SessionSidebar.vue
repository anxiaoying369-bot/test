<script setup lang="ts">
import { Plus, Search, MessageSquare, Trash2 } from 'lucide-vue-next';
import type { Session } from '../../types/hermes';

defineProps<{
  sessions: Session[];
  currentSessionId: string | null;
  searchQuery: string;
}>();

const emit = defineEmits<{
  (e: 'create'): void;
  (e: 'select', id: string): void;
  (e: 'delete', id: string): void;
  (e: 'update:searchQuery', val: string): void;
}>();
</script>

<template>
  <div class="w-80 bg-gray-900 border-r border-gray-800 flex flex-col overflow-hidden">
    <div class="p-6 space-y-4">
      <button
        @click="emit('create')"
        class="w-full bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 rounded-xl flex items-center justify-center gap-2 transition-all shadow-lg shadow-blue-900/20"
      >
        <Plus class="w-5 h-5" />
        新建会话
      </button>

      <div class="relative group">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500 group-focus-within:text-blue-400 transition-colors" />
        <input
          :value="searchQuery"
          @input="e => emit('update:searchQuery', (e.target as HTMLInputElement).value)"
          type="text"
          placeholder="搜索会话..."
          class="w-full bg-gray-950 border border-gray-800 rounded-xl py-2.5 pl-10 pr-4 text-sm text-gray-200 focus:outline-none focus:border-blue-500 transition-all placeholder-gray-600"
        />
      </div>
    </div>

    <div class="flex-1 overflow-y-auto custom-scrollbar px-3 pb-6 space-y-1">
      <div
        v-for="s in sessions"
        :key="s.id"
        @click="emit('select', s.id)"
        :class="[
          'p-3.5 rounded-xl cursor-pointer transition-all border group relative',
          currentSessionId === s.id
            ? 'bg-blue-600/10 border-blue-500/50 shadow-inner'
            : 'hover:bg-gray-800 border-transparent text-gray-400'
        ]"
      >
        <div class="flex items-center gap-3">
          <div :class="['w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors', currentSessionId === s.id ? 'bg-blue-600/20 text-blue-400' : 'bg-gray-800 text-gray-600']">
            <MessageSquare class="w-4 h-4" />
          </div>
          <div class="flex-1 min-w-0">
            <div :class="['text-sm font-medium truncate mb-0.5', currentSessionId === s.id ? 'text-white' : 'group-hover:text-gray-200']">
              {{ s.title }}
            </div>
            <div class="text-[10px] text-gray-600 font-mono">
              {{ s.messages.length }} 条消息 · {{ new Date(s.updatedAt).toLocaleTimeString() }}
            </div>
          </div>
          <button
            @click.stop="emit('delete', s.id)"
            class="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/20 hover:text-red-500 rounded-lg transition-all"
            title="删除会话"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
