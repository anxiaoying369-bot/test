import os
import fal_client
from typing import Optional, Dict, Any
from .base import VideoProvider

class FalProvider(VideoProvider):
    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)
        if api_key:
            os.environ["FAL_KEY"] = api_key

    def text_to_video(self, prompt: str, duration: int = 5, ratio: str = "9:16") -> str:
        """
        使用 fal-ai/luma-dream-machine 作为默认实现
        """
        handler = fal_client.submit(
            "fal-ai/luma-dream-machine",
            arguments={
                "prompt": prompt,
                "aspect_ratio": ratio.replace(":", "_"),
                "loop": False
            },
        )
        return handler.request_id

    def image_to_video(self, image_url: str, prompt: Optional[str] = None, duration: int = 5) -> str:
        """
        图生视频
        """
        handler = fal_client.submit(
            "fal-ai/luma-dream-machine/image-to-video",
            arguments={
                "image_url": image_url,
                "prompt": prompt or "",
            },
        )
        return handler.request_id

    def poll_task(self, task_id: str) -> Dict[str, Any]:
        """
        轮询任务状态。
        由于 fal-client SDK 主要设计为同步/阻塞调用，这里我们通过 status() 接口实现非阻塞查询。
        """
        try:
            # 获取任务状态快照
            status = fal_client.status("fal-ai/luma-dream-machine", task_id, with_logs=False)
            
            if status is None:
                return {"status": "processing", "video_url": None, "error": None}

            # 尝试获取结果。fal_client.result 在任务未完成时会抛出异常或阻塞。
            # 我们这里捕获异常以确保不阻塞调用进程。
            try:
                result = fal_client.result("fal-ai/luma-dream-machine", task_id)
                if result and "video" in result:
                    video_url = result["video"]["url"]
                    print(f"[FalProvider] Task {task_id} completed. URL: {video_url}")
                    return {
                        "status": "completed", 
                        "video_url": video_url, 
                        "error": None
                    }
            except Exception as e:
                # 任务可能还在运行中或排队
                pass

            return {"status": "processing", "video_url": None, "error": None}
            
        except Exception as e:
            error_msg = str(e)
            if "not found" in error_msg.lower():
                return {"status": "error", "video_url": None, "error": f"Task not found: {task_id}"}
            print(f"[FalProvider] Error polling task {task_id}: {error_msg}")
            return {"status": "processing", "video_url": None, "error": None}
