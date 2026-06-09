#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os
import sys
import threading
import logging

# 屏蔽 modelscope/funasr 的冗余日志
logging.getLogger('modelscope').setLevel(logging.ERROR)
logging.getLogger('funasr').setLevel(logging.ERROR)

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

    def _ensure_model(self):
        with self._load_lock:
            if self.model is not None:
                return
            
            try:
                from funasr import AutoModel
                import torch
                
                device = "cuda" if torch.cuda.is_available() else "cpu"
                # 如果是 Mac M1/M2/M3，可以使用 mps
                if sys.platform == "darwin" and torch.backends.mps.is_available():
                    device = "mps"
                
                print(f"[stt-helper] Loading SenseVoiceSmall on {device}...", file=sys.stderr)
                
                # 尝试定位本地模型
                model_path = "iic/SenseVoiceSmall"
                resources_path = os.environ.get("WCDB_RESOURCES_PATH")
                if resources_path:
                    # WCDB_RESOURCES_PATH 通常指向 .../resources/wechat
                    # 本地模型预期在 .../resources/models/SenseVoiceSmall
                    local_model = os.path.join(os.path.dirname(resources_path), "models", "SenseVoiceSmall")
                    if os.path.exists(local_model) and os.path.isdir(local_model):
                        model_path = local_model
                        print(f"[stt-helper] Using local model: {model_path}", file=sys.stderr)
                
                self.model = AutoModel(
                    model=model_path,
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
            # SenseVoiceSmall generate returns a list of results
            res = self.model.generate(input=wav_path, cache={}, language="auto", use_itn=True)
            if res and len(res) > 0:
                # res[0] is usually {'text': '...', 'token': [...]}
                # SenseVoiceSmall output might contain event tags like <|zh|><|NEUTRAL|><|Speech|>
                text = res[0].get('text', '')
                # 清洗掉 SenseVoice 的标签
                import re
                text = re.sub(r'<\|.*?\|>', '', text).strip()
                return text
            return ""
        except Exception as e:
            print(f"[stt-helper] Transcription error: {e}", file=sys.stderr)
            return f"[识别失败: {e}]"

def transcribe_wav(wav_path):
    return STTHelper.get_instance().transcribe(wav_path)
