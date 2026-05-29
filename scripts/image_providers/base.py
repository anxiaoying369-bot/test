from abc import ABC, abstractmethod
from typing import Optional


class ImageProvider(ABC):
    """图片生成 Provider 抽象。同步返回图片 URL（或 data:URL）。"""

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key

    @abstractmethod
    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        """文生图。size 形如 '1024x1024'、'720x1280'。返回图片 URL 或 data:URL。"""
        raise NotImplementedError

    @abstractmethod
    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        """图生图。image_path_or_url 可以是本地文件路径或 URL，由实现自行处理。"""
        raise NotImplementedError
