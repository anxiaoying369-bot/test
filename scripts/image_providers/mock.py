import hashlib
import base64
import html
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
        try:
            wi, hi = int(w), int(h)
        except Exception:
            wi, hi = 1024, 1024
        label = html.escape(f"Mock Image {seed}")
        svg = (
            f"<svg xmlns='http://www.w3.org/2000/svg' width='{wi}' height='{hi}' viewBox='0 0 {wi} {hi}'>"
            f"<rect width='100%' height='100%' fill='#0f172a'/>"
            f"<rect x='24' y='24' width='{max(wi - 48, 1)}' height='{max(hi - 48, 1)}' rx='18' fill='#164e63' opacity='0.55'/>"
            f"<text x='50%' y='50%' dominant-baseline='middle' text-anchor='middle' fill='#e0f2fe' font-size='32' font-family='Arial'>{label}</text>"
            f"</svg>"
        )
        return "data:image/svg+xml;base64," + base64.b64encode(svg.encode("utf-8")).decode("ascii")

    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        return self._placeholder(prompt, size)

    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        _ = image_path_or_url
        return self._placeholder(prompt + ":i2i", size)

    def inpaint(self, image_path_or_url: str, mask_path: str, prompt: str, size: str = "1024x1024") -> str:
        _ = image_path_or_url
        _ = mask_path
        return self._placeholder(prompt + ":inpaint", size)
