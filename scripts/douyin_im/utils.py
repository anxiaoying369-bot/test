from __future__ import annotations

import hashlib
import json
import random
import re
import subprocess
import sys
import time
import urllib.parse
from functools import partial
from pathlib import Path
from typing import Any

import execjs
import requests

requests.packages.urllib3.disable_warnings()

# PyExecJS 在部分环境下会用 subprocess 打开 node；沿用原项目的 utf-8 处理。
subprocess.Popen = partial(subprocess.Popen, encoding="utf-8")

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
STATIC_DIR = PACKAGE_ROOT / "static"
NODE_MODULES_DIR = PACKAGE_ROOT / "node_modules"


def _compile_js(filename: str):
    js_path = STATIC_DIR / filename
    if not js_path.exists():
        raise FileNotFoundError(f"缺少 JS 文件: {js_path}")
    return execjs.compile(js_path.read_text(encoding="utf-8"), cwd=str(NODE_MODULES_DIR))


_dy_js = None


def dy_js():
    global _dy_js
    if _dy_js is None:
        _dy_js = _compile_js("dy_ab.js")
    return _dy_js


def trans_cookies(cookies_str: str) -> dict[str, str]:
    cookies: dict[str, str] = {}
    for item in cookies_str.split("; "):
        if not item or "=" not in item:
            continue
        key, value = item.split("=", 1)
        cookies[key] = value
    return cookies


def generate_req_sign(payload: Any, private_key: str) -> str:
    return dy_js().call("get_req_sign", payload, private_key)


def generate_a_bogus(query: str, data: str = "") -> str:
    return dy_js().call("get_ab", query, data)


def generate_ree_key(private_key: str) -> str:
    return dy_js().call("get_ree_key", private_key)


def generate_bd_ticket_client_data(api: str, ticket: str, ts_sign: str, private_key: str) -> str:
    timestamp = int(time.time())
    sign_data = f"ticket={ticket}&path={api}&timestamp={timestamp}"
    payload = {
        "ts_sign": ts_sign,
        "req_content": "ticket,path,timestamp",
        "req_sign": generate_req_sign(sign_data, private_key),
        "timestamp": timestamp,
    }
    raw = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    import base64

    return base64.urlsafe_b64encode(raw.encode("utf-8")).decode("utf-8")


def generate_ms_token(randomlength: int = 107) -> str:
    alphabet = "ABCDEFGHIGKLMNOPQRSTUVWXYZabcdefghigklmnopqrstuvwxyz0123456789="
    return "".join(random.choice(alphabet) for _ in range(randomlength))


def generate_fake_webid(random_length: int = 19) -> str:
    return "".join(random.choice("0123456789") for _ in range(random_length))


def generate_webid(auth=None, url: str = "") -> str:
    if not url:
        url = "https://www.douyin.com/discover?modal_id=7376449060384935209"
    try:
        headers = {
            "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "accept-language": "zh-CN,zh;q=0.9,en;q=0.8",
            "cache-control": "no-cache",
            "cookie": auth.cookie_str if auth else "",
            "pragma": "no-cache",
            "upgrade-insecure-requests": "1",
            "user-agent": DEFAULT_UA,
        }
        response = requests.get(url, headers=headers, verify=False, timeout=20)
        return re.findall(r'\\"user_unique_id\\":\\"(.*?)\\"', response.text)[0]
    except Exception:
        return generate_fake_webid()


def generate_millisecond() -> int:
    return int(round(time.time() * 1000))


def splice_url(params: dict[str, Any]) -> str:
    parts = []
    for key, value in params.items():
        if value is None:
            value = ""
        parts.append(key + "=" + urllib.parse.quote(str(value)))
    return "&".join(parts)


def build_access_key(fp_id: str, app_key: str, device_id: str) -> str:
    raw = f"{fp_id + app_key + device_id}f8a69f1719916z"
    return hashlib.md5(raw.encode("utf-8")).hexdigest()


DEFAULT_UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/117.0"
