#!/usr/bin/env python3
"""
抖音手动登录 - DrissionPage + Chrome CDP 接管模式

流程：
  1. 检测 9222 端口是否占用
     - 未占用 → 用 --remote-debugging-port=9222 启动 Chrome
     - 已占用 → 直接连接已有 Chrome
  2. 连接不上则报错退出
  3. 用户在浏览器里手动完成登录
  4. 用户在 Tauri UI 上点"登录完成"
  5. Tauri 调用 POST /finish?save_dir=... 触发本脚本抓 cookie 并保存
  6. 断开连接（不关闭用户的 Chrome），进程退出

用法：
  python3 douyin_login.py --port 18001 --session-id <uuid>

HTTP API：
  GET  /status                    -> {status, error}
  POST /finish?save_dir=<path>    -> {ok, user_info, message}
  DELETE /cancel                  -> 取消并退出
"""

import argparse
import json
import os
import queue
import re
import signal
import socket
import subprocess
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Optional, Any
from urllib.parse import urlparse, parse_qs

try:
    from DrissionPage import ChromiumPage, ChromiumOptions
except ImportError:
    print("ERROR: DrissionPage 未安装，请运行: pip install DrissionPage", flush=True)
    sys.exit(1)

# compat 与脚本同目录
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from compat import get_chrome_path, get_chrome_user_data_dir, safe_signal, safe_kill_self  # noqa: E402
from douyin_cookie_utils import (  # noqa: E402
    extract_user_info, cookies_to_header_string,
    get_cookies_from_page, get_storage_state, save_cookie_files,
)

# ============ 配置 ============

CDP_PORT = 9222
CHROME_PATH = get_chrome_path()

# 如果 compat 没搜到，尝试用 DrissionPage 自带的探测
if not CHROME_PATH or CHROME_PATH == "chrome":
    try:
        dp_path = ChromiumOptions().browser_path
        if dp_path and os.path.exists(dp_path):
            CHROME_PATH = dp_path
            print(f"[DY] Using Chrome found by DrissionPage: {CHROME_PATH}", flush=True)
    except Exception:
        pass

CHROME_USER_DATA_DIR = get_chrome_user_data_dir()


# ============ 全局状态 ============

class LoginState:
    status: str = "starting"   # starting | ready | saving | saved | failed | cancelled
    error_msg: Optional[str] = None
    user_info: Optional[dict] = None


state = LoginState()
_page_obj = None       # DrissionPage ChromiumPage
_chrome_process = None  # 我们自己启动的 Chrome 子进程（仅当端口空闲时）

# 主线程任务队列：HTTP handler → main thread
_task_queue: queue.Queue = queue.Queue()


# ============ 端口检测 ============

