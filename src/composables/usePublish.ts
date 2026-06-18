import { ref, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 与 Rust db::PublishTaskRecord 一一对应（serde 默认 snake_case 序列化） */
export interface PublishTask {
  id: string;
  platform: string;
  account_name: string;
  video_path: string;
  title: string;
  tags: string[];
  description: string;
  cover_path: string | null;
  location: string | null;
  scheduled_at: number | null;
  status: 'pending' | 'publishing' | 'success' | 'failed' | 'cancelled';
  progress: number;
  stage: string | null;
  error_msg: string | null;
  result_url: string | null;
  created_at: number;
  updated_at: number;
}

export interface PublishableVideo {
  path: string;
  name: string;
  size: number;
  modified: number;
  /** true = 用户从磁盘自主上传的本地视频（非工作室成片） */
  local?: boolean;
}

/** 发布账号（对应 Rust AccountView） */
export interface PublishAccount {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: {
    user_id: string;
    nickname: string | null;
    avatar: string | null;
  };
}

interface PublishProgressEvent {
  id: string;
  status: PublishTask['status'];
  stage: string;
  progress: number;
  url?: string;
  error?: string;
}

export interface CreatePublishPayload {
  account_names: string[];
  video_path: string;
  title: string;
  tags: string[];
  description: string;
  cover_path?: string | null;
  location?: string | null;
  /** Unix 秒；0/undefined = 立即 */
  scheduled_at?: number | null;
}

/**
 * 多平台发布 / 定时排期 / 矩阵分发。
 * 任务进度由后端通过 publish-progress 事件推送，前端就地 upsert。
 */
export function usePublish() {
  const tasks = ref<PublishTask[]>([]);
  const videos = ref<PublishableVideo[]>([]);
  const accounts = ref<PublishAccount[]>([]);
  const lastError = ref('');
  let unlisten: (() => void) | null = null;

  async function refreshTasks() {
    try {
      tasks.value = await invoke<PublishTask[]>('list_publish_tasks');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function refreshVideos() {
    try {
      videos.value = await invoke<PublishableVideo[]>('list_publishable_videos');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function refreshAccounts() {
    try {
      accounts.value = await invoke<PublishAccount[]>('list_accounts', { platform: 'douyin' });
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function init() {
    await Promise.all([refreshTasks(), refreshVideos(), refreshAccounts()]);
    unlisten = await listen<PublishProgressEvent>('publish-progress', (event) => {
      const p = event.payload;
      const t = tasks.value.find((x) => x.id === p.id);
      if (t) {
        t.status = p.status;
        t.stage = p.stage;
        t.progress = p.progress;
        if (p.url) t.result_url = p.url;
        if (p.error) t.error_msg = p.error;
      } else {
        // 新任务（如调度器刚拾取）——拉一次全量
        refreshTasks();
      }
    });
  }

  async function createTasks(payload: CreatePublishPayload): Promise<string[]> {
    const ids = await invoke<string[]>('create_publish_tasks', {
      req: {
        platform: 'douyin',
        account_names: payload.account_names,
        video_path: payload.video_path,
        title: payload.title,
        tags: payload.tags,
        description: payload.description,
        cover_path: payload.cover_path ?? null,
        location: payload.location ?? null,
        scheduled_at: payload.scheduled_at ?? null,
      },
    });
    await refreshTasks();
    return ids;
  }

  async function cancelTask(id: string) {
    await invoke('cancel_publish_task', { id });
    await refreshTasks();
  }

  async function retryTask(id: string) {
    await invoke('retry_publish_task', { id });
    await refreshTasks();
  }

  async function deleteTask(id: string) {
    await invoke('delete_publish_task', { id });
    await refreshTasks();
  }

  onUnmounted(() => {
    if (unlisten) unlisten();
  });

  return {
    tasks,
    videos,
    accounts,
    lastError,
    init,
    refreshTasks,
    refreshVideos,
    refreshAccounts,
    createTasks,
    cancelTask,
    retryTask,
    deleteTask,
  };
}
