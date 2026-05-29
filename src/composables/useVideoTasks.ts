import { ref, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { VideoTask, FfmpegProgress } from '../types/video-studio';

export function useVideoTasks() {
  const activeTasks = ref<Record<string, VideoTask>>({});
  const ffmpegProgress = ref<Record<string, FfmpegProgress>>({});
  let unlistenProgress: UnlistenFn | null = null;

  onMounted(async () => {
    unlistenProgress = await listen<FfmpegProgress>('video-ffmpeg-progress', (event) => {
      ffmpegProgress.value[event.payload.task_id] = event.payload;
    });
  });

  onUnmounted(() => {
    if (unlistenProgress) unlistenProgress();
  });

  return { activeTasks, ffmpegProgress };
}
