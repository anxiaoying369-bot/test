<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { openUrl as openExternalUrl } from '@tauri-apps/plugin-opener';

import type { ChatMessage } from '../types/hermes';
import { useHermesConfig, useHermesSessions } from '../composables/useHermes';

import SessionSidebar from './hermes/SessionSidebar.vue';
import ChatWindow from './hermes/ChatWindow.vue';
import GatewayControl from './hermes/GatewayControl.vue';

// ============ State ============
const { gatewayUrl, apiKey, loadConfig } = useHermesConfig();
const { sessions, currentSessionId, loadSessions, saveSessions, createSession } = useHermesSessions();

const openUrl = async (url: string) => {
  try {
    await openExternalUrl(url);
  } catch (e) {
    console.error('Failed to open url:', e);
  }
};

const isConnected = ref(false);
const isInstalled = ref(true);
const isChecking = ref(false);
const isStarting = ref(false);
const isStopping = ref(false);
const isEnablingApi = ref(false);
const isSending = ref(false);
const searchQuery = ref('');

const streamingContent = ref('');
const streamingThinking = ref('');
// currentRunId：仅用于追踪当前 run（供 stop/approve 调用），不代表需要授权
const currentRunId = ref<string | null>(null);
// activeRunId：仅当 Agent 真正请求授权时才被设置，用于弹出授权面板
const activeRunId = ref<string | null>(null);

let unlistenChunk: UnlistenFn | null = null;
let unlistenThinking: UnlistenFn | null = null;
let unlistenDone: UnlistenFn | null = null;
let unlistenRunId: UnlistenFn | null = null;
let unlistenApproval: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;

const currentSession = computed(() => 
  sessions.value.find(s => s.id === currentSessionId.value) || null
);

const filteredSessions = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return sessions.value;
  return sessions.value.filter(s => 
    s.title.toLowerCase().includes(q) || 
    s.messages.some(m => m.content.toLowerCase().includes(q))
  );
});

// ============ Logic ============

onMounted(async () => {
  try {
    isInstalled.value = await invoke('check_hermes_installed');
  } catch (e) {
    isInstalled.value = false;
  }

  if (isInstalled.value) {
    loadSessions();
    await loadConfig();
    await checkHealth();
  }

  unlistenChunk = await listen<any>('hermes-chunk', (e) => {
    streamingContent.value += e.payload.content;
  });
  unlistenThinking = await listen<any>('hermes-thinking', (e) => {
    streamingThinking.value += e.payload.content;
  });
  unlistenDone = await listen<any>('hermes-done', () => {
    finishStreaming();
  });
  // run-id 只用于追踪当前 run，不再据此弹授权面板
  unlistenRunId = await listen<any>('hermes-run-id', (e) => {
    currentRunId.value = e.payload.run_id;
  });
  // 只有 Agent 真正请求授权时，才显示授权面板
  unlistenApproval = await listen<any>('hermes-approval-required', (e) => {
    activeRunId.value = e.payload?.run_id || currentRunId.value;
  });
  // 出错时清理流式与授权状态，避免面板卡住
  unlistenError = await listen<any>('hermes-error', (e) => {
    activeRunId.value = null;
    isSending.value = false;
    streamingContent.value = '';
    streamingThinking.value = '';
    if (e.payload?.message) alert('Hermes 错误: ' + e.payload.message);
  });
});

onUnmounted(() => {
  if (unlistenChunk) unlistenChunk();
  if (unlistenThinking) unlistenThinking();
  if (unlistenDone) unlistenDone();
  if (unlistenRunId) unlistenRunId();
  if (unlistenApproval) unlistenApproval();
  if (unlistenError) unlistenError();
});

const checkHealth = async () => {
  if (isChecking.value) return;
  isChecking.value = true;
  try {
    await invoke('check_hermes_gateway_health', { gatewayUrl: gatewayUrl.value, apiKey: apiKey.value });
    isConnected.value = true;
  } catch {
    isConnected.value = false;
  } finally {
    isChecking.value = false;
  }
};

const startGateway = async () => {
  isStarting.value = true;
  try {
    await invoke('start_hermes_gateway');
    await checkHealth();
  } catch (e) {
    alert('启动失败: ' + e);
  } finally {
    isStarting.value = false;
  }
};

const stopGateway = async () => {
  isStopping.value = true;
  try {
    await invoke('stop_hermes_gateway');
    isConnected.value = false;
  } catch (e) {
    alert('停止失败: ' + e);
  } finally {
    isStopping.value = false;
  }
};

const enableApi = async () => {
  isEnablingApi.value = true;
  try {
    const key = await invoke<string>('hermes_enable_api_server');
    apiKey.value = key;
    await startGateway();
  } catch (e) {
    alert('启用失败: ' + e);
  } finally {
    isEnablingApi.value = false;
  }
};

const sendMessage = async (content: string) => {
  if (!currentSession.value) createSession();
  if (!currentSession.value) return;

  const userMsg: ChatMessage = {
    role: 'user',
    content,
    timestamp: Date.now(),
  };
  currentSession.value.messages.push(userMsg);
  currentSession.value.updatedAt = Date.now();
  saveSessions();

  isSending.value = true;
  streamingContent.value = '';
  streamingThinking.value = '';

  try {
    await invoke('hermes_send_message', {
      gatewayUrl: gatewayUrl.value,
      apiKey: apiKey.value,
      messages: currentSession.value.messages.map(m => ({ role: m.role, content: m.content })),
      sessionId: currentSession.value.id,
    });
  } catch (e) {
    alert('发送失败: ' + e);
    isSending.value = false;
  }
};

