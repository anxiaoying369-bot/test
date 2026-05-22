#!/usr/bin/env python3
"""
账号 Cookie 验证脚本（DrissionPage CDP 版）
用法：
  python3 verify_account.py <platform> <cookie_path>

验证流程（两层）：
  L1: cookie.json expires 时间戳预检（~0ms，无浏览器）
  L2: 注入 cookie 到浏览器 → 访问页面 → 读取页面信息判断登录是否失效（CDP）

注意：日志输出到 stderr，stdout 只输出最终 JSON 结果（供 Rust 解析）
"""

import argparse
import json
import os
import socket
import sys
import time
from pathlib import Path


def _log(msg: str):
    """日志输出到 stderr，不污染 stdout"""
    print(msg, flush=True, file=sys.stderr)


# ============ 配置 ============

CDP_PORT = 9222
CHROME_PATH = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
CHROME_USER_DATA_DIR = os.path.expanduser("~/chrome-debug-profile")

PLATFORM_CONFIG = {
    "douyin": {
        "critical_cookies": ["sessionid", "sessionid_ss", "passport_auth_id"],
        "verify_url": "https://www.douyin.com/",
        "cookie_domain": ".douyin.com",
        "authed_indicators": [
            "[data-e2e='user-info-name']",
            "[class*='avatar-']",
            "[class*='user-info']",
        ],
        "login_indicators": [
            "手机号登录",
            "扫码登录",
        ],
        "js_user_info": """() => {
            try {
                if (window._ROUTER_DATA?.loaderData) {
                    for (let key in window._ROUTER_DATA.loaderData) {
                        const data = window._ROUTER_DATA.loaderData[key];
                        if (data?.user) return {found: true, user: data.user};
                    }
                }
                if (window.userData) return {found: true, user: window.userData};
            } catch(e) {}
            return {found: false};
        }""",
    },
    "xiaohongshu": {
        "critical_cookies": ["web_session", "webId"],
        "verify_url": "https://www.xiaohongshu.com/",
        "cookie_domain": ".xiaohongshu.com",
        "authed_indicators": [
            "[class*='user-avatar']",
            "[class*='side-bar']",
            ".publish-btn",
        ],
        "login_indicators": [
            "手机号登录",
            "扫码登录",
        ],
        "js_user_info": """() => {
            try {
                if (window.__INITIAL_STATE__?.user?.userInfo) {
                    return {found: true, user: window.__INITIAL_STATE__.user.userInfo};
                }
            } catch(e) {}
            return {found: false};
        }""",
    },
}


# ============ 工具函数 ============

def is_port_in_use(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", port)) == 0


# ============ L1: expires 时间戳预检 ============

def check_cookie_expiry(cookie_data: dict, critical_cookies: list) -> dict:
    cookies = cookie_data.get("cookies", [])
    if not cookies:
        _log("[L1] 输入: cookie 列表为空")
        return {"expired": True, "detail": "cookies 列表为空"}

    _log(f"[L1] 输入: 共 {len(cookies)} 条 cookie, 关键字段: {critical_cookies}")

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
            _log(f"[L1]   关键 cookie '{crit}': 缺失")
            continue
        exp = cookie_map[crit].get("expires", -1)
        if exp > 0:
            if exp > 1e12:
                exp = exp / 1000
            remaining = exp - now
            if exp < now:
                expired_names.append(crit)
                _log(f"[L1]   关键 cookie '{crit}': 已过期 (expires={exp}, 超期 {abs(remaining):.0f}s)")
            else:
                _log(f"[L1]   关键 cookie '{crit}': 有效 (剩余 {remaining:.0f}s)")
        else:
            _log(f"[L1]   关键 cookie '{crit}': 无 expires 字段 (session cookie)")

    if expired_names:
        _log(f"[L1] 输出: expired=True, {expired_names}")
        return {"expired": True, "detail": f"已过期: {expired_names}"}
    if missing_names:
        _log(f"[L1] 输出: expired=False, 缺失 {missing_names}")
        return {"expired": False, "detail": f"缺失但未过期: {missing_names}"}
    _log("[L1] 输出: 所有关键 cookie 均未过期")
    return {"expired": False, "detail": "所有关键 cookie 均未过期"}


def load_cookie_data(cookie_path: str) -> dict:
    path = Path(cookie_path)
    if not path.exists():
        return {}
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, dict):
            return data
    except Exception:
        pass
    return {}


