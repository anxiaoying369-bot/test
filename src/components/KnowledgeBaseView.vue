<script setup lang="ts">
import {
  Database, FileText, Trash2,
  RefreshCw, CheckCircle, AlertCircle, Info,
  Plus, Search, SearchCode, X
} from 'lucide-vue-next';
import { useKnowledgeBase } from '../composables/useKnowledgeBase';

const {
  files, isLoading, isUploading, errorMsg, successMsg,
  isDetailModalOpen, isDetailLoading, selectedFileName, fileChunks,
  handleUpload, deleteFile, showDetails,
} = useKnowledgeBase();
</script>

<template>
  <div class="flex flex-col flex-1 h-full bg-gray-950 p-8 overflow-y-auto">
    <div class="max-w-4xl mx-auto w-full">
      <!-- 头部 -->
      <div class="flex items-center justify-between mb-8">
        <div>
          <h2 class="text-2xl font-bold text-white flex items-center gap-3">
            <Database class="w-7 h-7 text-blue-500" />
            企业知识库
          </h2>
          <p class="text-gray-400 text-sm mt-1">上传 PDF/TXT/JSON 文件，让 AI 助理具备专业背景知识。</p>
        </div>
        
        <button
          @click="handleUpload"
          :disabled="isUploading"
          class="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-6 py-2.5 rounded-xl font-medium transition-all shadow-lg shadow-blue-900/20"
        >
          <RefreshCw v-if="isUploading" class="w-4 h-4 animate-spin" />
          <Plus v-else class="w-4 h-4" />
          {{ isUploading ? '正在解析中...' : '上传新文档' }}
        </button>
      </div>

      <!-- 提示信息 -->
      <div v-if="successMsg" class="mb-6 p-4 bg-green-500/10 border border-green-500/20 rounded-xl flex items-center gap-3 text-green-400 text-sm animate-in fade-in zoom-in duration-200">
        <CheckCircle class="w-5 h-5" />
        {{ successMsg }}
        <button @click="successMsg = ''" class="ml-auto hover:text-white">&times;</button>
      </div>

      <div v-if="errorMsg" class="mb-6 p-4 bg-red-500/10 border border-red-500/20 rounded-xl flex items-center gap-3 text-red-400 text-sm animate-in fade-in zoom-in duration-200">
        <AlertCircle class="w-5 h-5" />
        {{ errorMsg }}
        <button @click="errorMsg = ''" class="ml-auto hover:text-white">&times;</button>
      </div>

      <!-- 知识库列表 -->
      <div class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden">
        <div class="p-6 border-b border-gray-800 flex items-center justify-between">
          <h3 class="text-lg font-medium text-white flex items-center gap-2">
            <FileText class="w-5 h-5 text-gray-400" />
            已索引文档 ({{ files.length }})
          </h3>
          <div class="flex items-center gap-2 text-[10px] text-gray-500 uppercase tracking-widest bg-gray-950 px-3 py-1 rounded-full border border-gray-800">
            <SearchCode class="w-3 h-3" />
            AI RAG Enabled
          </div>
        </div>

        <div class="divide-y divide-gray-800">
          <div v-if="isLoading && files.length === 0" class="p-20 text-center text-gray-600">
            <RefreshCw class="w-8 h-8 animate-spin mx-auto mb-4 opacity-20" />
            <p>正在加载索引...</p>
          </div>

          <div v-else-if="files.length === 0" class="p-20 text-center text-gray-600">
            <Database class="w-12 h-12 mx-auto mb-4 opacity-10" />
            <p>知识库暂无内容，请点击上方按钮开始上传。</p>
          </div>

          <div 
            v-for="file in files" 
            :key="file"
            @click="showDetails(file)"
            class="group p-4 flex items-center justify-between hover:bg-gray-800/30 transition-colors cursor-pointer"
          >
            <div class="flex items-center gap-4 min-w-0">
              <div class="w-10 h-10 rounded-lg bg-gray-950 flex items-center justify-center border border-gray-800 text-gray-500 group-hover:text-blue-400 transition-colors">
                <FileText class="w-5 h-5" />
              </div>
              <div class="min-w-0">
                <div class="text-sm font-medium text-gray-200 truncate">{{ file }}</div>
                <div class="text-[10px] text-gray-500 flex items-center gap-2">
                  <span>已向量化</span>
                  <span>•</span>
                  <span>可实时检索</span>
                </div>
              </div>
            </div>

            <button
              @click.stop="deleteFile(file)"
              class="p-2 text-gray-600 hover:text-red-500 hover:bg-red-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100"
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <!-- 说明区域 -->
      <div class="mt-8 grid grid-cols-1 md:grid-cols-2 gap-6">
        <div class="p-5 bg-blue-500/5 border border-blue-500/10 rounded-2xl">
          <h4 class="text-sm font-bold text-blue-400 flex items-center gap-2 mb-3">
            <Search class="w-4 h-4" />
            它是如何工作的？
          </h4>
          <p class="text-xs text-gray-400 leading-relaxed">
            当你上传文档时，系统会将其切分为小的“知识块”，并通过 AI 模型转化为向量存储。在对话时，AI 会根据你的问题自动匹配最相关的知识块作为参考。
          </p>
        </div>
        <div class="p-5 bg-purple-500/5 border border-purple-500/10 rounded-2xl">
          <h4 class="text-sm font-bold text-purple-400 flex items-center gap-2 mb-3">
            <Info class="w-4 h-4" />
            支持的格式
          </h4>
          <p class="text-xs text-gray-400 leading-relaxed">
            目前支持 TXT (纯文本)、PDF (扫描件除外) 以及 JSON 格式。我们建议单个文件不超过 10MB 以获得最佳的解析速度和检索质量。
          </p>
        </div>
      </div>
    </div>

    <!-- 文档详情弹窗 -->
    <div v-if="isDetailModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 md:p-6 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div class="bg-gray-900 border border-gray-800 rounded-3xl w-full max-w-4xl max-h-[85vh] flex flex-col shadow-2xl zoom-in duration-300 overflow-hidden">
        <!-- 弹窗头部 -->
        <header class="p-6 border-b border-gray-800 flex items-center justify-between bg-gray-900/50">
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-2xl bg-blue-500/10 flex items-center justify-center text-blue-500">
              <FileText class="w-6 h-6" />
            </div>
            <div>
              <h3 class="text-xl font-bold text-white truncate max-w-md">{{ selectedFileName }}</h3>
              <p class="text-xs text-gray-500 mt-1">共 {{ fileChunks.length }} 个知识切片已建立索引</p>
            </div>
          </div>
          <button @click="isDetailModalOpen = false" class="p-2 hover:bg-gray-800 rounded-xl text-gray-400 hover:text-white transition-colors">
            <X class="w-6 h-6" />
          </button>
        </header>

        <!-- 弹窗内容 -->
        <div class="flex-1 overflow-y-auto p-6 space-y-4 bg-gray-950/30 custom-scrollbar">
          <!-- 正在加载状态 -->
          <div v-if="isDetailLoading" class="flex flex-col items-center justify-center py-20">
            <RefreshCw class="w-10 h-10 text-blue-500 animate-spin mb-4" />
            <p class="text-gray-400 text-sm">正在从向量数据库加载知识切片...</p>
          </div>

          <!-- 切片列表 -->
          <template v-else>
            <div v-for="(chunk, idx) in fileChunks" :key="idx" class="group bg-gray-900 border border-gray-800 rounded-2xl p-5 hover:border-blue-500/30 transition-all">
              <div class="flex items-center gap-2 mb-3">
                <span class="text-[10px] font-bold text-blue-500 bg-blue-500/10 px-2 py-0.5 rounded uppercase">切片 #{{ chunk.chunk_id + 1 }}</span>
                <div class="h-px flex-1 bg-gray-800 group-hover:bg-blue-500/10"></div>
              </div>
              <p class="text-sm text-gray-300 leading-relaxed font-sans">{{ chunk.text }}</p>
            </div>
          </template>
        </div>

        <!-- 弹窗底部 -->
        <footer class="p-4 border-t border-gray-800 bg-gray-900/50 flex justify-end gap-3">
          <button @click="isDetailModalOpen = false" class="px-8 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded-xl text-sm font-medium transition-all">
            关闭
          </button>
        </footer>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 自定义滚动条 */
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
</style>
