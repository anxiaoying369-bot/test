import asyncio
import hashlib
import json
import os
import time
import sys
from typing import Any, Dict, List, Optional, Tuple
import httpx
from urllib.parse import urlencode, urlparse

DEFAULT_UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"


def _douyin_params() -> Dict[str, Any]:
    return {
        "aid": 6383,
        "device_platform": "webapp",
        "channel": "channel_pc_web",
        "version_code": 170400,
        "version_name": "17.4.0",
        "platform": "PC",
        "pc_client_type": 1,
        "cookie_enabled": "true",
        "browser_language": "zh-CN",
        "browser_platform": "Windows",
        "browser_name": "Chrome",
        "browser_version": "124.0.0.0",
        "browser_online": "true",
        "engine_name": "Blink",
        "engine_version": "124.0.0.0",
        "os_name": "Windows",
    }


FAST_CHECKS: Dict[str, Dict[str, Any]] = {
    "xiaohongshu": {
        "method": "GET",
        "url": "https://edith.xiaohongshu.com/api/sns/web/v1/user/selfinfo",
        "domain_filter": "xiaohongshu.com",
        "headers": {
            "Referer": "https://creator.xiaohongshu.com/",
            "Origin": "https://creator.xiaohongshu.com",
        },
        "ok_key": lambda r: r.get("code") == 0,
        "extract": lambda r: (
            str((r.get("data") or {}).get("userId") or (r.get("data") or {}).get("user_id") or ""),
            (r.get("data") or {}).get("nickname") or (r.get("data") or {}).get("name") or "",
            (r.get("data") or {}).get("image") or (r.get("data") or {}).get("avatar") or "",
        ),
    },
    "douyin": {
        "method": "GET",
        "url": "https://www.douyin.com/aweme/v1/web/user/info/",
        "domain_filter": "douyin.com",
        "headers": {
            "Referer": "https://www.douyin.com/",
            "Origin": "https://www.douyin.com",
            "User-Agent": DEFAULT_UA,
        },
        "params": _douyin_params(),
        "ok_key": lambda r: r.get("status_code") == 0,
        "extract": lambda r: (
            str(((r.get("user_info") or {}).get("uid")) or ""),
            ((r.get("user_info") or {}).get("nickname")) or "",
            ((r.get("user_info") or {}).get("avatar_url")) or "",
        ),
    },
}


def _extract_cookies(data: Any) -> List[Dict[str, Any]]:
    cookies: List[Dict[str, Any]] = []
    if isinstance(data, dict):
        cookie_info = data.get("cookie_info")
        if isinstance(cookie_info, dict) and isinstance(cookie_info.get("cookies"), list):
            cookies.extend(cookie_info.get("cookies", []))
        if isinstance(data.get("cookies"), list):
            cookies.extend(data.get("cookies", []))
        if isinstance(data.get("origins"), list):
            for origin in data.get("origins", []):
                if isinstance(origin, dict) and isinstance(origin.get("cookies"), list):
                    cookies.extend(origin.get("cookies", []))
        if isinstance(data.get("cookie"), list):
            cookies.extend(data.get("cookie", []))
    elif isinstance(data, list):
        cookies.extend(data)
    return cookies


def _cookie_header_from_data(data: Any, domain_filter: Optional[str] = None) -> str:
    if isinstance(data, str):
        return data.strip()
    if isinstance(data, dict):
        for key in ("raw", "cookie", "cookie_str", "cookieString"):
            val = data.get(key)
            if isinstance(val, str) and val.strip():
                return val.strip()
    cookies = _extract_cookies(data)
    return _cookie_header(cookies, domain_filter=domain_filter)


def _cookie_header(cookies: List[Dict[str, Any]], domain_filter: Optional[str] = None) -> str:
    pairs: List[str] = []
    for item in cookies:
        if not isinstance(item, dict):
            continue
        name = item.get("name")
        value = item.get("value")
        if not name or value is None:
            continue
        domain = item.get("domain") or ""
        if domain_filter and domain and domain_filter not in str(domain):
            continue
        pairs.append(f"{name}={value}")
    return "; ".join(pairs)


def _xhs_sign_path(url: str) -> str:
    parsed = urlparse(url)
    path = parsed.path or "/"
    if parsed.query:
        path = f"{path}?{parsed.query}"
    return path


async def _sign_xhs(url: str, data: Any, cookie_data: Any) -> Optional[Dict[str, str]]:
    signer_url = os.getenv("XHS_SIGNER_URL")
    if not signer_url: return None
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.post(f"{signer_url.rstrip('/')}/sign", json={"uri": _xhs_sign_path(url), "data": data})
            if resp.status_code < 400: return resp.json()
    except: pass
    return None


async def validate_cookie(platform: str, cookie_file: str) -> Dict[str, Any]:
    if platform not in FAST_CHECKS: return {"status": "error", "error": "Unsupported platform"}
    if not os.path.exists(cookie_file): return {"status": "error", "error": "File not found"}

    try:
        with open(cookie_file, "r", encoding="utf-8") as f: data = json.load(f)
    except: return {"status": "error", "error": "Invalid JSON"}

    local_info = data.get("user_info") or {}
    local_uid, local_name, local_avatar = local_info.get("user_id"), local_info.get("name"), local_info.get("avatar")
    
    conf = FAST_CHECKS[platform]
    cookie_header = _cookie_header_from_data(data, domain_filter=conf.get("domain_filter"))

    if not cookie_header:
        return {"status": "expired", "user_id": local_uid, "name": local_name, "avatar": local_avatar, "data": data}

    headers = {"User-Agent": DEFAULT_UA, "Cookie": cookie_header}
    headers.update(conf.get("headers") or {})

    try:
        async with httpx.AsyncClient(timeout=10.0, follow_redirects=True) as client:
            resp = await client.get(conf["url"], headers=headers, params=conf.get("params"))
            
            # 处理小红书签名重试
            if platform == "xiaohongshu" and resp.status_code == 406:
                signed = await _sign_xhs(conf["url"], None, data)
                if signed: resp = await client.get(conf["url"], headers={**headers, **signed})

            # 解析结果
            if resp.status_code < 400:
                resp_json = resp.json()
                if conf["ok_key"](resp_json):
                    uid, name, avatar = conf["extract"](resp_json)
                    return {"status": "valid", "user_id": uid or local_uid, "name": name or local_name, "avatar": avatar or local_avatar, "data": data}
            
            # 失效路径：必须带上 data
            return {"status": "expired", "user_id": local_uid, "name": local_name, "avatar": local_avatar, "data": data}
    except Exception as e:
        return {"status": "error", "user_id": local_uid, "name": local_name, "data": data, "error": str(e)}

if __name__ == "__main__":
    if len(sys.argv) >= 3:
        print(json.dumps(asyncio.run(validate_cookie(sys.argv[1], sys.argv[2])), ensure_ascii=False))
