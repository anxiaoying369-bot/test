#!/usr/bin/env python3
"""
小红书扫码登录 - Playwright Async 版
参考 SynapseAutomation syn_backend/app_new/platforms/xiaohongshu.py

架构：
  1. Playwright async 打开 creator.xiaohongshu.com/new/home
  2. 提取二维码 base64
  3. asyncio 轮询 cookies 状态（避免 greenlet 线程冲突）
  4. 登录成功后返回 storage_state
  5. 浏览器自动关闭（新架构行为）

用法：
  python3 xiaohongshu_login.py --port 18001 --session-id <uuid>

HTTP API：
  GET /status   -> {status, qrcode_base64, user_name, user_id}
  GET /cookies -> {cookies, user_info, storage_state}
  DELETE /cancel
"""

import argparse
import asyncio
import base64
import json
import os
import signal
import sys
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Optional

try:
    from playwright.async_api import async_playwright, Page, BrowserContext
except ImportError:
    print("ERROR: playwright 未安装或版本不对")
    sys.exit(1)


# ============ 全局状态 ============
class LoginState:
    status: str = "pending"  # pending | waiting | confirmed | failed | cancelled
    qrcode_base64: Optional[str] = None
    user_name: Optional[str] = None
    user_id: Optional[str] = None
    cookies: list = []
    storage_state: Optional[dict] = None
    user_info: Optional[dict] = None
    error_msg: Optional[str] = None
    avatar: Optional[str] = None


state = LoginState()
browser_obj = None  # playwright browser 对象
pw_context = None   # playwright async context


# ============ 小红书登录核心（async） ============

async def get_qrcode():
    """Playwright 获取二维码"""
    global state, browser_obj, pw_context

    async with async_playwright() as p:
        browser_obj = await p.chromium.launch(headless=False)
        pw_context = await browser_obj.new_context()
        page = await pw_context.new_page()

        try:
            # 访问创作者后台
            await page.goto("https://creator.xiaohongshu.com/new/home", timeout=30000)
            await asyncio.sleep(2)

            # 尝试点击"扫码登录" tab
            scan_selectors = [
                "text=扫码登录",
                ".login-box-container",
            ]
            for sel in scan_selectors:
                try:
                    el = page.locator(sel).first
                    if await el.is_visible(timeout=3000):
                        await el.click()
                        await asyncio.sleep(1)
                        break
                except Exception:
                    pass

            # 提取二维码
            qr_found = False
            qr_selectors = [
                "img.css-1lhmg90",
                "img[src*='data:image']",
                ".login-box-container img",
                "img[class*='qrcode']",
            ]

            for sel in qr_selectors:
                try:
                    img = page.locator(sel).first
                    await img.wait_for(timeout=10000)
                    src = await img.get_attribute("src")
                    if src and src.startswith("data:image"):
                        state.qrcode_base64 = src.split(",", 1)[1] if "," in src else src
                        qr_found = True
                        print(f"[XHS] QR found via selector: {sel}", flush=True)
                        break
                    elif src and src.startswith("http"):
                        pass
                except Exception:
                    pass

            # 兜底：截图
            if not qr_found:
                try:
                    box = await page.query_selector(".login-box-container") or await page.query_selector("body")
                    if box:
                        png = await box.screenshot(type="png")
                        state.qrcode_base64 = base64.b64encode(png).decode("utf-8")
                        print("[XHS] QR fallback: screenshot login box", flush=True)
                        qr_found = True
                except Exception as e:
                    print(f"[XHS] Screenshot fallback failed: {e}", flush=True)

            if not qr_found:
                state.status = "failed"
                state.error_msg = "无法提取二维码"
                return False

            state._page = page
            state._context = pw_context
            state.status = "waiting"
            print("[XHS] Waiting for scan...", flush=True)
            return True

        except Exception as e:
            state.status = "failed"
            state.error_msg = str(e)
            print(f"[XHS] get_qrcode failed: {e}", flush=True)
            return False


async def extract_user_info(page: Page, cookies: list) -> dict:
    """从页面 DOM + JS 提取用户信息"""
    try:
        await asyncio.sleep(1)

        import re
        user_id = ""

        # 方法1: DOM 文本
        selectors = [
            'div.personal .description-text',
            '.description-text div',
            '.description-text',
        ]
        patterns = [
            r"小红书账号[:：]?\s*([\w_]+)",
            r"小红书号[:：]?\s*([\w_]+)",
        ]

        for sel in selectors:
            try:
                elem = page.locator(sel).first
                await elem.wait_for(timeout=2000)
                text = (await elem.inner_text() or "").strip()
                for pat in patterns:
                    m = re.search(pat, text)
                    if m:
                        cand = m.group(1).strip()
                        if cand and 3 <= len(cand) <= 30:
                            user_id = cand
                            break
                if user_id:
                    break
            except Exception:
                pass

        # 方法2: JS 全局变量
        if not user_id:
            try:
                js_data = await page.evaluate("""() => {
                    if (window.__INITIAL_SSR_STATE__?.Main?.user)
                        return window.__INITIAL_SSR_STATE__.Main.user;
                    if (window.userInfo) return window.userInfo;
                    return null;
                }""")
                if js_data and isinstance(js_data, dict):
                    user_id = js_data.get("redId", js_data.get("red_id", js_data.get("userId", "")))
            except Exception:
                pass

        # 方法3: cookie 兜底
        if not user_id:
            for c in cookies:
                if c.get("name") == "x-user-id-creator.xiaohongshu.com":
                    user_id = str(c.get("value", ""))
                    break

        # 提取 name
        name = ""
        name_sels = ['.base .text .account-name', '.account-name', '.user-name']
        for sel in name_sels:
            try:
                h = page.locator(sel).first
                await h.wait_for(timeout=2000)
                name = (await h.inner_text() or "").strip().split("\n")[0]
                if name:
                    break
            except Exception:
                pass

        # 提取 avatar
        avatar = ""
        avatar_sels = ['.base .avatar img', '.avatar img', "img[class*='avatar']"]
        for sel in avatar_sels:
            try:
                h = page.locator(sel).first
                await h.wait_for(timeout=2000)
                avatar = await h.get_attribute("src") or ""
                if avatar:
                    break
            except Exception:
                pass

        return {
            "user_id": user_id or "",
            "name": name or user_id or "",
            "avatar": avatar or "",
        }
    except Exception as e:
        print(f"[XHS] extract_user_info failed: {e}", flush=True)
        return {"user_id": "", "name": "", "avatar": ""}


