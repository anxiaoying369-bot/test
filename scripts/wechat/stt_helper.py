#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
语音转文字（STT）助手 —— 基于 sherpa-onnx + SenseVoice int8 ONNX 模型。

为什么不用 funasr/torch：funasr 依赖 torch（mac ~400MB / Windows ~1.5GB），会把
内置 Python 运行时撑到 >2GB，导致 Windows NSIS 安装包打包时 32 位偏移溢出
（"error mmapping file ... is out of range"）。sherpa-onnx 纯 ONNX 推理，wheel
仅 ~77MB、无 torch；模型 model.int8.onnx 仅 ~228MB，运行时按需从国内镜像下载。

模型不进安装包：下载到用户数据目录 <data>/models/sherpa-sense-voice/。
下载源：modelscope.cn（国内快），失败回退 hf-mirror.com。下载带真实字节进度。

对外接口（被 wechat_engine.py 调用，保持不变）：
  STTHelper.get_instance() / is_model_ready() / download_model() / transcribe(wav_path)
  进度通过 stderr 的 JSON（{"type":"stt_progress","percent","message"}）上报。
"""
import os
import sys
import json
import wave
import threading

# 模型文件 → 多镜像 URL（按顺序尝试）。SenseVoice 多语种(中英日韩粤) int8 onnx。
MODEL_FILES = {
    "model.int8.onnx": [
        "https://modelscope.cn/models/pengzhendong/sherpa-onnx-sense-voice-zh-en-ja-ko-yue/resolve/master/model.int8.onnx",
        "https://hf-mirror.com/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx",
    ],
    "tokens.txt": [
        "https://modelscope.cn/models/pengzhendong/sherpa-onnx-sense-voice-zh-en-ja-ko-yue/resolve/master/tokens.txt",
        "https://hf-mirror.com/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt",
    ],
}
# 完整模型的最小体积（字节）——用于判断是否下全（实际 ~228MB）
MODEL_MIN_SIZE = 200 * 1024 * 1024


def log_progress(current, total, message=""):
    """向 stderr 输出 JSON 进度，供 Rust 捕获后 emit 给前端。"""
    percent = int(current * 100 / total) if total > 0 else 0
    print(json.dumps({
        "type": "stt_progress",
        "percent": percent,
        "current": current,
        "total": total,
        "message": message,
    }), file=sys.stderr, flush=True)


def _fmt_mb(n):
    return f"{n / 1024 / 1024:.0f}MB"


class STTHelper:
    _instance = None
    _lock = threading.Lock()

    def __init__(self):
        self.recognizer = None
        self._load_lock = threading.Lock()

    @classmethod
    def get_instance(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = STTHelper()
            return cls._instance

    # ---- 路径 ----
    def get_model_dir(self):
        base_dir = os.environ.get("WECHAT_USER_DATA")
        if not base_dir:
            if sys.platform == "darwin":
                base_dir = os.path.expanduser("~/Library/Application Support/AutoCastAI/wechat")
            else:
                base_dir = os.path.join(os.environ.get("APPDATA", ""), "AutoCastAI", "wechat")
        # 放用户数据目录（不进安装包）：<data>/models/sherpa-sense-voice/
        return os.path.join(os.path.dirname(base_dir), "models", "sherpa-sense-voice")

    def is_model_ready(self):
        d = self.get_model_dir()
        onnx = os.path.join(d, "model.int8.onnx")
        tokens = os.path.join(d, "tokens.txt")
        try:
            return (os.path.exists(onnx) and os.path.getsize(onnx) >= MODEL_MIN_SIZE
                    and os.path.exists(tokens) and os.path.getsize(tokens) > 0)
        except OSError:
            return False

    # ---- 下载 ----
    @staticmethod
    def _remote_size(url):
        import urllib.request
        try:
            req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "AutoCastAI"})
            with urllib.request.urlopen(req, timeout=30) as r:
                return int(r.headers.get("Content-Length") or 0)
        except Exception:
            return 0

    def _download_one(self, urls, dest, base_done, grand_total):
        """下载单个文件到 dest（先写 .part 再原子重命名）；进度以 base_done 为起点累加。"""
        import urllib.request
        last_err = None
        for url in urls:
            try:
                req = urllib.request.Request(url, headers={"User-Agent": "AutoCastAI"})
                with urllib.request.urlopen(req, timeout=60) as r:
                    tmp = dest + ".part"
                    done = 0
                    with open(tmp, "wb") as f:
                        while True:
                            chunk = r.read(1024 * 256)
                            if not chunk:
                                break
                            f.write(chunk)
                            done += len(chunk)
                            cur = base_done + done
                            log_progress(cur, grand_total,
                                         f"下载语音模型 {_fmt_mb(cur)}/{_fmt_mb(grand_total)}")
                    os.replace(tmp, dest)
                    return done
            except Exception as e:
                last_err = e
                print(f"[stt] 下载失败 {url}: {e}", file=sys.stderr)
                continue
        raise RuntimeError(f"所有镜像下载失败: {last_err}")

    def download_model(self):
        """下载 SenseVoice onnx 模型（国内镜像优先），带真实字节进度。"""
        if self.is_model_ready():
            log_progress(100, 100, "模型已就绪")
            return True
        d = self.get_model_dir()
        os.makedirs(d, exist_ok=True)

        # 预先估算总大小（取首个镜像的 Content-Length）
        sizes = {name: self._remote_size(urls[0]) for name, urls in MODEL_FILES.items()}
        grand_total = sum(sizes.values()) or (MODEL_MIN_SIZE + 320 * 1024)

        log_progress(0, grand_total, "开始下载语音模型...")
        try:
            base = 0
            # 先下大文件后下小文件，进度更平滑
            for name in sorted(MODEL_FILES.keys(), key=lambda n: -sizes.get(n, 0)):
                dest = os.path.join(d, name)
                if os.path.exists(dest) and (name != "model.int8.onnx"
                                             or os.path.getsize(dest) >= MODEL_MIN_SIZE):
                    base += sizes.get(name, os.path.getsize(dest))
                    continue
                self._download_one(MODEL_FILES[name], dest, base, grand_total)
                base += sizes.get(name, 0)
            log_progress(grand_total, grand_total, "下载完成")
            return self.is_model_ready()
        except Exception as e:
            print(f"[stt] download_model 失败: {e}", file=sys.stderr)
            log_progress(0, 100, f"下载失败: {e}")
            return False

    # ---- 推理 ----
    def _ensure_recognizer(self):
        with self._load_lock:
            if self.recognizer is not None:
                return
            if not self.is_model_ready():
                raise RuntimeError("模型未就绪，请先下载")
            import sherpa_onnx
            d = self.get_model_dir()
            print("[stt] 加载 sherpa-onnx SenseVoice...", file=sys.stderr)
            self.recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                model=os.path.join(d, "model.int8.onnx"),
                tokens=os.path.join(d, "tokens.txt"),
                num_threads=2,
                use_itn=True,
                debug=False,
            )
            print("[stt] 模型已加载", file=sys.stderr)

    @staticmethod
    def _read_wav_16k(wav_path):
        """读 WAV → float32 单声道 16kHz（SenseVoice 要求 16k）。"""
        import numpy as np
        with wave.open(wav_path, "rb") as w:
            ch = w.getnchannels()
            sr = w.getframerate()
            sw = w.getsampwidth()
            raw = w.readframes(w.getnframes())
        if sw != 2:
            raise RuntimeError(f"不支持的采样位宽: {sw * 8}bit")
        x = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
        if ch > 1:
            x = x.reshape(-1, ch).mean(axis=1)
        if sr != 16000 and len(x) > 0:
            n_out = int(round(len(x) * 16000 / sr))
            x = np.interp(
                np.linspace(0, len(x), n_out, endpoint=False),
                np.arange(len(x)), x,
            ).astype(np.float32)
        return x, 16000

    def transcribe(self, wav_path):
        self._ensure_recognizer()
        try:
            samples, sr = self._read_wav_16k(wav_path)
            if len(samples) == 0:
                return ""
            stream = self.recognizer.create_stream()
            stream.accept_waveform(sr, samples)
            self.recognizer.decode_stream(stream)
            return (stream.result.text or "").strip()
        except Exception as e:
            print(f"[stt] 转写失败: {e}", file=sys.stderr)
            return f"[识别失败: {e}]"


def transcribe_wav(wav_path):
    return STTHelper.get_instance().transcribe(wav_path)


def check_stt_model():
    return STTHelper.get_instance().is_model_ready()


def download_stt_model():
    return STTHelper.get_instance().download_model()
