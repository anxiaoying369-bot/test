<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Download, FileAudio, Loader2, Mic, Play, RefreshCw, Save,
  Sparkles, Volume2, Wand2
} from 'lucide-vue-next';

type JsonMap = Record<string, any>;
type SynthesisProgress = {
  running?: boolean;
  progress?: number | null;
  stage?: string | null;
  message?: string | null;
  audio_path?: string | null;
  error?: string | null;
};

const status = ref('');
const busy = ref(false);
const asrReady = ref(false);
const asrModelDir = ref('');
const selectedAsrFile = ref('');
const transcript = ref('');

const cloneReady = ref(false);
const cloneHint = ref('');
const cloneCli = ref('');
const voiceName = ref('我的音色');
const referenceAudio = ref('');
const referenceText = ref('');
const synthText = ref('你好，这是一段由 AutoCast AI 本地语音克隆生成的试听音频。');
const selectedVoice = ref('');
const voices = ref<JsonMap[]>([]);
const generatedAudio = ref('');
const synthProgress = ref<SynthesisProgress | null>(null);
const unlistenSynth = ref<UnlistenFn | null>(null);
const polishing = ref(false);

const generatedAudioUrl = computed(() => generatedAudio.value ? convertFileSrc(generatedAudio.value) : '');
const synthRunning = computed(() => !!synthProgress.value?.running);
const synthProgressValue = computed(() => {
  const value = synthProgress.value?.progress;
  if (typeof value !== 'number') return synthRunning.value ? 8 : 0;
  return Math.max(0, Math.min(100, value));
});
const synthProgressMessage = computed(() => synthProgress.value?.message || (synthRunning.value ? '语音克隆合成中' : ''));
const synthProgressStage = computed(() => synthProgress.value?.stage || (synthRunning.value ? 'running' : 'idle'));
const synthProgressError = computed(() => synthProgress.value?.error || '');

function applySynthesisProgress(next?: SynthesisProgress | null) {
  if (!next) return;
  synthProgress.value = { ...(synthProgress.value || {}), ...next };
  if (next.audio_path) {
    generatedAudio.value = next.audio_path;
  }
  if (next.running) {
    status.value = next.message || '语音克隆合成中';
  } else if (next.error) {
    status.value = `语音克隆合成：失败 - ${next.error}`;
  } else if (next.stage === 'done') {
    status.value = '语音克隆合成：完成';
  }
}

async function run<T>(label: string, fn: () => Promise<T>): Promise<T | null> {
  busy.value = true;
  status.value = label;
  try {
    const res = await fn();
    status.value = `${label}：完成`;
    return res;
  } catch (e: any) {
    status.value = `${label}：失败 - ${String(e)}`;
    return null;
  } finally {
    busy.value = false;
  }
}

async function checkAsr() {
  const res = await run<JsonMap>('检查语音识别模型', () => invoke('audio_asr_check_model'));
  if (!res) return;
  asrReady.value = !!res.ready;
  asrModelDir.value = res.model_dir || '';
}

async function downloadAsr() {
  const res = await run<JsonMap>('下载 SenseVoice 语音识别模型', () => invoke('audio_asr_download_model'));
  if (!res) return;
  asrReady.value = !!res.ready;
  asrModelDir.value = res.model_dir || '';
}

async function pickAsrFile() {
  const p = await open({ multiple: false, filters: [{ name: 'Audio', extensions: ['wav', 'mp3', 'm4a', 'aac', 'flac', 'ogg'] }] });
  if (typeof p === 'string') selectedAsrFile.value = p;
}

async function transcribe() {
  if (!selectedAsrFile.value) { status.value = '请先选择音频文件'; return; }
  const res = await run<JsonMap>('语音识别', () => invoke('audio_transcribe_file', { path: selectedAsrFile.value }));
  if (!res) return;
  transcript.value = res.text || '';
}

async function checkClone() {
  const res = await run<JsonMap>('检查 F5-TTS 语音克隆环境', () => invoke('voice_clone_check'));
  if (!res) return;
  cloneReady.value = !!res.ready;
  cloneHint.value = res.install_hint || '';
  cloneCli.value = res.cli || '';
}

async function pickReferenceAudio() {
  const p = await open({ multiple: false, filters: [{ name: 'Audio', extensions: ['wav', 'mp3', 'm4a', 'aac', 'flac'] }] });
  if (typeof p === 'string') referenceAudio.value = p;
}

async function registerVoice() {
  if (!voiceName.value.trim()) { status.value = '请填写音色名称'; return; }
  if (!referenceAudio.value) { status.value = '请选择参考音频'; return; }
  if (!referenceText.value.trim()) { status.value = '请填写参考音频对应文本'; return; }
  const res = await run<JsonMap>('注册克隆音色', () => invoke('voice_clone_register', {
    name: voiceName.value,
    referenceAudio: referenceAudio.value,
    referenceText: referenceText.value,
  }));
  if (!res) return;
  await listVoices();
  selectedVoice.value = res.voice?.name || selectedVoice.value;
}

