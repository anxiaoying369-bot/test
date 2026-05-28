from .base import VideoProvider
from .fal import FalProvider
from .mock import MockProvider
from .volcengine import VolcengineProvider
from .openai_compatible import OpenAICompatibleProvider

def get_provider(name: str, api_key: str = None, **kwargs) -> VideoProvider:
    name = name.lower()
    if name == "fal":
        return FalProvider(api_key=api_key)
    elif name == "mock":
        return MockProvider(api_key=api_key)
    elif name == "volcengine" or name == "volc":
        return VolcengineProvider(api_key=api_key)
    elif name == "openai" or name == "openai-compatible":
        return OpenAICompatibleProvider(
            api_key=api_key, 
            base_url=kwargs.get("base_url", "https://api.openai.com/v1"),
            model=kwargs.get("model", "v0")
        )
    else:
        raise ValueError(f"Unknown provider: {name}")
