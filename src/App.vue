<script setup lang="ts">
import { ref } from 'vue';
import { Settings, MessageSquare, Smartphone, Send, Bot, TerminalSquare } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';

const inputQuery = ref('');
const chatHistory = ref([
  { role: 'assistant', content: '您好，我是您的 AutoCast AI 助手。您可以直接吩咐我执行自动化任务，比如：“帮我在小红书上查找抹茶软欧包教程，制作成脚本”。' }
]);
const logs = ref<string[]>([]);
const isProcessing = ref(false);

async function handleSend() {
  const query = inputQuery.value.trim();
  if (!query) return;

  chatHistory.value.push({ role: 'user', content: query });
  inputQuery.value = '';
  isProcessing.value = true;

  // 简单模拟意图识别
  if (query.includes('小红书') && (query.includes('查找') || query.includes('搜索') || query.includes('找') || query.includes('脚本'))) {
    chatHistory.value.push({ role: 'assistant', content: '好的，正在为您启动小红书自动化检索机制，提取相关内容与素材...' });
    
    logs.value.push(`[${new Date().toLocaleTimeString()}] [INTENT] Detected intent: Xiaohongshu content search`);
    logs.value.push(`[${new Date().toLocaleTimeString()}] [INFO] Starting CloakBrowser...`);
    
    try {
      const result = await invoke('spy_xiaohongshu', { keyword: query });
      logs.value.push(`[${new Date().toLocaleTimeString()}] [SUCCESS] Task completed: ${result}`);
      chatHistory.value.push({ role: 'assistant', content: '✅ 检索完成！已成功抓取小红书热门内容。配方、素材与评论已提取完毕，您可以随时在右侧日志中查看细节，或者让我继续为您生成最终脚本。' });
    } catch (error) {
      logs.value.push(`[${new Date().toLocaleTimeString()}] [ERROR] Task failed: ${error}`);
      chatHistory.value.push({ role: 'assistant', content: `❌ 检索过程中遇到错误：${error}` });
    }
  } else {
    setTimeout(() => {
      chatHistory.value.push({ role: 'assistant', content: '我明白了。如果您需要查找素材，可以尝试对我说：“帮我在小红书上查找相关内容”。' });
    }, 800);
  }
  isProcessing.value = false;
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
}
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-gray-950 text-gray-50 font-sans selection:bg-blue-600/30">
    <!-- 1. 左侧栏：全局导航与设备状态 -->
    <aside class="flex flex-col w-[20%] h-full bg-gray-950 border-r border-gray-800">
      <!-- 顶部 Logo -->
      <div class="p-6 flex items-center justify-between">
        <h1 class="text-xl font-bold tracking-tight">AutoCast AI</h1>
        <div class="flex items-center gap-2 text-xs text-emerald-500">
          <span class="relative flex h-2 w-2">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
          </span>
          Online
        </div>
      </div>

      <!-- 中部导航区 -->
      <nav class="flex-1 px-3 space-y-1">
        <a href="#" class="flex items-center gap-3 px-3 py-2.5 rounded-lg bg-gray-900 text-gray-50 relative group">
          <div class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-blue-600 rounded-r-md"></div>
          <MessageSquare class="w-5 h-5 text-blue-500" />
          <span class="font-medium text-sm">AI 助理对话</span>
        </a>
        <a href="#" class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-gray-50 hover:bg-gray-900/50 transition-colors">
          <TerminalSquare class="w-5 h-5" />
          <span class="font-medium text-sm">自动化任务</span>
        </a>
      </nav>

      <!-- 底部系统设置与设备状态 -->
      <div class="p-4 space-y-4">
        <a href="#" class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-gray-50 hover:bg-gray-900/50 transition-colors">
          <Settings class="w-5 h-5" />
          <span class="font-medium text-sm">⚙️ 系统全局设置</span>
        </a>
        
        <div class="bg-gray-900 rounded-xl p-3 border border-gray-800">
          <div class="flex items-center gap-2 mb-2">
            <Smartphone class="w-4 h-4 text-gray-400" />
            <span class="text-xs font-medium text-gray-300">📱 本地仿真器 #1</span>
          </div>
          <div class="text-[10px] text-gray-500 font-mono flex items-center justify-between">
            <span>IP: 127.0.0.1:5555</span>
            <span class="text-emerald-500 flex items-center gap-1"><div class="w-1.5 h-1.5 rounded-full bg-emerald-500"></div> Connected</span>
          </div>
        </div>
      </div>
    </aside>

    <!-- 2. 中间栏：LLM 聊天交互区 -->
    <main class="flex flex-col w-[40%] h-full bg-gray-950 border-r border-gray-800">
      <!-- 聊天标题 -->
      <div class="p-4 border-b border-gray-800 flex items-center justify-center shadow-sm z-10">
        <h2 class="text-sm font-medium text-gray-300 flex items-center gap-2">
          <Bot class="w-4 h-4 text-emerald-500" />
          智能体交互中心
        </h2>
      </div>

      <!-- 聊天内容区 -->
      <div class="flex-1 overflow-y-auto p-4 space-y-4 flex flex-col">
        <div v-for="(msg, index) in chatHistory" :key="index" 
             :class="['max-w-[85%] rounded-xl p-3 text-sm leading-relaxed', 
                      msg.role === 'user' ? 'bg-blue-600 text-white self-end rounded-br-none' : 'bg-gray-900 border border-gray-800 text-gray-200 self-start rounded-bl-none']">
          {{ msg.content }}
        </div>
        
        <div v-if="isProcessing" class="self-start bg-gray-900 border border-gray-800 rounded-xl rounded-bl-none p-3 max-w-[85%]">
          <div class="flex gap-1 items-center">
            <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce"></div>
            <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
            <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
          </div>
        </div>
      </div>

      <!-- 底部输入区 -->
      <div class="p-4 border-t border-gray-800 bg-gray-950">
        <div class="relative flex items-end gap-2 bg-gray-900 border border-gray-800 rounded-xl p-1 focus-within:ring-1 focus-within:ring-blue-600 focus-within:border-blue-600 transition-all">
          <textarea 
            v-model="inputQuery"
            @keydown="handleKeydown"
            placeholder="输入指令，如：帮我在小红书上查找相关内容..." 
            class="w-full bg-transparent border-none px-3 py-2 text-sm text-gray-50 placeholder-gray-500 focus:outline-none resize-none min-h-[44px] max-h-32"
            rows="1"
          ></textarea>
          <button 
            @click="handleSend"
            :disabled="!inputQuery.trim() || isProcessing"
            class="mb-1 mr-1 p-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Send class="w-4 h-4" />
          </button>
        </div>
        <div class="text-center mt-2 text-[11px] text-gray-500">AutoCast AI 可自动唤起工具执行您的指令</div>
      </div>
    </main>

    <!-- 3. 右侧栏：Agent 脑核控制台与本地日志 -->
    <aside class="flex flex-col w-[40%] h-full bg-black font-mono">
      <!-- 顶部状态栏 -->
      <div class="p-3 border-b border-gray-800 bg-gray-950 flex items-center justify-between">
        <div class="text-xs text-gray-400">
          当前大脑: <span class="text-emerald-500 font-bold">[ 🎬 导演/剪辑智能体 ]</span>
        </div>
        <div class="flex gap-1">
          <div class="w-3 h-3 rounded-full bg-gray-800"></div>
          <div class="w-3 h-3 rounded-full bg-gray-800"></div>
          <div class="w-3 h-3 rounded-full bg-gray-800"></div>
        </div>
      </div>

      <!-- 滚动日志区 -->
      <div class="flex-1 overflow-y-auto p-4 text-[13px] leading-relaxed space-y-1">
        <div v-for="log in logs" :key="log" class="text-gray-300">
          {{ log }}
        </div>
        <!-- Blinking cursor -->
        <div class="inline-block w-2 h-4 bg-gray-500 animate-pulse mt-2"></div>
      </div>
    </aside>
  </div>
</template>

<style>
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #374151;
  border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
  background: #4B5563;
}
</style>
