<script setup lang="ts">
import { computed } from 'vue';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import {
  Film, FileText, Tag, Settings2, Sparkles, Loader2, Plus, X,
  RefreshCw, ArrowRight, ArrowLeft, Wand2, Upload, FolderOpen, CheckCircle2, AlertTriangle,
} from 'lucide-vue-next';

import ProjectSidebar from './video-studio/Sidebar.vue';
import { useVideoStudioView } from '../composables/useVideoStudioView';

const vm = useVideoStudioView();
const {
  PLATFORM_OPTIONS, ASPECT_OPTIONS, EDGE_VOICES, BGM_OPTIONS, SUBTITLE_PROVIDER_OPTIONS,
  projects, currentProject, createProject, selectProject, deleteProject,
  videoMaterials, isUploadingMaterial, deleteMaterial,
  step,
  productInfo, referenceScript, scriptText, scriptFeedback, isGeneratingScript, selectedPlatform, videoAspect,
  terms, newTerm, isGeneratingTerms,
  videoSource, voiceName, voiceRate, subtitleEnabled, subtitleProvider, subtitlePosition,
  textForeColor, strokeColor, fontSize, bgmType, bgmVolume, clipDuration, concatMode, videoCount,
  selectedLocalMaterialIds,
  isGenerating, progress, stageLabel, finalVideoPath, errorMsg,
  canProceedFromScript, canGenerate,
  generateScript, confirmScriptStep, generateTerms, addTerm, removeTerm,
  uploadLocalMaterial, toggleLocalMaterial, startGenerate,
} = vm;

const STEPS = [
  { id: 'script', n: '脚本', i: FileText },
  { id: 'keywords', n: '关键词', i: Tag },
  { id: 'options', n: '参数', i: Settings2 },
  { id: 'generate', n: '生成', i: Film },
] as const;

const stepIndex = computed(() => STEPS.findIndex(s => s.id === step.value));

const openInFinder = async () => {
  if (finalVideoPath.value) {
    try { await invoke('open_file_in_finder', { path: finalVideoPath.value }); } catch (e) { alert('打开失败: ' + e); }
  }
};
</script>

