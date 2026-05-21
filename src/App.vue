<script setup lang="ts">
import { ref } from 'vue';
import { 
  Settings, 
  MessageSquare, 
  Smartphone, 
  Send, 
  Bot, 
  TerminalSquare, 
  Users, 
  QrCode 
} from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';

const inputQuery = ref('');
const chatHistory = ref([
  { role: 'assistant', content: '您好，我是您的 AutoCast AI 助手。您可以直接吩咐我执行自动化任务，比如：“帮我在小红书上查找抹茶软欧包教程，制作成脚本”。' }
]);
const isProcessing = ref(false);
const showAccounts = ref(false);
const accounts = ref([
  { id: 1, platform: 'xiaohongshu', name: '默认小红书号', userId: 'xhs_001', status: 'active', avatar: '' },
  { id: 2, platform: 'douyin', name: '默认抖音号', userId: 'dy_001', status: 'expired', avatar: '' },
]);
const isLoginModalOpen = ref(false);
const currentLoginPlatform = ref('');
const qrcodeUrl = ref('');

async function handleSend() {
  const query = inputQuery.value.trim();
  if (!query) return;

  chatHistory.value.push({ role: 'user', content: query });
  inputQuery.value = '';
  isProcessing.value = true;

  // 简单模拟意图识别
  if (query.includes('小红书') && (query.includes('查找') || query.includes('搜索') || query.includes('找') || query.includes('脚本'))) {
    chatHistory.value.push({ role: 'assistant', content: '好的，正在为您启动小红书自动化检索机制，提取相关内容与素材...' });
    
    try {
      const result = await invoke('spy_xiaohongshu', { keyword: query });
      chatHistory.value.push({ role: 'assistant', content: '✅ 检索完成！已成功抓取小红书热门内容。配方、素材与评论已提取完毕，您可以让我继续为您生成最终脚本。' });
    } catch (error) {
      chatHistory.value.push({ role: 'assistant', content: `❌ 检索过程中遇到错误：${error}` });
    }
  } else {
    setTimeout(() => {
      chatHistory.value.push({ role: 'assistant', content: '我明白了。如果您需要查找素材，可以尝试对我说：“帮我在小红书上查找抹茶软欧包教程”。' });
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

function openLogin(platform: string) {
  currentLoginPlatform.value = platform;
  isLoginModalOpen.value = true;
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
        <a href="#" @click="showAccounts = false" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg relative group transition-colors cursor-pointer', !showAccounts ? 'bg-gray-900 text-gray-50' : 'text-gray-400 hover:text-gray-50 hover:bg-gray-900/50']">
          <div class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-blue-600 rounded-r-md"></div>
          <MessageSquare class="w-5 h-5 text-blue-500" />
          <span class="font-medium text-sm">AI 助理对话</span>
        </a>
        <a href="#" @click="showAccounts = true" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg relative group transition-colors cursor-pointer', showAccounts ? 'bg-gray-900 text-gray-50' : 'text-gray-400 hover:text-gray-50 hover:bg-gray-900/50']">
          <Users class="w-5 h-5" />
          <span class="font-medium text-sm">账号管理</span>
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

    <!-- 2. 主内容栏：LLM 聊天交互区 -->
    <main v-if="!showAccounts" class="flex flex-col flex-1 h-full bg-gray-950">
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

    <!-- 账号管理面板 -->
    <main v-if="showAccounts" class="flex flex-col flex-1 h-full bg-gray-950 p-6 overflow-y-auto">
      <div class="flex items-center justify-between mb-8">
        <h2 class="text-xl font-bold text-gray-50">账号管理</h2>
      </div>

      <div class="grid grid-cols-2 gap-6">
        <!-- 小红书平台 -->
        <div class="bg-gray-900 border border-gray-800 rounded-xl p-5">
          <div class="flex items-center justify-between mb-4 border-b border-gray-800 pb-4">
            <h3 class="text-lg font-medium text-gray-50 flex items-center gap-2">
              <span class="text-red-500">📕</span> 小红书
            </h3>
            <button @click="openLogin('xiaohongshu')" class="text-xs bg-blue-600 hover:bg-blue-700 text-white px-3 py-1.5 rounded-lg transition-colors cursor-pointer">
              + 新增授权
            </button>
          </div>
          <div class="space-y-3">
            <div v-for="acc in accounts.filter(a => a.platform === 'xiaohongshu')" :key="acc.id" class="flex items-center justify-between p-3 bg-gray-950 rounded-lg border border-gray-800">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center text-xs overflow-hidden">
                  <img v-if="acc.avatar" :src="acc.avatar" class="w-full h-full object-cover"/>
                  <span v-else>{{ acc.name.substring(0, 1) }}</span>
                </div>
                <div>
                  <div class="text-sm font-medium text-gray-50">{{ acc.name }}</div>
                  <div class="text-xs text-gray-500 font-mono">{{ acc.userId }}</div>
                </div>
              </div>
              <div :class="['text-xs px-2 py-1 rounded-md', acc.status === 'active' ? 'bg-emerald-500/10 text-emerald-500' : 'bg-red-500/10 text-red-500']">
                {{ acc.status === 'active' ? '有效' : '已失效' }}
              </div>
            </div>
          </div>
        </div>

        <!-- 抖音平台 -->
        <div class="bg-gray-900 border border-gray-800 rounded-xl p-5">
          <div class="flex items-center justify-between mb-4 border-b border-gray-800 pb-4">
            <h3 class="text-lg font-medium text-gray-50 flex items-center gap-2">
              <span class="text-black bg-white rounded-sm px-1 leading-tight font-bold">♪</span> 抖音
            </h3>
            <button @click="openLogin('douyin')" class="text-xs bg-blue-600 hover:bg-blue-700 text-white px-3 py-1.5 rounded-lg transition-colors cursor-pointer">
              + 新增授权
            </button>
          </div>
          <div class="space-y-3">
            <div v-for="acc in accounts.filter(a => a.platform === 'douyin')" :key="acc.id" class="flex items-center justify-between p-3 bg-gray-950 rounded-lg border border-gray-800">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center text-xs overflow-hidden">
                  <img v-if="acc.avatar" :src="acc.avatar" class="w-full h-full object-cover"/>
                  <span v-else>{{ acc.name.substring(0, 1) }}</span>
                </div>
                <div>
                  <div class="text-sm font-medium text-gray-50">{{ acc.name }}</div>
                  <div class="text-xs text-gray-500 font-mono">{{ acc.userId }}</div>
                </div>
              </div>
              <div :class="['text-xs px-2 py-1 rounded-md', acc.status === 'active' ? 'bg-emerald-500/10 text-emerald-500' : 'bg-red-500/10 text-red-500']">
                {{ acc.status === 'active' ? '有效' : '已失效' }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>

  <!-- 扫码登录弹窗 -->
  <div v-if="isLoginModalOpen" class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center">
    <div class="bg-gray-900 border border-gray-800 rounded-xl p-6 w-[400px] shadow-2xl relative">
      <button @click="isLoginModalOpen = false" class="absolute top-4 right-4 text-gray-400 hover:text-white cursor-pointer">✕</button>
      <h3 class="text-lg font-bold text-gray-50 mb-6 text-center">扫码登录 {{ currentLoginPlatform === 'douyin' ? '抖音' : '小红书' }}</h3>
      <div class="bg-white rounded-xl p-4 flex flex-col items-center justify-center min-h-[250px] mb-6">
        <div v-if="!qrcodeUrl" class="text-gray-400 flex flex-col items-center gap-2"><div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div><span class="text-sm mt-2">正在获取二维码...</span></div>
        <img v-else :src="qrcodeUrl" class="w-48 h-48" alt="Login QR Code" />
      </div>
    </div>
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