async function listVoices() {
  const res = await run<JsonMap>('读取本地音色', () => invoke('voice_clone_list'));
  if (!res) return;
  voices.value = res.voices || [];
  if (!selectedVoice.value && voices.value.length) selectedVoice.value = voices.value[0].name;
}

async function polishSynthText() {
  if (!synthText.value.trim()) { status.value = '请输入需要润色的文本'; return; }
  polishing.value = true;
  status.value = 'AI 正在润色合成文本';
  try {
    const res = await invoke<JsonMap>('audio_polish_speech_text', { text: synthText.value });
    const polished = String(res.text || '').trim();
    if (!polished) throw new Error('LLM 返回内容为空');
    synthText.value = polished;
    status.value = 'AI 润色：完成';
  } catch (e: any) {
    status.value = `AI 润色：失败 - ${String(e)}`;
  } finally {
    polishing.value = false;
  }
}

async function synthesize() {
  if (!selectedVoice.value) { status.value = '请先注册/选择音色'; return; }
  if (!synthText.value.trim()) { status.value = '请输入合成文本'; return; }
  generatedAudio.value = '';
  synthProgress.value = {
    running: true,
    progress: 1,
    stage: 'start',
    message: '准备语音克隆合成',
    error: null,
  };
  const res = await run<JsonMap>('语音克隆合成', () => invoke('voice_clone_synthesize', {
    voice: selectedVoice.value,
    text: synthText.value,
    output: null,
  }));
  if (!res) return;
  generatedAudio.value = res.audio_path || '';
}

onMounted(async () => {
  unlistenSynth.value = await listen<SynthesisProgress>('voice-clone-synthesis-progress', (event) => {
    applySynthesisProgress(event.payload);
  });
  await checkAsr();
  await checkClone();
  await listVoices();
});

onBeforeUnmount(() => {
  unlistenSynth.value?.();
  unlistenSynth.value = null;
});
</script>

