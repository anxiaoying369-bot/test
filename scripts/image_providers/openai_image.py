"""OpenAI 兼容协议图片生成（DALL-E / 兼容 OpenAI Images API 的国产服务）。

接口契约：POST {base_url}/images/generations
body: {"model": "...", "prompt": "...", "n": 1, "size": "1024x1024"}
返回：{"data": [{"url": "..."} or {"b64_json": "..."}]}
"""
import base64
import json
import os
import urllib.request
import urllib.error
from typing import Optional
from .base import ImageProvider


class OpenAICompatibleImageProvider(ImageProvider):
    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: str = "https://api.openai.com/v1",
        model: str = "dall-e-3",
    ):
        super().__init__(api_key)
        self.base_url = base_url.rstrip("/").removesuffix("/chat/completions").rstrip("/")
        self.model = model

    def _do_post(self, path: str, payload: dict) -> dict:
        url = f"{self.base_url}{path}"
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.api_key or ''}",
            },
            method="POST",
        )
        # 不捕获 HTTPError —— 让 provider_errors.classify_exception 分类成 AUTH/RATE_LIMIT 等
        with urllib.request.urlopen(req, timeout=120) as resp:
            return json.loads(resp.read().decode("utf-8"))

    def _extract_image(self, resp: dict) -> str:
        data = resp.get("data") or []
        if not data:
            raise RuntimeError(f"图片生成返回空: {resp}")
        first = data[0]
        url = first.get("url")
        if url:
            return url
        b64 = first.get("b64_json")
        if b64:
            # 转成 data URL，下游可以直接下载
            return f"data:image/png;base64,{b64}"
        raise RuntimeError(f"图片生成返回格式异常: {resp}")

    def text_to_image(self, prompt: str, size: str = "1024x1024") -> str:
        payload = {
            "model": self.model,
            "prompt": prompt,
            "n": 1,
            "size": size,
        }
        return self._extract_image(self._do_post("/images/generations", payload))

    def image_to_image(self, image_path_or_url: str, prompt: str, size: str = "1024x1024") -> str:
        """图生图。优先走 OpenAI 标准 /images/edits（multipart/form-data，必须 PNG）；
        若返回 4xx（部分国产兼容服务不支持 edits），降级到 /images/generations 把 base64 拼进 prompt。"""
        # 读出参考图字节
        if image_path_or_url.startswith("data:"):
            try:
                comma = image_path_or_url.index(",")
                img_bytes = base64.b64decode(image_path_or_url[comma + 1 :])
            except Exception as e:
                raise RuntimeError(f"data URL 解析失败: {e}")
        elif image_path_or_url.startswith("http"):
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
                raise RuntimeError(f"读取本地参考图失败: {e}")

        # ── 主路径：multipart /images/edits ──
        try:
            return self._extract_image(self._do_edits_multipart(img_bytes, prompt, size))
        except urllib.error.HTTPError as e:
            # 部分国产兼容服务（SiliconFlow / 智谱等）不支持 /edits → 降级到 generations 把 base64 拼进 prompt
            # 4xx 中除了 401/403（鉴权）以外，其它认为是接口不支持，走降级
            if e.code in (404, 405, 400, 422) and e.code not in (401, 403):
                b64 = base64.b64encode(img_bytes).decode("ascii")
                fallback_prompt = (
                    f"参考图（base64）：data:image/png;base64,{b64[:200]}...\n\n"
                    f"请以上面这张图为视觉参考（构图/配色/主体形态），生成新图。\n\n{prompt}"
                )
                payload = {"model": self.model, "prompt": fallback_prompt, "n": 1, "size": size}
                return self._extract_image(self._do_post("/images/generations", payload))
            raise

    def _do_edits_multipart(self, image_bytes: bytes, prompt: str, size: str) -> dict:
        """手写 multipart/form-data 调 /images/edits（避免引入额外依赖）。"""
        boundary = "----autocastFormBoundary" + base64.b64encode(os.urandom(9)).decode("ascii").replace("/", "_")
        crlf = b"\r\n"

        def part(name: str, value: bytes, filename: str = "", ctype: str = "") -> bytes:
            head = f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"'
            if filename:
                head += f'; filename="{filename}"'
            head += "\r\n"
            if ctype:
                head += f"Content-Type: {ctype}\r\n"
            head += "\r\n"
            return head.encode("utf-8") + value + crlf

        body = b""
        body += part("model", self.model.encode("utf-8"))
        body += part("prompt", prompt.encode("utf-8"))
        body += part("n", b"1")
        body += part("size", size.encode("utf-8"))
        body += part("image", image_bytes, filename="reference.png", ctype="image/png")
        body += f"--{boundary}--\r\n".encode("utf-8")

        url = f"{self.base_url}/images/edits"
        req = urllib.request.Request(
            url,
            data=body,
            headers={
                "Content-Type": f"multipart/form-data; boundary={boundary}",
                "Authorization": f"Bearer {self.api_key or ''}",
            },
            method="POST",
        )
        # 同样让 HTTPError 直接冒出去由 provider_errors 分类
        with urllib.request.urlopen(req, timeout=180) as resp:
            return json.loads(resp.read().decode("utf-8"))
