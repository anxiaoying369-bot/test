#!/usr/bin/env python3
"""
账号 Cookie 三层验证脚本
用法：
  python3 verify_account.py <platform> <cookie_path>

原理（Spreado 三层验证）：
  L1: storage_state.json expires 时间戳预检（~0ms，无浏览器）
  L2: Playwright positive DOM 检测（authed_selectors 出现 = 已登录）
  L3: Playwright negative DOM 检测（login_selectors 出现 = 未登录）
"""

import argparse
import json
import os
import sys
import time
from pathlib import Path

# ============ 平台配置 ============

PLATFORM_CONFIG = {
    "douyin": {
        # L1: 检查 storage_state 中关键 cookie 的 expires
        "check_urls": ["https://www.douyin.com/creator-micro/content/upload"],
        "critical_cookies": ["sessionid", "sessionid_ss", "passport_auth_id"],
        "authed_selectors": [
            # 上传页面特有的元素
            "input[placeholder*='标题']",
            "input[placeholder*='作品标题']",
            "[class*='upload']",
            "[class*='video-upload']",
            "[data-e2e='upload-title']",
            # 创作者后台通用
            "div[class*='container']",
            "[class*='creator-content']",
        ],
        "login_selectors": [
            "text='手机号登录'",
            "text='扫码登录'",
            "text='登录'",
            ".login-btn",
            "[class*='login-mask']",
        ],
        # L2 positive — HTTP API
        "http_check": {
            "method": "GET",
            "url": "https://www.douyin.com",
            "auth_headers": ["cookie"],
            "success_status": [200],
        },
    },
    "xiaohongshu": {
        "check_urls": ["https://edith.xiaohongshu.com"],
        "critical_cookies": ["web_session", "webId"],
        "authed_selectors": [
            "text='我'",
            "[class*='user-avatar']",
            "[class*='side-bar']",
            "[class*='header-container']",
            ".publish-btn",
            "[class*='upload']",
        ],
        "login_selectors": [
            "text='登录'",
            "text='手机号登录'",
            "text='扫码登录'",
            "[class*='login']",
        ],
        "http_check": {
            "method": "GET",
            "url": "https://edith.xiaohongshu.com/explore",
            "auth_headers": ["cookie"],
            "success_status": [200],
        },
    },
}


# ============ L1: expires 时间戳预检 ============

def check_cookie_expiry(storage_state: dict, critical_cookies: list) -> dict:
    """
    检查 storage_state.json 中关键 cookie 是否已过期。
    返回: {"expired": bool, "detail": str}
    """
    if not storage_state:
        return {"expired": True, "detail": "storage_state 为空"}

    cookies = storage_state.get("cookies", [])
    if isinstance(cookies, list) and len(cookies) == 0:
        return {"expired": True, "detail": "cookies 列表为空"}

    cookie_map = {}
    for c in cookies:
        name = c.get("name", "")
        cookie_map[name] = c

    now = time.time()
    expired_names = []
    missing_names = []

    for crit in critical_cookies:
        if crit not in cookie_map:
            missing_names.append(crit)
            continue
        exp = cookie_map[crit].get("expires", -1)
        if exp > 0 and exp < now:
            expired_names.append(crit)

    if expired_names:
        return {
            "expired": True,
            "detail": f"已过期: {expired_names}",
        }
    if missing_names:
        # 部分关键 cookie 缺失，不算过期（可能是 HttpOnly 等）
        return {
            "expired": False,
            "detail": f"缺失但未过期: {missing_names}",
        }

    return {
        "expired": False,
        "detail": "所有关键 cookie 均未过期",
    }


def load_storage_state(cookie_path: str) -> dict:
    """加载 account.json（Playwright storage_state 格式）"""
    path = Path(cookie_path)
    if not path.exists():
        return {}

    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # 兼容旧格式 cookie.json
    if "cookies" in data and "origins" not in data:
        return data

    # 标准 Playwright storage_state 格式
    if "cookies" in data or "origins" in data:
        return data

    return {}


# ============ L2: HTTP API 快速检查 ============

def check_http_api(platform: str, cookie_path: str) -> dict:
    """
    用 HTTP 请求快速验证 cookie 是否有效（不启动浏览器，最快 ~200ms）。
    适用于小红书的 edith.xiaohongshu.com 等内部 API。
    """
    config = PLATFORM_CONFIG.get(platform, {})
    http_cfg = config.get("http_check")
    if not http_cfg:
        return {"valid": None, "detail": "该平台无 HTTP API 检查"}

    try:
        with open(cookie_path, "r", encoding="utf-8") as f:
            cookie_data = json.load(f)

        cookies = cookie_data.get("cookies", [])
        cookie_str = "; ".join(f"{c['name']}={c['value']}" for c in cookies if "name" in c and "value" in c)

        import urllib.request
        req = urllib.request.Request(http_cfg["url"])
        req.add_header("Cookie", cookie_str)
        req.add_header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        req.add_header("Referer", "https://www.xiaohongshu.com/")

        with urllib.request.urlopen(req, timeout=10) as resp:
            status = resp.status
            if status in http_cfg.get("success_status", [200]):
                body = resp.read().decode("utf-8", errors="ignore")
                # 小红书：检查返回内容是否含登录标识
                if "登录" in body and "user_id" not in body:
                    return {"valid": False, "detail": f"HTTP 返回登录页 (status={status})"}
                return {"valid": True, "detail": f"HTTP API 返回 {status}"}
            else:
                return {"valid": False, "detail": f"HTTP 状态码 {status}"}

    except Exception as e:
        return {"valid": None, "detail": f"HTTP 检查异常: {e}"}


