from .base import VideoProvider
from .fal import FalProvider
from .mock import MockProvider
from .volcengine import VolcengineProvider

def get_provider(name: str, api_key: str = None) -> VideoProvider:
    name = name.lower()
    if name == "fal":
        return FalProvider(api_key=api_key)
    elif name == "mock":
        return MockProvider(api_key=api_key)
    elif name == "volcengine" or name == "volc":
        return VolcengineProvider(api_key=api_key)
    else:
        raise ValueError(f"Unknown provider: {name}")
