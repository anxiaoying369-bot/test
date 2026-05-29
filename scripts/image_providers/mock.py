import hashlib
import urllib.parse
from typing import Optional
from .base import ImageProvider


class MockImageProvider(ImageProvider):
    """不调任何 API，用 picsum/placeholder 占位图返回，方便流程测试。"""

    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)

    def _placeholder(self, prompt: str, size: str) -> str:
        # 把 prompt 做哈希作为 seed，让相同 prompt 得到相同图
        seed = hashlib.md5(prompt.encode("utf-8")).hexdigest()[:8]
        w, h = (size.split("x") + ["1024"])[:2]
        # picsum.photos 是无 API key 占位图服务
        return f"https://picsum.photos/seed/{seed}/{w}/{h}"

    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        return self._placeholder(prompt, size)

    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        _ = image_path_or_url
        return self._placeholder(prompt + ":i2i", size)
