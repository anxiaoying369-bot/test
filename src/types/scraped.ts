// 采集结果相关类型

export interface ScrapedUser {
  sec_uid: string;
  nickname: string;
  video_count: number;
  comment_count: number;
  has_avatar: boolean;
  avatar_path: string | null;
  avatar_data: string | null;
  last_scrape: number;
}

export interface ScrapedVideo {
  aweme_id: string;
  desc: string;
  create_time: number;
  thumb: string;
  comment_count: number;
}

export interface ScrapedComment {
  cid: string;
  text: string;
  create_time: number;
  user_nickname: string;
  user_avatar: string;
  digg_count: number;
  ip_label: string;
}

export interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}
