#!/usr/bin/env python3
"""
抖音登录凭证刷新（保活）——用旧 cookie 打开抖音、多页走一圈、抓回刷新后的 cookie 重新保存。

为什么需要：sessionid 等关键 cookie 会随时间过期；定期带着有效 cookie 访问几个抖音页面，
服务端会下发刷新后的 Set-Cookie（滑动续期），把它们抓回来重存即可延长登录态、推迟掉线。
账号管理里「登录超过 3 天」的账号会提示刷新，点击后跑本脚本。

关键正确性：**像登录一样访问 www.douyin.com / user/self / chat 抓全量 cookie + storage_state**
（复用 douyin_cookie_utils），保存的 cookie.json 与一次新登录结构一致——绝不只抓某个子域子集
（那会把 www.douyin.com 的 sessionid 等丢掉、搞坏登录态，发布器上踩过这个坑）。

用法：
  python3 douyin_refresh.py --account-dir <cookies/douyin/<name>>

stdout 协议（每行一个 JSON）：
  {"type":"progress","stage":"注入登录态","progress":30}
  {"type":"result","success":true,"user_info":{...}}
  {"type":"result","success":false,"error":"cookie 已失效，请重新扫码登录"}
"""

import argparse
import json
import os
import sys
import time
import traceback
from pathlib import Path

# compat / cookie 工具与本脚本同目录（scripts/ 根）
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    from DrissionPage import ChromiumPage, ChromiumOptions
except ImportError:
    print(json.dumps({"type": "result", "success": False,
                      "error": "DrissionPage 未安装，请运行 pip install DrissionPage"}), flush=True)
    sys.exit(1)

from compat import get_chrome_path  # noqa: E402
from douyin_cookie_utils import (  # noqa: E402
    extract_user_info, get_cookies_from_page, get_storage_state, save_cookie_files,
)


def emit(stage: str, progress: int) -> None:
    print(json.dumps({"type": "progress", "stage": stage, "progress": progress},
                     ensure_ascii=False), flush=True)


def result(success: bool, user_info: dict = None, error: str = "") -> None:
    print(json.dumps({"type": "result", "success": success,
                      "user_info": user_info or {}, "error": error},
                     ensure_ascii=False), flush=True)


def log(msg: str) -> None:
    print(f"[refresh] {msg}", file=sys.stderr, flush=True)


def load_cookies(account_dir: Path) -> list:
    cookie_json = account_dir / "cookie.json"
    if not cookie_json.exists():
        raise FileNotFoundError(f"找不到 cookie.json：{cookie_json}")
    data = json.loads(cookie_json.read_text(encoding="utf-8"))
    cookies = data.get("cookies") or []
    if not cookies:
        raise ValueError("cookie.json 中没有 cookies")
    for c in cookies:
        if isinstance(c, dict) and not c.get("domain"):
            c["domain"] = ".douyin.com"
    return cookies


def build_browser(account_name: str) -> ChromiumPage:
    """独立 user-data 目录 + 自动端口起 headless Chrome（不碰登录用的 9222）。"""
    data_dir = os.environ.get("AUTOCAST_DATA_DIR") or str(Path.home() / ".autocast")
    profile = Path(data_dir) / "refresh" / "chrome_profile" / account_name
    profile.mkdir(parents=True, exist_ok=True)

    co = ChromiumOptions()
    chrome_path = get_chrome_path()
    if chrome_path:
        co.set_browser_path(chrome_path)
    co.set_user_data_path(str(profile))
    co.auto_port()  # DrissionPage 自起，会自动带 --remote-allow-origins=*
    co.headless(True)
    co.set_argument("--no-first-run")
    co.set_argument("--no-default-browser-check")
    co.set_argument("--window-size", "1280,900")
    return ChromiumPage(co)


def is_logged_in(page) -> bool:
    for kw in ("手机号登录", "扫码登录", "二维码登录"):
        try:
            if page.ele(f"text:{kw}", timeout=1):
                return False
        except Exception:
            continue
    return True


def run(args) -> None:
    account_dir = Path(args.account_dir)
    account_name = account_dir.name

    emit("准备浏览器", 5)
    cookies = load_cookies(account_dir)
    page = None
    try:
        page = build_browser(account_name)

        emit("注入登录态", 25)
        page.get("https://www.douyin.com")
        page.set.cookies(cookies)
        page.get("https://www.douyin.com")  # 带 cookie 重新加载
        time.sleep(3)

        if not is_logged_in(page):
            result(False, error="cookie 已失效（旧凭证已无法登录），请重新扫码登录该账号")
            return

        # 多页走一圈让服务端下发刷新 cookie：extract_user_info 会跳 /user/self，
        # get_storage_state 会跳 www.douyin.com 首页 + /chat —— 天然覆盖多个页面。
        # 注意 extract_user_info 返回的是 (name, id, avatar) 元组，要转成 dict（与登录流程一致）。
        emit("访问个人主页", 45)
        user_name, user_id, avatar = extract_user_info(page)

        # 真正的登录态判定：首页的「扫码登录」文案不一定出现，is_logged_in 会漏判；
        # 个人主页能否取到真实 user_id/昵称才是可靠信号。取不到 → 视为已掉线，
        # **不写回 cookie**（避免用掉线态覆盖、搞坏现有凭证），直接报失效让用户重新扫码。
        if not user_id and (not user_name or user_name in ("未登录", "登录")):
            result(False, error="cookie 已失效（个人主页显示未登录），请重新扫码登录该账号")
            return

        user_info = {
            "user_id": user_id or "",
            "name": user_name or "",
            "avatar": avatar or "",
        }

        emit("访问首页/私信页刷新凭证", 65)
        storage_state = get_storage_state(page)

        emit("抓取刷新后的 Cookie", 85)
        fresh_cookies = get_cookies_from_page(page)
        if not fresh_cookies:
            result(False, error="未能抓到刷新后的 Cookie")
            return

        save_cookie_files(str(account_dir), fresh_cookies, user_info, storage_state)
        emit("已更新凭证", 100)
        log(f"刷新完成：{len(fresh_cookies)} 条 cookie，user={user_info.get('name')}")
        result(True, user_info=user_info)
    finally:
        if page is not None:
            try:
                page.quit()
            except Exception:
                pass


def main() -> None:
    parser = argparse.ArgumentParser(description="抖音登录凭证刷新")
    parser.add_argument("--account-dir", required=True, help="账号 cookie 目录 cookies/douyin/<name>")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as e:
        log(traceback.format_exc())
        result(False, error=str(e))


if __name__ == "__main__":
    main()