# ============ L2: CDP 浏览器验证 ============

def check_drissionpage(platform: str, cookie_data: dict) -> dict:
    """
    核心流程：
    1. 连接浏览器（优先已有 Chrome，否则启动 headless）
    2. 先访问目标域名（建立上下文）
    3. 注入所有 cookie
    4. 访问验证页面
    5. 从页面读取信息，判断登录是否有效
    """
    config = PLATFORM_CONFIG.get(platform, {})
    verify_url = config.get("verify_url")
    cookie_domain = config.get("cookie_domain", "")
    authed_indicators = config.get("authed_indicators", [])
    login_indicators = config.get("login_indicators", [])
    js_user_info = config.get("js_user_info")

    _log(f"[L2] 输入: platform={platform}, cookie 数量={len(cookie_data.get('cookies', []))}")
    _log(f"[L2]   verify_url={verify_url}, cookie_domain={cookie_domain}")

    if not verify_url:
        _log("[L2] 输出: 无验证 URL")
        return {"status": "unknown", "detail": "无验证 URL"}

    try:
        from DrissionPage import ChromiumPage, ChromiumOptions
    except ImportError:
        _log("[L2] 输出: DrissionPage 未安装")
        return {"status": "unknown", "detail": "DrissionPage 未安装"}

    page = None

    try:
        co = ChromiumOptions()
        co.set_browser_path(CHROME_PATH)

        if is_port_in_use(CDP_PORT):
            co.set_address(f"127.0.0.1:{CDP_PORT}")
        else:
            co.headless()
            co.set_argument("--no-sandbox")
            co.set_argument(f"--user-data-dir={CHROME_USER_DATA_DIR}")
            co.set_argument(f"--remote-debugging-port={CDP_PORT}")

        cdp_in_use = is_port_in_use(CDP_PORT)
        _log(f"[L2] 连接方式: {'接管已有 Chrome (port ' + str(CDP_PORT) + ')' if cdp_in_use else '启动 headless Chrome'}")

        try:
            page = ChromiumPage(co)
        except Exception as e:
            _log(f"[L2] 输出: 连接浏览器失败 {e}")
            return {"status": "unknown", "detail": f"连接浏览器失败: {e}"}

        try:
            page = page.latest_tab
        except Exception:
            pass

        # 1) 先访问目标域名根页面（建立浏览器上下文）
        domain_root = verify_url.split("/")[0] + "//" + verify_url.split("/")[2]
        _log(f"[L2] Step 1: 访问 {domain_root} 建立上下文")
        try:
            page.get(domain_root)
            time.sleep(2)
        except Exception as e:
            _log(f"[L2] 访问域名失败: {e}")

        # 2) 注入所有 cookie
        cookies = cookie_data.get("cookies", [])
        injected = 0
        _log(f"[L2] Step 2: 注入 {len(cookies)} 条 cookie")
        for c in cookies:
            try:
                name = c.get("name")
                value = c.get("value")
                if not name or value is None:
                    continue
                domain = c.get("domain", cookie_domain)
                path = c.get("path", "/")

                # httpOnly 的 cookie 用 CDP 设置
                if c.get("httpOnly"):
                    try:
                        page.run_cdp("Network.setCookie", **{
                            "name": name,
                            "value": str(value),
                            "domain": domain,
                            "path": path,
                            "secure": bool(c.get("secure", False)),
                            "httpOnly": True,
                        })
                        injected += 1
                    except Exception:
                        pass
                    continue

                # 普通 cookie 用 JS 注入
                cookie_str = f"{name}={value}; domain={domain}; path={path}"
                if c.get("secure"):
                    cookie_str += "; secure"
                page.run_js(f"document.cookie = '{cookie_str}'")
                injected += 1
            except Exception:
                continue

        _log(f"[L2] 成功注入 {injected}/{len(cookies)} 条 cookie")

        # 3) 访问验证页面
        _log(f"[L2] Step 3: 访问验证页面 {verify_url}")
        try:
            page.get(verify_url)
            time.sleep(4)
        except Exception as e:
            return {"status": "unknown", "detail": f"访问验证页面失败: {e}"}

        current_url = page.url or ""
        _log(f"[L2] 当前 URL: {current_url}")

        # 4) 从页面读取信息，判断登录状态

        # 4a) 用 JS 读用户信息
        user_info = None
        if js_user_info:
            try:
                js_result = page.run_js(js_user_info)
                if js_result and isinstance(js_result, dict) and js_result.get("found"):
                    user_info = js_result.get("user", {})
                    _log(f"[L2] JS 读到用户信息: {user_info}")
            except Exception as e:
                _log(f"[L2] JS 读取失败: {e}")

        # 4b) 检测页面元素
        found_authed = False
        found_login = False

        for sel in authed_indicators:
            try:
                el = page.ele(f"css:{sel}", timeout=2)
                if el:
                    found_authed = True
                    _log(f"[L2] 找到登录态元素: {sel}")
                    break
            except Exception:
                pass

        for text in login_indicators:
            try:
                el = page.ele(f"text={text}", timeout=2)
                if el:
                    found_login = True
                    _log(f"[L2] 找到未登录元素: {text}")
                    break
            except Exception:
                pass

        # 4c) 检查 URL 是否被重定向到登录页
        redirected_to_login = any(kw in current_url.lower() for kw in ["login", "passport", "signin"])

        # 5) 综合判断
        if user_info:
            name = user_info.get("nickname") or user_info.get("name") or ""
            uid = user_info.get("uniqueId") or user_info.get("userId") or user_info.get("user_id") or ""
            return {
                "status": "valid",
                "detail": f"已登录，用户: {name} (ID: {uid})",
                "user_info": {"name": name, "user_id": uid},
            }

        if found_authed and not found_login:
            return {"status": "valid", "detail": "页面检测到登录态元素"}

        if found_login and not found_authed:
            return {"status": "invalid", "detail": "页面检测到登录/注册元素，Cookie 已失效"}

        if redirected_to_login:
            return {"status": "invalid", "detail": f"页面被重定向到登录页: {current_url}"}

        if found_authed and found_login:
            return {"status": "valid", "detail": "页面检测到登录态元素（忽略登录引导）"}

        return {"status": "unknown", "detail": f"无法判断登录状态, URL={current_url}"}

    except Exception as e:
        return {"status": "unknown", "detail": f"DrissionPage 检测异常: {e}"}

    finally:
        try:
            if page:
                page.quit()
        except Exception:
            pass


