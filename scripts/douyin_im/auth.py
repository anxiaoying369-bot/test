from __future__ import annotations

import base64
import json
from dataclasses import dataclass

from .utils import generate_ms_token, trans_cookies


@dataclass
class DouyinAuth:
    """抖音私信所需认证信息。

    cookies_str 必填。
    发送私信还需要 web_protect 和 keys；只接收 WebSocket 私信通常只需要 cookie。
    """

    cookie: dict[str, str]
    cookie_str: str
    msToken: str
    private_key: str | None = None
    ticket: str | None = None
    ts_sign: str | None = None
    client_cert: str | None = None
    ree_public_key: str | None = None
    uid: int | None = None

    @classmethod
    def from_strings(
        cls,
        cookies_str: str,
        web_protect: str = "",
        keys: str = "",
        uid: int | None = None,
    ) -> "DouyinAuth":
        cookie = trans_cookies(cookies_str)
        ms_token = cookie.get("msToken") or generate_ms_token()
        cookie["msToken"] = ms_token
        normalized_cookie_str = "; ".join([f"{k}={v}" for k, v in cookie.items()])
        auth = cls(cookie=cookie, cookie_str=normalized_cookie_str, msToken=ms_token, uid=uid)
        auth.apply_web_protect(web_protect)
        auth.apply_keys(keys)
        return auth

    def apply_web_protect(self, web_protect: str = "") -> None:
        if not web_protect:
            return
        data = _parse_nested_json_data(web_protect)
        self.ticket = data["ticket"]
        self.ts_sign = data["ts_sign"]
        self.client_cert = data["client_cert"]

    def apply_keys(self, keys: str = "") -> None:
        if not keys:
            return
        data = _parse_nested_json_data(keys)
        self.private_key = data["ec_privateKey"]
        self.ree_public_key = base64.b64encode(self.private_key.encode()).decode()

    def require_send_credentials(self) -> None:
        missing = []
        if not self.private_key:
            missing.append("keys/ec_privateKey")
        if not self.ticket:
            missing.append("web_protect/ticket")
        if not self.ts_sign:
            missing.append("web_protect/ts_sign")
        if not self.client_cert:
            missing.append("web_protect/client_cert")
        if "s_v_web_id" not in self.cookie:
            missing.append("cookie s_v_web_id")
        if missing:
            raise ValueError("发送私信缺少认证字段: " + ", ".join(missing))

    def require_cookie_keys(self, *keys: str) -> None:
        missing = [key for key in keys if key not in self.cookie or self.cookie[key] == ""]
        if missing:
            raise ValueError("cookie 缺少字段: " + ", ".join(missing))


def _parse_nested_json_data(raw: str) -> dict:
    """兼容原项目格式: {"data":"{...}"}，也兼容直接传 {...}。"""
    obj = json.loads(raw)
    if isinstance(obj, dict) and "data" in obj:
        data = obj["data"]
        if isinstance(data, str):
            return json.loads(data)
        if isinstance(data, dict):
            return data
    if isinstance(obj, dict):
        return obj
    raise ValueError("认证参数格式不正确，必须是 JSON object 或包含 data 的 JSON object")