<template>
  <div class="h-full flex bg-gray-950 text-gray-100 overflow-hidden">
    <ProjectSidebar
      :projects="projects"
      :currentProject="currentProject"
      @create="createProject"
      @select="selectProject"
      @delete="deleteProject"
    />

    <!-- 无项目 -->
    <div v-if="!currentProject" class="flex-1 flex flex-col items-center justify-center">
      <div class="p-8 rounded-full bg-gray-900/50 mb-6"><Film class="w-16 h-16 text-gray-700" /></div>
      <h3 class="text-xl font-bold text-gray-300">选择或创建一个项目开始</h3>
      <p class="text-gray-500 mt-2">输入主题，自动生成脚本 · 配音 · 字幕 · 素材拼接成片（MoneyPrinterTurbo 引擎）</p>
      <button @click="createProject" class="mt-8 bg-blue-600 hover:bg-blue-700 text-white px-8 py-3 rounded-xl font-bold transition-all shadow-xl shadow-blue-900/30">创建第一个项目</button>
    </div>

    <div v-else class="flex-1 flex flex-col">
      <!-- 头部 + 步骤条 -->
      <div class="px-8 pt-6 pb-4 border-b border-gray-800 bg-gray-950/60 backdrop-blur">
        <div class="flex items-center gap-3 mb-5">
          <div class="p-2.5 bg-blue-600/10 rounded-xl border border-blue-500/20"><Film class="w-5 h-5 text-blue-400" /></div>
          <div>
            <h2 class="text-sm font-bold text-white">{{ currentProject.title }}</h2>
            <p class="text-[10px] text-gray-500 uppercase tracking-widest mt-0.5">素材拼接成片 · MPT Engine</p>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <template v-for="(s, idx) in STEPS" :key="s.id">
            <button
              @click="step = s.id"
              :class="['flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium border transition-all',
                step === s.id ? 'bg-blue-600/15 border-blue-500/40 text-blue-300'
                : idx < stepIndex ? 'border-gray-700 text-gray-300' : 'border-transparent text-gray-600']">
              <component :is="s.i" class="w-4 h-4" />{{ s.n }}
            </button>
            <div v-if="idx < STEPS.length - 1" class="w-6 h-px bg-gray-800" />
          </template>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
        <!-- ───── 步骤 1：脚本 ───── -->
        <div v-if="step === 'script'" class="max-w-3xl space-y-6">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="text-xs text-gray-400 font-medium">视频画幅</label>
              <select v-model="videoAspect" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option v-for="a in ASPECT_OPTIONS" :key="a.id" :value="a.id">{{ a.label }} · {{ a.hint }}</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-gray-400 font-medium">平台风格</label>
              <select v-model="selectedPlatform" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option v-for="p in PLATFORM_OPTIONS" :key="p.id" :value="p.id">{{ p.emoji }} {{ p.label }}</option>
              </select>
            </div>
          </div>

          <div>
            <label class="text-xs text-gray-400 font-medium">主题 / 产品信息</label>
            <textarea v-model="productInfo" rows="3" placeholder="例如：金钱的作用 / 一款主打深层清洁的氨基酸洗面奶……"
              class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm resize-none" />
          </div>
          <div>
            <label class="text-xs text-gray-400 font-medium">参考脚本 / 期望方向（可选）</label>
            <textarea v-model="referenceScript" rows="2" placeholder="给定方向或留空让 AI 自由发挥"
              class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm resize-none" />
          </div>

          <div class="flex gap-3">
            <button @click="generateScript(false)" :disabled="isGeneratingScript"
              class="bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-5 py-2.5 rounded-xl text-sm font-bold flex items-center gap-2">
              <Loader2 v-if="isGeneratingScript" class="w-4 h-4 animate-spin" /><Sparkles v-else class="w-4 h-4" />
              {{ scriptText ? '重新生成脚本' : 'AI 生成脚本' }}
            </button>
          </div>

          <div v-if="scriptText" class="space-y-3">
            <label class="text-xs text-gray-400 font-medium">口播文案（可编辑，将用于配音与字幕）</label>
            <textarea v-model="scriptText" rows="7" class="w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm leading-relaxed" />
            <div class="flex gap-2">
              <input v-model="scriptFeedback" placeholder="对脚本不满意？输入修改意见后点「按反馈重写」"
                class="flex-1 bg-gray-900 border border-gray-800 rounded-xl px-3 py-2 text-sm" />
              <button @click="generateScript(true)" :disabled="isGeneratingScript || !scriptFeedback.trim()"
                class="border border-gray-700 hover:border-gray-500 disabled:opacity-40 px-4 py-2 rounded-xl text-sm flex items-center gap-2">
                <RefreshCw class="w-4 h-4" />按反馈重写
              </button>
            </div>
          </div>

          <div class="flex justify-end pt-2">
            <button @click="confirmScriptStep" :disabled="!canProceedFromScript"
              class="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-40 text-white px-6 py-2.5 rounded-xl text-sm font-bold flex items-center gap-2">
              下一步<ArrowRight class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- ───── 步骤 2：关键词 ───── -->
        <div v-else-if="step === 'keywords'" class="max-w-3xl space-y-6">
          <div>
            <h3 class="text-base font-bold text-white">素材搜索关键词</h3>
            <p class="text-xs text-gray-500 mt-1">用于在 Pexels 素材库检索匹配画面。可增删，建议 3–6 个。</p>
          </div>
          <div class="flex flex-wrap gap-2">
            <span v-for="t in terms" :key="t" class="flex items-center gap-1.5 bg-blue-600/15 border border-blue-500/30 text-blue-200 px-3 py-1.5 rounded-lg text-sm">
              {{ t }}<button @click="removeTerm(t)" class="hover:text-red-400"><X class="w-3.5 h-3.5" /></button>
            </span>
            <span v-if="terms.length === 0" class="text-sm text-gray-600">暂无关键词，点下方「AI 生成关键词」或手动添加</span>
          </div>
          <div class="flex gap-2">
            <input v-model="newTerm" @keyup.enter="addTerm" placeholder="手动添加关键词后回车"
              class="flex-1 bg-gray-900 border border-gray-800 rounded-xl px-3 py-2 text-sm" />
            <button @click="addTerm" class="border border-gray-700 hover:border-gray-500 px-4 py-2 rounded-xl text-sm flex items-center gap-1.5"><Plus class="w-4 h-4" />添加</button>
            <button @click="generateTerms" :disabled="isGeneratingTerms"
              class="bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-4 py-2 rounded-xl text-sm flex items-center gap-1.5">
              <Loader2 v-if="isGeneratingTerms" class="w-4 h-4 animate-spin" /><Wand2 v-else class="w-4 h-4" />AI 生成关键词
            </button>
          </div>
          <div class="flex justify-between pt-2">
            <button @click="step = 'script'" class="border border-gray-700 hover:border-gray-500 px-5 py-2.5 rounded-xl text-sm flex items-center gap-2"><ArrowLeft class="w-4 h-4" />上一步</button>
            <button @click="step = 'options'" :disabled="terms.length === 0"
              class="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-40 text-white px-6 py-2.5 rounded-xl text-sm font-bold flex items-center gap-2">下一步<ArrowRight class="w-4 h-4" /></button>
          </div>
        </div>

        <!-- ───── 步骤 3：参数 ───── -->
        <div v-else-if="step === 'options'" class="max-w-3xl space-y-6">
          <!-- 素材来源 -->
          <div>
            <label class="text-xs text-gray-400 font-medium">素材来源</label>
            <div class="mt-2 flex gap-3">
              <button @click="videoSource = 'pexels'" :class="['flex-1 px-4 py-3 rounded-xl border text-sm text-left', videoSource === 'pexels' ? 'bg-blue-600/15 border-blue-500/40 text-blue-200' : 'border-gray-800 text-gray-400']">
                <div class="font-bold">Pexels 在线素材库</div><div class="text-xs opacity-70 mt-0.5">按关键词自动下载免版权高清视频（需 Pexels Key）</div>
              </button>
              <button @click="videoSource = 'local'" :class="['flex-1 px-4 py-3 rounded-xl border text-sm text-left', videoSource === 'local' ? 'bg-blue-600/15 border-blue-500/40 text-blue-200' : 'border-gray-800 text-gray-400']">
                <div class="font-bold">本地素材</div><div class="text-xs opacity-70 mt-0.5">使用你上传的视频素材进行拼接</div>
              </button>
            </div>
          </div>

          <!-- 本地素材选择 -->
          <div v-if="videoSource === 'local'" class="bg-gray-900/50 border border-gray-800 rounded-xl p-4 space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-sm font-medium text-gray-300">本地视频素材（勾选要使用的）</span>
              <button @click="uploadLocalMaterial" :disabled="isUploadingMaterial" class="text-xs border border-gray-700 hover:border-gray-500 px-3 py-1.5 rounded-lg flex items-center gap-1.5">
                <Loader2 v-if="isUploadingMaterial" class="w-3.5 h-3.5 animate-spin" /><Upload v-else class="w-3.5 h-3.5" />上传视频
              </button>
            </div>
            <div v-if="videoMaterials.length === 0" class="text-xs text-gray-600 py-4 text-center">还没有视频素材，点右上角上传</div>
            <div v-else class="grid grid-cols-4 gap-2">
              <div v-for="m in videoMaterials" :key="m.id"
                @click="toggleLocalMaterial(m.id)"
                :class="['relative rounded-lg overflow-hidden border-2 cursor-pointer aspect-video bg-black group', selectedLocalMaterialIds.includes(m.id) ? 'border-blue-500' : 'border-transparent']">
                <video v-if="m.local_path" :src="convertFileSrc(m.local_path)" class="w-full h-full object-cover" muted />
                <CheckCircle2 v-if="selectedLocalMaterialIds.includes(m.id)" class="absolute top-1 right-1 w-4 h-4 text-blue-400" />
                <button @click.stop="deleteMaterial(currentProject!.id, m.id)" class="absolute bottom-1 right-1 opacity-0 group-hover:opacity-100 bg-black/60 p-1 rounded text-red-400"><X class="w-3 h-3" /></button>
              </div>
            </div>
          </div>

          <!-- 配音 -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="text-xs text-gray-400 font-medium">配音音色（Edge TTS · 免费）</label>
              <select v-model="voiceName" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option v-for="v in EDGE_VOICES" :key="v.id" :value="v.id">{{ v.name }}</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-gray-400 font-medium">语速 {{ voiceRate.toFixed(2) }}x</label>
              <input type="range" min="0.5" max="1.5" step="0.05" v-model.number="voiceRate" class="mt-3 w-full accent-blue-500" />
            </div>
          </div>

          <!-- 字幕 -->
          <div class="bg-gray-900/50 border border-gray-800 rounded-xl p-4 space-y-3">
            <label class="flex items-center gap-2 text-sm font-medium text-gray-300">
              <input type="checkbox" v-model="subtitleEnabled" class="accent-blue-500 w-4 h-4" />烧录字幕
            </label>
            <div v-if="subtitleEnabled" class="grid grid-cols-2 gap-4">
              <div>
                <label class="text-xs text-gray-400">字幕生成方式</label>
                <select v-model="subtitleProvider" class="mt-1 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2 text-sm">
                  <option v-for="o in SUBTITLE_PROVIDER_OPTIONS" :key="o.id" :value="o.id">{{ o.label }}</option>
                </select>
              </div>
              <div>
                <label class="text-xs text-gray-400">位置</label>
                <select v-model="subtitlePosition" class="mt-1 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2 text-sm">
                  <option value="bottom">底部</option><option value="center">居中</option><option value="top">顶部</option>
                </select>
              </div>
              <div>
                <label class="text-xs text-gray-400">字号 {{ fontSize }}</label>
                <input type="range" min="30" max="100" step="2" v-model.number="fontSize" class="mt-2 w-full accent-blue-500" />
              </div>
              <div class="flex gap-4 items-end">
                <div><label class="text-xs text-gray-400 block mb-1">字色</label><input type="color" v-model="textForeColor" class="w-10 h-9 rounded bg-transparent" /></div>
                <div><label class="text-xs text-gray-400 block mb-1">描边</label><input type="color" v-model="strokeColor" class="w-10 h-9 rounded bg-transparent" /></div>
              </div>
            </div>
          </div>

          <!-- 背景音乐 + 拼接 -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="text-xs text-gray-400 font-medium">背景音乐</label>
              <select v-model="bgmType" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option v-for="b in BGM_OPTIONS" :key="b.id" :value="b.id">{{ b.label }}</option>
              </select>
            </div>
            <div v-if="bgmType">
              <label class="text-xs text-gray-400 font-medium">BGM 音量 {{ bgmVolume.toFixed(2) }}</label>
              <input type="range" min="0" max="1" step="0.05" v-model.number="bgmVolume" class="mt-3 w-full accent-blue-500" />
            </div>
          </div>
          <div class="grid grid-cols-3 gap-4">
            <div>
              <label class="text-xs text-gray-400 font-medium">单段时长 {{ clipDuration }}s</label>
              <input type="range" min="2" max="10" step="1" v-model.number="clipDuration" class="mt-3 w-full accent-blue-500" />
            </div>
            <div>
              <label class="text-xs text-gray-400 font-medium">拼接顺序</label>
              <select v-model="concatMode" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option value="random">随机</option><option value="sequential">顺序</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-gray-400 font-medium">生成数量</label>
              <select v-model.number="videoCount" class="mt-1.5 w-full bg-gray-900 border border-gray-800 rounded-xl px-3 py-2.5 text-sm">
                <option :value="1">1 条</option><option :value="2">2 条</option><option :value="3">3 条</option>
              </select>
            </div>
          </div>

          <div class="flex justify-between pt-2">
            <button @click="step = videoSource === 'local' ? 'script' : 'keywords'" class="border border-gray-700 hover:border-gray-500 px-5 py-2.5 rounded-xl text-sm flex items-center gap-2"><ArrowLeft class="w-4 h-4" />上一步</button>
            <button @click="step = 'generate'" class="bg-emerald-600 hover:bg-emerald-700 text-white px-6 py-2.5 rounded-xl text-sm font-bold flex items-center gap-2">下一步<ArrowRight class="w-4 h-4" /></button>
          </div>
        </div>

        <!-- ───── 步骤 4：生成 ───── -->
        <div v-else-if="step === 'generate'" class="max-w-2xl space-y-6">
          <div class="bg-gray-900/50 border border-gray-800 rounded-xl p-5 text-sm space-y-1.5">
            <div class="flex justify-between"><span class="text-gray-500">主题</span><span class="text-gray-200 truncate max-w-[70%]">{{ productInfo || '—' }}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">画幅 / 音色</span><span class="text-gray-200">{{ videoAspect }} · {{ EDGE_VOICES.find(v => v.id === voiceName)?.name }}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">素材来源</span><span class="text-gray-200">{{ videoSource === 'pexels' ? `Pexels（${terms.length} 关键词）` : `本地（${selectedLocalMaterialIds.length} 个素材）` }}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">字幕 / BGM</span><span class="text-gray-200">{{ subtitleEnabled ? subtitleProvider : '关闭' }} · {{ bgmType ? '随机BGM' : '无BGM' }}</span></div>
          </div>

          <button @click="startGenerate" :disabled="isGenerating || !canGenerate"
            class="w-full bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white py-3.5 rounded-xl font-bold flex items-center justify-center gap-2 shadow-xl shadow-blue-900/30">
            <Loader2 v-if="isGenerating" class="w-5 h-5 animate-spin" /><Film v-else class="w-5 h-5" />
            {{ isGenerating ? '生成中…' : '开始生成视频' }}
          </button>

          <div v-if="isGenerating || progress > 0" class="space-y-2">
            <div class="flex justify-between text-xs text-gray-400"><span>{{ stageLabel || '处理中' }}</span><span>{{ progress }}%</span></div>
            <div class="h-2 bg-gray-800 rounded-full overflow-hidden"><div class="h-full bg-blue-500 transition-all duration-300" :style="{ width: progress + '%' }" /></div>
          </div>

          <div v-if="errorMsg" class="bg-red-500/10 border border-red-500/30 text-red-300 rounded-xl p-4 text-sm flex gap-2">
            <AlertTriangle class="w-5 h-5 shrink-0" /><div>{{ errorMsg }}</div>
          </div>

          <div v-if="finalVideoPath && !isGenerating" class="space-y-3">
            <div class="flex items-center gap-2 text-emerald-400 text-sm font-medium"><CheckCircle2 class="w-5 h-5" />生成完成</div>
            <video :src="convertFileSrc(finalVideoPath)" controls class="w-full rounded-xl bg-black max-h-[60vh]" />
            <button @click="openInFinder" class="border border-gray-700 hover:border-gray-500 px-4 py-2 rounded-xl text-sm flex items-center gap-2"><FolderOpen class="w-4 h-4" />在访达中显示</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #1f2937; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #374151; }
</style>
