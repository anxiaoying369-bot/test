// 系统设置相关类型定义（由 SettingsView 及各设置子组件共享）

export interface GeoModelConfig {
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
  enabled: boolean;
}

export interface GeoPublishPlatform {
  name: string;
  url: string;
  description: string;
  system_prompt: string;
}

export interface LLMConfig {
  api_key: string;
  base_url: string;
  model: string;
  kb_api_key: string;
  kb_base_url: string;
  embedding_model: string;
  analysis_prompt: string;
  live_reply_prompt: string;
  live_theme: string;
  live_content: string;
  geo_models: GeoModelConfig[];
  geo_publish_platforms: GeoPublishPlatform[];
}

export interface HermesConfig {
  enabled: boolean;
  gateway_url: string;
  api_key: string;
}

export interface TtsVoice {
  voice_id: string;   // OpenAI 协议实际用的 voice 值
  name: string;       // 前端显示的友好名称
}

export interface VideoConfig {
  fal_key: string;
  volc_key: string;
  openai_api_key: string;
  openai_base_url: string;
  openai_model: string;
  default_provider: string;
  // TTS
  tts_provider?: string;
  tts_api_key?: string;
  tts_base_url?: string;
  tts_model?: string;
  default_tts_voice?: string;
  default_tts_speed?: number;
  tts_voices?: TtsVoice[];          // 自定义音色组
  script_system_prompt?: string;    // 脚本生成系统提示词
}

export interface AppConfig {
  llm: LLMConfig;
  hermes: HermesConfig;
  video: VideoConfig;
}
