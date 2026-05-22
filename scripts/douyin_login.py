#!/usr/bin/env python3
"""
抖音手动登录 - Playwright 版（手动确认模式）

流程：
  1. 启动浏览器，跳转到抖音创作者中心登录页
  2. 用户在浏览器里手动完成登录
  3. 用户在 Tauri UI 上点"登录完成"
  4. Tauri 调用 POST /finish?save_dir=... 触发本脚本抓 cookie 并保存
  5. 关闭浏览器，进程退出

用法：
  python3 douyin_login.py --port 18001 --session-id <uuid>

HTTP API：
  GET  /status                    -> {status, error}
  POST /finish?save_dir=<path>    -> {status, user_info, message}
  DELETE /cancel                  -> 取消并退出
"""

import argparse
import json
import os
import queue
import re
import signal
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Optional
from urllib.parse import urlparse, parse_qs

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    print("ERROR: playwright 未安装", flush=True)
    sys.exit(1)


# ============ 全局状态 ============
class LoginState:
    status: str = "starting"   # starting | ready | saving | saved | failed | cancelled
    error_msg: Optional[str] = None
    user_info: Optional[dict] = None


state = LoginState()
browser_obj = None
_context_obj = None
_pw_obj = None
_page_obj = None

# 主线程任务队列：HTTP handler → main thread
# 每个任务是 (save_dir: str, result_queue: queue.Queue)
_task_queue: queue.Queue = queue.Queue()


# ============ 用户信息提取 ============
def extract_user_info(page):
    user_name, user_id, avatar = None, None, None
    try:
        try:
            page.goto("https://www.douyin.com/user/self", timeout=15000)
            page.wait_for_load_state("domcontentloaded", timeout=8000)
            time.sleep(2)
        except Exception as e:
            print(f"[DY] navigate user/self failed (non-fatal): {e}", flush=True)

        try:
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
        except Exception as e:
            print(f"[DY] js extract failed: {e}", flush=True)

        if not user_id:
            try:
                body_text = page.evaluate("() => document.body.innerText")
                m = re.search(r"(抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)", body_text)
                if m:
                    user_id = m.group(2)
            except Exception:
                pass

        if not user_name:
            for sel in ['[data-e2e="user-info-name"]', 'h1', '.header-right-name']:
                try:
                    el = page.locator(sel).first
                    if el.is_visible(timeout=1000):
                        user_name = (el.text_content() or "").strip().split("\n")[0]
                        if user_name:
                            break
                except Exception:
                    pass

        if not avatar:
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


# ============ Cookie 保存 ============
def cookies_to_header_string(cookies: list) -> str:
    parts = []
    for c in cookies:
        domain = (c.get("domain") or "").lstrip(".")
        if not domain.endswith("douyin.com"):
            continue
        name = c.get("name")
        value = c.get("value")
        if name and value is not None:
            parts.append(f"{name}={value}")
    return "; ".join(parts)


def save_cookie_files(save_dir: str, cookies: list, user_info: dict, storage_state: Optional[dict]) -> None:
    os.makedirs(save_dir, exist_ok=True)

    cookie_txt = cookies_to_header_string(cookies)
    with open(os.path.join(save_dir, "cookie.txt"), "w", encoding="utf-8") as f:
        f.write(cookie_txt)

    cookie_json = {"cookies": cookies, "storage_state": storage_state}
    with open(os.path.join(save_dir, "cookie.json"), "w", encoding="utf-8") as f:
        json.dump(cookie_json, f, ensure_ascii=False, indent=2)

    with open(os.path.join(save_dir, "meta.json"), "w", encoding="utf-8") as f:
        json.dump({
            "platform": "douyin",
            "user_info": user_info,
            "saved_at": int(time.time()),
        }, f, ensure_ascii=False, indent=2)


def _do_finish_in_main_thread(save_dir: str) -> dict:
    """
    实际的 Playwright 操作，必须在主线程（创建浏览器的线程）中调用。
    由主循环从 _task_queue 取出任务后调用。
    """
    global state

    if state.status not in ("ready", "saving"):
        return {"ok": False, "error": f"当前状态为 {state.status}，无法保存"}

    state.status = "saving"
    print(f"[DY] finish triggered in main thread, save_dir={save_dir}", flush=True)

    if not _page_obj or not _context_obj:
        state.status = "failed"
        state.error_msg = "browser not running"
        return {"ok": False, "error": state.error_msg}

    try:
        cookies = _context_obj.cookies()
        cookie_names = {c["name"] for c in cookies}
        login_markers = {"sessionid", "sessionid_ss", "sid_guard", "sid_tt", "passport_auth_id",
                         "ttwid", "s_v_web_id", "LOGIN_STATUS"}
        if not (cookie_names & login_markers):
            state.status = "ready"
            return {"ok": False, "error": "尚未检测到登录 Cookie，请先在浏览器里完成登录"}

        user_name, user_id, avatar = extract_user_info(_page_obj)
        user_info = {
            "user_id": user_id or "",
            "name": user_name or "",
            "avatar": avatar or "",
        }
        state.user_info = user_info

        storage_state = None
        try:
            storage_state = _context_obj.storage_state()
        except Exception as e:
            print(f"[DY] storage_state failed: {e}", flush=True)

        try:
            cookies = _context_obj.cookies()
        except Exception:
            pass

        save_cookie_files(save_dir, cookies, user_info, storage_state)

        state.status = "saved"
        print(f"[DY] Saved to {save_dir}, user={user_name} uid={user_id}", flush=True)
        return {"ok": True, "user_info": user_info, "save_dir": save_dir}

    except Exception as e:
        state.status = "failed"
        state.error_msg = str(e)
        print(f"[DY] finish error: {e}", flush=True)
        import traceback
        traceback.print_exc()
        return {"ok": False, "error": str(e)}


