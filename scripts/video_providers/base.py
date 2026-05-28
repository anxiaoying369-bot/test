from abc import ABC, abstractmethod
from typing import Optional, Dict, Any

class VideoProvider(ABC):
    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key

    @abstractmethod
    def text_to_video(self, prompt: str, duration: int = 5, ratio: str = "9:16") -> str:
        """
        发起文生视频任务，返回 task_id 或异步结果标识
        """
        pass

    @abstractmethod
    def image_to_video(self, image_url: str, prompt: Optional[str] = None, duration: int = 5) -> str:
        """
        发起图生视频任务
        """
        pass

    @abstractmethod
    def poll_task(self, task_id: str) -> Dict[str, Any]:
        """
        轮询任务状态
        返回: {"status": "processing" | "completed" | "error", "video_url": Optional[str], "error": Optional[str]}
        """
        pass
