// 直播监控相关类型

export interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

export interface LiveMessage {
  time: string;
  user_name: string;
  user_id: string;
  sec_uid?: string;     // 弹幕协议自带，可空（部分消息不填充）
  display_id?: string;  // 抖音号
  content?: string;
  gift_name?: string;
  gift_count?: number;
  count?: number;   // for likes
  gender?: string;  // for member join
}

export interface LiveRoom {
  id: string;
  anchor_name: string;
  status: 'connecting' | 'running' | 'stopped' | 'error';
  messages: { type: string; payload: LiveMessage }[];
  error?: string;
}
