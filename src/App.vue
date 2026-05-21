<script setup lang="ts">
import { ref } from 'vue';
import { Settings, Video, MessageSquare, Box, Play, Smartphone, AlertCircle, Clock } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';

const showInspiration = ref(true);
const logs = ref<string[]>([]);

async function triggerXiaohongshuSpy() {
  showInspiration.value = false;
  logs.value.push(`[${new Date().toLocaleTimeString()}] [INFO] Starting CloakBrowser for Xiaohongshu...`);
  
  try {
    const result = await invoke('spy_xiaohongshu', { keyword: '抹茶软欧包教程' });
    logs.value.push(`[${new Date().toLocaleTimeString()}] [SUCCESS] Task completed: ${result}`);
  } catch (error) {
    logs.value.push(`[${new Date().toLocaleTimeString()}] [ERROR] Task failed: ${error}`);
  }
}
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-gray-950 text-gray-50 font-sans selection:bg-blue-600/30">
    <!-- 全屏灵感遮罩 -->
    <div v-if="showInspiration" class="absolute inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center pointer-events-none">
       <div class="grid grid-cols-2 gap-4 pointer-events-auto">
          <button @click="triggerXiaohongshuSpy" class="bg-gray-900 border border-gray-800 hover:border-blue-500/50 hover:bg-gray-800/80 p-6 rounded-xl text-left transition-all group cursor-pointer">
             <h3 class="text-lg font-bold text-gray-50 mb-2 group-hover:text-blue-400">抹茶软欧包教程</h3>
             <p class="text-sm text-gray-400">一键复刻小红书烘焙爆款</p>
          </button>
          <button class="bg-gray-900 border border-gray-800 hover:border-blue-500/50 hover:bg-gray-800/80 p-6 rounded-xl text-left transition-all group cursor-pointer">
             <h3 class="text-lg font-bold text-gray-50 mb-2 group-hover:text-blue-400">数字化笔记</h3>
             <p class="text-sm text-gray-400">效率达人必备框架</p>
          </button>
          <button class="bg-gray-900 border border-gray-800 hover:border-blue-500/50 hover:bg-gray-800/80 p-6 rounded-xl text-left transition-all group cursor-pointer">
             <h3 class="text-lg font-bold text-gray-50 mb-2 group-hover:text-blue-400">健康轻食</h3>
             <p class="text-sm text-gray-400">减脂期餐单</p>
          </button>
          <button class="bg-gray-900 border border-gray-800 hover:border-blue-500/50 hover:bg-gray-800/80 p-6 rounded-xl text-left transition-all group cursor-pointer">
             <h3 class="text-lg font-bold text-gray-50 mb-2 group-hover:text-blue-400">穿搭公式</h3>
             <p class="text-sm text-gray-400">早秋高级感</p>
          </button>
       </div>
    </div>

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
          <Video class="w-5 h-5 text-blue-500" />
          <span class="font-medium text-sm">🎬 视频发布矩阵</span>
        </a>
        <a href="#" class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-gray-50 hover:bg-gray-900/50 transition-colors">
          <MessageSquare class="w-5 h-5" />
          <span class="font-medium text-sm">💬 私域微信客服</span>
        </a>
        <a href="#" class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-gray-50 hover:bg-gray-900/50 transition-colors">
          <Box class="w-5 h-5" />
          <span class="font-medium text-sm">📦 Obsidian 知识库</span>
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

    <!-- 2. 中间栏：工作流任务队列中控台 -->
    <main class="flex flex-col w-[40%] h-full bg-gray-950 border-r border-gray-800">
      <!-- 顶部操控区 -->
      <div class="p-4 border-b border-gray-800 flex gap-3">
        <div class="relative flex-1">
          <input type="text" placeholder="输入搞钱关键词 (如: 抹茶欧包)..." 
            class="w-full bg-gray-900 border border-gray-800 rounded-lg px-4 py-2 text-sm text-gray-50 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-600 focus:border-blue-600 transition-all shadow-sm" />
        </div>
        <button class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm font-medium flex items-center gap-2 transition-colors shadow-sm whitespace-nowrap cursor-pointer">
          <Play class="w-4 h-4" />
          + 投放新任务
        </button>
      </div>

      <!-- 中部队列列表 -->
      <div class="flex-1 overflow-y-auto p-4 space-y-3">
        <!-- 执行中卡片 -->
        <div class="bg-gray-900 rounded-xl p-4 border border-emerald-500/30 shadow-[0_0_15px_rgba(16,185,129,0.1)] relative overflow-hidden">
          <div class="absolute top-0 left-0 h-1 bg-emerald-500 w-[68%] transition-all duration-1000"></div>
          <div class="flex justify-between items-start mb-3">
            <div class="flex items-center gap-2">
              <div class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></div>
              <span class="text-xs font-mono text-emerald-500">#Task-8821</span>
            </div>
            <span class="text-xs font-mono text-gray-400">68%</span>
          </div>
          <p class="text-sm font-medium text-gray-50">正在剪辑视频...</p>
          <div class="mt-3 flex gap-2 text-gray-500">
             <span class="animate-bounce">🎬</span>
             <span class="animate-bounce" style="animation-delay: 100ms">✂️</span>
             <span class="animate-bounce" style="animation-delay: 200ms">🎵</span>
          </div>
        </div>

        <!-- 高优先级插队卡片 -->
        <div class="bg-gray-900 rounded-xl p-4 border border-red-500/20">
          <div class="flex items-center gap-2 mb-2">
            <AlertCircle class="w-4 h-4 text-red-500 animate-pulse" />
            <span class="text-xs font-bold text-red-500">[⚡ 优先响应]</span>
          </div>
          <p class="text-sm text-gray-300">自动回复: <span class="text-gray-50 font-medium">客户 张步步</span></p>
        </div>

        <!-- 等待中卡片 -->
        <div class="bg-gray-900/50 rounded-xl p-4 border border-gray-800">
          <div class="flex justify-between items-center mb-2">
            <span class="text-xs text-gray-500 font-mono">#Task-8822</span>
            <span class="text-xs font-medium text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded flex items-center gap-1">
              <Clock class="w-3 h-3" /> 排队中
            </span>
          </div>
          <p class="text-sm text-gray-400">关键词: <span class="text-gray-300">肉松小贝</span></p>
        </div>

        <!-- 等待中卡片 -->
        <div class="bg-gray-900/50 rounded-xl p-4 border border-gray-800">
          <div class="flex justify-between items-center mb-2">
            <span class="text-xs text-gray-500 font-mono">#Task-8823</span>
            <span class="text-xs font-medium text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded flex items-center gap-1">
              <Clock class="w-3 h-3" /> 排队中
            </span>
          </div>
          <p class="text-sm text-gray-400">关键词: <span class="text-gray-300">提拉米苏</span></p>
        </div>
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
        <div class="text-gray-400"><span class="text-gray-500">[09:42:01]</span> <span class="text-blue-400">[INFO]</span> Workspace initialized at AppData/Roaming/autocast/xhs_writer</div>
        <div class="text-gray-300"><span class="text-gray-500">[09:42:03]</span> <span class="text-emerald-500">[OCR]</span> Crawled 15 top notes from Xiaohongshu successfully.</div>
        <div class="text-gray-300"><span class="text-gray-500">[09:42:06]</span> <span class="text-purple-400">[LLM]</span> Thinking: Analyzing audience pain points for "Matcha Bread"...</div>
        <div class="text-gray-300"><span class="text-gray-500">[09:42:10]</span> <span class="text-amber-400">[AIGC]</span> Triggered Stable Diffusion API. Generating merchant-grade images...</div>
        <div class="text-gray-300"><span class="text-gray-500">[09:42:15]</span> <span class="text-pink-400">[RPA]</span> ADB Command Executed: adb shell input tap 950 1850 (Click Send)</div>
        
        <div class="bg-amber-500/10 border-l-2 border-amber-500 pl-2 py-1 my-2">
          <span class="text-gray-500">[09:43:00]</span> <span class="text-amber-500 font-bold">[WECHAT]</span> <span class="text-amber-400">New message from [张步步]: "系统怎么卖？"</span>
        </div>
        
        <div class="text-gray-300"><span class="text-gray-500">[09:43:01]</span> <span class="text-blue-400">[AGENT]</span> Loaded Obsidian Vault context: <span class="text-gray-50">📦 品牌产品知识库/价格表.md</span></div>
        
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
