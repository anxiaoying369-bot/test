export interface ChatMessage {
  role: 'user' | 'assistant' | 'system' | 'tool' | 'thought';
  content: string;
  timestamp: number;
  toolName?: string;
  toolCallId?: string;
  toolStatus?: 'running' | 'completed';
  isStreaming?: boolean;
}

export interface Session {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface HermesSkill {
  name: string;
  category: string;
  source: string;
  trust: string;
  status: string;
}

export interface HermesTool {
  name: string;
  enabled: boolean;
  description: string;
  keyword: string;
}
