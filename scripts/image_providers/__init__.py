from typing import Optional
from .base import ImageProvider
from .mock import MockImageProvider


def get_image_provider(name: str, api_key: Optional[str] = None, **kwargs) -> ImageProvider:
    name = (name or "").lower()
    if name in ("mock", ""):
        return MockImageProvider(api_key=api_key)
    if name == "fal":
        from .fal_image import FalImageProvider
        return FalImageProvider(api_key=api_key)
    if name in ("volcengine", "volc"):
        from .volcengine_image import VolcengineImageProvider
        return VolcengineImageProvider(api_key=api_key, model=kwargs.get("model", ""))
    if name in ("openai", "openai-compatible", "openai-image"):
        from .openai_image import OpenAICompatibleImageProvider
        return OpenAICompatibleImageProvider(
            api_key=api_key,
            base_url=kwargs.get("base_url", "https://api.openai.com/v1"),
            model=kwargs.get("model", "dall-e-3"),
        )
    raise ValueError(f"Unknown image provider: {name}")
