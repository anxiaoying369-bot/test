<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  FileText, Loader2, RefreshCw, CheckCircle2,
  Music, Zap, Clock, Gauge, Users, Tag, Search,
  Pencil, Check, X
} from 'lucide-vue-next';
import { convertFileSrc } from '@tauri-apps/api/core';

const props = defineProps<{
  productInfo: string;
  referenceScript: string;
  generatedScript: string;
  scriptFeedback: string;
  isGeneratingScript: boolean;
  scriptConfirmed: boolean;
  selectedPlatform: string;
  selectedScriptType: 'voiceover' | 'ai-video';
  videoRatio: string;
  ttsVoiceId: string;
  ttsSpeed: number;
  isSynthesizingVoice: boolean;
  isLoadingVoices: boolean;
  availableVoices: any[];
  latestVoiceoverPath: string | null;
  PLATFORM_OPTIONS: any[];
  SCRIPT_TYPE_OPTIONS: any[];
}>();

const emit = defineEmits<{
  (e: 'update:productInfo', val: string): void;
  (e: 'update:referenceScript', val: string): void;
  (e: 'update:scriptFeedback', val: string): void;
  (e: 'update:selectedPlatform', val: string): void;
  (e: 'update:selectedScriptType', val: 'voiceover' | 'ai-video'): void;
  (e: 'update:videoRatio', val: string): void;
  (e: 'update:ttsVoiceId', val: string): void;
  (e: 'update:ttsSpeed', val: number): void;
  (e: 'generateScript', feedback: boolean): void;
  (e: 'resetScriptFlow'): void;
  (e: 'confirmScript'): void;
  (e: 'loadVoices'): void;
  (e: 'synthesizeVoice'): void;
  (e: 'updateResolution', ratio: string): void;
  (e: 'saveScript', json: string): void;
}>();

// 脚本现在是固定 JSON 字符串，解析成结构化对象供卡片展示
interface ScriptData {
  视频标题?: string;
  总时长?: string;
  语速?: number | string;
  目标受众?: string;
  口播文案?: string;
  核心卖点关键词?: string[];
  建议素材关键词?: string[];
}
const scriptData = computed<ScriptData | null>(() => {
  if (!props.generatedScript) return null;
  try {
    return JSON.parse(props.generatedScript);
  } catch {
    return null;
  }
});
// JSON 解析失败时的兜底原文（极少数模型不听话）
const rawFallback = computed(() => (!scriptData.value ? props.generatedScript : ''));

const localProductInfo = computed({
  get: () => props.productInfo,
  set: (val) => emit('update:productInfo', val)
});

const localReferenceScript = computed({
  get: () => props.referenceScript,
  set: (val) => emit('update:referenceScript', val)
});

const localScriptFeedback = computed({
  get: () => props.scriptFeedback,
  set: (val) => emit('update:scriptFeedback', val)
});

const localTtsVoiceId = computed({
  get: () => props.ttsVoiceId,
  set: (val) => emit('update:ttsVoiceId', val)
});

const localTtsSpeed = computed({
  get: () => props.ttsSpeed,
  set: (val) => emit('update:ttsSpeed', val)
});

// 失焦时把语速夹紧到 [0.5, 2.0] 并保留两位小数
function normalizeSpeed() {
  let v = Number(props.ttsSpeed);
  if (!Number.isFinite(v)) v = 1.0;
  v = Math.min(2.0, Math.max(0.5, v));
  v = Math.round(v * 100) / 100;
  emit('update:ttsSpeed', v);
}

// ── 脚本手动编辑 ──
const isEditingScript = ref(false);
const editTitle = ref('');
const editVoiceText = ref('');

function startEditScript() {
  if (!scriptData.value) return;
  editTitle.value = scriptData.value.视频标题 || '';
  editVoiceText.value = scriptData.value.口播文案 || '';
  isEditingScript.value = true;
}
function cancelEditScript() {
  isEditingScript.value = false;
}
function saveEditScript() {
  const merged: ScriptData = { ...(scriptData.value || {}) };
  merged.视频标题 = editTitle.value.trim();
  merged.口播文案 = editVoiceText.value.trim();
  emit('saveScript', JSON.stringify(merged));
  isEditingScript.value = false;
}

