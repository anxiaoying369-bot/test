#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os
import sys
import threading
import logging
import json

# 屏蔽 modelscope/funasr 的冗余日志
logging.getLogger('modelscope').setLevel(logging.ERROR)
logging.getLogger('funasr').setLevel(logging.ERROR)

def log_progress(current, total, message=""):
    """向 stderr 输出 JSON 格式的进度，供 Rust/Vue 捕获"""
    percent = int(current * 100 / total) if total > 0 else 0
    print(json.dumps({
        "type": "stt_progress",
        "percent": percent,
        "current": current,
        "total": total,
        "message": message
    }), file=sys.stderr, flush=True)

class STTHelper:
    _instance = None
    _lock = threading.Lock()

    def __init__(self):
        self.model = None
        self._load_lock = threading.Lock()

    @classmethod
    def get_instance(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = STTHelper()
            return cls._instance

    def get_model_path(self):
        """获取模型存储路径：~/Library/Application Support/AutoCastAI/models/SenseVoiceSmall"""
        base_dir = os.environ.get("WECHAT_USER_DATA")
        if not base_dir:
            # Fallback
            if sys.platform == "darwin":
                base_dir = os.path.expanduser("~/Library/Application Support/AutoCastAI/wechat")
            else:
                base_dir = os.path.join(os.environ.get("APPDATA", ""), "AutoCastAI", "wechat")
        
        # 将模型放在 User Data 目录下，而不是 Bundle 内，这样打包更轻量
        models_dir = os.path.join(os.path.dirname(base_dir), "models")
        return os.path.join(models_dir, "SenseVoiceSmall")

    def is_model_ready(self):
        path = self.get_model_path()
        return os.path.exists(os.path.join(path, "model.pt"))

    def download_model(self):
        """下载模型（国内镜像）"""
        path = self.get_model_path()
        if self.is_model_ready():
            return True
        
        try:
            # 使用国内镜像
            os.environ["MODELSCOPE_DOMAIN"] = "www.modelscope.cn"
            
            try:
                from modelscope.hub.snapshot_download import snapshot_download
            except ImportError:
                print("[stt-helper] Missing modelscope, attempting to install...", file=sys.stderr)
                log_progress(0, 100, "正在安装必需的依赖 (modelscope)...")
                import subprocess
                # 尝试静默安装
                subprocess.check_call([sys.executable, "-m", "pip", "install", "modelscope>=1.14.0", "-i", "https://pypi.tuna.tsinghua.edu.cn/simple"])
                from modelscope.hub.snapshot_download import snapshot_download

            print(f"[stt-helper] Starting download to {path}...", file=sys.stderr)
            
            # 先输出一个开始信号
            log_progress(0, 100, "正在从 ModelScope 下载模型 (约 900MB)...")
            
            snapshot_download(
                'iic/SenseVoiceSmall',
                local_dir=path
            )
            
            log_progress(100, 100, "下载完成")
            return True
        except Exception as e:
            err_msg = str(e)
            print(f"[stt-helper] Download failed: {err_msg}", file=sys.stderr)
            log_progress(0, 100, f"下载失败: {err_msg}")
            # 抛出异常让外层捕获
            raise Exception(err_msg)

    def _ensure_model(self):
        with self._load_lock:
            if self.model is not None:
                return
            
            if not self.is_model_ready():
                raise Exception("模型未就绪，请先下载")

            try:
                from funasr import AutoModel
                import torch
                
                device = "cuda" if torch.cuda.is_available() else "cpu"
                if sys.platform == "darwin" and torch.backends.mps.is_available():
                    device = "mps"
                
                print(f"[stt-helper] Loading SenseVoiceSmall from {self.get_model_path()} on {device}...", file=sys.stderr)
                self.model = AutoModel(
                    model=self.get_model_path(),
                    device=device,
                    disable_pbar=True,
                    disable_log=True
                )
                print("[stt-helper] Model loaded.", file=sys.stderr)
            except Exception as e:
                print(f"[stt-helper] Error loading model: {e}", file=sys.stderr)
                raise

    def transcribe(self, wav_path):
        self._ensure_model()
        try:
            res = self.model.generate(input=wav_path, cache={}, language="auto", use_itn=True)
            if res and len(res) > 0:
                text = res[0].get('text', '')
                import re
                text = re.sub(r'<\|.*?\|>', '', text).strip()
                return text
            return ""
        except Exception as e:
            print(f"[stt-helper] Transcription error: {e}", file=sys.stderr)
            return f"[识别失败: {e}]"

def transcribe_wav(wav_path):
    return STTHelper.get_instance().transcribe(wav_path)

def check_stt_model():
    return STTHelper.get_instance().is_model_ready()

def download_stt_model():
    return STTHelper.get_instance().download_model()
