from typing import Optional
from .base import TTSProvider
from .mock import MockTTSProvider


def get_tts_provider(name: str, api_key: Optional[str] = None, **kwargs) -> TTSProvider:
    name = (name or "").lower()
    if name in ("mock", ""):
        return MockTTSProvider(api_key=api_key)
    if name in ("openai", "openai-tts", "openai-compatible"):
        from .openai_tts import OpenAITTSProvider
        return OpenAITTSProvider(
            api_key=api_key,
            base_url=kwargs.get("base_url", "https://api.openai.com/v1"),
            model=kwargs.get("model", "tts-1"),
        )
    if name in ("volc", "volcengine", "volcengine-tts"):
        from .volcengine_tts import VolcengineTTSProvider
        return VolcengineTTSProvider(api_key=api_key)
    if name in ("minimax", "minimax-tts"):
        from .minimax_tts import MiniMaxTTSProvider
        return MiniMaxTTSProvider(
            api_key=api_key,
            base_url=kwargs.get("base_url", "http://pan.ptyxlm.com:3000/v1"),
            model=kwargs.get("model", "speech-2.8-hd"),
        )
    raise ValueError(f"Unknown TTS provider: {name}")
