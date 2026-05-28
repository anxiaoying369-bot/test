import json
import httpx
from typing import Optional, Dict, Any
from .base import VideoProvider

class OpenAICompatibleProvider(VideoProvider):
    def __init__(self, api_key: Optional[str] = None, base_url: str = "https://api.openai.com/v1", model: str = "v0"):
        super().__init__(api_key)
        self.base_url = base_url.rstrip("/")
        self.model = model

    def text_to_video(self, prompt: str, duration: int = 5, ratio: str = "9:16") -> str:
        """
        发起文生视频任务。
        假设协议遵循 POST /v1/video/generations (类似 OpenAI Image API)
        """
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        payload = {
            "model": self.model,
            "prompt": prompt,
            "aspect_ratio": ratio,
            "duration": duration
        }
        
        url = f"{self.base_url}/video/generations"
        
        try:
            with httpx.Client(timeout=30.0) as client:
                response = client.post(url, headers=headers, json=payload)
                response.raise_for_status()
                data = response.json()
                
                # 兼容不同返回格式：有的直接返回 id，有的返回在 data 数组里
                if "id" in data:
                    return data["id"]
                elif "data" in data and len(data["data"]) > 0:
                    return data["data"][0].get("id") or data["data"][0].get("url")
                
                return data.get("task_id", "unknown_task_id")
        except Exception as e:
            print(f"[OpenAICompatible] Error starting task: {e}")
            raise

    def image_to_video(self, image_url: str, prompt: Optional[str] = None, duration: int = 5) -> str:
        """
        图生视频
        """
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        payload = {
            "model": self.model,
            "image_url": image_url,
            "prompt": prompt or "",
            "duration": duration
        }
        
        url = f"{self.base_url}/video/generations"
        
        try:
            with httpx.Client(timeout=30.0) as client:
                response = client.post(url, headers=headers, json=payload)
                response.raise_for_status()
                data = response.json()
                return data.get("id") or data.get("task_id", "unknown_task_id")
        except Exception as e:
            print(f"[OpenAICompatible] Error starting task: {e}")
            raise

    def poll_task(self, task_id: str) -> Dict[str, Any]:
        """
        轮询任务状态。
        假设协议遵循 GET /v1/video/generations/{task_id}
        """
        # 如果 task_id 本身就是 URL (某些同步返回的 mock)，直接返回完成
        if task_id.startswith("http"):
            return {
                "status": "completed",
                "video_url": task_id,
                "error": None
            }

        headers = {
            "Authorization": f"Bearer {self.api_key}"
        }
        
        url = f"{self.base_url}/video/generations/{task_id}"
        
        try:
            with httpx.Client(timeout=10.0) as client:
                response = client.get(url, headers=headers)
                response.raise_for_status()
                data = response.json()
                
                status_raw = data.get("status", "processing").lower()
                
                if status_raw in ["completed", "succeeded", "success"]:
                    # 尝试获取 URL
                    video_url = None
                    if "video" in data:
                        video_url = data["video"].get("url")
                    elif "data" in data and len(data["data"]) > 0:
                        video_url = data["data"][0].get("url")
                    
                    return {
                        "status": "completed",
                        "video_url": video_url or data.get("url"),
                        "error": None
                    }
                elif status_raw in ["failed", "error"]:
                    return {
                        "status": "error",
                        "video_url": None,
                        "error": data.get("error", "Unknown error")
                    }
                else:
                    return {
                        "status": "processing",
                        "video_url": None,
                        "error": None
                    }
        except Exception as e:
            print(f"[OpenAICompatible] Error polling task {task_id}: {e}")
            return {
                "status": "processing",
                "video_url": None,
                "error": None
            }
