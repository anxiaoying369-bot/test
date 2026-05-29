"""火山引擎视觉智能（文生图）。

接入的是火山「视觉智能服务 - CV 进程」(visual_drawing) 的通用文生图接口。
appid:access_token 格式填入设置中的 API Key（与火山 TTS 同样的格式）。

接口文档参考：https://www.volcengine.com/docs/6791/119548
默认模型：general_v2.0L（通用 2.0 大模型）

注意：火山 CV 接口实际签名比较复杂（V4 签名），这里走的是简化的 token 直传方式，
若你的实际接入需要 AK/SK 签名，可在 _do_request 里换成签名请求。
"""
import base64
import json
import urllib.request
import urllib.error
import uuid
from typing import Optional
from .base import ImageProvider


class VolcengineImageProvider(ImageProvider):
    DEFAULT_MODEL = "general_v2.0L"
    API_URL = "https://visual.volcengineapi.com"

    def __init__(self, api_key: Optional[str] = None, model: str = ""):
        super().__init__(api_key)
        if api_key and ":" in api_key:
            self.appid, self.access_token = api_key.split(":", 1)
        else:
            self.appid = ""
            self.access_token = api_key or ""
        self.model = model or self.DEFAULT_MODEL

    @staticmethod
    def _parse_size(size: str) -> tuple[int, int]:
        try:
            w, h = size.split("x")
            return int(w), int(h)
        except Exception:
            return 1024, 1024

    def _do_request(self, payload: dict) -> dict:
        if not self.appid or not self.access_token:
            raise RuntimeError("火山引擎需要 'appid:access_token' 格式的 API Key")

        req = urllib.request.Request(
            self.API_URL,
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.access_token}",
                "X-AppId": self.appid,
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=180) as resp:
                if resp.status != 200:
                    body = resp.read().decode("utf-8", errors="ignore")
                    raise RuntimeError(f"火山图片生成 HTTP {resp.status}: {body[:300]}")
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            body = ""
            try:
                body = e.read().decode("utf-8", errors="ignore")
            except Exception:
                pass
            raise RuntimeError(f"火山图片生成 HTTP {e.code}: {body or e.reason}")
        except Exception as e:
            raise RuntimeError(f"火山图片生成请求失败: {e}")

    def _extract_image(self, res: dict) -> str:
        """火山返回结构通常是 {"code": 10000, "data": {"image_urls": ["..."]}} 或 base64。"""
        if res.get("code") not in (10000, 0, None):
            raise RuntimeError(
                f"火山图片生成错误 {res.get('code')}: {res.get('message') or res.get('msg')}"
            )
        data = res.get("data") or {}
        urls = data.get("image_urls") or data.get("urls")
        if urls and isinstance(urls, list) and urls[0]:
            return urls[0]
        # 有些模型只返回 base64
        b64 = data.get("binary_data_base64") or data.get("image_b64")
        if isinstance(b64, list) and b64:
            return f"data:image/png;base64,{b64[0]}"
        if isinstance(b64, str) and b64:
            return f"data:image/png;base64,{b64}"
        raise RuntimeError(f"火山图片生成返回结构异常: {res}")

    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        w, h = self._parse_size(size)
        payload = {
            "req_key": self.model,
            "prompt": prompt,
            "width": w,
            "height": h,
            "seed": -1,
            "scale": 3.5,
            "ddim_steps": 25,
            "use_sr": False,
            "return_url": True,
            "logo_info": {"add_logo": False},
        }
        return self._extract_image(self._do_request(payload))

    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        w, h = self._parse_size(size)
        # 火山图生图需要 base64 编码参考图
        if image_path_or_url.startswith("http"):
            # 远端 URL 先下下来再 base64
            try:
                with urllib.request.urlopen(image_path_or_url, timeout=60) as r:
                    img_bytes = r.read()
            except Exception as e:
                raise RuntimeError(f"下载参考图失败: {e}")
        else:
            try:
                with open(image_path_or_url, "rb") as f:
                    img_bytes = f.read()
            except Exception as e:
                raise RuntimeError(f"读取参考图失败: {e}")

        b64 = base64.b64encode(img_bytes).decode("ascii")

        payload = {
            "req_key": self.model,
            "prompt": prompt,
            "binary_data_base64": [b64],
            "width": w,
            "height": h,
            "seed": -1,
            "scale": 3.5,
            "ddim_steps": 25,
            "return_url": True,
            "logo_info": {"add_logo": False},
        }
        # uuid 仅用作请求标识（部分火山服务要求），不影响输出
        _ = uuid.uuid4()
        return self._extract_image(self._do_request(payload))
