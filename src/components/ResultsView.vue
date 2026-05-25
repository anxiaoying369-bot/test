<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { 
  Users, Video, MessageSquare, ChevronRight, 
  Calendar, Heart, MessageCircle, ArrowLeft,
  RefreshCw, ExternalLink, AlertCircle,
  Wand2, Sparkles, X, BarChart3, PieChart, Info, Trash2
} from 'lucide-vue-next';
import { marked } from 'marked';

interface ScrapedUser {
  sec_uid: string;
  nickname: string;
  video_count: number;
  comment_count: number;
  has_avatar: boolean;
  avatar_path: string | null;
  avatar_data: string | null;
  last_scrape: number;
}

interface ScrapedVideo {
  aweme_id: string;
  desc: string;
  create_time: number;
  thumb: string;
  comment_count: number;
}

interface ScrapedComment {
  cid: string;
  text: string;
  create_time: number;
  user_nickname: string;
  user_avatar: string;
  digg_count: number;
  ip_label: string;
}

interface Account {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

interface AIAnalysisResult {
  summary: string;
  sentiment: {
    positive: number;
    neutral: number;
    negative: number;
  };
  key_themes: string[];
  top_comments_summary: string;
  suggestions: string[];
}

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

const renderedReport = computed(() => {
  return marked(analysisReport.value);
});

const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));