const finishStreaming = () => {
  if (!currentSession.value) return;
  
  if (streamingThinking.value) {
    currentSession.value.messages.push({
      role: 'thought',
      content: streamingThinking.value,
      timestamp: Date.now(),
    });
  }
  
  if (streamingContent.value) {
    currentSession.value.messages.push({
      role: 'assistant',
      content: streamingContent.value,
      timestamp: Date.now(),
    });
  }

  streamingContent.value = '';
  streamingThinking.value = '';
  isSending.value = false;
  activeRunId.value = null;
  currentRunId.value = null;
  saveSessions();
};

const approveRun = async (runId: string, approved: boolean) => {
  try {
    await invoke('hermes_approve_run', { gatewayUrl: gatewayUrl.value, apiKey: apiKey.value, runId, approved });
    activeRunId.value = null;
  } catch (e) {
    alert('审批失败: ' + e);
  }
};

const stopRun = async (runId: string) => {
  try {
    await invoke('hermes_stop_run', { gatewayUrl: gatewayUrl.value, apiKey: apiKey.value, runId });
    activeRunId.value = null;
    currentRunId.value = null;
    isSending.value = false;
  } catch (e) {
    alert('停止任务失败: ' + e);
  }
};

const deleteSession = (id: string) => {
  if (!confirm('确定要删除该会话吗？')) return;
  sessions.value = sessions.value.filter(s => s.id !== id);
  if (currentSessionId.value === id) currentSessionId.value = null;
  saveSessions();
};
</script>

<template>
  <div class="h-full flex flex-col bg-gray-950 text-gray-100 overflow-hidden">
    <!-- 未安装 Hermes 的指引页面 -->
    <div v-if="!isInstalled" class="flex-1 flex flex-col items-center justify-center p-8 text-center space-y-6">
      <div class="w-20 h-20 bg-indigo-500/10 rounded-full flex items-center justify-center mb-4">
        <svg class="w-10 h-10 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>
      </div>
      <h2 class="text-2xl font-bold text-gray-100">未检测到 Hermes Agent</h2>
      <p class="text-gray-400 max-w-md leading-relaxed text-sm">
        Hermes 是一个独立的本地 AI 智能体应用，负责安全地管理系统工具调用、跨应用操作以及多模型统一调度。
      </p>
      <div class="bg-gray-900 border border-gray-800 rounded-xl p-6 text-left max-w-lg w-full shadow-xl">
        <h3 class="text-sm font-bold text-gray-300 mb-4 flex items-center gap-2">
          <span class="w-5 h-5 rounded bg-indigo-500/20 text-indigo-400 flex items-center justify-center text-xs">1</span>
          安装 Hermes
        </h3>
        <p class="text-xs text-gray-500 mb-4">
          请前往 Hermes 官方网站获取最新的安装指引和环境配置要求：
        </p>
        <div class="bg-gray-950 p-4 rounded-lg border border-gray-800 flex items-center justify-center mb-6">
          <a href="#" @click.prevent="openUrl('https://hermesagent.org.cn')" class="text-sm font-bold text-indigo-400 hover:text-indigo-300 transition-colors flex items-center gap-2">
            https://hermesagent.org.cn
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path></svg>
          </a>
        </div>
        
        <h3 class="text-sm font-bold text-gray-300 mb-4 flex items-center gap-2">
          <span class="w-5 h-5 rounded bg-indigo-500/20 text-indigo-400 flex items-center justify-center text-xs">2</span>
          重启系统
        </h3>
        <p class="text-xs text-gray-500">
          安装完成后，请彻底关闭并重新启动 AutoCast AI。系统将在启动时自动探测并完成 Hermes 网关的初始化配置。
        </p>
      </div>
    </div>

    <!-- 正常工作页面 -->
    <template v-else>
      <!-- Gateway Control Bar -->
      <GatewayControl
        v-model:gatewayUrl="gatewayUrl"
        v-model:apiKey="apiKey"
        :isConnected="isConnected"
        :isChecking="isChecking"
        :isStarting="isStarting"
        :isStopping="isStopping"
        :isEnablingApi="isEnablingApi"
        @checkHealth="checkHealth"
        @start="startGateway"
        @stop="stopGateway"
        @enableApi="enableApi"
      />

      <div class="flex-1 flex overflow-hidden">
        <!-- Session Sidebar -->
        <SessionSidebar
          v-model:searchQuery="searchQuery"
          :sessions="filteredSessions"
          :currentSessionId="currentSessionId"
          @create="createSession"
          @select="id => currentSessionId = id"
          @delete="deleteSession"
        />

        <!-- Chat Window -->
        <ChatWindow
          :session="currentSession"
          :isSending="isSending"
          :streamingContent="streamingContent"
          :streamingThinking="streamingThinking"
          :activeRunId="activeRunId"
          :toolCallProgress="null"
          @send="sendMessage"
          @approveRun="approveRun"
          @stopRun="stopRun"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #374151;
}
</style>
