"""火山引擎语音合成。

火山的语音合成 API 需要 appid + access_token，走 HTTP POST 协议传 base64 mp3。
appid:access_token 格式填入设置中的 API Key。

中文音色推荐：
  - BV700_streaming（甜美女声）
  - BV701_streaming（磁性男声）
  - BV407_V2_streaming（霸气男声）
"""
import json
import os
import uuid
import base64
import urllib.request
import urllib.error
from typing import Optional
from .base import TTSProvider


class VolcengineTTSProvider(TTSProvider):
    DEFAULT_VOICES = [
        {"id": "BV700_streaming",   "name": "甜美女声", "gender": "f", "language": "zh-CN"},
        {"id": "BV701_streaming",   "name": "磁性男声", "gender": "m", "language": "zh-CN"},
        {"id": "BV407_V2_streaming","name": "霸气男声", "gender": "m", "language": "zh-CN"},
        {"id": "BV705_streaming",   "name": "活泼少女", "gender": "f", "language": "zh-CN"},
    ]

    def __init__(self, api_key: Optional[str] = None):
        super().__init__(api_key)
        # 火山的 api_key 约定格式: "appid:access_token"
        if api_key and ":" in api_key:
            self.appid, self.access_token = api_key.split(":", 1)
        else:
            self.appid = ""
            self.access_token = api_key or ""

    def list_voices(self) -> list[dict]:
        return self.DEFAULT_VOICES

    def synthesize(self, text: str, voice_id: str = "BV700_streaming", speed: float = 1.0, output_path: str = "") -> str:
        if not output_path:
            raise ValueError("output_path 必填")
        if not self.appid or not self.access_token:
            raise ValueError("火山引擎需要填入 'appid:access_token' 格式的 API Key")
            
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        url = "https://openspeech.bytedance.com/api/v1/tts"
        payload = {
            "app": {
                "appid": self.appid,
                "token": self.access_token,
                "cluster": "volcano_tts"
            },
            "user": {
                "uid": "autocast_user"
            },
            "audio": {
                "voice_type": voice_id,
                "encoding": "mp3",
                "speed_ratio": float(speed),
                "volume_ratio": 1.0,
                "pitch_ratio": 1.0
            },
            "request": {
                "reqid": str(uuid.uuid4()),
                "text": text,
                "text_type": "plain",
                "operation": "query"
            }
        }
        
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.access_token}",
            },
            method="POST",
        )
        
        try:
            with urllib.request.urlopen(req, timeout=180) as resp:
                if resp.status != 200:
                    err_body = resp.read().decode("utf-8", errors="ignore")
                    raise RuntimeError(f"火山 TTS HTTP {resp.status}: {err_body[:300]}")
                
                res_data = json.loads(resp.read().decode("utf-8"))
                
                # 火山成功码是 3000
                if res_data.get("code") != 3000:
                    raise RuntimeError(f"火山 TTS 业务错误 {res_data.get('code')}: {res_data.get('message')}")
                
                audio_b64 = res_data.get("data")
                if not audio_b64:
                    raise RuntimeError("火山 TTS 返回数据中缺少音频内容")
                
                audio_data = base64.b64decode(audio_b64)
                
        except urllib.error.HTTPError as e:
            body = ""
            try: body = e.read().decode("utf-8", errors="ignore")
            except Exception: pass
            raise RuntimeError(f"火山 TTS HTTP {e.code}: {body or e.reason}")
        except Exception as e:
            raise RuntimeError(f"火山 TTS 合成失败: {str(e)}")

        with open(output_path, "wb") as f:
            f.write(audio_data)
        return output_path