# ============ L3: Playwright DOM 检测 ============

def check_playwright(platform: str, cookie_path: str) -> dict:
    """
    Playwright 三层 DOM 检测（参考 Spreado 架构）：
      - positive DOM 出现 → 已登录
      - negative DOM 出现 → 未登录
      - 两者都不出现 → 无法判断（超时）
    """
    config = PLATFORM_CONFIG.get(platform, {})
    check_urls = config.get("check_urls", [])
    authed_selectors = config.get("authed_selectors", [])
    login_selectors = config.get("login_selectors", [])

    if not check_urls:
        return {"status": "unknown", "detail": "无检查 URL"}

    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        return {"status": "unknown", "detail": "playwright 未安装"}

    pw = None
    browser = None
    try:
        pw = sync_playwright().start()
        browser = pw.chromium.launch(headless=True, channel="chrome")
        context = browser.new_context(storage_state=cookie_path)
        page = context.new_page()

        results = []

        for url in check_urls:
            try:
                page.goto(url, timeout=15000, wait_until="domcontentloaded")
                time.sleep(2)

                # positive 检测
                for sel in authed_selectors:
                    try:
                        if sel.startswith("text="):
                            el = page.get_by_text(sel[5:], exact=False)
                        else:
                            el = page.locator(sel).first
                        if el.is_visible(timeout=2000):
                            results.append(("authed", sel))
                            break
                    except:
                        pass

                # negative 检测
                for sel in login_selectors:
                    try:
                        if sel.startswith("text="):
                            el = page.get_by_text(sel[5:], exact=False)
                        else:
                            el = page.locator(sel).first
                        if el.is_visible(timeout=2000):
                            results.append(("login", sel))
                            break
                    except:
                        pass

            except Exception as e:
                results.append(("error", str(e)))

        browser.close()
        pw.stop()

        # 解析结果
        has_authed = any(r[0] == "authed" for r in results)
        has_login = any(r[0] == "login" for r in results)

        if has_authed and not has_login:
            return {"status": "valid", "detail": "positive DOM 检测通过"}
        elif has_login and not has_authed:
            return {"status": "invalid", "detail": "login selector 检测到未登录"}
        elif has_authed and has_login:
            # 两者都出现，以 positive 为准（已登录页面也可能显示登录引导）
            return {"status": "valid", "detail": "positive DOM 检测通过（忽略同页面 login 元素）"}
        else:
            return {"status": "unknown", "detail": f"DOM 检测无结果: {results}"}

    except Exception as e:
        try:
            if browser:
                browser.close()
            if pw:
                pw.stop()
        except Exception:
            pass
        return {"status": "unknown", "detail": f"Playwright 检测异常: {e}"}


# ============ 主验证流程 ============

def verify(platform: str, cookie_path: str) -> dict:
    """三层验证入口"""
    result = {
        "status": "unknown",
        "method": "none",
        "message": "",
        "layers": {},
    }

    if platform not in PLATFORM_CONFIG:
        result["status"] = "error"
        result["message"] = f"不支持的平台: {platform}"
        return result

    config = PLATFORM_CONFIG[platform]

    # ---- L1: expires 时间戳预检 ----
    storage_state = load_storage_state(cookie_path)
    l1 = check_cookie_expiry(storage_state, config["critical_cookies"])
    result["layers"]["L1_expiry"] = l1

    if l1["expired"]:
        result["status"] = "invalid"
        result["method"] = "L1_expiry"
        result["message"] = f"Cookie 已过期: {l1['detail']}"
        return result

    # ---- L2: HTTP API 检查 ----
    l2 = check_http_api(platform, cookie_path)
    result["layers"]["L2_http"] = l2

    if l2.get("valid") is True:
        result["status"] = "valid"
        result["method"] = "L2_http"
        result["message"] = f"HTTP API 验证通过: {l2['detail']}"
        return result

    if l2.get("valid") is False:
        # HTTP 明确返回未登录，降级 L3
        result["layers"]["L3_fallback_reason"] = f"HTTP 验证失败: {l2['detail']}"

    # ---- L3: Playwright DOM 检测 ----
    l3 = check_playwright(platform, cookie_path)
    result["layers"]["L3_playwright"] = l3

    if l3["status"] == "valid":
        result["status"] = "valid"
        result["method"] = "L3_playwright"
        result["message"] = f"Playwright DOM 验证通过: {l3['detail']}"
    elif l3["status"] == "invalid":
        result["status"] = "invalid"
        result["method"] = "L3_playwright"
        result["message"] = f"Playwright DOM 检测未登录: {l3['detail']}"
    else:
        # L3 也无法判断
        result["status"] = "unknown"
        result["method"] = "L3_playwright"
        result["message"] = f"无法判断: {l3['detail']}"

    return result


# ============ 入口 ============

def main():
    parser = argparse.ArgumentParser(description="账号 Cookie 三层验证")
    parser.add_argument("platform", help="平台: douyin, xiaohongshu")
    parser.add_argument("cookie_path", help="account.json 路径")
    args = parser.parse_args()

    result = verify(args.platform, args.cookie_path)

    print(json.dumps(result, ensure_ascii=False, indent=2))
    sys.exit(0 if result["status"] == "valid" else 1)


if __name__ == "__main__":
    main()