def shutdown_browser():
    global browser_obj, _context_obj, _pw_obj, _page_obj
    try:
        if _context_obj:
            _context_obj.close()
    except Exception:
        pass
    try:
        if browser_obj:
            browser_obj.close()
    except Exception:
        pass
    try:
        if _pw_obj:
            _pw_obj.stop()
    except Exception:
        pass
    browser_obj = None
    _context_obj = None
    _pw_obj = None
    _page_obj = None


# ============ HTTP Server ============
class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def send_json(self, data: dict, status: int = 200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))

    def do_GET(self):
        if self.path.startswith("/status"):
            self.send_json({
                "status": state.status,
                "error": state.error_msg,
                "user_info": state.user_info,
            })
        else:
            self.send_json({"error": "Not found"}, 404)

    def do_POST(self):
        parsed = urlparse(self.path)
        if parsed.path == "/finish":
            qs = parse_qs(parsed.query or "")
            save_dir = (qs.get("save_dir") or [""])[0]
            if not save_dir:
                self.send_json({"ok": False, "error": "missing save_dir"}, 400)
                return

            # 把任务投递给主线程，等待结果（Playwright 不能跨线程调用）
            result_q: queue.Queue = queue.Queue()
            _task_queue.put((save_dir, result_q))

            try:
                result = result_q.get(timeout=60)
            except queue.Empty:
                result = {"ok": False, "error": "主线程处理超时"}

            self.send_json(result, 200 if result.get("ok") else 400)

            if result.get("ok"):
                threading.Thread(target=shutdown_browser, daemon=True).start()
        else:
            self.send_json({"error": "Not found"}, 404)

    def do_DELETE(self):
        if self.path.startswith("/cancel"):
            state.status = "cancelled"
            shutdown_browser()
            self.send_json({"status": "cancelled"})
        else:
            self.send_json({"error": "Not found"}, 404)


def run_server(port: int):
    srv = HTTPServer(("127.0.0.1", port), Handler)
    srv.serve_forever()


# ============ 看门狗 ============
def _parent_watchdog(initial_ppid: int):
    while True:
        time.sleep(1)
        try:
            current_ppid = os.getppid()
        except Exception:
            current_ppid = 1
        if current_ppid != initial_ppid or current_ppid == 1:
            print(f"[DY] Parent gone (ppid {initial_ppid}->{current_ppid}), self-terminating", flush=True)
            try:
                os.kill(os.getpid(), signal.SIGTERM)
            except Exception:
                pass
            time.sleep(5)
            os._exit(0)


# ============ 主流程 ============
def open_browser_and_wait():
    global browser_obj, _context_obj, _pw_obj, _page_obj, state

    print("[DY] Launching browser...", flush=True)
    try:
        _pw_obj = sync_playwright().start()
        browser_obj = _pw_obj.chromium.launch(headless=False, channel="chrome")
        _context_obj = browser_obj.new_context(viewport={"width": 1280, "height": 800})
        _page_obj = _context_obj.new_page()

        for url in ["https://www.douyin.com"]:
            try:
                _page_obj.goto(url, timeout=20000)
                print(f"[DY] Page loaded: {url}", flush=True)
                break
            except Exception as e:
                print(f"[DY] goto {url} failed: {e}", flush=True)

        state.status = "ready"
        print("[DY] Ready. Waiting for user to login and click '登录完成'...", flush=True)

    except Exception as e:
        state.status = "failed"
        state.error_msg = str(e)
        print(f"[DY] open_browser failed: {e}", flush=True)
        import traceback
        traceback.print_exc()
        shutdown_browser()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--session-id", type=str, required=True)
    args = parser.parse_args()

    print(f"[DY] Starting on port {args.port}", flush=True)

    def on_signal(sig, frame):
        print(f"[DY] Signal {sig}", flush=True)
        state.status = "cancelled"
        shutdown_browser()
        sys.exit(0)

    signal.signal(signal.SIGINT, on_signal)
    signal.signal(signal.SIGTERM, on_signal)

    threading.Thread(target=_parent_watchdog, args=(os.getppid(),), daemon=True).start()

    t = threading.Thread(target=run_server, args=(args.port,), daemon=True)
    t.start()
    time.sleep(0.3)

    open_browser_and_wait()

    # 主循环：从 _task_queue 取任务，在主线程执行 Playwright 操作
    while state.status not in ("cancelled",):
        try:
            task = _task_queue.get(timeout=1)
        except queue.Empty:
            if state.status == "saved":
                time.sleep(3)
                break
            continue

        save_dir, result_q = task
        result = _do_finish_in_main_thread(save_dir)
        result_q.put(result)

        if result.get("ok"):
            time.sleep(3)
            break

    shutdown_browser()
    print("[DY] Exit", flush=True)


if __name__ == "__main__":
    main()
