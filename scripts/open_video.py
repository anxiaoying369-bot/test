#!/usr/bin/env python3
"""
使用采集时的 Cookie 打开抖音视频页面

用法:
  python3 open_video.py --cookie-path <cookie.json> --video-id <id>
"""

import argparse
import json
import os
import sys
import time
from DrissionPage import ChromiumPage, ChromiumOptions

CDP_PORT = 9222
CHROME_PATH = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
CHROME_USER_DATA_DIR = os.path.expanduser("~/chrome-debug-profile")

def is_port_in_use(port: int) -> bool:
    import socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", port)) == 0

def launch_chrome():
    import subprocess
    if not os.path.exists(CHROME_PATH):
        raise FileNotFoundError(f"Chrome not found at {CHROME_PATH}")
    os.makedirs(CHROME_USER_DATA_DIR, exist_ok=True)
    cmd = [
        CHROME_PATH,
        f"--remote-debugging-port={CDP_PORT}",
        f"--user-data-dir={CHROME_USER_DATA_DIR}",
    ]
    subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(30):
        time.sleep(0.5)
        if is_port_in_use(CDP_PORT):
            return True
    return False

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cookie-path", required=True)
    parser.add_argument("--video-id", required=True)
    args = parser.parse_args()

    if not os.path.exists(args.cookie_path):
        print(f"Error: Cookie file not found: {args.cookie_path}", file=sys.stderr)
        sys.exit(1)

    with open(args.cookie_path, 'r', encoding='utf-8') as f:
        cookie_data = json.load(f)
    
    cookies = cookie_data.get("cookies", [])

    if not is_port_in_use(CDP_PORT):
        if not launch_chrome():
            print("Error: Failed to launch Chrome", file=sys.stderr)
            sys.exit(1)

    co = ChromiumOptions()
    co.set_address(f"127.0.0.1:{CDP_PORT}")
    page = ChromiumPage(co)
    
    # 注入 Cookie
    # DrissionPage 的 set.cookies 接收列表
    # 我们需要确保格式正确
    formatted_cookies = []
    for c in cookies:
        formatted_cookies.append({
            'name': c.get('name'),
            'value': c.get('value'),
            'domain': c.get('domain'),
            'path': c.get('path', '/'),
            'secure': c.get('secure', False)
        })
    
    # 先访问一下域名以便设置 cookie
    page.get("https://www.douyin.com")
    page.set.cookies(formatted_cookies)
    
    # 跳转到视频页
    video_url = f"https://www.douyin.com/video/{args.video_id}"
    page.get(video_url)
    
    print(f"Opened {video_url}")

if __name__ == "__main__":
    main()
