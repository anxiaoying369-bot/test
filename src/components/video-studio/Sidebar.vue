<script setup lang="ts">
import { Film, Plus, Trash2 } from 'lucide-vue-next';
import type { VideoProject } from '../../types/video-studio';

defineProps<{
  projects: VideoProject[];
  currentProject: VideoProject | null;
}>();

const emit = defineEmits<{
  (e: 'create'): void;
  (e: 'select', p: VideoProject): void;
  (e: 'delete', id: string): void;
}>();

const handleCreate = () => emit('create');
const handleSelect = (p: VideoProject) => emit('select', p);
const handleDelete = (id: string, e: Event) => {
  e.stopPropagation();
  emit('delete', id);
};
</script>

<template>
  <div class="w-72 bg-gray-900 border-r border-gray-800 flex flex-col">
    <div class="p-6">
      <button @click="handleCreate" class="w-full bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 rounded-xl flex items-center justify-center gap-2 transition-all shadow-lg shadow-blue-900/20">
        <Plus class="w-5 h-5" />
        新建创作项目
      </button>
    </div>
    
    <div class="flex-1 overflow-y-auto custom-scrollbar px-4 pb-6 space-y-2">
      <div 
        v-for="p in projects" 
        :key="p.id"
        @click="handleSelect(p)"
        :class="['p-4 rounded-xl cursor-pointer transition-all border group relative', currentProject?.id === p.id ? 'bg-blue-600/10 border-blue-500/50 shadow-inner' : 'hover:bg-gray-800 border-transparent text-gray-400']"
      >
        <div class="flex items-center gap-2">
          <Film :class="['w-4 h-4', currentProject?.id === p.id ? 'text-blue-400' : 'text-gray-600']" />
          <div class="flex-1 truncate text-sm font-medium">{{ p.title }}</div>
          <button
            @click="handleDelete(p.id, $event)"
            class="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/20 hover:text-red-500 rounded-lg transition-all"
            title="删除项目"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
