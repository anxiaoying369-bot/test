<script setup lang="ts">
import { ref, onMounted, provide } from 'vue';
import { MessageSquare, Users, Search, FileText, Radio, Settings, Database, Sparkles, Terminal, Film, LayoutDashboard } from 'lucide-vue-next';
import ScraperView from './components/ScraperView.vue';
import ResultsView from './components/ResultsView.vue';
import LiveMonitorView from './components/LiveMonitorView.vue';
import SettingsView from './components/SettingsView.vue';
import ChatView from './components/ChatView.vue';
import KnowledgeBaseView from './components/KnowledgeBaseView.vue';
import ContentStudioView from './components/ContentStudioView.vue';
import VideoStudioView from './components/VideoStudioView.vue';
import HermesGatewayView from './components/HermesGatewayView.vue';
import AccountsView from './components/AccountsView.vue';
import DashboardView from './components/DashboardView.vue';
import { useLiveEvents } from './composables/useLiveEvents';

type PageKey = 'dashboard' | 'chat' | 'accounts' | 'scraper' | 'results' | 'live_monitor' | 'douyin_im' | 'settings' | 'kb' | 'studio' | 'video_studio' | 'hermes';
const currentPage = ref<PageKey>('dashboard');
const settingsInitialTab = ref<string>('llm');

function navigateTo(page: PageKey, settingsTab?: string) {
  currentPage.value = page;
  if (page === 'settings' && settingsTab) {
    settingsInitialTab.value = settingsTab;
  }
}

provide('navigateTo', navigateTo);
provide('settingsInitialTab', settingsInitialTab);

const { liveMonitorRooms, initLiveEventListener } = useLiveEvents();

onMounted(initLiveEventListener);
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-gray-950 text-gray-50 font-sans">
    <!-- 左侧导航（固定宽度，不随内容区压缩） -->
    <aside class="flex flex-col w-56 flex-shrink-0 h-full bg-gray-950 border-r border-gray-800">
      <div class="p-6 font-bold tracking-tight">AutoCast AI</div>
      <nav class="flex-1 px-3 space-y-1">
        <a href="#" @click="currentPage = 'dashboard'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'dashboard' ? 'bg-gray-900' : 'text-gray-400']">
          <LayoutDashboard class="w-5 h-5 text-blue-400" />
          <span>任务调度中心</span>
        </a>
        <a href="#" @click="currentPage = 'chat'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'chat' ? 'bg-gray-900' : 'text-gray-400']">
          <MessageSquare class="w-5 h-5 text-blue-500" />
          <span>AI 助理对话</span>
        </a>
        <a href="#" @click="currentPage = 'studio'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'studio' ? 'bg-gray-900' : 'text-gray-400']">
          <Sparkles class="w-5 h-5 text-purple-400" />
          <span>AI 创作中心</span>
        </a>
        <a href="#" @click="currentPage = 'video_studio'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'video_studio' ? 'bg-gray-900' : 'text-gray-400']">
          <Film class="w-5 h-5 text-orange-400" />
          <span>视频创作中心</span>
        </a>
        <a href="#" @click="currentPage = 'accounts'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'accounts' ? 'bg-gray-900' : 'text-gray-400']">
          <Users class="w-5 h-5" />
          <span>账号管理</span>
        </a>
        <a href="#" @click="currentPage = 'scraper'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'scraper' ? 'bg-gray-900' : 'text-gray-400']">
          <Search class="w-5 h-5 text-purple-500" />
          <span>评论采集</span>
        </a>
        <a href="#" @click="currentPage = 'results'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'results' ? 'bg-gray-900' : 'text-gray-400']">
          <FileText class="w-5 h-5 text-green-500" />
          <span>采集结果</span>
        </a>
        <a href="#" @click="currentPage = 'kb'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'kb' ? 'bg-gray-900' : 'text-gray-400']">
          <Database class="w-5 h-5 text-blue-500" />
          <span>企业知识库</span>
        </a>
        <a href="#" @click="currentPage = 'live_monitor'" :class="['flex items-center justify-between px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'live_monitor' ? 'bg-gray-900' : 'text-gray-400']">
          <div class="flex items-center gap-3">
            <Radio class="w-5 h-5 text-red-500" />
            <span>直播监控</span>
          </div>
          <span v-if="Object.keys(liveMonitorRooms).length > 0" class="text-[10px] bg-red-600 text-white px-1.5 py-0.5 rounded-full font-bold">
            {{ Object.values(liveMonitorRooms).filter(r => r.status === 'running').length }}
          </span>
        </a>
        <div class="pt-4 mt-4 border-t border-gray-900 space-y-1">
          <a href="#" @click="currentPage = 'settings'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'settings' ? 'bg-gray-900' : 'text-gray-400']">
            <Settings class="w-5 h-5" />
            <span>系统设置</span>
          </a>
          <a href="#" @click="currentPage = 'hermes'" :class="['flex items-center gap-3 px-3 py-2.5 rounded-lg cursor-pointer', currentPage === 'hermes' ? 'bg-gray-900' : 'text-gray-400']">
            <Terminal class="w-5 h-5 text-indigo-400" />
            <span>Hermes Agent</span>
          </a>
        </div>
      </nav>
    </aside>

    <!-- 主内容：任务调度中心 -->
    <main v-if="currentPage === 'dashboard'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <DashboardView />
    </main>

    <!-- 主内容：AI 助理 -->
    <main v-if="currentPage === 'chat'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ChatView />
    </main>

    <!-- 主内容：AI 创作中心 -->
    <main v-if="currentPage === 'studio'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ContentStudioView />
    </main>

    <!-- 主内容：视频创作中心 -->
    <main v-if="currentPage === 'video_studio'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <VideoStudioView />
    </main>

    <!-- 主内容：Hermes 网关 -->
    <main v-if="currentPage === 'hermes'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <HermesGatewayView />
    </main>

    <!-- 主内容：账号管理 -->
    <AccountsView v-if="currentPage === 'accounts'" />

    <!-- 主内容：评论采集 -->
    <main v-if="currentPage === 'scraper'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ScraperView />
    </main>

    <!-- 主内容：采集结果 -->
    <main v-if="currentPage === 'results'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <ResultsView />
    </main>

    <!-- 主内容：企业知识库 -->
    <main v-if="currentPage === 'kb'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <KnowledgeBaseView />
    </main>

    <!-- 主内容：直播监控 -->
    <main v-if="currentPage === 'live_monitor'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <LiveMonitorView :globalRooms="liveMonitorRooms" />
    </main>

    <!-- 主内容：系统设置 -->
    <main v-if="currentPage === 'settings'" class="flex flex-col flex-1 min-w-0 h-full bg-gray-950">
      <SettingsView />
    </main>
  </div>
</template>
