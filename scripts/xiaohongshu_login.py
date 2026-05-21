#!/usr/bin/env python3
"""
小红书扫码登录 - CloakBrowser 版
浏览器永不关闭，截图保存到文件供调试。
用法：
  python3 xiaohongshu_login.py --port 18001 --session-id <uuid>
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
    status: str = "pending"
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
        pass

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


# ============ 用户信息提取 ============
def get_user_info(pg):
    user_name = None
    user_id = None

    try:
        # 等待页面加载
        pg.wait_for_selector(".user-name, .user-nickname, .nickname, [class*='user-nickname']", timeout=5000)

        # 1. 尝试从 DOM 提取
        # 用户昵称
        name_selectors = [
            ".user-name", ".user-nickname", ".nickname", 
            "div.info-container .name", "div.user-info .name",
            "[class*='user-nickname']", "[class*='user_nickname']"
        ]
        for sel in name_selectors:
            try:
                el = pg.locator(sel).first
                if el.is_visible(timeout=1000):
                    user_name = el.text_content().strip()
                    if user_name: break
            except: pass

        # 小红书号
        id_selectors = [
            ".user-redId", ".redId", ".user-id",
            "div.info-container .info-item", "div.user-info .info-item",
            "[class*='user-redId']", "[class*='redId']"
        ]
        for sel in id_selectors:
            try:
                elements = pg.locator(sel).all()
                for el in elements:
                    text = el.text_content().strip()
                    if "小红书号" in text or "ID" in text or "redId" in text:
                        if "：" in text:
                            user_id = text.split("：")[-1].strip()
                        elif ":" in text:
                            user_id = text.split(":")[-1].strip()
                        else:
                            user_id = text.replace("小红书号", "").replace("ID", "").strip()
                        if user_id: break
                if user_id: break
            except: pass

        # 2. 备选：从 window 变量提取
        if not user_name or not user_id:
            try:
                raw_state = pg.evaluate("() => JSON.stringify(window.__INITIAL_STATE__ || {})")
                state_data = json.loads(raw_state)
                user_data = state_data.get("user", {}).get("userPageData", {})
                if not user_name: user_name = user_data.get("nickname")
                if not user_id: user_id = user_data.get("redId") or user_data.get("userId")
            except: pass

    except Exception as e:
        log(f"   提取用户信息异常: {e}")

    return user_name, user_id


# ============ 主登录流程 ============
def run_login():
    global browser, page, context_global, state
    state = LoginState()
    log("🚀 run_login 开始")
    try:
        log("调用 cloakbrowser.launch()...")
        browser = launch(headless=False)
        context_global = browser.new_context(viewport={"width": 1280, "height": 900})
        page = context_global.new_page()

        log("🌐 访问小红书官网...")
        page.goto("https://www.xiaohongshu.com", timeout=30000)
        sleep(3000)

        # 检查是否已登录
        if page.locator('.side-bar .user-avatar, .header-container .user-avatar').first.is_visible(timeout=3000):
            log("✅ 检测到已处于登录状态")
        else:
            log("📸 等待登录弹窗或点击登录...")
            login_selectors = ['text=登录', '.login-btn', 'button:has-text("登录")', '.header-container .login-button']
            for sel in login_selectors:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=2000):
                        el.click()
                        break
                except: pass

            log("📸 正在抓取二维码...")
            qr_b64 = None
            for _ in range(40):
                try:
                    el = page.locator('.login-container img, img.qrcode-img, .qr-canvas canvas').first
                    if el.is_visible(timeout=1000):
                        if el.evaluate("node => node.tagName") == "CANVAS":
                            qr_b64 = base64.b64encode(el.screenshot()).decode()
                        else:
                            src = el.get_attribute("src") or ""
                            if src.startswith("data:image"):
                                qr_b64 = src.split(",", 1)[1]
                            elif src.startswith("http"):
                                qr_b64 = base64.b64encode(el.screenshot()).decode()
                        if qr_b64: break
                except: pass
                sleep(500)

            if not qr_b64:
                try:
                    area = page.locator('.login-box, .login-container, .reds-modal-content').first
                    if area.is_visible(timeout=2000):
                        qr_b64 = base64.b64encode(area.screenshot()).decode()
                except: pass

            if not qr_b64:
                log("❌ 无法获取二维码")
                state.status = "error"
                state.error_msg = "无法获取登录二维码"
                return

            state.qrcode_base64 = qr_b64
            state.status = "qrcode"
            log("📸 二维码已就绪，等待扫码...")

            start_time = time.time()
            scanned = False
            while time.time() - start_time < 300:
                try:
                    # 只有当二维码消失，且明确检测到已登录特征时，才判定为扫码成功
                    # 1. 检查二维码是否还在
                    qr_still_there = page.locator('.login-container img, img.qrcode-img, .qr-canvas canvas').first.is_visible(timeout=500)
                    
                    if not qr_still_there:
                        # 2. 检查是否有登录后的特有元素 (例如左侧导航栏的发布按钮、通知图标等)
                        is_logged_in = (
                            "xiaohongshu.com/explore" in page.url or 
                            "xiaohongshu.com/notification" in page.url or
                            page.locator('.side-bar .user-avatar').first.is_visible(timeout=1000) or
                            page.locator('.header-container .user-avatar').first.is_visible(timeout=1000)
                        )
                        
                        if is_logged_in:
                            scanned = True
                            break
                except: pass
                sleep(1000)

            if not scanned:
                state.status = "error"
                state.error_msg = "扫码超时"
                return

            state.status = "scanned"
            log("✅ 扫码确认成功")

        sleep(3000)
        log("🔍 点击“我”进入个人主页提取信息...")
        try:
            # 尝试多种方式定位“我”按钮（因为 URL 包含动态 ID，直接跳转不可靠）
            me_selectors = [
                'a:has-text("我")', 
                'a[href*="/user/profile/"]',
                '.side-bar a:has(.reds-icon)',
                '//a[span[text()="我"]]',
                '.bottom-channel:has-text("我")'
            ]
            
            clicked_me = False
            for sel in me_selectors:
                try:
                    if sel.startswith('//'):
                        el = page.locator(f"xpath={sel}").first
                    else:
                        el = page.locator(sel).first
                        
                    if el.is_visible(timeout=2000):
                        el.click()
                        clicked_me = True
                        log(f"✅ 通过选择器 {sel} 点击成功")
                        break
                except: pass
            
            if not clicked_me:
                log("⚠️ 无法定位“我”按钮，尝试兜底跳转...")
                page.goto("https://www.xiaohongshu.com/user/profile", timeout=10000)
        except Exception as e:
            log(f"⚠️ 导航至个人主页失败: {e}")

        sleep(4000)
        user_name, red_id = get_user_info(page)

        if not user_name: user_name = "未知用户"
        if not red_id: red_id = "unknown"

        log(f"✅ 提取成功: {user_name} (ID: {red_id})")

        state.cookies = context_global.cookies()
        state.user_info = {"user_id": red_id, "name": user_name}
        state.user_name = user_name
        state.user_id = red_id
        state.status = "confirmed"

        # ============ 自动化存储 ============
        try:
            script_dir = os.path.dirname(os.path.abspath(__file__))
            save_dir = os.path.join(script_dir, "xiaohongshu")
            if not os.path.exists(save_dir):
                os.makedirs(save_dir)

            save_path = os.path.join(save_dir, "cookie.json")
            save_data = {
                "user_info": state.user_info,
                "cookies": state.cookies,
                "updated_at": time.strftime("%Y-%m-%d %H:%M:%S")
            }
            with open(save_path, "w", encoding="utf-8") as f:
                json.dump(save_data, f, ensure_ascii=False, indent=2)
            log(f"💾 数据已自动保存至: {save_path}")
        except Exception as se:
            log(f"⚠️ 自动存储失败: {se}")

        log("🎉 登录流程完全结束")

        while True: time.sleep(10)


    except Exception as e:
        log(f"❌ 运行异常: {e}")
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
    log(f"🎯 小红书登录服务器启动 (port={args.port}, session={args.session_id})")

    server_thread = threading.Thread(target=run_server, args=(args.port,), daemon=True)
    server_thread.start()
    log(f"🌐 HTTP 服务器运行在 http://127.0.0.1:{args.port}")

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

    run_login()


if __name__ == "__main__":
    main()
