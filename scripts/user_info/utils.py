import os
import re
import sys
import json
import time
from pathlib import Path
from typing import Optional
from urllib.parse import urlparse, parse_qs

# Base64 风格: 字母+数字+'-'+'_', 长度 30+, 常以 MS4 开头 (抖音 sec_uid 通用前缀)
SEC_UID_PATTERN = re.compile(r"^[A-Za-z0-9_\-]{20,}$")
# 纯数字 (uid 或 short_id 都需要走进一步判定)
NUMERIC_PATTERN = re.compile(r"^\d{6,20}$")
# 抖音号 (unique_id): 字母/数字/下划线/点, 长度 2-30, 用户自设
UNIQUE_ID_PATTERN = re.compile(r"^[A-Za-z0-9_.]{2,30}$")

def _log(msg: str) -> None:
    """所有日志走 stderr，stdout 留给 JSON 结果。"""
    print(msg, flush=True, file=sys.stderr)

def classify_user_id(raw: str) -> str:
    """
    分类输入 ID 的类型。
    返回: 'sec_uid' | 'uid_or_short' | 'unique_id' | 'url' | 'unknown'
    """
    raw = raw.strip()
    if not raw:
        return "unknown"

    # 1. URL
    if raw.startswith(("http://", "https://", "v.douyin.com/")):
        return "url"

    # 2. sec_uid (Base64 风格)
    if SEC_UID_PATTERN.match(raw) and not raw.isdigit():
        return "sec_uid"

    # 3. 纯数字 → uid 或 short_id, 进一步判定需要拉一次
    if NUMERIC_PATTERN.match(raw):
        return "uid_or_short"

    # 4. 抖音号
    if UNIQUE_ID_PATTERN.match(raw):
        return "unique_id"

    return "unknown"

def parse_share_url(url: str) -> dict:
    result = {"sec_uid": None, "uid": None, "short_id": None, "unique_id": None, "raw": url}
    try:
        u = urlparse(url)
        host = (u.netloc or "").lower()
        path = u.path or ""
        query = parse_qs(u.query)

        # 1. /user/{sec_uid} 形式
        m = re.match(r"^/user/([A-Za-z0-9_\-]+)/?$", path)
        if m and "douyin.com" in host and "iesdouyin" not in host:
            result["sec_uid"] = m.group(1)
            if "unique_id" in query:
                result["unique_id"] = query["unique_id"][0]

        # 2. /share/user/{uid}?sec_uid=xxx 形式
        m = re.match(r"^/share/user/?$", path)
        if not m:
            m = re.match(r"^/share/user/(\d+)/?$", path)
            if m:
                result["uid"] = m.group(1)
        if "sec_uid" in query:
            result["sec_uid"] = query["sec_uid"][0]

        if "uniqueId" in query and not result["unique_id"]:
            result["unique_id"] = query["uniqueId"][0]
        if "unique_id" in query and not result["unique_id"]:
            result["unique_id"] = query["unique_id"][0]

    except Exception as e:
        _log(f"[GET_USER] URL 解析失败 (非致命): {e}")

    return result

def load_cookie_data(cookie_path: str) -> list:
    path = Path(cookie_path)
    if not path.exists():
        raise FileNotFoundError(f"cookie 文件不存在: {cookie_path}")

    suffix = path.suffix.lower()
    if suffix == ".json":
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, dict) and "cookies" in data:
            cookies = data["cookies"]
        elif isinstance(data, list):
            cookies = data
        else:
            raise ValueError(f"cookie.json 格式异常: {type(data).__name__}")
        for c in cookies:
            if isinstance(c, dict) and "path" not in c:
                c["path"] = "/"
        return cookies

    raw = path.read_text(encoding="utf-8").strip()
    if not raw:
        raise ValueError("cookie 文件为空")

    cookies = []
    for part in raw.split(";"):
        part = part.strip()
        if not part or "=" not in part:
            continue
        name, _, value = part.partition("=")
        cookies.append({
            "name": name.strip(),
            "value": value.strip(),
            "domain": ".douyin.com",
            "path": "/",
        })
    return cookies

def expand_short_url(page, short_url: str) -> str:
    try:
        page.get(short_url)
        time.sleep(1.5)
        final = page.url or ""
        if final and final != short_url:
            _log(f"[GET_USER] 短链展开: {short_url[:40]}... → {final[:80]}")
            return final
    except Exception as e:
        _log(f"[GET_USER] 短链展开失败: {e}")
    return short_url

def build_user_home_url(sec_uid: str) -> str:
    return f"https://www.douyin.com/user/{sec_uid}"

def save_user_json(sec_uid: str, user_data: dict, data_dir_base: Optional[str] = None) -> str:
    if data_dir_base:
        base = Path(data_dir_base)
    else:
        # Import here to avoid circular dependency or requiring main script setup
        from compat import get_data_dir
        base = get_data_dir() / "scraper_data"

    out_path = base / sec_uid / "data" / sec_uid / "user.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    payload = {
        "sec_uid": sec_uid,
        "uid": user_data.get("uid"),
        "short_id": user_data.get("short_id"),
        "unique_id": user_data.get("unique_id"),
        "nickname": user_data.get("nickname"),
        "signature": user_data.get("signature"),
        "avatar_url": user_data.get("avatar_url"),
        "avatar_larger": user_data.get("avatar_larger"),
        "avatar_medium": user_data.get("avatar_medium"),
        "avatar_thumb": user_data.get("avatar_thumb"),
        "follower_count": user_data.get("follower_count"),
        "following_count": user_data.get("following_count"),
        "aweme_count": user_data.get("aweme_count"),
        "favoriting_count": user_data.get("favoriting_count"),
        "total_favorited": user_data.get("total_favorited"),
        "gender": user_data.get("gender"),
        "location": user_data.get("location"),
        "school": user_data.get("school"),
        "enterprise_verify_reason": user_data.get("enterprise_verify_reason"),
        "custom_verify": user_data.get("custom_verify"),
        "verification_type": user_data.get("verification_type"),
        "is_verified": user_data.get("is_verified"),
        "share_url": user_data.get("share_url"),
        "queried_at": int(time.time()),
        "source_strategy": user_data.get("source_strategy"),
    }
    payload = {k: v for k, v in payload.items() if v is not None and v != ""}
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(payload, f, ensure_ascii=False, indent=2)
    _log(f"[GET_USER] user.json 已写入: {out_path}")
    return str(out_path)