// ── AI 视频流：开发中提示 ──
const showDevNotice = ref(false);
let devNoticeTimer: ReturnType<typeof setTimeout> | null = null;
function selectScriptType(id: 'voiceover' | 'ai-video') {
  if (id === 'ai-video') {
    showDevNotice.value = true;
    if (devNoticeTimer) clearTimeout(devNoticeTimer);
    devNoticeTimer = setTimeout(() => { showDevNotice.value = false; }, 3000);
    return; // 功能开发中，不切换
  }
  showDevNotice.value = false;
  emit('update:selectedScriptType', id);
}
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-6 animate-in fade-in slide-in-from-bottom-2">
    <!-- Step 1: 输入产品 + 参考脚本 + 视频比例 -->
    <div class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-2xl">
      <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center gap-2">
        <div class="w-6 h-6 rounded-full bg-blue-600 text-white text-xs font-bold flex items-center justify-center">1</div>
        <h3 class="text-sm font-bold text-gray-200">填写产品信息</h3>
      </div>

      <div class="p-6 space-y-5">
        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">要卖的产品 <span class="text-red-400">*</span></label>
          <textarea
            v-model="localProductInfo"
            :disabled="!!generatedScript"
            placeholder="例如：3 代花西子空气蜜粉，含珍珠粉成分，定妆 8 小时不脱妆，0.04mm 微细粉质，遮瑕力强..."
            class="w-full h-28 p-4 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none disabled:opacity-60"
          ></textarea>
          <p class="text-[11px] text-gray-600 mt-1">填得越详细，AI 越能写出有信息密度的脚本（卖点/规格/差异化）。</p>
        </div>

        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">参考脚本 <span class="text-gray-600">(可选)</span></label>
          <textarea
            v-model="localReferenceScript"
            :disabled="!!generatedScript"
            placeholder="如果有同类爆款脚本可粘贴在这里，AI 会借鉴节奏与表达，但不会照抄..."
            class="w-full h-24 p-4 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none disabled:opacity-60"
          ></textarea>
        </div>

        <!-- 目标平台 -->
        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">
            目标平台 <span class="text-[10px] text-gray-600">（影响剧本的语气、节奏、CTA 风格）</span>
          </label>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-2">
            <button
              v-for="p in PLATFORM_OPTIONS"
              :key="p.id"
              @click="emit('update:selectedPlatform', p.id)"
              :disabled="!!generatedScript"
              :class="[
                'p-3 rounded-xl border text-left transition-all disabled:opacity-60 disabled:cursor-not-allowed',
                selectedPlatform === p.id
                  ? 'bg-blue-600/15 border-blue-500/50 text-white'
                  : 'bg-gray-950 border-gray-800 text-gray-400 hover:border-gray-700'
              ]"
            >
              <div class="text-sm font-bold flex items-center gap-1.5">
                <span>{{ p.emoji }}</span> {{ p.label }}
              </div>
              <div class="text-[10px] text-gray-500 mt-0.5 leading-snug">{{ p.desc }}</div>
            </button>
          </div>
        </div>

        <!-- 剧本类型 -->
        <div>
          <label class="block text-xs font-medium text-gray-400 mb-2">
            剧本类型 <span class="text-[10px] text-gray-600">（决定后续走口播合成还是 AI 视频生成）</span>
          </label>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="t in SCRIPT_TYPE_OPTIONS"
              :key="t.id"
              @click="selectScriptType(t.id)"
              :disabled="!!generatedScript"
              :class="[
                'relative p-3 rounded-xl border text-left transition-all disabled:opacity-60 disabled:cursor-not-allowed',
                t.id === 'ai-video' ? 'cursor-not-allowed' : '',
                selectedScriptType === t.id
                  ? 'bg-purple-600/15 border-purple-500/50 text-white'
                  : 'bg-gray-950 border-gray-800 text-gray-400 hover:border-gray-700'
              ]"
            >
              <span v-if="t.id === 'ai-video'"
                    class="absolute top-2 right-2 text-[9px] font-bold px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-400 border border-amber-500/30">
                开发中
              </span>
              <div class="text-sm font-bold" :class="t.id === 'ai-video' ? 'text-gray-500' : ''">{{ t.label }}</div>
              <div class="text-[10px] text-gray-500 mt-0.5 leading-snug">{{ t.desc }}</div>
            </button>
          </div>
          <p v-if="showDevNotice"
             class="mt-2 text-[11px] text-amber-400 flex items-center gap-1.5 animate-in fade-in slide-in-from-top-1">
            <Zap class="w-3 h-3" /> AI 视频流功能开发中，敬请期待。当前请使用「口播带货」。
          </p>
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-1.5">
            <span class="text-[10px] text-gray-600 font-bold uppercase tracking-wider">视频比例</span>
            <div class="flex bg-gray-950 border border-gray-800 p-1 rounded-lg">
              <button
                v-for="r in ['9:16', '16:9', '1:1']"
                :key="r"
                @click="emit('updateResolution', r)"
                :disabled="!!generatedScript"
                :class="['px-4 py-1.5 rounded-md text-xs font-medium transition-all disabled:cursor-not-allowed', videoRatio === r ? 'bg-blue-600 text-white' : 'text-gray-500 hover:text-gray-300']"
              >{{ r }}</button>
            </div>
          </div>

          <button
            v-if="!generatedScript"
            @click="emit('generateScript', false)"
            :disabled="isGeneratingScript || !productInfo.trim()"
            class="bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white px-8 py-3 rounded-xl font-bold transition-all shadow-lg shadow-blue-900/30 flex items-center gap-2"
          >
            <Loader2 v-if="isGeneratingScript" class="w-4 h-4 animate-spin" />
            <FileText v-else class="w-4 h-4" />
            {{ isGeneratingScript ? 'AI 检索知识库并生成中...' : '生成脚本' }}
          </button>
          <button
            v-else
            @click="emit('resetScriptFlow')"
            class="text-xs text-gray-500 hover:text-gray-300 px-3 py-2 border border-gray-800 rounded-lg flex items-center gap-1"
          >
            <RefreshCw class="w-3.5 h-3.5" /> 重新输入
          </button>
        </div>
      </div>
    </div>

    <!-- Step 2: 预览 + 反馈重生成 -->
    <div v-if="generatedScript" class="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-2xl">
      <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <div class="w-6 h-6 rounded-full bg-blue-600 text-white text-xs font-bold flex items-center justify-center">2</div>
          <h3 class="text-sm font-bold text-gray-200">脚本预览</h3>
          <span v-if="scriptConfirmed" class="text-[10px] px-2 py-0.5 bg-green-500/20 text-green-400 rounded-full border border-green-500/30">已确认</span>
        </div>
        <!-- 手动编辑脚本 -->
        <div v-if="scriptData" class="flex items-center gap-2">
          <template v-if="!isEditingScript">
            <button @click="startEditScript"
                    class="text-xs text-gray-400 hover:text-blue-400 px-3 py-1.5 border border-gray-700 hover:border-blue-500/40 rounded-lg flex items-center gap-1.5 transition-all">
              <Pencil class="w-3.5 h-3.5" /> 编辑脚本
            </button>
          </template>
          <template v-else>
            <button @click="cancelEditScript"
                    class="text-xs text-gray-400 hover:text-gray-200 px-3 py-1.5 border border-gray-700 rounded-lg flex items-center gap-1.5 transition-all">
              <X class="w-3.5 h-3.5" /> 取消
            </button>
            <button @click="saveEditScript"
                    class="text-xs text-white bg-blue-600 hover:bg-blue-500 px-3 py-1.5 rounded-lg flex items-center gap-1.5 transition-all">
              <Check class="w-3.5 h-3.5" /> 保存修改
            </button>
          </template>
        </div>
      </div>

      <!-- JSON 卡片展示 -->
      <div v-if="scriptData" class="p-6 max-h-[520px] overflow-y-auto custom-scrollbar space-y-4">
        <!-- 标题 -->
        <div v-if="isEditingScript">
          <label class="block text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-1.5">视频标题</label>
          <input v-model="editTitle" type="text" placeholder="视频标题"
                 class="w-full bg-gray-950 border border-blue-500/40 rounded-lg px-3 py-2 text-lg font-bold text-white focus:outline-none focus:border-blue-500" />
        </div>
        <div v-else-if="scriptData.视频标题" class="text-lg font-bold text-white leading-snug">
          {{ scriptData.视频标题 }}
        </div>

        <!-- 元信息行：时长 / 语速 / 受众 -->
        <div class="flex flex-wrap gap-2">
          <span v-if="scriptData.总时长" class="inline-flex items-center gap-1.5 px-3 py-1.5 bg-blue-500/10 border border-blue-500/20 rounded-lg text-xs text-blue-300">
            <Clock class="w-3.5 h-3.5" /> {{ scriptData.总时长 }}
          </span>
          <span v-if="scriptData.语速" class="inline-flex items-center gap-1.5 px-3 py-1.5 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-xs text-emerald-300">
            <Gauge class="w-3.5 h-3.5" /> {{ scriptData.语速 }}x 语速
          </span>
          <span v-if="scriptData.目标受众" class="inline-flex items-center gap-1.5 px-3 py-1.5 bg-amber-500/10 border border-amber-500/20 rounded-lg text-xs text-amber-300">
            <Users class="w-3.5 h-3.5" /> {{ scriptData.目标受众 }}
          </span>
        </div>

        <!-- 口播文案 -->
        <div v-if="isEditingScript" class="bg-gray-950/60 border border-purple-500/40 rounded-xl p-4">
          <div class="flex items-center gap-1.5 text-[10px] font-bold text-purple-400 uppercase tracking-widest mb-2">
            <FileText class="w-3 h-3" /> 口播文案（可编辑，合成时以此为准）
          </div>
          <textarea v-model="editVoiceText" rows="6" placeholder="主播要念出来的完整口播文案..."
                    class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 leading-relaxed focus:outline-none focus:border-purple-500 resize-y"></textarea>
        </div>
        <div v-else-if="scriptData.口播文案" class="bg-gray-950/60 border border-gray-800 rounded-xl p-4">
          <div class="flex items-center gap-1.5 text-[10px] font-bold text-purple-400 uppercase tracking-widest mb-2">
            <FileText class="w-3 h-3" /> 口播文案
          </div>
          <p class="text-sm text-gray-200 leading-relaxed whitespace-pre-wrap">{{ scriptData.口播文案 }}</p>
        </div>

        <!-- 核心卖点关键词 -->
        <div v-if="scriptData.核心卖点关键词?.length">
          <div class="flex items-center gap-1.5 text-[10px] font-bold text-pink-400 uppercase tracking-widest mb-2">
            <Tag class="w-3 h-3" /> 核心卖点关键词
          </div>
          <div class="flex flex-wrap gap-2">
            <span v-for="(kw, i) in scriptData.核心卖点关键词" :key="i"
                  class="px-2.5 py-1 bg-pink-500/10 border border-pink-500/25 rounded-full text-xs text-pink-300">
              {{ kw }}
            </span>
          </div>
        </div>

        <!-- 建议素材关键词 -->
        <div v-if="scriptData.建议素材关键词?.length">
          <div class="flex items-center gap-1.5 text-[10px] font-bold text-cyan-400 uppercase tracking-widest mb-2">
            <Search class="w-3 h-3" /> 建议素材关键词
          </div>
          <div class="flex flex-wrap gap-2">
            <span v-for="(kw, i) in scriptData.建议素材关键词" :key="i"
                  class="px-2.5 py-1 bg-cyan-500/10 border border-cyan-500/25 rounded-full text-xs text-cyan-300 font-mono">
              {{ kw }}
            </span>
          </div>
        </div>
      </div>

      <!-- JSON 解析失败兜底：原文展示 -->
      <div v-else class="p-6 max-h-[400px] overflow-y-auto custom-scrollbar">
        <div class="text-[11px] text-amber-400 mb-2">⚠ 脚本未按标准 JSON 返回，显示原始内容：</div>
        <pre class="text-xs text-gray-300 whitespace-pre-wrap leading-relaxed">{{ rawFallback }}</pre>
      </div>

      <!-- 反馈 + 重生成 -->
      <div v-if="!scriptConfirmed" class="border-t border-gray-800 p-6 bg-gray-950/40 space-y-3">
        <label class="block text-xs font-medium text-gray-400">不满意？告诉 AI 怎么改</label>
        <textarea
          v-model="localScriptFeedback"
          placeholder="例如：开头钩子改成提问式；中段加上 30 天无理由退换；结尾去掉过于硬广的语气..."
          class="w-full h-20 p-3 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 placeholder-gray-700 focus:outline-none focus:border-blue-500 resize-none"
        ></textarea>
        <div class="flex items-center justify-end gap-3">
          <button
            @click="emit('generateScript', true)"
            :disabled="isGeneratingScript || !scriptFeedback.trim()"
            class="bg-gray-800 hover:bg-gray-700 disabled:opacity-50 text-white px-5 py-2.5 rounded-lg font-medium text-sm flex items-center gap-2 border border-gray-700"
          >
            <Loader2 v-if="isGeneratingScript" class="w-4 h-4 animate-spin" />
            <RefreshCw v-else class="w-4 h-4" />
            根据意见重新生成
          </button>
          <button
            @click="emit('confirmScript')"
            class="bg-green-600 hover:bg-green-500 text-white px-5 py-2.5 rounded-lg font-bold text-sm flex items-center gap-2 shadow-lg shadow-green-900/30"
          >
            <CheckCircle2 class="w-4 h-4" />
            确认脚本，进入下一步
          </button>
        </div>
      </div>
    </div>

    <!-- 口播剧本：TTS 合成 -->
    <div v-if="scriptConfirmed"
         class="bg-gray-900 border border-purple-500/30 rounded-2xl overflow-hidden shadow-2xl">
      <div class="px-6 py-4 bg-gray-800/50 border-b border-gray-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <div class="w-6 h-6 rounded-full bg-purple-600 text-white text-xs font-bold flex items-center justify-center">3</div>
          <h3 class="text-sm font-bold text-gray-200">合成口播音频（TTS）</h3>
        </div>
      </div>

      <div class="p-6 space-y-4">
        <!-- 音色 + 语速 -->
        <div class="grid grid-cols-3 gap-3">
          <div class="col-span-2">
            <label class="block text-xs font-medium text-gray-400 mb-2">音色</label>
            <div class="flex gap-2">
              <select v-model="localTtsVoiceId" :disabled="isSynthesizingVoice"
                      class="flex-1 bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200">
                <option value="">— 请选择音色 —</option>
                <option v-for="v in availableVoices" :key="v.id" :value="v.id">
                  {{ v.name }} ({{ v.id }})
                </option>
              </select>
              <button @click="emit('loadVoices')" :disabled="isLoadingVoices"
                      class="px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded-lg border border-gray-700 flex items-center gap-1"
                      title="刷新音色列表">
                <Loader2 v-if="isLoadingVoices" class="w-3.5 h-3.5 animate-spin" />
                <RefreshCw v-else class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
          <div>
            <label class="block text-xs font-medium text-gray-400 mb-2">语速</label>
            <input v-model.number="localTtsSpeed" type="number" step="0.01" min="0.5" max="2.0"
                   :disabled="isSynthesizingVoice"
                   @blur="normalizeSpeed"
                   class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200" />
            <p class="text-[10px] text-gray-600 mt-1">支持两位小数，如 1.25（范围 0.5 ~ 2.0）</p>
          </div>
        </div>

        <!-- 已合成的音频 -->
        <div v-if="latestVoiceoverPath"
             class="p-3 bg-purple-950/20 border border-purple-500/30 rounded-xl">
          <div class="flex items-center gap-3 mb-2">
            <Music class="w-4 h-4 text-purple-400" />
            <span class="text-xs font-bold text-purple-300">已合成口播音频</span>
            <span class="text-[10px] text-gray-600 font-mono ml-auto">{{ latestVoiceoverPath.split(/[/\\]/).pop() }}</span>
          </div>
          <audio :src="convertFileSrc(latestVoiceoverPath)" controls
                 class="w-full"></audio>
        </div>

        <!-- 合成按钮 -->
        <div class="flex items-center justify-between gap-4 pt-2">
          <p class="text-[11px] text-gray-500 flex-1 leading-relaxed">
            将脚本中的「口播文案」送入 TTS 合成为旁白音频，自动保存到素材库（可在「素材库」标签查看/试听）。
          </p>
          <button @click="emit('synthesizeVoice')"
                  :disabled="isSynthesizingVoice || !ttsVoiceId || !scriptData?.口播文案"
                  class="bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white px-6 py-2.5 rounded-xl font-bold text-sm flex items-center gap-2 shadow-lg shadow-purple-900/30 flex-shrink-0">
            <Loader2 v-if="isSynthesizingVoice" class="w-4 h-4 animate-spin" />
            <Music v-else class="w-4 h-4" />
            {{ isSynthesizingVoice ? '合成中...' : (latestVoiceoverPath ? '重新合成' : '合成口播音频') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 提示 -->
    <div v-if="!generatedScript" class="p-5 bg-blue-950/10 border border-blue-900/20 rounded-2xl flex gap-4">
      <div class="p-3 bg-blue-600/20 rounded-xl h-fit"><Zap class="w-5 h-5 text-blue-400" /></div>
      <div>
        <h4 class="font-bold text-blue-200 text-sm">工作流说明</h4>
        <p class="text-xs text-blue-400/80 mt-1 leading-relaxed">
          填写产品 → AI 会两次检索企业知识库（一次品牌资料、一次综合检索）→ 用 AI 助理同一个 LLM 生成口播脚本 →
          你可以多次反馈重生成 → 最后确认进入视频生成。
        </p>
      </div>
    </div>
  </div>
</template>
