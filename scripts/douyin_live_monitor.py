#!/usr/bin/env python3
"""
抖音直播监控桥接脚本

用法:
  python3 douyin_live_monitor.py --account-name <name> --room-id <id>
"""

import argparse
import json
import os
import sys
import threading
import time
import tempfile
import ssl
import urllib3

# 禁用 InsecureRequestWarning (针对 verify=False)
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# 将 DouyinBarrage 及 compat 加入路径
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BARRAGE_DIR = os.path.join(SCRIPT_DIR, 'DouyinBarrage')
sys.path.insert(0, BARRAGE_DIR)
sys.path.insert(0, SCRIPT_DIR)

from compat import get_data_dir, safe_signal  # noqa: E402

# 禁用全局 SSL 验证（针对某些环境下的证书问题）
try:
    ssl._create_default_https_context = ssl._create_unverified_context
except:
    pass

from service.fetcher import DouyinBarrage
from base.output import DataRecorder

def safe_print(data):
    try:
        print(json.dumps(data, ensure_ascii=False))
        sys.stdout.flush()
    except BrokenPipeError:
        # 父进程已关闭管道，优雅退出
        sys.exit(0)
    except Exception:
        pass

class BridgeRecorder(DataRecorder):
    def __init__(self, live_id, config):
        super().__init__(live_id, config)
        self.live_id = live_id
        self.anchor_name = ""
        # 准备持久化目录
        self.data_dir = str(get_data_dir() / "live_data" / live_id)
        os.makedirs(self.data_dir, exist_ok=True)
        self.history_path = os.path.join(self.data_dir, "history.jsonl")

    def open(self, room_id):
        # 如果已经有主播名了，带上它
        safe_print({"type": "init", "room_id": room_id, "live_id": self.live_id, "anchor_name": self.anchor_name})

    def record(self, dtype, data):
        # 持久化到本地文件
        event = {
            "type": "data",
            "live_id": self.live_id,
            "anchor_name": self.anchor_name,
            "data_type": dtype,
            "payload": data,
            "timestamp": time.time()
        }
        
        # 写入历史记录
        try:
            with open(self.history_path, 'a', encoding='utf-8') as f:
                f.write(json.dumps(event, ensure_ascii=False) + "\n")
        except:
            pass

        # 输出到 stdout
        safe_print(event)

def load_cookie_string(account_name):
    # 复用 AutoCastAI 的 cookie 路径
    data_dir = str(get_data_dir() / "cookies" / "douyin")
    cookie_path = os.path.join(data_dir, account_name, "cookie.txt")
    if not os.path.exists(cookie_path):
        return None
    with open(cookie_path, 'r') as f:
        return f.read().strip()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--account-name", required=True)
    parser.add_argument("--room-id", required=True)
    args = parser.parse_args()

    cookie_str = load_cookie_string(args.account_name)
    if not cookie_str:
        safe_print({"type": "error", "message": f"账号 {args.account_name} 的 Cookie 不存在"})
        sys.exit(1)

    # 准备 cookie 文件供 DouyinBarrage 读取
    tmp_cookie_fd, tmp_cookie_path = tempfile.mkstemp(prefix=f"dy_cookie_{args.room_id}_", suffix=".txt")
    try:
        with os.fdopen(tmp_cookie_fd, 'w') as f:
            f.write(cookie_str)

        # 修改工作目录
        os.chdir(BARRAGE_DIR)
        
        # 预先获取房间信息以拿到主播名
        from service.network import enter_room_api, fetch_ttwid
        
        # 初始化实例
        instance = DouyinBarrage(args.room_id, log_level='ERROR')
        
        # 指向临时的 cookie 文件
        instance._cookie_file = tmp_cookie_path
        
        # 针对 requests 禁用验证
        instance.session.verify = False
        
        # 尝试拿到主播名
        anchor_name = ""
        try:
            ttwid = fetch_ttwid(instance._ua, instance.session)
            info = enter_room_api(ttwid, instance._ua, instance._ua_version, args.room_id, session=instance.session)
            anchor_name = info.get('anchor_name', '')
        except:
            pass

        # 注入我们的 BridgeRecorder
        recorder = BridgeRecorder(args.room_id, instance.config)
        recorder.anchor_name = anchor_name
        instance._recorder = recorder
        
        # 信号处理：确保优雅退出并断开 WebSocket
        import signal
        def handle_term(sig, frame):
            try:
                instance.stop()
            except:
                pass
            sys.exit(0)
        safe_signal(signal.SIGTERM, handle_term)

        # 定义回调来实时更新主播名
        def on_room_info(rid, name):
            if name:
                recorder.anchor_name = name
                safe_print({"type": "status", "status": "running", "live_id": rid, "anchor_name": name})
        
        instance._on_room_info = on_room_info
        
        # 启动
        safe_print({"type": "status", "status": "starting", "live_id": args.room_id, "anchor_name": anchor_name})
        
        instance.start()
    except Exception as e:
        safe_print({"type": "error", "message": str(e)})
    finally:
        if os.path.exists(tmp_cookie_path):
            try:
                os.remove(tmp_cookie_path)
            except:
                pass

if __name__ == "__main__":
    main()