<template>
  <div class="flex-1 overflow-y-auto bg-gray-950 text-gray-100">
    <div class="max-w-6xl mx-auto p-8 space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold flex items-center gap-3">
            <Mic class="w-7 h-7 text-blue-400" /> 本地音频实验室
          </h1>
          <p class="text-gray-400 mt-2">接入本地语音识别（SenseVoice/sherpa-onnx）和语音克隆（F5-TTS）。模型按需下载，不打进安装包。</p>
        </div>
        <button @click="() => { checkAsr(); checkClone(); listVoices(); }" class="px-4 py-2 rounded-xl border border-gray-700 hover:border-gray-500 flex items-center gap-2">
          <RefreshCw class="w-4 h-4" />刷新状态
        </button>
      </div>

      <div v-if="status" class="rounded-xl border border-gray-800 bg-gray-900/70 px-4 py-3 text-sm" :class="status.includes('失败') ? 'text-red-300' : 'text-gray-300'">
        <Loader2 v-if="busy" class="w-4 h-4 inline mr-2 animate-spin" />{{ status }}
      </div>

      <section class="rounded-2xl border border-gray-800 bg-gray-900/50 p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-lg font-semibold flex items-center gap-2"><FileAudio class="w-5 h-5 text-emerald-400" />语音识别 ASR</h2>
            <p class="text-sm text-gray-400 mt-1">SenseVoice int8 ONNX，本地离线识别，复用微信语音转文字模型。</p>
          </div>
          <span :class="['px-3 py-1 rounded-full text-xs', asrReady ? 'bg-emerald-500/15 text-emerald-300' : 'bg-yellow-500/15 text-yellow-300']">
            {{ asrReady ? '模型已就绪' : '模型未下载' }}
          </span>
        </div>
        <div class="text-xs text-gray-500 break-all">模型目录：{{ asrModelDir || '-' }}</div>
        <div class="flex flex-wrap gap-3">
          <button @click="downloadAsr" :disabled="busy" class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-xl flex items-center gap-2">
            <Download class="w-4 h-4" />下载/修复模型
          </button>
          <button @click="pickAsrFile" class="px-4 py-2 border border-gray-700 hover:border-gray-500 rounded-xl">选择音频</button>
          <button @click="transcribe" :disabled="busy || !selectedAsrFile" class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded-xl flex items-center gap-2">
            <Wand2 class="w-4 h-4" />开始识别
          </button>
        </div>
        <div class="text-sm text-gray-400 break-all">当前音频：{{ selectedAsrFile || '未选择' }}</div>
        <textarea v-model="transcript" rows="5" class="w-full bg-gray-950 border border-gray-800 rounded-xl p-3 text-sm" placeholder="识别结果会显示在这里" />
      </section>

      <section class="rounded-2xl border border-gray-800 bg-gray-900/50 p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-lg font-semibold flex items-center gap-2"><Sparkles class="w-5 h-5 text-purple-400" />语音克隆 Voice Clone</h2>
            <p class="text-sm text-gray-400 mt-1">F5-TTS 本地推理：上传参考音频 + 对应文本，注册为本地音色后合成。</p>
          </div>
          <span :class="['px-3 py-1 rounded-full text-xs', cloneReady ? 'bg-emerald-500/15 text-emerald-300' : 'bg-yellow-500/15 text-yellow-300']">
            {{ cloneReady ? 'F5-TTS 可用' : 'F5-TTS 未安装' }}
          </span>
        </div>
        <div v-if="cloneCli" class="text-xs text-gray-500 break-all">CLI：{{ cloneCli }}</div>
        <div v-if="cloneHint" class="rounded-xl bg-yellow-500/10 border border-yellow-500/30 text-yellow-200 text-sm p-3">
          {{ cloneHint }}
          <div class="mt-1 text-yellow-100/90">可以在「系统设置 → 系统组件」页面进行下载。</div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-5">
          <div class="space-y-3">
            <h3 class="font-medium">1. 注册参考音色</h3>
            <input v-model="voiceName" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-3 py-2" placeholder="音色名称" />
            <div class="flex gap-2">
              <button @click="pickReferenceAudio" class="px-4 py-2 border border-gray-700 hover:border-gray-500 rounded-xl">选择参考音频</button>
              <button @click="registerVoice" :disabled="busy" class="px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 rounded-xl flex items-center gap-2">
                <Save class="w-4 h-4" />注册音色
              </button>
            </div>
            <div class="text-xs text-gray-500 break-all">{{ referenceAudio || '建议 5-15 秒清晰人声 WAV/MP3' }}</div>
            <textarea v-model="referenceText" rows="4" class="w-full bg-gray-950 border border-gray-800 rounded-xl p-3 text-sm" placeholder="参考音频逐字文本（必须和音频内容一致）" />
          </div>

          <div class="space-y-3">
            <h3 class="font-medium">2. 选择音色并合成</h3>
            <select v-model="selectedVoice" class="w-full bg-gray-950 border border-gray-800 rounded-xl px-3 py-2">
              <option value="" disabled>请选择音色</option>
              <option v-for="v in voices" :key="v.name" :value="v.name">{{ v.display_name || v.name }}</option>
            </select>
            <div class="space-y-2">
              <div class="flex items-center justify-between gap-3">
                <span class="text-xs text-gray-500">合成文本 · AI 润色会让句子更自然顺口</span>
                <button
                  @click="polishSynthText"
                  :disabled="busy || polishing || synthRunning || !synthText.trim()"
                  class="px-3 py-1.5 border border-blue-500/40 text-blue-200 hover:bg-blue-500/10 disabled:opacity-50 rounded-lg text-xs flex items-center gap-1.5"
                >
                  <Loader2 v-if="polishing" class="w-3.5 h-3.5 animate-spin" />
                  <Sparkles v-else class="w-3.5 h-3.5" />AI 润色
                </button>
              </div>
              <textarea v-model="synthText" rows="5" class="w-full bg-gray-950 border border-gray-800 rounded-xl p-3 text-sm" placeholder="输入要合成的文本" />
            </div>
            <button @click="synthesize" :disabled="busy || !cloneReady || synthRunning" class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded-xl flex items-center gap-2">
              <Loader2 v-if="synthRunning" class="w-4 h-4 animate-spin" />
              <Volume2 v-else class="w-4 h-4" />{{ synthRunning ? '合成中…' : '克隆合成' }}
            </button>
            <div v-if="synthProgress" class="rounded-xl border border-blue-500/20 bg-blue-950/10 p-4 space-y-3">
              <div class="flex items-center justify-between text-xs">
                <span class="text-blue-200 font-medium">合成进度：{{ synthProgressStage }}</span>
                <span class="text-blue-300 font-mono">{{ synthProgressValue }}%</span>
              </div>
              <div class="h-2 rounded-full bg-gray-800 overflow-hidden">
                <div class="h-full bg-blue-500 transition-all duration-300" :style="{ width: `${synthProgressValue}%` }"></div>
              </div>
              <div class="text-xs text-gray-300 break-all">
                <Loader2 v-if="synthRunning" class="w-3.5 h-3.5 inline mr-1 animate-spin text-blue-300" />
                {{ synthProgressMessage }}
              </div>
              <div v-if="synthProgressError" class="text-xs text-red-300 whitespace-pre-wrap break-all">{{ synthProgressError }}</div>
            </div>
            <div v-if="generatedAudio" class="space-y-2">
              <div class="text-xs text-gray-500 break-all">输出：{{ generatedAudio }}</div>
              <audio :src="generatedAudioUrl" controls class="w-full" />
              <div class="text-xs text-gray-400 flex items-center gap-1"><Play class="w-3 h-3" />可直接试听，文件已保存在本地数据目录。</div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
