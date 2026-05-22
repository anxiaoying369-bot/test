#!/usr/bin/env python3
"""
抖音扫码登录 - Playwright 直连版（新架构）
参考 SynapseAutomation syn_backend/app_new/platforms/douyin.py

核心行为变更（vs 旧架构）：
  - 登录成功后：设置 status=confirmed，返回 cookies + storage_state，然后退出
  - 不再：while True + time.sleep(10) 永久阻塞浏览器
  - 不再：自动保存到 scripts/douyin/storage_state.json

用法：
  python3 douyin_login.py --port 18001 --session-id <uuid>

HTTP API：
  GET /status   -> {status, qrcode_base64, user_name, user_id}
  GET /cookies -> {cookies, user_info, storage_state}
  DELETE /cancel
"""

import argparse
import base64
import json
import os
import random
import re
import signal
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Optional

try:
    from cloakbrowser import launch
except ImportError:
    print("ERROR: cloakbrowser 未安装")
    sys.exit(1)


# ============ 全局状态 ============
class LoginState:
    status: str = "pending"  # pending | qrcode | scanned | confirmed | failed | cancelled
    qrcode_base64: Optional[str] = None
    user_name: Optional[str] = None
    user_id: Optional[str] = None
    cookies: list = []
    storage_state: Optional[dict] = None
    user_info: Optional[dict] = None
    error_msg: Optional[str] = None


state = LoginState()
browser_obj = None
_context_obj = None


# ============ HTTP Server ============
class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # 静默

    def send_json(self, data: dict, status: int = 200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))

    def do_GET(self):
        if self.path == "/status":
            self.send_json({
                "status": state.status,
                "qrcode_base64": state.qrcode_base64,
                "user_name": state.user_name,
                "user_id": state.user_id,
                "error": state.error_msg,
            })
        elif self.path == "/cookies":
            if state.status != "confirmed":
                self.send_json({"error": "Login not confirmed"}, 400)
            else:
                self.send_json({
                    "cookies": state.cookies,
                    "user_info": state.user_info or {
                        "user_id": state.user_id or "",
                        "name": state.user_name or "",
                        "avatar": "",
                    },
                    "storage_state": state.storage_state,
                })
        else:
            self.send_json({"error": "Not found"}, 404)

    def do_DELETE(self):
        if self.path.startswith("/cancel"):
            state.status = "cancelled"
            try:
                if _context_obj:
                    _context_obj.close()
                if browser_obj:
                    browser_obj.close()
            except Exception:
                pass
            self.send_json({"status": "cancelled"})
        else:
            self.send_json({"error": "Not found"}, 404)


def run_server(port: int):
    srv = HTTPServer(("127.0.0.1", port), Handler)
    srv.serve_forever()


# ============ 辅助 ============
def sleep_ms(ms):
    time.sleep(ms / 1000.0)


# ============ 用户信息提取 ============
def extract_user_info(page):
    user_name, user_id, avatar = None, None, None
    try:
        # 1. JS 内存
        js_info = page.evaluate("""() => {
            if (window._ROUTER_DATA?.loaderData) {
                for (let key in window._ROUTER_DATA.loaderData) {
                    const data = window._ROUTER_DATA.loaderData[key];
                    if (data?.user) return data.user;
                }
            }
            if (window.userData) return window.userData;
            return null;
        }""")
        if js_info and isinstance(js_info, dict):
            user_id = js_info.get("uniqueId") or js_info.get("unique_id") or js_info.get("userId")
            user_name = js_info.get("nickname") or js_info.get("name")
            avatar = js_info.get("avatarUrl") or js_info.get("avatar_url")

        # 2. DOM 正则
        if not user_id:
            body_text = page.evaluate("() => document.body.innerText")
            m = re.search(r"(抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)", body_text)
            if m:
                user_id = m.group(2)

        # 3. 视觉 Selector
        if not user_name:
            for sel in ['[data-e2e="user-info-name"]', 'h1', '.header-right-name', "div[class*='name-_lSSDc']"]:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=1000):
                        user_name = el.text_content().strip().split("\n")[0]
                        if user_name:
                            break
                except Exception:
                    pass

        if not user_id:
            for sel in ['[data-e2e="user-info-id"]', "div[class*='unique_id-']"]:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=1000):
                        txt = el.text_content().strip()
                        user_id = txt.split("：")[-1].strip() if "：" in txt else txt
                        break
                except Exception:
                    pass

        # 4. 头像
        for sel in ["div[class*='avatar-'] img", ".semi-avatar img", "img[src*='aweme-avatar']"]:
            try:
                el = page.locator(sel).first
                if el.is_visible(timeout=1000):
                    avatar = el.get_attribute("src")
                    if avatar:
                        break
            except Exception:
                pass

    except Exception as e:
        print(f"[DY] extract_user_info error: {e}", flush=True)

    return user_name, user_id, avatar


