import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { marked } from 'marked';
import type { ScrapedUser, ScrapedVideo, ScrapedComment, Account } from '../types/scraped';

export function useScrapedResults() {
  const users = ref<ScrapedUser[]>([]);
  const selectedUser = ref<ScrapedUser | null>(null);
  const videos = ref<ScrapedVideo[]>([]);
  const selectedVideo = ref<ScrapedVideo | null>(null);
  const comments = ref<ScrapedComment[]>([]);

  const accounts = ref<Account[]>([]);
  const selectedAccount = ref('');

  const isLoading = ref(false);
  const errorMsg = ref('');
  const viewMode = ref<'users' | 'videos' | 'comments'>('users');
  const confirmingDeleteId = ref<string | null>(null);

  // AI 分析相关状态
  const isAnalysisModalOpen = ref(false);
  const isAnalyzing = ref(false);
  const analysisReport = ref('');
  const analyzingVideo = ref<ScrapedVideo | null>(null);

  const renderedReport = computed(() => marked(analysisReport.value) as string);
  const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));

  async function loadAccounts() {
    try {
      const res = await invoke('list_accounts', { platform: null }) as Account[];
      accounts.value = res;
      if (res.length > 0) selectedAccount.value = res[0].name;
    } catch (e) {
      console.error('加载账号失败:', e);
    }
  }

  async function loadUsers() {
    isLoading.value = true;
    errorMsg.value = '';
    try {
      users.value = await invoke('list_scraped_users');
    } catch (e: any) {
      console.error('Failed to load users:', e);
      errorMsg.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function deleteUser(sec_uid: string, event: Event) {
    event.stopPropagation();
    if (confirmingDeleteId.value !== sec_uid) {
      confirmingDeleteId.value = sec_uid;
      setTimeout(() => {
        if (confirmingDeleteId.value === sec_uid) confirmingDeleteId.value = null;
      }, 3000);
      return;
    }
    try {
      confirmingDeleteId.value = null;
      await invoke('delete_scraped_user', { secUid: sec_uid });
      await loadUsers();
    } catch (e: any) {
      console.error('[Frontend] Failed to delete user:', e);
      errorMsg.value = '删除失败: ' + String(e);
    }
  }

  async function selectUser(user: ScrapedUser) {
    selectedUser.value = user;
    viewMode.value = 'videos';
    selectedVideo.value = null;
    isLoading.value = true;
    errorMsg.value = '';
    try {
      videos.value = await invoke('get_scraped_videos', { secUid: user.sec_uid, limit: 100, offset: 0 });
    } catch (e: any) {
      console.error('Failed to load videos:', e);
      errorMsg.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function selectVideo(video: ScrapedVideo) {
    if (!selectedUser.value) return;
    selectedVideo.value = video;
    viewMode.value = 'comments';
    isLoading.value = true;
    errorMsg.value = '';
    try {
      comments.value = await invoke('get_scraped_comments', {
        secUid: selectedUser.value.sec_uid,
        awemeId: String(video.aweme_id),
        limit: 200,
        offset: 0,
      });
    } catch (e: any) {
      console.error('Failed to load comments:', e);
      errorMsg.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function openVideo(awemeId: string) {
    if (!selectedAccount.value) {
      errorMsg.value = '请先在顶部选择一个账号来提供 Cookie';
      return;
    }
    try {
      await invoke('open_video_in_browser', { awemeId: String(awemeId), accountName: selectedAccount.value });
    } catch (e: any) {
      console.error('Failed to open video:', e);
      errorMsg.value = String(e);
    }
  }

  async function analyzeVideoWithAI(video: ScrapedVideo) {
    if (!selectedUser.value) return;

    analyzingVideo.value = video;
    isAnalysisModalOpen.value = true;
    isAnalyzing.value = true;
    analysisReport.value = '';
    errorMsg.value = '';

    try {
      const videoComments = await invoke('get_scraped_comments', {
        secUid: selectedUser.value.sec_uid,
        awemeId: String(video.aweme_id),
        limit: 100, // 抓取前 100 条进行分析
        offset: 0,
      }) as ScrapedComment[];

      if (videoComments.length === 0) {
        throw new Error('该视频暂未采集到评论。请先在「评论采集」页对该博主采集评论（采集类型选「评论」或「全部」）后再分析。');
      }

      const report = await invoke('studio_analyze_video_comments', {
        comments: videoComments.map(c => ({ text: c.text })),
      }) as string;

      analysisReport.value = report;
    } catch (e: any) {
      console.error('AI 分析失败:', e);
      errorMsg.value = 'AI 分析失败: ' + String(e);
      isAnalysisModalOpen.value = false;
    } finally {
      isAnalyzing.value = false;
    }
  }

  function goBack() {
    errorMsg.value = '';
    if (viewMode.value === 'comments') {
      viewMode.value = 'videos';
      selectedVideo.value = null;
    } else if (viewMode.value === 'videos') {
      viewMode.value = 'users';
      selectedUser.value = null;
    }
  }

  function formatDate(timestamp: number) {
    if (!timestamp) return '—';
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // 单视频评论采集上限 100，超过则显示 99+
  function formatCommentCount(n: number) {
    return n > 99 ? '99+' : String(n ?? 0);
  }

  onMounted(() => {
    loadUsers();
    loadAccounts();
  });

  return {
    users, selectedUser, videos, selectedVideo, comments, accounts, selectedAccount,
    isLoading, errorMsg, viewMode, confirmingDeleteId,
    isAnalysisModalOpen, isAnalyzing, analysisReport, analyzingVideo, renderedReport, douyinAccounts,
    loadUsers, deleteUser, selectUser, selectVideo, openVideo, analyzeVideoWithAI,
    goBack, formatDate, formatCommentCount,
  };
}
