"""fal.ai 文生图 / 图生图（FLUX schnell 系列，速度最快）。"""
import os
from typing import Optional
from .base import ImageProvider


class FalImageProvider(ImageProvider):
    DEFAULT_T2I_MODEL = "fal-ai/flux/schnell"
    DEFAULT_I2I_MODEL = "fal-ai/flux/dev/image-to-image"

    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)
        if api_key:
            os.environ["FAL_KEY"] = api_key

    @staticmethod
    def _to_image_size(size: str) -> dict:
        try:
            w, h = size.split("x")
            return {"width": int(w), "height": int(h)}
        except Exception:
            return {"width": 1024, "height": 1024}

    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        import fal_client
        result = fal_client.run(
            self.DEFAULT_T2I_MODEL,
            arguments={
                "prompt": prompt,
                "image_size": self._to_image_size(size),
                "num_images": 1,
            },
        )
        # fal 返回 {"images": [{"url": "..."}]}
        images = result.get("images") if isinstance(result, dict) else None
        if not images:
            raise RuntimeError(f"fal 返回无图片: {result}")
        return images[0].get("url") or ""

    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        import fal_client
        # 本地路径需要先上传到 fal 拿 URL
        if not image_path_or_url.startswith("http"):
            image_path_or_url = fal_client.upload_file(image_path_or_url)
        result = fal_client.run(
            self.DEFAULT_I2I_MODEL,
            arguments={
                "prompt": prompt,
                "image_url": image_path_or_url,
                "image_size": self._to_image_size(size),
                "num_images": 1,
                "strength": 0.85,
            },
        )
        images = result.get("images") if isinstance(result, dict) else None
        if not images:
            raise RuntimeError(f"fal 返回无图片: {result}")
        return images[0].get("url") or ""
