"""OpenAI 兼容协议 TTS（POST {base_url}/audio/speech）。

国产兼容服务（硅基流动 SiliconFlow 等）多数都支持这个接口。
官方音色：alloy / echo / fable / onyx / nova / shimmer
"""
import json
import os
import urllib.request
import urllib.error
from typing import Optional
from .base import TTSProvider


class OpenAITTSProvider(TTSProvider):
    DEFAULT_VOICES = [
        {"id": "alloy",   "name": "Alloy（中性）",       "gender": "n", "language": "multi"},
        {"id": "echo",    "name": "Echo（男声）",        "gender": "m", "language": "multi"},
        {"id": "fable",   "name": "Fable（英伦）",       "gender": "m", "language": "en"},
        {"id": "onyx",    "name": "Onyx（低沉男声）",    "gender": "m", "language": "multi"},
        {"id": "nova",    "name": "Nova（明亮女声）",    "gender": "f", "language": "multi"},
        {"id": "shimmer", "name": "Shimmer（温柔女声）", "gender": "f", "language": "multi"},
    ]

    def __init__(self, api_key: Optional[str] = None, base_url: str = "https://api.openai.com/v1", model: str = "tts-1"):
        super().__init__(api_key)
        self.base_url = base_url.rstrip("/").removesuffix("/chat/completions").rstrip("/")
        self.model = model or "tts-1"

    def list_voices(self) -> list[dict]:
        return self.DEFAULT_VOICES

    def synthesize(self, text: str, voice_id: str = "alloy", speed: float = 1.0, output_path: str = "") -> str:
        if not output_path:
            raise ValueError("output_path 必填")
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        url = f"{self.base_url}/audio/speech"
        payload = {
            "model": self.model,
            "voice": voice_id,
            "input": text,
            "response_format": "mp3",
            "speed": max(0.25, min(4.0, float(speed))),
        }
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.api_key or ''}",
            },
            method="POST",
        )
        # 不捕获 HTTPError —— 让上层 provider_errors.classify_exception 分类成 AUTH/RATE_LIMIT 等
        with urllib.request.urlopen(req, timeout=180) as resp:
            if resp.status != 200:
                raise RuntimeError(f"TTS HTTP {resp.status}: {resp.read().decode('utf-8', errors='ignore')[:300]}")
            data = resp.read()

        with open(output_path, "wb") as f:
            f.write(data)
        return output_path
