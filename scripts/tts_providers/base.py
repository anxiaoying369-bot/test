from abc import ABC, abstractmethod
from typing import Optional


class TTSProvider(ABC):
    """文字转语音 Provider 抽象。"""

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key

    @abstractmethod
    def list_voices(self) -> list[dict]:
        """返回可用音色列表：[{"id": "...", "name": "...", "gender": "f|m", "language": "zh-CN"}]"""
        raise NotImplementedError

    @abstractmethod
    def synthesize(self, text: str, voice_id: str, speed: float = 1.0, output_path: str = "") -> str:
        """合成语音。返回最终保存的本地音频文件路径（mp3）。"""
        raise NotImplementedError
