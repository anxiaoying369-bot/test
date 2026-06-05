#!/usr/bin/env python3
"""根据 sec_uid 获取抖音用户完整资料（签名 API 版，无需浏览器）。

复用 DouyinComment 的签名客户端 DouyinClient（a_bogus），调用
`/aweme/v1/web/user/profile/other/`，返回昵称 / 抖音号 / 粉丝 / 关注 /
获赞 / 作品 / 头像 / sec_uid 等。比浏览器抓取（douyin_get_user_info.py）
快得多，是用户资料查询的主路径。

用法:
  python3 douyin_user_profile.py --cookie-path <cookie.txt> --sec-uid <MS4w...>

stdout: 只输出最终 JSON 结果（供 Rust 解析）
stderr: 日志
"""

import argparse
import asyncio
import json
import os
import sys
from pathlib import Path

# DouyinComment 的签名客户端位于 scripts/DouyinComment/scripts/
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(SCRIPT_DIR, "DouyinComment", "scripts"))
sys.path.insert(0, SCRIPT_DIR)


def _log(msg: str):
    print(msg, flush=True, file=sys.stderr)


def load_cookie_string(cookie_path: str) -> str:
    path = Path(cookie_path)
    if not path.exists():
        raise FileNotFoundError(f"cookie 文件不存在: {cookie_path}")
    cookie_str = path.read_text(encoding="utf-8").strip()
    if not cookie_str:
        raise ValueError("cookie 文件为空")
    return cookie_str


def _first_url(obj) -> str:
    if not isinstance(obj, dict):
        return ""
    urls = obj.get("url_list") or []
    return urls[0] if urls else ""


def _parse_user(user: dict) -> dict:
    avatar = (
        _first_url(user.get("avatar_larger"))
        or _first_url(user.get("avatar_300x300"))
        or _first_url(user.get("avatar_medium"))
        or _first_url(user.get("avatar_thumb"))
    )
    # 抖音号：优先自定义 unique_id，未设置时用 short_id
    douyin_id = user.get("unique_id") or ""
    if not douyin_id:
        douyin_id = str(user.get("short_id") or "")

    return {
        "sec_uid": user.get("sec_uid") or "",
        "uid": str(user.get("uid") or ""),
        "unique_id": douyin_id,
        "nickname": user.get("nickname") or "",
        "avatar_url": avatar,
        "signature": user.get("signature") or "",
        "follower_count": user.get("follower_count") or 0,
        "following_count": user.get("following_count") or 0,
        "total_favorited": user.get("total_favorited") or 0,
        "aweme_count": user.get("aweme_count") or 0,
        "ip_location": user.get("ip_location") or "",
        "custom_verify": user.get("custom_verify") or "",
        "enterprise_verify_reason": user.get("enterprise_verify_reason") or "",
    }


async def fetch_profile(cookie_str: str, sec_uid: str) -> dict:
    try:
        from douyin_api import DouyinClient, CookieExpiredError
    except ImportError as e:
        return {"status": "error", "error": f"无法加载签名客户端: {e}"}

    client = DouyinClient(cookie_str)
    try:
        _log(f"[PROFILE] 请求 user/profile/other sec_uid={sec_uid[:16]}...")
        d = await client._get(
            "/aweme/v1/web/user/profile/other/",
            {
                "sec_user_id": sec_uid,
                "source": "channel_pc_web",
                "publish_video_strategy_type": "2",
            },
        )
        user = d.get("user")
        if not user:
            return {"status": "not_found", "error": "接口未返回用户信息（用户可能不存在或已注销）"}
        info = _parse_user(user)
        _log(f"[PROFILE] 成功: {info.get('nickname')} (抖音号 {info.get('unique_id')}, 粉丝 {info.get('follower_count')})")
        return {"status": "ok", "user": info}
    except CookieExpiredError as e:
        _log(f"[PROFILE] Cookie 失效: {e}")
        return {"status": "cookie_expired", "error": str(e)}
    except Exception as e:
        _log(f"[PROFILE] 请求异常: {e}")
        return {"status": "error", "error": str(e)}
    finally:
        await client.close()


def main():
    parser = argparse.ArgumentParser(description="根据 sec_uid 获取抖音用户完整资料")
    parser.add_argument("--cookie-path", required=True, help="cookie.txt 路径")
    parser.add_argument("--sec-uid", required=True, help="目标用户 sec_uid")
    args = parser.parse_args()

    sec_uid = args.sec_uid.strip()
    if not sec_uid:
        print(json.dumps({"status": "error", "error": "sec_uid 为空"}, ensure_ascii=False))
        sys.exit(1)

    try:
        cookie_str = load_cookie_string(args.cookie_path)
    except Exception as e:
        print(json.dumps({"status": "error", "error": f"加载 cookie 失败: {e}"}, ensure_ascii=False))
        sys.exit(1)

    result = asyncio.run(fetch_profile(cookie_str, sec_uid))
    print(json.dumps(result, ensure_ascii=False))
    sys.exit(0 if result.get("status") == "ok" else 1)


if __name__ == "__main__":
    main()