# ============ 主验证流程 ============

def verify(platform: str, cookie_path: str) -> dict:
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

    # ---- L1 过期预检 ----
    cookie_data = load_cookie_data(cookie_path)
    l1 = check_cookie_expiry(cookie_data, config["critical_cookies"])
    result["layers"]["L1_expiry"] = l1

    if l1["expired"]:
        result["status"] = "invalid"
        result["method"] = "L1_expiry"
        result["message"] = f"Cookie 已过期: {l1['detail']}"
        return result

    # ---- L2 CDP 浏览器验证 ----
    l2 = check_drissionpage(platform, cookie_data)
    result["layers"]["L2_cdp"] = l2

    if l2["status"] == "valid":
        result["status"] = "valid"
        result["method"] = "L2_cdp"
        result["message"] = f"CDP 验证通过: {l2['detail']}"
    elif l2["status"] == "invalid":
        result["status"] = "invalid"
        result["method"] = "L2_cdp"
        result["message"] = f"CDP 检测未登录: {l2['detail']}"
    else:
        result["status"] = "unknown"
        result["method"] = "L2_cdp"
        result["message"] = f"无法判断: {l2['detail']}"

    return result


# ============ 入口 ============

def main():
    parser = argparse.ArgumentParser(description="账号 Cookie 三层验证")
    parser.add_argument("platform", help="平台: douyin, xiaohongshu")
    parser.add_argument("cookie_path", help="cookie.json 路径")
    args = parser.parse_args()

    result = verify(args.platform, args.cookie_path)

    # stdout 只输出最终 JSON，供 Rust 解析
    print(json.dumps(result, ensure_ascii=False, indent=2))
    sys.exit(0 if result["status"] == "valid" else 1)


if __name__ == "__main__":
    main()
