"""Mock TTS Provider：生成估算时长的静音 MP3，方便流程联调。
依赖 ffmpeg（已 bundle）。
"""
import os
import shutil
import subprocess
from typing import Optional
from .base import TTSProvider


class MockTTSProvider(TTSProvider):
    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)

    def list_voices(self) -> list[dict]:
        return [
            {"id": "mock-female-1", "name": "Mock 女声 1", "gender": "f", "language": "zh-CN"},
            {"id": "mock-male-1",   "name": "Mock 男声 1", "gender": "m", "language": "zh-CN"},
        ]

    def synthesize(self, text: str, voice_id: str = "mock-female-1", speed: float = 1.0, output_path: str = "") -> str:
        # 按中文每字 0.25 秒估算时长，speed 越快总时长越短
        approx_duration = max(2.0, len(text) * 0.25 / max(0.5, speed))

        if not output_path:
            raise ValueError("output_path 必填")
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        ffmpeg = shutil.which("ffmpeg") or "ffmpeg"
        # 生成静音 mp3
        subprocess.run(
            [
                ffmpeg, "-y",
                "-f", "lavfi",
                "-i", f"anullsrc=channel_layout=stereo:sample_rate=44100",
                "-t", f"{approx_duration:.2f}",
                "-c:a", "libmp3lame",
                "-b:a", "192k",
                output_path,
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return output_path
