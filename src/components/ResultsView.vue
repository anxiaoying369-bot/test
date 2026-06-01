<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  Users, Video, MessageSquare, ChevronRight,
  Calendar, Heart, MessageCircle, ArrowLeft,
  RefreshCw, ExternalLink, AlertCircle,
  Wand2, Trash2
} from 'lucide-vue-next';
import { useScrapedResults } from '../composables/useScrapedResults';
import AnalysisModal from './results/AnalysisModal.vue';
import { renderDouyinText } from '../lib/utils';

const {
  users, selectedUser, videos, selectedVideo, comments, selectedAccount,
  isLoading, errorMsg, viewMode, confirmingDeleteId,
  isAnalysisModalOpen, isAnalyzing, analysisReport, analyzingVideo, renderedReport, douyinAccounts,
  loadUsers, deleteUser, selectUser, selectVideo, openVideo, analyzeVideoWithAI,
  goBack, formatDate, formatCommentCount,
} = useScrapedResults();
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
                  {{ formatCommentCount(video.comment_count) }} 评论
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
          <div class="text-sm text-gray-300 leading-relaxed pl-11" v-html="renderDouyinText(comment.text)"></div>
        </div>
      </div>
    </main>

    <!-- AI 分析弹窗 -->
    <AnalysisModal
      :open="isAnalysisModalOpen"
      :analyzing="isAnalyzing"
      :analyzingVideo="analyzingVideo"
      :analysisReport="analysisReport"
      :renderedReport="renderedReport"
      @close="isAnalysisModalOpen = false"
    />
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
