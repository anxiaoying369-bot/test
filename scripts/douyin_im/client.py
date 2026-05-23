from __future__ import annotations

import json
from typing import Any

import requests
from google.protobuf.json_format import MessageToDict

from static import Response_pb2 as ResponseProto

from .auth import DouyinAuth
from .headers import HeaderBuilder, HeaderType
from .params import Params
from .proto import ProtoBuilder
from .utils import generate_a_bogus, generate_ms_token, splice_url

requests.packages.urllib3.disable_warnings()


class DouyinIMClient:
    """抖音私信发送相关 HTTP/protobuf API。"""

    douyin_url = "https://www.douyin.com"

    def __init__(self, auth: DouyinAuth, timeout: int = 30):
        self.auth = auth
        self.timeout = timeout

    def get_my_uid(self) -> int:
        if self.auth.uid is not None:
            return int(self.auth.uid)
        self.auth.require_cookie_keys("s_v_web_id")
        url = "https://www.douyin.com/aweme/v1/web/query/user/"
        headers = HeaderBuilder.build(HeaderType.GET)
        refer = "https://www.douyin.com/"
        headers.set_header("referer", refer)
        params = Params()
        params.with_platform()
        params.with_web_id(self.auth, refer)
        params.with_ms_token()
        params.add_param("verifyFp", self.auth.cookie["s_v_web_id"])
        params.add_param("fp", self.auth.cookie["s_v_web_id"])
        params.with_a_bogus()
        resp = requests.get(url, params=params.get(), verify=False, headers=headers.get(), cookies=self.auth.cookie, timeout=self.timeout)
        resp.raise_for_status()
        resp_json = json.loads(resp.text)
        self.auth.uid = int(resp_json["user_uid"])
        return self.auth.uid

    def get_device_id(self) -> str:
        self.auth.require_cookie_keys("s_v_web_id")
        url = "https://www.douyin.com/aweme/v1/web/query/user"
        headers = HeaderBuilder.build(HeaderType.GET)
        refer = "https://www.douyin.com/discover"
        headers.set_header("referer", refer)
        params = Params()
        params.with_platform()
        params.add_param("publish_video_strategy_type", "2")
        params.with_web_id(self.auth, refer)
        params.with_ms_token()
        params.add_param("verifyFp", self.auth.cookie["s_v_web_id"])
        params.add_param("fp", self.auth.cookie["s_v_web_id"])
        params.with_a_bogus()
        resp = requests.get(url, params=params.get(), verify=False, headers=headers.get(), cookies=self.auth.cookie, timeout=self.timeout)
        resp.raise_for_status()
        resp_json = json.loads(resp.text)
        return str(resp_json["id"])

    def create_conversation(self, to_user_id: int) -> tuple[str, int, str, dict[str, Any]]:
        """创建私信对话，返回 conversation_id, conversation_short_id, ticket, 原始响应 dict。"""
        self.auth.require_send_credentials()
        my_uid = self.get_my_uid()
        url = "https://imapi.douyin.com/v2/conversation/create"
        request_proto = ProtoBuilder.build_create_conversation_request(self.auth, int(to_user_id), my_uid)
        headers = HeaderBuilder.build(HeaderType.PROTOBUF)
        headers.set_header("referer", "https://www.douyin.com/")
        resp = requests.post(
            url,
            headers=headers.get(),
            cookies=self.auth.cookie,
            data=request_proto.SerializeToString(),
            verify=False,
            timeout=self.timeout,
        )
        resp.raise_for_status()
        response_proto = ResponseProto.Response()
        response_proto.ParseFromString(resp.content)
        resp_json = MessageToDict(response_proto, preserving_proto_field_name=True)
        conversation = resp_json["body"]["create_conversation_v2_body"]["conversation_info_list"][0]
        return (
            conversation["conversation_id"],
            int(conversation["conversation_short_id"]),
            conversation["ticket"],
            resp_json,
        )

    def send_msg(self, conversation_id, conversation_short_id, ticket, content: str) -> dict[str, Any]:
        """发送文本私信，返回响应 dict。"""
        self.auth.require_send_credentials()
        url = "https://imapi.douyin.com/v1/message/send"
        headers = HeaderBuilder.build(HeaderType.PROTOBUF)
        headers.set_header("referer", "https://www.douyin.com/")
        request_proto = ProtoBuilder.build_send_message_request(self.auth, conversation_id, conversation_short_id, ticket, content)
        params = {
            "verifyFp": self.auth.cookie["s_v_web_id"],
            "fp": self.auth.cookie["s_v_web_id"],
            "msToken": generate_ms_token(),
        }
        params["a_bogus"] = generate_a_bogus(splice_url(params))
        resp = requests.post(
            url,
            params=params,
            headers=headers.get(),
            verify=False,
            cookies=self.auth.cookie,
            data=request_proto.SerializeToString(),
            timeout=self.timeout,
        )
        resp.raise_for_status()
        response_proto = ResponseProto.Response()
        response_proto.ParseFromString(resp.content)
        return MessageToDict(response_proto, preserving_proto_field_name=True)

    def send_to_user(self, to_user_id: int, content: str) -> dict[str, Any]:
        """创建会话并发送文本私信。"""
        conversation_id, conversation_short_id, ticket, create_resp = self.create_conversation(to_user_id)
        send_resp = self.send_msg(conversation_id, conversation_short_id, ticket, content)
        return {
            "conversation_id": conversation_id,
            "conversation_short_id": conversation_short_id,
            "ticket": ticket,
            "create_response": create_resp,
            "send_response": send_resp,
        }
