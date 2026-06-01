import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { AppConfig } from '../types/settings';

// 单例：默认配置（首次加载前的占位值，也用于结构兜底）
function defaultConfig(): AppConfig {
  return {
    llm: {
      api_key: '',
      base_url: 'https://api.openai.com/v1',
      model: 'gpt-4o',
      kb_api_key: '',
      kb_base_url: 'https://api.openai.com/v1',
      embedding_model: 'text-embedding-3-small',
      analysis_prompt: '',
      live_reply_prompt: '',
      live_theme: '',
      live_content: '',
      geo_models: [],
      geo_publish_platforms: [],
    },
    hermes: {
      enabled: false,
      gateway_url: 'http://127.0.0.1:8642',
      api_key: '',
    },
    video: {
      fal_key: '',
      volc_key: '',
      openai_api_key: '',
      openai_base_url: 'https://api.openai.com/v1',
      openai_model: 'v0',
      default_provider: 'fal',
      tts_provider: 'mock',
      tts_api_key: '',
      tts_base_url: 'https://api.openai.com/v1',
      tts_model: 'tts-1',
      default_tts_voice: '',
      default_tts_speed: 1.0,
      tts_voices: [],
      script_system_prompt: '',
    },
  };
}

// 模块级单例 config：SettingsView 与各设置子组件共享同一份响应式配置
const config = ref<AppConfig>(defaultConfig());

export function useAppConfig() {
  const loadConfig = async () => {
    try {
      config.value = await invoke('get_config') as AppConfig;
    } catch (err) {
      console.error('Failed to load config:', err);
    }
  };

  const saveConfig = async () => {
    await invoke('save_config', { config: config.value });
  };

  const resetConfig = async () => {
    config.value = await invoke('get_default_config') as AppConfig;
  };

  return { config, loadConfig, saveConfig, resetConfig };
}
