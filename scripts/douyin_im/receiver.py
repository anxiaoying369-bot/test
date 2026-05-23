from __future__ import annotations

import json
import ssl
import sys
import base64
from typing import Callable

import websocket
from websocket import WebSocketApp

from static import Live_pb2, Response_pb2

from .auth import DouyinAuth
from .client import DouyinIMClient
from .headers import HeaderBuilder
from .params import Params
from .utils import build_access_key


class DouyinMessageReceiver:
    """通过 Douyin Frontier WebSocket 接收私信。"""

    app_key = "e1bd35ec9db7b8d846de66ed140b1ad9"
    fp_id = "9"

    def __init__(self, auth: DouyinAuth, auto_reconnect: bool = True, on_event: Callable[[dict], None] | None = None):
        auth.require_cookie_keys("sessionid")
        self.auto_reconnect = auto_reconnect
        self.auth = auth
        self.on_event = on_event or self.default_on_event
        self.ws = None
        device_id = DouyinIMClient(auth).get_device_id()
        access_key = build_access_key(self.fp_id, self.app_key, device_id)
        params = Params()
        (
            params.add_param("aid", "6383")
            .add_param("device_platform", "douyin_pc")
            .add_param("fpid", self.fp_id)
            .add_param("device_id", device_id)
            .add_param("token", self.auth.cookie["sessionid"])
            .add_param("access_key", access_key)
        )
        self.url = f"wss://frontier-im.douyin.com/ws/v2?{params.to_string()}"

    def on_open(self, ws):
        print("[douyin_im_ws] WebSocket connection open.", flush=True, file=sys.stderr)

    def on_message(self, ws, message):
        self.log_raw_message(message)
        event = self.parse_message(message)
        if event is not None:
            print(f"[douyin_im_ws] parsed_event={json.dumps(event, ensure_ascii=False, default=str)}", flush=True, file=sys.stderr)
            self.on_event(event)

    @staticmethod
    def log_raw_message(message) -> None:
        try:
            if isinstance(message, bytes):
                preview = base64.b64encode(message[:4096]).decode("ascii")
                print(
                    f"[douyin_im_ws] raw bytes len={len(message)} base64_prefix={preview}",
                    flush=True,
                    file=sys.stderr,
                )
            else:
                text = str(message)
                print(
                    f"[douyin_im_ws] raw text len={len(text)} body={text[:4096]}",
                    flush=True,
                    file=sys.stderr,
                )
        except Exception as e:
            print(f"[douyin_im_ws] raw log failed: {e}", flush=True, file=sys.stderr)

    def parse_message(self, message) -> dict | None:
        frame = Live_pb2.PushFrame()
        frame.ParseFromString(message)
        if frame.payloadType == "pb":
            response = Response_pb2.Response()
            response.ParseFromString(frame.payload)
            msg = response.body.new_message_notify.message
            if not msg:
                return None
            try:
                content = json.loads(msg.content) if msg.content else {}
            except Exception:
                content = {"raw": msg.content}
            event = {
                "payload_type": "pb",
                "sender": str(msg.sender),
                "conversation_id": msg.conversation_id,
                "index": msg.index_in_conversation,
                "message_type": msg.message_type,
                "content": content,
            }
            event["summary"] = self.format_event(event)
            return event
        if frame.payloadType == "text/json":
            try:
                data = json.loads(frame.payload)
            except Exception:
                data = {"raw": frame.payload.decode("utf-8", errors="replace") if isinstance(frame.payload, bytes) else frame.payload}
            return {"payload_type": "text/json", "content": data, "summary": str(data)}
        return {"payload_type": frame.payloadType, "summary": f"未处理 payloadType: {frame.payloadType}"}

    @staticmethod
    def format_event(event: dict) -> str:
        sender = event.get("sender")
        conversation_id = event.get("conversation_id")
        index = event.get("index")
        msg_type = event.get("message_type")
        content = event.get("content") or {}
        prefix = f"【消息编号:{index}】【聊天室ID:{conversation_id}】【来自:{sender}】"
        try:
            if msg_type == 7:
                return prefix + f"文本消息:{content.get('text', '')}"
            if msg_type == 5:
                return prefix + f"用户表情包消息:{content['url']['url_list'][0]}"
            if msg_type == 17:
                return prefix + f"语音信息:{content['resource_url']['url_list'][0]}"
            if msg_type == 27:
                return prefix + f"图片信息:{content['resource_url']['origin_url_list'][0]}"
            if msg_type == 8:
                return prefix + f"分享视频信息:视频ID{content.get('itemId')}"
            if msg_type == 50001:
                return f"对方已读，消息标号:{content.get('read_index')}"
        except Exception:
            pass
        return prefix + f"类型{msg_type}:{content}"

    @staticmethod
    def default_on_event(event: dict) -> None:
        print(event.get("summary") or event)

    def on_error(self, ws, error):
        print("[douyin_im_ws] ### error ###", flush=True, file=sys.stderr)
        print(error, flush=True, file=sys.stderr)
        print("[douyin_im_ws] ### ===error=== ###", flush=True, file=sys.stderr)
        if self.auto_reconnect and isinstance(
            error,
            (
                ConnectionRefusedError,
                BrokenPipeError,
                OSError,
                websocket._exceptions.WebSocketConnectionClosedException,
            ),
        ):
            self.start()

    def on_close(self, ws, close_status_code, close_msg):
        print("[douyin_im_ws] ### closed ###", flush=True, file=sys.stderr)
        print(f"[douyin_im_ws] status_code: {close_status_code}, msg: {close_msg}", flush=True, file=sys.stderr)
        print("[douyin_im_ws] ### ===closed=== ###", flush=True, file=sys.stderr)

    def start(self):
        self.ws = WebSocketApp(
            url=self.url,
            header={
                "Pragma": "no-cache",
                "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
                "User-Agent": HeaderBuilder.ua,
                "Cache-Control": "no-cache",
                "Sec-WebSocket-Protocol": "binary, base64, pbbp2",
                "Sec-WebSocket-Extensions": "permessage-deflate; client_max_window_bits",
            },
            cookie=self.auth.cookie_str,
            on_message=self.on_message,
            on_error=self.on_error,
            on_close=self.on_close,
            on_open=self.on_open,
        )
        try:
            self.ws.run_forever(
                origin="https://www.douyin.com",
                sslopt={"cert_reqs": ssl.CERT_NONE, "check_hostname": False},
            )
        except KeyboardInterrupt:
            self.ws.close()
        except Exception:
            self.ws.close()
            raise