def is_port_in_use(port: int) -> bool:
    """检测端口是否被占用"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        result = s.connect_ex(("127.0.0.1", port))
        return result == 0


def launch_chrome() -> subprocess.Popen:
    """启动 Chrome 并开启远程调试端口，返回子进程对象"""
    print(f"[DY] Port {CDP_PORT} is free, launching Chrome...", flush=True)

    # 检查 Chrome 是否存在（支持绝对路径和 PATH 中的命令名）
    import shutil
    if os.path.isabs(CHROME_PATH):
        if not os.path.exists(CHROME_PATH):
            raise FileNotFoundError(f"Chrome not found at {CHROME_PATH}")
    else:
        if not shutil.which(CHROME_PATH):
            raise FileNotFoundError(f"Chrome command '{CHROME_PATH}' not found in PATH")

    os.makedirs(CHROME_USER_DATA_DIR, exist_ok=True)

    cmd = [
        CHROME_PATH,
        f"--remote-debugging-port={CDP_PORT}",
        f"--user-data-dir={CHROME_USER_DATA_DIR}",
    ]

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    print(f"[DY] Chrome launched (PID={proc.pid}), waiting for CDP port...", flush=True)

    # 等待 Chrome 启动并监听端口（最多 15 秒）
    for i in range(30):
        time.sleep(0.5)
        if is_port_in_use(CDP_PORT):
            print(f"[DY] Chrome CDP port {CDP_PORT} is ready", flush=True)
            return proc
        # 检查进程是否已退出
        if proc.poll() is not None:
            raise RuntimeError(f"Chrome exited unexpectedly (code={proc.returncode})")

    raise TimeoutError(f"Chrome started but CDP port {CDP_PORT} not ready within 15s")


def connect_to_chrome():
    """通过 DrissionPage 连接到 Chrome（CDP）"""
    global _page_obj

    print(f"[DY] Connecting to Chrome on port {CDP_PORT}...", flush=True)

    co = ChromiumOptions()
    co.set_address(f"127.0.0.1:{CDP_PORT}")

    try:
        browser = ChromiumPage(co)
    except Exception as e:
        _page_obj = None
        raise ConnectionError(
            f"无法连接到 127.0.0.1:{CDP_PORT} 上的 Chrome。"
            f"请确认 Chrome 是以 --remote-debugging-port={CDP_PORT} 启动的。"
            f"原始错误: {e}"
        )

    # 切到最新标签页
    try:
        latest_tab = browser.latest_tab
        _page_obj = latest_tab
        print(f"[DY] Got latest tab: {latest_tab.title}", flush=True)
    except Exception:
        _page_obj = browser
        print("[DY] Using browser object directly", flush=True)

    # 等页面就绪
    time.sleep(1)
    print("[DY] Connected to Chrome via DrissionPage", flush=True)




def _do_finish_in_main_thread(save_dir: str) -> dict:
    """实际的 DrissionPage 操作，由主循环从 _task_queue 取出任务后调用。"""
    global state

    if state.status not in ("ready", "saving"):
        return {"ok": False, "error": f"当前状态为 {state.status}，无法保存"}

    state.status = "saving"
    print(f"[DY] finish triggered, save_dir={save_dir}", flush=True)

    if not _page_obj:
        state.status = "failed"
        state.error_msg = "browser not running"
        return {"ok": False, "error": state.error_msg}

    try:
        cookies = get_cookies_from_page(_page_obj)
        cookie_names = {c.get("name", "") for c in cookies}
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

        storage_state = get_storage_state(_page_obj)

        # 再取一次 cookie（extract_user_info/get_storage_state 可能刷新了页面）
        cookies = get_cookies_from_page(_page_obj)

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
    """
    断开 DrissionPage 连接。不关闭用户自己的 Chrome。
    如果 Chrome 是我们启动的，终止子进程。
    """
    global _page_obj, _chrome_process

    try:
        if _page_obj:
            _page_obj.quit()
    except Exception:
        pass
    _page_obj = None

    if _chrome_process:
        try:
            _chrome_process.terminate()
            _chrome_process.wait(timeout=5)
        except Exception:
            try:
                _chrome_process.kill()
            except Exception:
                pass
        _chrome_process = None


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
            safe_kill_self()
            time.sleep(5)
            os._exit(0)


# ============ 主流程 ============

def open_browser_and_wait():
    global _page_obj, _chrome_process, state

    print(f"[DY] Checking CDP port {CDP_PORT}...", flush=True)
    try:
        if is_port_in_use(CDP_PORT):
            # 端口已被占用，直接连接
            connect_to_chrome()
        else:
            # 端口空闲，启动 Chrome
            try:
                _chrome_process = launch_chrome()
            except (FileNotFoundError, RuntimeError, TimeoutError) as e:
                state.status = "failed"
                state.error_msg = str(e)
                print(f"[DY] Chrome launch failed: {e}", flush=True)
                return

            # 连接到刚启动的 Chrome
            try:
                connect_to_chrome()
            except ConnectionError as e:
                state.status = "failed"
                state.error_msg = str(e)
                print(f"[DY] CDP connect failed: {e}", flush=True)
                if _chrome_process:
                    try:
                        _chrome_process.terminate()
                    except Exception:
                        pass
                    _chrome_process = None
                return

        # 跳转到抖音
        print("[DY] Navigating to douyin.com...", flush=True)
        try:
            _page_obj.get("https://www.douyin.com")
            time.sleep(2)
            current_url = _page_obj.url or ""
            print(f"[DY] Page loaded, current URL: {current_url}", flush=True)
        except Exception as e:
            print(f"[DY] goto douyin.com failed: {e}", flush=True)

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

    safe_signal(signal.SIGINT, on_signal)
    safe_signal(signal.SIGTERM, on_signal)

    threading.Thread(target=_parent_watchdog, args=(os.getppid(),), daemon=True).start()

    t = threading.Thread(target=run_server, args=(args.port,), daemon=True)
    t.start()
    time.sleep(0.3)

    open_browser_and_wait()

    # 主循环：从 _task_queue 取任务，在主线程执行
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
