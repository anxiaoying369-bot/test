<script setup lang="ts">
import { Heart, Gift, Users, Check, Wand2, Cpu } from 'lucide-vue-next';
import type { LiveMessage } from '../../types/live-monitor';
import { renderDouyinText } from '../../lib/utils';

defineProps<{
  msg: { type: string; payload: LiveMessage };
  copiedId: string | null;
  aiReply?: { content: string; loading: boolean };
}>();

const emit = defineEmits<{
  (e: 'copy', userId: string): void;
  (e: 'generateReply'): void;
}>();
</script>

<template>
  <div class="animate-in fade-in slide-in-from-bottom-1 duration-300">
    <!-- 聊天消息 -->
    <div v-if="msg.type === 'chat' || msg.type === 'emoji'" class="flex gap-3">
      <div class="flex-shrink-0 mt-1">
        <div class="w-2 h-2 rounded-full bg-blue-500 mt-1.5"></div>
      </div>
      <div class="flex-1 min-w-0">
        <div class="flex items-baseline gap-2 mb-0.5">
          <span class="text-xs font-bold text-blue-400">
            {{ msg.payload.user_name }}
            <span @click="emit('copy', msg.payload.user_id)"
              class="text-[10px] opacity-60 font-mono ml-0.5 cursor-pointer hover:opacity-100 hover:text-white transition-all bg-gray-800/50 px-1 rounded flex-inline items-center gap-1">
              ({{ msg.payload.user_id }})
              <Check v-if="copiedId === msg.payload.user_id" class="w-2.5 h-2.5 inline text-green-400" />
            </span>
          </span>
          <span class="text-[10px] text-gray-600">{{ msg.payload.time }}</span>
        </div>
        <div 
          class="text-sm text-gray-300 leading-relaxed bg-gray-900/50 p-2 rounded-lg border border-gray-800 inline-block relative group/msg max-w-[85%] break-words"
        >
          <div v-html="renderDouyinText(msg.payload.content || '')"></div>

          <!-- AI 回复按钮 -->
          <button
            @click="emit('generateReply')"
            class="absolute -right-10 top-0 p-1.5 bg-gray-900 border border-gray-800 rounded-lg text-gray-500 hover:text-blue-400 opacity-0 group-hover/msg:opacity-100 transition-all shadow-xl"
            title="AI 生成回复建议"
          >
            <Wand2 class="w-3.5 h-3.5" />
          </button>
        </div>

        <!-- AI 生成的内容展示 -->
        <div v-if="aiReply"
          class="mt-1.5 bg-blue-600/10 border border-blue-500/20 rounded-lg p-2 max-w-sm animate-in fade-in slide-in-from-top-1">
          <div class="flex items-center gap-1.5 mb-1">
            <Cpu class="w-3 h-3 text-blue-400" />
            <span class="text-[10px] font-bold text-blue-400 uppercase tracking-tighter">AI 建议回复</span>
            <div v-if="aiReply.loading" class="flex gap-1 ml-1">
              <div class="w-1 h-1 bg-blue-500 rounded-full animate-bounce"></div>
              <div class="w-1 h-1 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 0.2s"></div>
            </div>
          </div>
          <p v-if="!aiReply.loading" class="text-xs text-gray-200 leading-relaxed italic">
            “{{ aiReply.content }}”
          </p>
        </div>
      </div>
    </div>

    <!-- 礼物消息 -->
    <div v-else-if="msg.type === 'gift'" class="flex gap-3">
      <div class="flex-shrink-0 mt-1">
        <div class="w-2 h-2 rounded-full bg-purple-500 mt-1.5"></div>
      </div>
      <div class="flex-1 min-w-0">
        <div class="flex items-baseline gap-2 mb-0.5">
          <span class="text-xs font-bold text-purple-400">
            {{ msg.payload.user_name }}
            <span @click="emit('copy', msg.payload.user_id)"
              class="text-[10px] opacity-60 font-mono ml-0.5 cursor-pointer hover:opacity-100 hover:text-white transition-all bg-gray-800/50 px-1 rounded flex-inline items-center gap-1">
              ({{ msg.payload.user_id }})
              <Check v-if="copiedId === msg.payload.user_id" class="w-2.5 h-2.5 inline text-green-400" />
            </span>
          </span>
          <span class="text-[10px] text-gray-600">{{ msg.payload.time }}</span>
        </div>
        <div class="text-sm bg-purple-500/10 border border-purple-500/30 text-purple-200 px-3 py-1.5 rounded-lg flex items-center gap-2">
          <Gift class="w-4 h-4 text-purple-400" />
          送出 <span class="font-bold">{{ msg.payload.gift_name }}</span> x{{ msg.payload.gift_count }}
        </div>
      </div>
    </div>

    <!-- 点赞消息 -->
    <div v-else-if="msg.type === 'like'" class="flex gap-3">
      <div class="flex-shrink-0 mt-1 text-red-500">
        <Heart class="w-4 h-4" />
      </div>
      <div class="flex-1 min-w-0">
        <div class="text-xs">
          <span class="font-bold text-red-400">
            {{ msg.payload.user_name }}
            <span @click="emit('copy', msg.payload.user_id)"
              class="text-[10px] opacity-60 font-mono ml-0.5 cursor-pointer hover:opacity-100 hover:text-white transition-all bg-gray-800/50 px-1 rounded flex-inline items-center gap-1">
              ({{ msg.payload.user_id }})
              <Check v-if="copiedId === msg.payload.user_id" class="w-2.5 h-2.5 inline text-green-400" />
            </span>
          </span>
          <span class="text-gray-500"> 连点 {{ msg.payload.count }} 个赞</span>
          <span class="text-[10px] text-gray-600 ml-2">{{ msg.payload.time }}</span>
        </div>
      </div>
    </div>

    <!-- 进场消息 -->
    <div v-else-if="msg.type === 'member'" class="flex gap-3">
      <div class="flex-shrink-0 mt-1 text-gray-600">
        <Users class="w-4 h-4" />
      </div>
      <div class="flex-1 min-w-0">
        <div class="text-xs italic text-gray-500">
          {{ msg.payload.user_name }}
          <span @click="emit('copy', msg.payload.user_id)"
            class="text-[10px] opacity-60 font-mono cursor-pointer hover:opacity-100 hover:text-white transition-all bg-gray-800/50 px-1 rounded flex-inline items-center gap-1 not-italic">
            ({{ msg.payload.user_id }})
            <Check v-if="copiedId === msg.payload.user_id" class="w-2.5 h-2.5 inline text-green-400" />
          </span>
          来了
          <span class="text-[10px] text-gray-700 ml-2">{{ msg.payload.time }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