# ============ 主登录流程 ============
def run_login():
    global browser_obj, _context_obj, state

    state = LoginState()
    print("[DY] Launching browser...", flush=True)

    try:
        browser_obj = launch(headless=False)
        ctx = browser_obj.new_context(viewport={"width": 1280, "height": 800})
        _context_obj = ctx
        page = ctx.new_page()

        # 访问创作者登录页
        try:
            page.goto("https://creator.douyin.com/creator-micro/login?enter_from=qr", timeout=20000)
        except Exception:
            try:
                page.goto("https://creator.douyin.com/", timeout=15000)
            except Exception:
                page.goto("https://www.douyin.com", timeout=15000)

        print("[DY] Page loaded, waiting for QR code...", flush=True)
        sleep_ms(2000)

        # 等待二维码
        deadline = time.time() + 60
        qr_found = False
        while time.time() < deadline:
            cookies_list = ctx.cookies()
            cookies_dict = {c["name"]: c["value"] for c in cookies_list}
            critical = ["sessionid", "sessionid_ss", "passport_auth_id", "sid_guard"]
            has_auth = any(cookies_dict.get(n) for n in critical)

            # 已登录（cookie 直接带过来了）
            if has_auth and "login" not in page.url.lower():
                print("[DY] Already logged in (cookies present)", flush=True)
                break

            # JS 提取二维码
            qr_data = page.evaluate("""() => {
                // 1. 创作者中心专用容器
                const q1 = document.querySelector('#animate_qrcode_container img.qrcode_img');
                if (q1 && q1.src && q1.src.length > 500)
                    return q1.src.startsWith('data:') ? q1.src : null;

                // 2. aria-label
                const q2 = document.querySelector('img[aria-label="二维码"]');
                if (q2 && q2.src && q2.src.startsWith('data:image') && q2.src.length > 500)
                    return q2.src;

                // 3. 视觉特征扫描（正方形 + 大尺寸）
                const imgs = Array.from(document.querySelectorAll('img'));
                for (const img of imgs) {
                    const src = img.src || "";
                    if (src.startsWith('data:image') && src.length > 1000) {
                        const r = img.getBoundingClientRect();
                        if (Math.abs(r.width - r.height) < 10 && r.width > 120)
                            return src;
                    }
                }
                return null;
            }""")

            if qr_data:
                if "," in qr_data:
                    state.qrcode_base64 = qr_data.split(",", 1)[1]
                else:
                    state.qrcode_base64 = qr_data
                state.status = "qrcode"
                print("[DY] QR code captured", flush=True)
                qr_found = True
                break

            # 兜底截图
            for sel in ['#animate_qrcode_container img', 'img[aria-label="二维码"]']:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=100):
                        src = el.get_attribute("src") or ""
                        if "data:image" in src and len(src) > 1000:
                            raw = src.split(",", 1)[1] if "," in src else src
                            state.qrcode_base64 = base64.b64encode(base64.b64decode(raw)).decode() if len(raw) > 100 else raw
                            # 直接用 screenshot 更可靠
                            png = el.screenshot()
                            state.qrcode_base64 = base64.b64encode(png).decode()
                            state.status = "qrcode"
                            qr_found = True
                            print(f"[DY] QR via screenshot: {sel}", flush=True)
                            break
                except Exception:
                    pass

            if qr_found:
                break
            sleep_ms(300)

        if not qr_found and state.status == "pending":
            state.status = "failed"
            state.error_msg = "二维码提取超时"
            print("[DY] QR timeout", flush=True)
            return

        # 等待扫码确认
        print("[DY] Waiting for scan...", flush=True)
        scan_deadline = time.time() + 300
        while time.time() < scan_deadline:
            if state.status == "cancelled":
                return
            cookies_list = ctx.cookies()
            ck = {c["name"]: c["value"] for c in cookies_list}
            if ck.get("sessionid") or ck.get("passport_auth_id"):
                state.status = "scanned"
                print("[DY] Scan confirmed", flush=True)
                break
            sleep_ms(1000)

        # 访问创作者上传页，触发 msToken 生成
        try:
            page.goto("https://creator.douyin.com/creator-micro/content/upload", timeout=20000)
            page.wait_for_load_state("networkidle", timeout=15000)
            sleep_ms(3000)
        except Exception as e:
            print(f"[DY] Upload page error (non-fatal): {e}", flush=True)

        # 访问个人页提取信息
        try:
            page.goto("https://www.douyin.com/user/self", timeout=15000)
            sleep_ms(2500)
        except Exception:
            pass

        user_name, user_id, avatar = extract_user_info(page)
        final_cookies = ctx.cookies()

        state.cookies = [{
            "name": c["name"],
            "value": c["value"],
            "domain": c["domain"],
            "path": c.get("path", "/"),
            "expires": c.get("expires"),
            "http_only": c.get("httpOnly"),
            "secure": c.get("secure"),
            "same_site": c.get("sameSite"),
        } for c in final_cookies]

        state.user_name = user_name
        state.user_id = user_id
        state.user_info = {
            "user_id": user_id or "",
            "name": user_name or "",
            "avatar": avatar or "",
        }

        # 获取 storage_state
        try:
            state.storage_state = ctx.storage_state()
        except Exception as e:
            print(f"[DY] storage_state error: {e}", flush=True)
            state.storage_state = None

        state.status = "confirmed"
        print(f"[DY] Login confirmed: user={user_name} uid={user_id}", flush=True)

        # ✅ 新架构：关闭浏览器（不再永久阻塞）
        try:
            ctx.close()
            browser_obj.close()
            print("[DY] Browser closed (new architecture)", flush=True)
        except Exception as e:
            print(f"[DY] Browser close error: {e}", flush=True)

        # 进程保持运行，Rust 会通过 /cookies 取数据，然后 cleanup_login_session 杀进程

    except Exception as e:
        state.status = "failed"
        state.error_msg = str(e)
        print(f"[DY] run_login exception: {e}", flush=True)
        import traceback
        traceback.print_exc()


# ============ 入口 ============
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--session-id", type=str, required=True)
    args = parser.parse_args()

    print(f"[DY] Starting on port {args.port}", flush=True)

    def on_signal(sig, frame):
        print(f"[DY] Signal {sig}", flush=True)
        try:
            if _context_obj:
                _context_obj.close()
            if browser_obj:
                browser_obj.close()
        except Exception:
            pass
        sys.exit(0)
    signal.signal(signal.SIGINT, on_signal)
    signal.signal(signal.SIGTERM, on_signal)

    # HTTP 服务器线程
    t = threading.Thread(target=run_server, args=(args.port,), daemon=True)
    t.start()
    sleep_ms(300)

    # 登录流程（阻塞直到 confirmed/failed）
    run_login()

    # 登录结束后保持运行（等待 Rust 调用 /cookies 或 cleanup）
    t.join()


if __name__ == "__main__":
    main()
