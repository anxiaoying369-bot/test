#!/usr/bin/env python3
"""
抖音扫码登录 - CloakBrowser 版
浏览器永不关闭，截图保存到文件供调试。
用法：
  python3 douyin_login.py --port 18001 --session-id <uuid>
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
import traceback
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Optional
from cloakbrowser import launch


# ============ 全局状态 ============
class LoginState:
    status: str = "pending"          # pending | qrcode | scanned | confirmed | error
    qrcode_base64: Optional[str] = None
    user_name: Optional[str] = None
    user_id: Optional[str] = None
    cookies: list = []
    user_info: Optional[dict] = None
    error_msg: Optional[str] = None


state = LoginState()
browser = None
page = None
context_global = None
_session_id = ""


# ============ 日志 ============
def log(msg):
    ts = time.strftime("%H:%M:%S")
    line = f"[{ts}] {msg}"
    print(line, flush=True)
    sid = _session_id[:8] if _session_id else "nofile"
    try:
        with open(f"/tmp/autocast_{sid}.log", "a") as f:
            f.write(line + "\n")
    except Exception:
        pass


# ============ HTTP Server ============
class LoginHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # 静默

    def do_GET(self):
        if self.path == "/status":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            resp = {
                "status": state.status,
                "qrcode_base64": state.qrcode_base64,
                "user_name": state.user_name,
                "user_id": state.user_id,
                "error": state.error_msg,
            }
            self.wfile.write(json.dumps(resp).encode())
        elif self.path == "/cookies":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            resp = {
                "cookies": state.cookies,
                "user_info": state.user_info,
            }
            self.wfile.write(json.dumps(resp).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path == "/close":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"ok": True}).encode())
            if browser:
                try:
                    browser.close()
                except Exception:
                    pass
            threading.Timer(1.0, lambda: sys.exit(0)).start()
        else:
            self.send_response(404)
            self.end_headers()


def run_server(port: int):
    server = HTTPServer(("127.0.0.1", port), LoginHandler)
    server.serve_forever()


# ============ 辅助函数 ============
def sleep(ms: int):
    time.sleep(ms / 1000.0)


def random_string(length: int = 16) -> str:
    chars = "abcdefghijklmnopqrstuvwxyz0123456789"
    return "".join(random.choice(chars) for _ in range(length))


# ============ 用户信息提取 ============
def extract_user_info(pg):
    user_name = None
    user_id = None
    avatar = None

    log("🔍 开始多重维度提取用户信息...")
    try:
        # 1. JS 内存数据提取 (最准确)
        js_info = pg.evaluate("""() => {
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
            if user_id:
                log(f"✅ 从 JS 内存成功提取: {user_id}")

        # 2. DOM 文本正则匹配 (备选)
        if not user_id:
            body_text = pg.evaluate("() => document.body.innerText")
            # 匹配 抖音号 / 抖音ID / 抖音id
            match = re.search(r"(抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)", body_text)
            if match:
                user_id = match.group(2)
                log(f"✅ 从 DOM 正则成功匹配 ID: {user_id}")

        # 3. 视觉 Selector 提取 (备选)
        if not user_name:
            name_sels = ['[data-e2e="user-info-name"]', 'h1', '.header-right-name', 'div[class*="name-_lSSDc"]']
            for sel in name_sels:
                try:
                    el = pg.locator(sel).first
                    if el.is_visible(timeout=1000):
                        user_name = el.text_content().strip().split("\n")[0]
                        break
                except: pass

        if not user_id:
            id_sels = ['[data-e2e="user-info-id"]', 'div[class*="unique_id-"]']
            for sel in id_sels:
                try:
                    el = pg.locator(sel).first
                    if el.is_visible(timeout=1000):
                        txt = el.text_content().strip()
                        if "：" in txt: user_id = txt.split("：")[-1].strip()
                        elif ":" in txt: user_id = txt.split(":")[-1].strip()
                        else: user_id = txt
                        break
                except: pass

        # 4. 头像提取
        if not avatar:
            avatar_sels = ["div[class*='avatar-'] img", ".semi-avatar img", "img[src*='aweme-avatar']", '[data-e2e="user-avatar"] img']
            for sel in avatar_sels:
                try:
                    el = pg.locator(sel).first
                    if el.is_visible(timeout=1000):
                        avatar = el.get_attribute("src")
                        break
                except: pass

    except Exception as e:
        log(f"⚠️ 提取过程异常: {e}")

    return user_name, user_id, avatar


# ============ 主登录流程 ============
def run_login():
    global browser, page, context_global, state

    state = LoginState()
    log("🚀 run_login 开始 (Synapse 优化版)")

    try:
        log("调用 cloakbrowser.launch()...")
        browser = launch(headless=False)
        # 设置较大的视口以确保 QR 渲染完整
        context_global = browser.new_context(viewport={"width": 1280, "height": 800})
        page = context_global.new_page()

        log("🌐 访问抖音创作者中心...")
        try:
            # 优先访问创作者中心登录页
            page.goto("https://creator.douyin.com/creator-micro/login?enter_from=qr", timeout=20000)
        except:
            page.goto("https://www.douyin.com", timeout=15000)

        start_time = time.time()
        qr_captured = False
        qr_detected_once = False

        log("⚡ 启动极速捕捉监控...")
        while time.time() - start_time < 60:
            current_url = page.url
            
            # 检查关键 Cookie 判断是否已登录
            cookies_list = context_global.cookies()
            cookies_dict = {c["name"]: c["value"] for c in cookies_list}
            critical_cookies = ["sessionid", "sessionid_ss", "passport_auth_id", "sid_guard"]
            has_auth = any(cookies_dict.get(name) for name in critical_cookies)

            if ("/user/self" in current_url or "creator.douyin.com" in current_url) and has_auth and "login" not in current_url.lower():
                log("✅ 检测到已登录状态 (Cookies & URL)")
                break

            # 首次发现二维码容器强制延迟
            if not qr_detected_once:
                exists = page.evaluate("""
                    () => {
                        return !!document.querySelector('img[aria-label="二维码"]') || 
                               !!document.getElementById('animate_qrcode_container') ||
                               !!document.querySelector('.login-mask-enter-done');
                    }
                """)
                if exists:
                    log("👀 发现二维码容器，等待 3.5 秒以确保内容完全加载...")
                    time.sleep(3.5)
                    qr_detected_once = True

            # 多重策略抓取二维码
            qr_data = page.evaluate("""
                () => {
                    // 1. 定向策略: 创作者中心专用容器
                    const synapseQR = document.querySelector('#animate_qrcode_container img.qrcode_img');
                    if (synapseQR && synapseQR.src && synapseQR.src.length > 500) return synapseQR.src.split(',')[1];

                    // 2. aria-label 策略
                    const labeledQR = document.querySelector('img[aria-label="二维码"]');
                    if (labeledQR && labeledQR.src && labeledQR.src.startsWith('data:image') && labeledQR.src.length > 500) {
                        return labeledQR.src.split(',')[1];
                    }

                    // 3. 视觉特征扫描
                    const imgs = Array.from(document.querySelectorAll('img'));
                    for (const img of imgs) {
                        const src = img.src || "";
                        if (src.startsWith('data:image') && src.length > 1000) {
                            const rect = img.getBoundingClientRect();
                            if (Math.abs(rect.width - rect.height) < 10 && rect.width > 120) {
                                return src.split(',')[1];
                            }
                        }
                    }
                    return null;
                }
            """)

            if qr_data:
                state.qrcode_base64 = qr_data
                state.status = "qrcode"
                log("🎯 二维码抓取成功")
                qr_captured = True
                break
            
            # 兜底截图
            selectors = ['#animate_qrcode_container img', 'img[aria-label="二维码"]', '.login-mask-enter-done img']
            for sel in selectors:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=50):
                        src = el.get_attribute("src") or ""
                        if "data:image" in src and len(src) > 1000:
                            state.qrcode_base64 = base64.b64encode(el.screenshot()).decode()
                            state.status = "qrcode"
                            log(f"🎯 通过局部截图成功: {sel}")
                            qr_captured = True
                            break
                except: pass

            if qr_captured: break
            time.sleep(0.3)

        # 等待扫码确认
        if qr_captured:
            log("🔄 二维码已反显，请在手机端确认登录...")
            start_scan = time.time()
            while time.time() - start_scan < 300:
                cookies = context_global.cookies()
                ck_dict = {c["name"]: c["value"] for c in cookies}
                if ck_dict.get("sessionid") or ck_dict.get("passport_auth_id"):
                    state.status = "scanned"
                    log("✅ 扫码确认成功")
                    break
                time.sleep(1)

        # 最终确认并提取信息
        log("🔄 正在同步用户信息...")

        # 关键：访问创作者上传页，触发服务器设置 msToken 和 sec_user_id
        try:
            log("🌐 访问创作者中心以触发 msToken 生成...")
            page.goto("https://creator.douyin.com/creator-micro/content/upload", timeout=20000)
            page.wait_for_load_state("networkidle", timeout=15000)
            time.sleep(3)
        except Exception as e:
            log(f"⚠️ 创作者页访问异常: {e}")

        # 再访问个人页确保数据完整
        try:
            page.goto("https://www.douyin.com/user/self", timeout=15000)
            time.sleep(2.5)
        except: pass

        user_name, user_id, avatar = extract_user_info(page)

        # 提取完整 Cookies（用于兼容旧格式）
        final_cookies = context_global.cookies()
        state.cookies = [{
            "name": c["name"], "value": c["value"],
            "domain": c["domain"], "path": c["path"],
            "expires": c.get("expires"), "http_only": c.get("httpOnly"),
            "secure": c.get("secure"), "same_site": c.get("sameSite")
        } for c in final_cookies]

        state.user_info = {"user_id": user_id, "name": user_name, "avatar": avatar}
        state.user_name = user_name
        state.user_id = user_id
        state.status = "confirmed"

        # ============ 自动化存储 ============
        try:
            script_dir = os.path.dirname(os.path.abspath(__file__))
            save_dir = os.path.join(script_dir, "douyin")
            if not os.path.exists(save_dir):
                os.makedirs(save_dir)

            # 保存完整 Playwright storage_state（包含 msToken 等关键 cookie）
            # 需要在新线程的事件循环中执行，因为 CloakBrowser 已有一个运行中的事件循环
            def _save_storage():
                import asyncio
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                try:
                    storage_state = loop.run_until_complete(context_global.storage_state())
                    with open(storage_state_path, "w", encoding="utf-8") as f:
                        json.dump(storage_state, f, ensure_ascii=False, indent=2)
                    log(f"💾 Playwright storage_state 已保存至: {storage_state_path}")
                finally:
                    loop.close()

            storage_state_path = os.path.join(save_dir, "storage_state.json")
            import threading
            save_thread = threading.Thread(target=_save_storage, daemon=True)
            save_thread.start()
            save_thread.join(timeout=10)

            # 同时保存兼容旧格式的 cookie.json
            cookie_path = os.path.join(save_dir, "cookie.json")
            save_data = {
                "user_info": state.user_info,
                "cookies": state.cookies,
                "storage_state": storage_state_path,
                "updated_at": time.strftime("%Y-%m-%d %H:%M:%S")
            }
            with open(cookie_path, "w", encoding="utf-8") as f:
                json.dump(save_data, f, ensure_ascii=False, indent=2)
            log(f"💾 兼容格式 cookie 已保存至: {cookie_path}")

            # 检查 msToken 和 sec_user_id 是否已包含
            cookie_dict = {c["name"]: c["value"] for c in final_cookies}
            ms_token = cookie_dict.get("msToken", "")
            sec_uid = cookie_dict.get("sec_user_id", "")
            log(f"🔍 msToken: {'✅ 已生成' if ms_token else '❌ 缺失'} (length={len(ms_token)})")
            log(f"🔍 sec_user_id: {'✅ 已生成' if sec_uid else '❌ 缺失'} (length={len(sec_uid)})")

        except Exception as se:
            log(f"⚠️ 自动存储失败: {se}")

        log(f"🎉 登录圆满完成: {user_name}")

        while True: time.sleep(10)

    except Exception as e:
        log(f"❌ 运行崩溃: {e}")
        traceback.print_exc()
        state.status = "error"
        state.error_msg = str(e)


# ============ 入口 ============
def main():
    global _session_id
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=18001)
    parser.add_argument("--session-id", type=str, default="default")
    args = parser.parse_args()

    _session_id = args.session_id
    log(f"🎯 抖音登录服务器启动 (port={args.port}, session={args.session_id})")

    # 启动 HTTP 服务器（后台线程）
    server_thread = threading.Thread(target=run_server, args=(args.port,), daemon=True)
    server_thread.start()
    log(f"🌐 HTTP 服务器运行在 http://127.0.0.1:{args.port}")

    # 处理 Ctrl+C
    def signal_handler(sig, frame):
        log("🛑 收到终止信号，正在强制退出...")
        try:
            if browser:
                # 尝试优雅关闭，但不阻塞退出
                threading.Thread(target=browser.close, daemon=True).start()
        except:
            pass
        os._exit(0)
    signal.signal(signal.SIGINT, signal_handler)

    # 运行登录流程
    run_login()


if __name__ == "__main__":
    main()