async function loadAccounts() {
  try {
    const res = await invoke('list_accounts', { platform: null }) as Account[];
    accounts.value = res;
    if (res.length > 0) {
      selectedAccount.value = res[0].name;
    }
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
    // 3秒后自动重置状态
    setTimeout(() => {
      if (confirmingDeleteId.value === sec_uid) {
        confirmingDeleteId.value = null;
      }
    }, 3000);
    return;
  }
  
  try {
    confirmingDeleteId.value = null;
    console.log('[Frontend] Invoking delete_scraped_user for:', sec_uid);
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
    videos.value = await invoke('get_scraped_videos', { 
      secUid: user.sec_uid, 
      limit: 100, 
      offset: 0 
    });
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
      offset: 0 
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
    await invoke('open_video_in_browser', { 
      awemeId: String(awemeId), 
      accountName: selectedAccount.value 
    });
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
    // 1. 先获取该视频的评论
    const videoComments = await invoke('get_scraped_comments', { 
      secUid: selectedUser.value.sec_uid,
      awemeId: String(video.aweme_id),
      limit: 100, // 抓取前 100 条进行分析
      offset: 0 
    }) as ScrapedComment[];

    if (videoComments.length === 0) {
      throw new Error('该视频暂无评论，无法分析');
    }

    // 2. 调用后端 AI 分析接口
    const report = await invoke('analyze_comments', { 
      comments: videoComments.map(c => ({ text: c.text })) 
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

onMounted(() => {
  loadUsers();
  loadAccounts();
});
</script>

<template>
  <div class="flex flex-col h-full bg-gray-950 text-gray-50 overflow-hidden relative">
    <!-- 头部导航 -->
    <header class="p-4 border-b border-gray-800 flex items-center justify-between">
      <div class="flex items-center gap-3 min-w-0">
        <button 
          v-if="viewMode !== 'users'" 
          @click="goBack"
          class="p-1.5 hover:bg-gray-800 rounded-lg transition-colors text-gray-400 flex-shrink-0"
        >
          <ArrowLeft class="w-5 h-5" />
        </button>
        <h2 class="text-xl font-bold flex items-center gap-2 truncate">
          <span v-if="viewMode === 'users'">采集结果查看</span>
          <span v-else-if="viewMode === 'videos'" class="flex items-center gap-2 truncate">
            <span class="text-gray-500 font-normal">用户:</span>
            <span class="truncate">{{ selectedUser?.nickname || selectedUser?.sec_uid.substring(0, 12) }}</span>
          </span>
          <span v-else-if="viewMode === 'comments'" class="flex items-center gap-2 truncate">
            <span class="text-gray-500 font-normal">作品评论:</span>
            <span class="truncate">{{ selectedVideo?.desc.substring(0, 15) || selectedVideo?.aweme_id }}...</span>
          </span>
        </h2>
      </div>
      
      <div class="flex items-center gap-4 flex-shrink-0">
        <!-- 账号选择（用于打开视频） -->
        <div class="flex items-center gap-2 bg-gray-900 px-3 py-1.5 rounded-lg border border-gray-800">
          <span class="text-xs text-gray-500">Cookie 来源:</span>
          <select v-model="selectedAccount" class="bg-transparent border-none text-xs text-gray-300 focus:outline-none focus:ring-0 cursor-pointer">
            <option v-for="acc in douyinAccounts" :key="acc.name" :value="acc.name">
              {{ acc.name }}
            </option>
          </select>
        </div>

        <button @click="loadUsers" class="p-2 hover:bg-gray-800 rounded-lg text-gray-400" title="刷新">
          <RefreshCw class="w-5 h-5" :class="{ 'animate-spin': isLoading }" />
        </button>
      </div>
    </header>

    <!-- 错误信息 -->
    <div v-if="errorMsg" class="m-4 p-3 bg-red-900/20 border border-red-800 rounded-lg flex items-center gap-3 text-red-400 text-sm">
      <AlertCircle class="w-5 h-5 flex-shrink-0" />
      <div class="flex-1">
        <div class="font-bold mb-0.5">提示</div>
        <div class="opacity-80 font-mono text-xs">{{ errorMsg }}</div>
      </div>
      <button @click="errorMsg = ''" class="hover:text-white">&times;</button>
    </div>

    <!-- 内容区域 -->
    <main class="flex-1 overflow-y-auto p-4">
      <!-- 加载中 -->
      <div v-if="isLoading && !users.length && !videos.length && !comments.length" class="flex flex-col items-center justify-center h-full py-20 text-gray-500">
        <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mb-4"></div>
        <p>正在读取数据...</p>
      </div>

      <!-- 用户列表 -->
      <div v-else-if="viewMode === 'users'" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div v-if="users.length === 0 && !isLoading" class="col-span-full flex flex-col items-center justify-center py-20 text-gray-500 border border-dashed border-gray-800 rounded-xl">
          <Users class="w-12 h-12 mb-3 opacity-20" />
          <p>暂无采集数据</p>
        </div>
        
        <div 
          v-for="user in users" 
          :key="user.sec_uid"
          @click="selectUser(user)"
          class="bg-gray-900 border border-gray-800 p-4 rounded-xl hover:border-blue-500/50 cursor-pointer transition-all group"
        >
          <div class="flex items-center gap-4 mb-4">
            <div class="w-12 h-12 rounded-full bg-gray-800 flex items-center justify-center text-xl font-bold text-blue-400 overflow-hidden border border-gray-700">
              <span v-if="!user.has_avatar">👤</span>
              <img v-else :src="user.avatar_data || (user.avatar_path ? convertFileSrc(user.avatar_path) : '')" class="w-full h-full object-cover" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-bold text-gray-200 truncate">{{ user.nickname }}</div>
              <div class="text-[10px] text-gray-500 truncate">{{ user.sec_uid }}</div>
              <div class="text-[10px] text-gray-400 font-mono mt-0.5">最后采集: {{ formatDate(user.last_scrape) }}</div>
            </div>
            <div class="flex flex-col items-end justify-between self-stretch">
              <div class="flex items-center">
                <button 
                  @click.stop="deleteUser(user.sec_uid, $event)"
                  :class="[
                    'px-2 py-1.5 rounded-lg transition-all flex items-center gap-1.5 text-xs font-bold whitespace-nowrap',
                    confirmingDeleteId === user.sec_uid 
                      ? 'bg-red-600 text-white opacity-100' 
                      : 'text-gray-500 hover:text-red-500 hover:bg-red-500/10 opacity-0 group-hover:opacity-100'
                  ]"
                  title="删除记录"
                >
                  <Trash2 class="w-4 h-4" />
                  <span v-if="confirmingDeleteId === user.sec_uid">确定删除?</span>
                </button>
              </div>
              <ChevronRight class="w-5 h-5 text-gray-700 group-hover:text-blue-500 transition-colors" />
            </div>
          </div>
          
          <div class="grid grid-cols-2 gap-2">
            <div class="bg-gray-950 rounded-lg p-2 border border-gray-800/50">
              <div class="text-[10px] text-gray-500 uppercase tracking-wider mb-1">作品数量</div>
              <div class="text-lg font-bold flex items-center gap-1.5">
                <Video class="w-4 h-4 text-purple-400" />
                {{ user.video_count }}
              </div>
            </div>
            <div class="bg-gray-950 rounded-lg p-2 border border-gray-800/50">
              <div class="text-[10px] text-gray-500 uppercase tracking-wider mb-1">评论数量</div>
              <div class="text-lg font-bold flex items-center gap-1.5">
                <MessageSquare class="w-4 h-4 text-green-400" />
                {{ user.comment_count }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 视频列表 -->
      <div v-else-if="viewMode === 'videos'" class="space-y-3">
        <div 
          v-for="video in videos" 
          :key="video.aweme_id"
          class="bg-gray-900 border border-gray-800 p-3 rounded-xl flex gap-4 group relative"
        >
          <div @click="selectVideo(video)" class="w-24 h-32 bg-gray-800 rounded-lg overflow-hidden flex-shrink-0 relative border border-gray-700 cursor-pointer">
            <img v-if="video.thumb" :src="video.thumb" referrerpolicy="no-referrer" class="w-full h-full object-cover" />
            <div v-else class="w-full h-full flex items-center justify-center text-gray-600">
              <Video class="w-8 h-8 opacity-20" />
            </div>
          </div>
          <div class="flex-1 flex flex-col justify-between py-1 min-w-0">
            <div>
              <div @click="selectVideo(video)" class="text-sm text-gray-200 line-clamp-2 leading-relaxed mb-2 font-medium cursor-pointer hover:text-blue-400">
                {{ video.desc || '(无标题)' }}
              </div>
              <div class="flex items-center gap-3 text-xs text-gray-500">
                <span class="flex items-center gap-1">
                  <Calendar class="w-3.5 h-3.5" />
                  {{ formatDate(video.create_time) }}
                </span>
                <span class="flex items-center gap-1">
                  <MessageCircle class="w-3.5 h-3.5" />
                  {{ video.comment_count }} 评论
                </span>
              </div>
            </div>
            <div class="flex items-center justify-between mt-2">
              <div @click="openVideo(video.aweme_id)" class="text-[10px] font-mono text-gray-600 cursor-pointer hover:text-blue-500 flex items-center gap-1">
                ID: {{ video.aweme_id }} <ExternalLink class="w-2.5 h-2.5" />
              </div>
              <button @click="analyzeVideoWithAI(video)" class="text-xs bg-blue-600/20 hover:bg-blue-600 text-blue-400 hover:text-white px-3 py-1 rounded-lg transition-all flex items-center gap-1.5 border border-blue-600/30">
                <Wand2 class="w-3 h-3" />
                AI 分析
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 评论列表 -->
      <div v-else-if="viewMode === 'comments'" class="space-y-4">
        <div 
          v-for="comment in comments" 
          :key="comment.cid"
          class="bg-gray-900 border border-gray-800 p-4 rounded-xl"
        >
          <div class="flex items-center gap-3 mb-3">
            <div class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center text-sm font-bold overflow-hidden border border-gray-700">
              <img v-if="comment.user_avatar" :src="comment.user_avatar" referrerpolicy="no-referrer" class="w-full h-full object-cover" />
              <span v-else>👤</span>
            </div>
            <div class="flex-1">
              <div class="text-sm font-bold text-gray-200">{{ comment.user_nickname }}</div>
              <div class="text-[10px] text-gray-500 flex items-center gap-2">
                <span>{{ formatDate(comment.create_time) }}</span>
                <span v-if="comment.ip_label">• {{ comment.ip_label }}</span>
              </div>
            </div>
            <div class="flex items-center gap-1 text-xs text-gray-500 bg-gray-950 px-2 py-1 rounded-full border border-gray-800">
              <Heart class="w-3 h-3" />
              {{ comment.digg_count }}
            </div>
          </div>
          <div class="text-sm text-gray-300 leading-relaxed pl-11">
            {{ comment.text }}
          </div>
        </div>
      </div>
    </main>

    <!-- AI 分析弹窗 -->
    <div v-if="isAnalysisModalOpen" class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-6">
      <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden animate-in fade-in zoom-in duration-200">
        <!-- 弹窗头部 -->
        <header class="p-4 border-b border-gray-800 flex items-center justify-between bg-gray-900/50">
          <div class="flex items-center gap-2 text-blue-400">
            <Sparkles class="w-5 h-5" />
            <h3 class="font-bold">AI 深度分析报告</h3>
          </div>
          <button @click="isAnalysisModalOpen = false" class="text-gray-500 hover:text-white p-1 hover:bg-gray-800 rounded-full transition-colors">
            <X class="w-5 h-5" />
          </button>
        </header>

        <!-- 弹窗内容 -->
        <div class="flex-1 overflow-y-auto p-6 custom-scrollbar bg-gray-950/30">
          <!-- 正在分析状态 -->
          <div v-if="isAnalyzing" class="flex flex-col items-center justify-center py-20">
            <div class="relative w-16 h-16 mb-6">
              <div class="absolute inset-0 border-4 border-blue-500/20 rounded-full"></div>
              <div class="absolute inset-0 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
              <Wand2 class="absolute inset-0 m-auto w-6 h-6 text-blue-400 animate-pulse" />
            </div>
            <p class="text-gray-400 animate-pulse text-sm">正在调动 LLM 分析作品及 {{ analyzingVideo?.comment_count }} 条评论内容...</p>
          </div>

          <!-- 分析结果展示 -->
          <div v-else-if="analysisReport" class="prose prose-invert max-w-none">
            <div 
              class="markdown-content text-gray-300 leading-relaxed text-sm font-sans bg-gray-900/50 p-6 rounded-2xl border border-gray-800 shadow-inner"
              v-html="renderedReport"
            ></div>
          </div>
        </div>

        <!-- 弹窗底部 -->
        <footer class="p-4 border-t border-gray-800 bg-gray-900/50 flex justify-end">
          <button @click="isAnalysisModalOpen = false" class="px-6 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded-xl text-sm font-medium transition-colors">
            返回列表
          </button>
        </footer>
      </div>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #374151;
}

@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes zoom-in { from { transform: scale(0.95); opacity: 0; } to { transform: scale(1); opacity: 1; } }
.animate-in { animation: fade-in 0.2s ease-out; }
.zoom-in { animation: zoom-in 0.2s ease-out; }

/* Markdown 样式覆盖 */
:deep(.markdown-content) h1,
:deep(.markdown-content) h2,
:deep(.markdown-content) h3 {
  color: #f3f4f6;
  margin-top: 1.5rem;
  margin-bottom: 0.75rem;
  font-weight: 600;
}
:deep(.markdown-content) h1 { font-size: 1.25rem; border-bottom: 1px solid #374151; padding-bottom: 0.5rem; }
:deep(.markdown-content) h2 { font-size: 1.1rem; }
:deep(.markdown-content) h3 { font-size: 1rem; }
:deep(.markdown-content) p { margin-bottom: 1rem; }
:deep(.markdown-content) ul, 
:deep(.markdown-content) ol { margin-bottom: 1rem; padding-left: 1.25rem; }
:deep(.markdown-content) li { margin-bottom: 0.25rem; }
:deep(.markdown-content) strong { color: #60a5fa; font-weight: 600; }
:deep(.markdown-content) blockquote {
  border-left: 4px solid #3b82f6;
  padding-left: 1rem;
  font-style: italic;
  color: #9ca3af;
  margin: 1rem 0;
}
:deep(.markdown-content) code {
  background-color: #1f2937;
  padding: 0.2rem 0.4rem;
  border-radius: 0.25rem;
  font-family: monospace;
  font-size: 0.875rem;
}
</style>
