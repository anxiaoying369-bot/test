import time
import uuid
from typing import Optional, Dict, Any
from .base import VideoProvider

class MockProvider(VideoProvider):
    """
    仅用于 UI/流程测试的 Mock Provider。
    不消耗任何 API Key，模拟生成过程。
    """
    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)
        self.tasks = {}

    def text_to_video(self, prompt: str, duration: int = 5, ratio: str = "9:16") -> str:
        task_id = f"mock_{uuid.uuid4().hex[:8]}"
        self.tasks[task_id] = {
            "start_time": time.time(),
            "status": "processing"
        }
        print(f"[MockProvider] Task started: {task_id}")
        return task_id

    def image_to_video(self, image_url: str, prompt: Optional[str] = None, duration: int = 5) -> str:
        return self.text_to_video(prompt or "image_to_video")

    def poll_task(self, task_id: str) -> Dict[str, Any]:
        task = self.tasks.get(task_id)
        if not task:
            return {"status": "error", "error": "Task not found"}
        
        elapsed = time.time() - task["start_time"]
        
        # 模拟生成需要 10 秒
        if elapsed > 10:
            return {
                "status": "completed",
                # 返回一个示例视频 URL
                "video_url": "https://sample-videos.com/video123/mp4/720/big_buck_bunny_720p_1mb.mp4",
                "error": None
            }
        else:
            return {"status": "processing", "video_url": None, "error": None}
