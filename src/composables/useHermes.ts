import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Session } from '../types/hermes';

export function useHermesConfig() {
  const gatewayUrl = ref('http://127.0.0.1:8642');
  const apiKey = ref('');

  const loadConfig = async () => {
    try {
      const config: any = await invoke('get_config');
      if (config?.hermes?.gateway_url) gatewayUrl.value = config.hermes.gateway_url;
      if (config?.hermes?.api_key) apiKey.value = config.hermes.api_key;
    } catch {}
  };

  watch([gatewayUrl, apiKey], async ([newUrl, newKey]) => {
    try {
      const config: any = await invoke('get_config');
      await invoke('save_config', {
        config: {
          ...config,
          hermes: { ...(config?.hermes || {}), gateway_url: newUrl, api_key: newKey }
        }
      });
    } catch {}
  });

  return { gatewayUrl, apiKey, loadConfig };
}

export function useHermesSessions() {
  const sessions = ref<Session[]>([]);
  const currentSessionId = ref<string | null>(null);

  const loadSessions = () => {
    const raw = localStorage.getItem('hermes_sessions_v2');
    if (raw) sessions.value = JSON.parse(raw);
  };

  const saveSessions = () => {
    localStorage.setItem('hermes_sessions_v2', JSON.stringify(sessions.value));
  };

  const createSession = (title: string = '新对话') => {
    const newSession: Session = {
      id: crypto.randomUUID(),
      title,
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    sessions.value.unshift(newSession);
    currentSessionId.value = newSession.id;
    saveSessions();
    return newSession;
  };

  return { sessions, currentSessionId, loadSessions, saveSessions, createSession };
}