async def poll_loop(page: Page, context: BrowserContext):
    """asyncio 轮询：等待扫码完成，提取 cookies"""
    if state.status != "waiting":
        return

    try:
        deadline = time.time() + 300  # 5 分钟超时
        while time.time() < deadline:
            if state.status != "waiting":
                return

            await asyncio.sleep(2)

            try:
                cookies = await context.cookies()
                cookie_names = [c["name"] for c in cookies]
                current_url = page.url
                print(f"[XHS] poll cookies: {cookie_names[:10]}, url={current_url}", flush=True)

                # 关键 cookie：web_session（HttpOnly 主会话）或 xhsuid（小红书用户ID）
                # 注意：a1/webId 在未登录时就存在，不能作为登录依据
                has_auth = any(name in ["web_session", "xhsuid"] for name in cookie_names)

                if has_auth:
                    # 登录成功！提取用户信息
                    user_info = await extract_user_info(page, cookies)
                    state.user_id = user_info.get("user_id", "")
                    state.user_name = user_info.get("name", "")
                    state.avatar = user_info.get("avatar", "")
                    state.cookies = cookies
                    state.user_info = user_info

                    # 获取 storage_state
                    try:
                        state.storage_state = await context.storage_state()
                    except Exception as e:
                        print(f"[XHS] storage_state failed: {e}", flush=True)
                        state.storage_state = None

                    state.status = "confirmed"
                    print(f"[XHS] Login confirmed! user={state.user_name} uid={state.user_id}", flush=True)

                    # 关闭浏览器
                    try:
                        await context.close()
                        await browser_obj.close()
                        print("[XHS] Browser closed", flush=True)
                    except Exception as e:
                        print(f"[XHS] Browser close error: {e}", flush=True)

                    return

            except Exception as e:
                print(f"[XHS] Poll error: {e}", flush=True)

        # 超时
        state.status = "failed"
        state.error_msg = "扫码超时（5分钟）"

    except Exception as e:
        state.status = "failed"
        state.error_msg = str(e)
        print(f"[XHS] poll_loop failed: {e}", flush=True)


# ============ HTTP Server ============

class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        print(f"[HTTP] {args[0]}", flush=True)

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
                        "avatar": getattr(state, 'avatar', '') or "",
                    },
                    "storage_state": state.storage_state,
                })
        else:
            self.send_json({"error": "Not found"}, 404)

    def do_DELETE(self):
        if self.path.startswith("/cancel"):
            state.status = "cancelled"
            self.send_json({"status": "cancelled"})
        else:
            self.send_json({"error": "Not found"}, 404)


def run_server(port: int):
    srv = HTTPServer(("127.0.0.1", port), Handler)
    print(f"[XHS] HTTP server listening on port {port}", flush=True)
    srv.serve_forever()


# ============ 主入口 ============

async def main_async(port: int, session_id: str):
    # 获取二维码
    if not await get_qrcode():
        print(f"[XHS] Failed: {state.error_msg}", flush=True)
        return

    # 启动 asyncio 轮询任务
    page = getattr(state, "_page", None)
    context = getattr(state, "_context", None)
    if page and context:
        poll_task = asyncio.create_task(poll_loop(page, context))
        await poll_task
    else:
        print("[XHS] No page/context, exit", flush=True)
        return


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--session-id", type=str, required=True)
    args = parser.parse_args()

    port = args.port
    session_id = args.session_id

    # 信号处理
    def on_signal(sig, frame):
        print(f"[XHS] Signal {sig}, exit", flush=True)
        try:
            if getattr(state, "_context", None):
                asyncio.get_event_loop().run_until_complete(
                    state._context.close()
                )
            if browser_obj:
                asyncio.get_event_loop().run_until_complete(
                    browser_obj.close()
                )
        except Exception:
            pass
        sys.exit(0)
    signal.signal(signal.SIGINT, on_signal)
    signal.signal(signal.SIGTERM, on_signal)

    # 启动 HTTP 服务器
    t = __import__("threading").Thread(target=run_server, args=(port,), daemon=True)
    t.start()
    time.sleep(0.3)

    # 运行 async 主循环
    asyncio.run(main_async(port, session_id))


if __name__ == "__main__":
    main()
