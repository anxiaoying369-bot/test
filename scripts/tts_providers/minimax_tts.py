"""MiniMax 语音合成（OpenAI 兼容协议 /v1/audio/speech）。

API:  http://pan.ptyxlm.com:3000/v1/audio/speech
模型:  speech-2.8-hd
认证:  Bearer <api_key>

音色列表（来自 MiniMax 开放平台）：
  male-qn-qingse         — 青涩青年音色
  male-qn-jingying      — 精英青年音色
  male-qn-badao         — 霸道青年音色
  male-qn-daxuesheng    — 青年大学生音色
  female-shaonv         — 少女音色
  female-yujie          — 御姐音色
  female-chengshu       — 成熟女性音色
  female-tianmei        — 甜美女性音色
  male-qn-qingse-jingpin    — 青涩青年音色-beta
  male-qn-jingying-jingpin — 精英青年音色-beta
  male-qn-badao-jingpin    — 霸道青年音色-beta
  male-qn-daxuesheng-jingpin — 青年大学生音色-beta
  female-shaonv-jingpin    — 少女音色-beta
  female-yujie-jingpin     — 御姐音色-beta
  female-chengshu-jingpin  — 成熟女性音色-beta
  ... (更多音色参考 platform.minimaxi.com/docs/faq/system-voice-id)
"""
from .openai_tts import OpenAITTSProvider


MINIMAX_VOICES = [
    # 中文音色
    {"id": "male-qn-qingse",        "name": "青涩青年音色",     "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-jingying",       "name": "精英青年音色",     "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-badao",          "name": "霸道青年音色",     "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-daxuesheng",     "name": "青年大学生音色",   "gender": "m", "language": "zh-CN"},
    {"id": "female-shaonv",          "name": "少女音色",         "gender": "f", "language": "zh-CN"},
    {"id": "female-yujie",           "name": "御姐音色",         "gender": "f", "language": "zh-CN"},
    {"id": "female-chengshu",        "name": "成熟女性音色",      "gender": "f", "language": "zh-CN"},
    {"id": "female-tianmei",         "name": "甜美女性音色",     "gender": "f", "language": "zh-CN"},
    # Beta 精品音色
    {"id": "male-qn-qingse-jingpin",    "name": "青涩青年音色-beta",   "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-jingying-jingpin",  "name": "精英青年音色-beta",   "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-badao-jingpin",      "name": "霸道青年音色-beta",   "gender": "m", "language": "zh-CN"},
    {"id": "male-qn-daxuesheng-jingpin", "name": "青年大学生音色-beta", "gender": "m", "language": "zh-CN"},
    {"id": "female-shaonv-jingpin",      "name": "少女音色-beta",       "gender": "f", "language": "zh-CN"},
    {"id": "female-yujie-jingpin",       "name": "御姐音色-beta",       "gender": "f", "language": "zh-CN"},
    {"id": "female-chengshu-jingpin",    "name": "成熟女性音色-beta",   "gender": "f", "language": "zh-CN"},
    # 英文音色
    {"id": "male-qn-jianjie",         "name": "简洁男声",          "gender": "m", "language": "en-US"},
    {"id": "female-yujie-yingyu",     "name": "御姐英语音色",      "gender": "f", "language": "en-US"},
]


class MiniMaxTTSProvider(OpenAITTSProvider):
    """MiniMax 语音合成 Provider。

    与 OpenAI 兼容协议完全一致，仅默认模型和音色列表不同。
    """

    def __init__(
        self,
        api_key: str | None = None,
        base_url: str = "http://pan.ptyxlm.com:3000/v1",
        model: str = "speech-2.8-hd",
    ):
        super().__init__(api_key=api_key, base_url=base_url, model=model)

    def list_voices(self) -> list[dict]:
        return MINIMAX_VOICES