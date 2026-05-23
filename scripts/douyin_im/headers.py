from __future__ import annotations

from enum import Enum

from .utils import DEFAULT_UA


class HeaderType(Enum):
    GET = "GET"
    POST = "POST"
    FORM = "FORM"
    PROTOBUF = "PROTOBUF"
    DOC = "DOC"


class Header:
    def __init__(self):
        self.headers: dict[str, str] = {}

    def set_header(self, key: str, value: str):
        self.headers[key] = value
        return self

    def set_referer(self, url: str):
        self.headers["referer"] = url
        return self

    def get(self) -> dict[str, str]:
        return self.headers


class HeaderBuilder:
    ua = DEFAULT_UA

    @staticmethod
    def build(header_type: HeaderType) -> Header:
        header = Header()
        header.set_header("user-agent", HeaderBuilder.ua)
        header.set_header("cache-control", "no-cache")
        header.set_header("pragma", "no-cache")
        header.set_header("sec-ch-ua", '"Microsoft Edge";v="125", "Chromium";v="125", "Not.A/Brand";v="24"')
        header.set_header("sec-ch-ua-mobile", "?0")
        header.set_header("sec-ch-ua-platform", '"Windows"')
        header.set_header("sec-fetch-dest", "empty")
        header.set_header("sec-fetch-mode", "cors")
        header.set_header("sec-fetch-site", "same-origin")
        header.set_header("priority", "u=1, i")
        header.set_header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6")
        if header_type == HeaderType.POST:
            header.set_header("accept", "*/*")
            header.set_header("content-type", "application/json; charset=UTF-8")
        elif header_type == HeaderType.FORM:
            header.set_header("accept", "application/json, text/plain, */*")
            header.set_header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
        elif header_type == HeaderType.PROTOBUF:
            header.set_header("accept", "application/x-protobuf")
            header.set_header("content-type", "application/x-protobuf")
        elif header_type == HeaderType.GET:
            header.set_header("accept", "application/json, text/plain, */*")
        elif header_type == HeaderType.DOC:
            header = Header()
            header.headers.update(
                {
                    "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
                    "accept-language": "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
                    "cache-control": "no-cache",
                    "cookie": "",
                    "pragma": "no-cache",
                    "priority": "u=0, i",
                    "sec-ch-ua": '"Microsoft Edge";v="125", "Chromium";v="125", "Not.A/Brand";v="24"',
                    "sec-ch-ua-mobile": "?0",
                    "sec-ch-ua-platform": '"Windows"',
                    "sec-fetch-dest": "document",
                    "sec-fetch-mode": "navigate",
                    "sec-fetch-site": "none",
                    "sec-fetch-user": "?1",
                    "upgrade-insecure-requests": "1",
                    "user-agent": HeaderBuilder.ua,
                }
            )
        return header
