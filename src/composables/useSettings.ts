import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export function useSettings() {
  const config = ref<any>({
    llm: {
      api_key: '',
      base_url: 'https://api.openai.com/v1',
      model: 'gpt-4o',
      kb_api_key: '',
      kb_base_url: '',
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
      openai_base_url: '',
      openai_model: '',
      default_provider: 'fal',
      tts_provider: 'openai',
      tts_api_key: '',
      tts_base_url: 'https://api.openai.com/v1',
      tts_model: 'tts-1',
      default_tts_voice: '',
      default_tts_speed: 1.0,
    }
  });

  const loadSettings = async () => {
    try {
      const saved = await invoke<any>('get_config');
      config.value = { ...config.value, ...saved };
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  };

  const saveSettings = async () => {
    try {
      await invoke('save_config', { config: config.value });
    } catch (e) {
      alert('保存失败: ' + e);
    }
  };

  return { config, loadSettings, saveSettings };
}
