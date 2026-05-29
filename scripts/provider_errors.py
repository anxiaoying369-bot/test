"""统一的 Provider 错误分类与友好提示。

让所有 Provider（video/image/tts）使用同一套错误码，方便 Rust/前端做特殊提示。
所有错误最终以 ProviderError 抛出，包含 code + message + details。

错误码：
  - AUTH       API Key 无效 / 缺失（401, 403）
  - RATE_LIMIT 限流（429）
  - QUOTA      余额/配额不足（402, 业务码）
  - TIMEOUT    超时（连接超时、读超时）
  - NETWORK    网络错误（DNS 解析失败、连接拒绝）
  - SERVER     服务端 5xx
  - INVALID    参数错误（400）
  - UNKNOWN    其它
"""
import json
import socket
import urllib.error
from typing import Optional


class ProviderError(Exception):
    """统一的 Provider 错误类。"""

    def __init__(self, code: str, message: str, status: Optional[int] = None, details: str = ""):
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status
        self.details = details

    def to_dict(self) -> dict:
        return {
            "error_code": self.code,
            "error": self.message,
            "http_status": self.status,
            "details": self.details[:500] if self.details else "",
        }


def classify_http_status(status: int, body: str = "") -> tuple[str, str]:
    """把 HTTP 状态码转成 (error_code, 用户可读的提示)。"""
    if status == 401:
        return "AUTH", "API Key 无效或已过期，请到设置页检查配置"
    if status == 403:
        return "AUTH", "API Key 无权访问该接口（权限不足或服务未开通）"
    if status == 402:
        return "QUOTA", "账户余额不足或配额耗尽，请充值后重试"
    if status == 429:
        return "RATE_LIMIT", "请求被限流，稍后重试或降低并发"
    if status == 400:
        # 部分 body 暗示余额不足
        if any(k in body.lower() for k in ("insufficient", "余额", "quota", "balance")):
            return "QUOTA", "账户余额或配额不足"
        return "INVALID", "请求参数有误（检查模型 ID / Prompt 是否合法）"
    if status == 404:
        return "INVALID", "接口不存在或模型 ID 错误"
    if 500 <= status < 600:
        return "SERVER", f"服务端错误 {status}，稍后重试或换 Provider"
    return "UNKNOWN", f"HTTP {status} 未分类错误"


def classify_exception(exc: Exception) -> ProviderError:
    """把任意 Exception 转成 ProviderError。"""
    # urllib HTTPError
    if isinstance(exc, urllib.error.HTTPError):
        body = ""
        try:
            body = exc.read().decode("utf-8", errors="ignore")
        except Exception:
            pass
        code, msg = classify_http_status(exc.code, body)
        # 尝试从 JSON body 拿真正的 error message
        detail = body
        try:
            j = json.loads(body)
            real_msg = (j.get("error") or {}).get("message") if isinstance(j.get("error"), dict) else j.get("error") or j.get("message")
            if real_msg:
                detail = f"{real_msg}（{body[:200]}）"
        except Exception:
            pass
        return ProviderError(code, msg, status=exc.code, details=detail)

    # 超时
    if isinstance(exc, (socket.timeout, TimeoutError)):
        return ProviderError("TIMEOUT", "请求超时，请检查网络或稍后重试", details=str(exc))

    # URLError 包含 DNS / 连接错误
    if isinstance(exc, urllib.error.URLError):
        reason = str(exc.reason) if hasattr(exc, "reason") else str(exc)
        if "Name or service not known" in reason or "getaddrinfo" in reason.lower():
            return ProviderError("NETWORK", "DNS 解析失败，请检查 base_url 域名是否正确", details=reason)
        if "Connection refused" in reason or "refused" in reason.lower():
            return ProviderError("NETWORK", "连接被拒绝，请检查 base_url 或服务是否可访问", details=reason)
        return ProviderError("NETWORK", f"网络错误: {reason}", details=reason)

    # 已经是 ProviderError
    if isinstance(exc, ProviderError):
        return exc

    # 其它
    return ProviderError("UNKNOWN", str(exc), details=repr(exc))
