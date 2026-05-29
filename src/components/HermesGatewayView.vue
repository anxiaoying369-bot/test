<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { ChatMessage } from '../types/hermes';
import { useHermesConfig, useHermesSessions } from '../composables/useHermes';

import SessionSidebar from './hermes/SessionSidebar.vue';
import ChatWindow from './hermes/ChatWindow.vue';
import GatewayControl from './hermes/GatewayControl.vue';

// ============ State ============
const { gatewayUrl, apiKey, loadConfig } = useHermesConfig();
const { sessions, currentSessionId, loadSessions, saveSessions, createSession } = useHermesSessions();

const isConnected = ref(false);
const isChecking = ref(false);
const isStarting = ref(false);
const isStopping = ref(false);
const isEnablingApi = ref(false);
const isSending = ref(false);
const searchQuery = ref('');

const streamingContent = ref('');
const streamingThinking = ref('');
const activeRunId = ref<string | null>(null);

let unlistenChunk: UnlistenFn | null = null;
let unlistenThinking: UnlistenFn | null = null;
let unlistenDone: UnlistenFn | null = null;
let unlistenRunId: UnlistenFn | null = null;

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
  loadSessions();
  await loadConfig();
  await checkHealth();

  unlistenChunk = await listen<any>('hermes-chunk', (e) => {
    streamingContent.value += e.payload.content;
  });
  unlistenThinking = await listen<any>('hermes-thinking', (e) => {
    streamingThinking.value += e.payload.content;
  });
  unlistenDone = await listen<any>('hermes-done', () => {
    finishStreaming();
  });
  unlistenRunId = await listen<any>('hermes-run-id', (e) => {
    activeRunId.value = e.payload.run_id;
  });
});

onUnmounted(() => {
  if (unlistenChunk) unlistenChunk();
  if (unlistenThinking) unlistenThinking();
  if (unlistenDone) unlistenDone();
  if (unlistenRunId) unlistenRunId();
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
