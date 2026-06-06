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
  /** AI 助理：动作工具确认执行后是否额外调用 LLM 生成自然语言总结（默认 false，省配额） */
  ai_summarize_actions?: boolean;
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

export interface SttConfig {
  api_key: string;
  base_url: string;
  model: string;
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
  tts_prosody_tags?: string;        // TTS 语气/声调标注模板（后端 generation.rs 用）
  // MoneyPrinterTurbo 视频引擎
  pexels_api_keys?: string;         // Pexels 素材库 API Key（多个逗号分隔）
  mpt_voice_name?: string;          // 默认 edge-tts 音色，如 zh-CN-XiaoxiaoNeural-Female
  mpt_subtitle_provider?: string;   // 字幕方式：edge / whisper
}

export interface AppConfig {
  llm: LLMConfig;
  hermes: HermesConfig;
  video: VideoConfig;
  stt: SttConfig;
}
