#!/usr/bin/env python3
# pyright: reportAttributeAccessIssue=false
"""AutoCast AI 抖音私信命令桥接脚本。

stdout 只输出最终 JSON；监控模式下 stdout 输出 JSONL 事件，stderr 输出运行日志。
"""
from __future__ import annotations

import argparse
import contextlib
import datetime
import json
import os
import socket
import sys
import time
from pathlib import Path
from typing import Any

# 凭证有效期（小时）。超过此时间 cmd_check 会标记 credentials_expired=True。
CREDENTIALS_MAX_AGE_HOURS: float = 20.0

BASE_DIR = Path(__file__).resolve().parent
if str(BASE_DIR) not in sys.path:
    sys.path.insert(0, str(BASE_DIR))

from douyin_im import DouyinAuth, DouyinIMClient, DouyinMessageReceiver  # noqa: E402

from compat import get_chrome_path, get_chrome_user_data_dir  # noqa: E402

CDP_PORT = 9222
CHROME_PATH = get_chrome_path()
CHROME_USER_DATA_DIR = get_chrome_user_data_dir()


def log(msg: str) -> None:
    print(msg, flush=True, file=sys.stderr)


def ok(**kwargs: Any) -> dict[str, Any]:
    return {"ok": True, **kwargs}


def fail(message: str) -> dict[str, Any]:
    return {"ok": False, "error": message}


def read_text(path: str | None) -> str:
    if not path:
        return ""
    p = Path(path)
    if not p.exists():
        return ""
    return p.read_text(encoding="utf-8").strip()


def _extract_local_storage_value(cookie_path: str | None, storage_key: str) -> str:
    """从登录保存的 cookie.json/storage_state 读取 localStorage 原始值。

    对齐 /Users/make/project/DouYin_Spider/dy_apis/login_api.py：
    keys 来自 localStorage["security-sdk/s_sdk_crypt_sdk"]；
    web_protect 来自 localStorage["security-sdk/s_sdk_sign_data_key/web_protect"]。
    这里只读取账号保存阶段已经持久化的值，不在发送动作里打开或接管 Chrome。
    """
    path = cookie_json_path_from_cookie_txt(cookie_path)
    if not path:
        return ""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        log(f"[douyin_im_bridge] cookie.json 读取失败: {e}")
        return ""

    for origin in (data.get("storage_state") or {}).get("origins") or []:
        for item in origin.get("localStorage") or []:
            if item.get("name") == storage_key:
                return str(item.get("value") or "").strip()
    return ""


def _read_credentials_saved_at(cookie_path: str | None) -> datetime.datetime | None:
    """读取 cookie.json 中上次保存凭证的时间戳，返回 datetime（UTC）或 None。"""
    path = cookie_json_path_from_cookie_txt(cookie_path)
    if not path:
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        ts = data.get("credentials_saved_at")
        if ts:
            return datetime.datetime.fromisoformat(ts)
    except Exception:
        pass
    return None


def _write_credentials_saved_at(cookie_path: str | None) -> None:
    """在 cookie.json 中写入当前 UTC 时间作为凭证保存时间戳。"""
    path = cookie_json_path_from_cookie_txt(cookie_path)
    if not path:
        return
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        data["credentials_saved_at"] = datetime.datetime.utcnow().isoformat()
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception as e:
        log(f"[douyin_im_bridge] 写入 credentials_saved_at 失败: {e}")


def load_auth(args: argparse.Namespace, require_cookie: bool = True) -> DouyinAuth:
    cookie = (args.cookie or read_text(args.cookie_path) or os.getenv("DOUYIN_COOKIE", "")).strip()
    if require_cookie and not cookie:
        raise ValueError("缺少 Cookie：请选择已登录账号或提供 DOUYIN_COOKIE")

    web_protect = (
        args.web_protect
        or read_text(args.web_protect_path)
        or _extract_local_storage_value(args.cookie_path, "security-sdk/s_sdk_sign_data_key/web_protect")
        or os.getenv("DOUYIN_WEB_PROTECT", "")
    ).strip()
    keys = (
        args.keys
        or read_text(args.keys_path)
        or _extract_local_storage_value(args.cookie_path, "security-sdk/s_sdk_crypt_sdk")
        or os.getenv("DOUYIN_KEYS", "")
    ).strip()
    uid_raw = (args.uid or os.getenv("DOUYIN_UID", "")).strip()
    uid = int(uid_raw) if uid_raw else None
    return DouyinAuth.from_strings(cookie, web_protect=web_protect, keys=keys, uid=uid)


def cookie_json_path_from_cookie_txt(cookie_path: str | None) -> Path | None:
    if not cookie_path:
        return None
    p = Path(cookie_path)
    candidate = p.with_name("cookie.json") if p.name == "cookie.txt" else p
    return candidate if candidate.exists() else None


def is_port_in_use(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", port)) == 0


def load_cookie_json(cookie_path: str | None) -> dict[str, Any]:
    path = cookie_json_path_from_cookie_txt(cookie_path)
    if not path:
        return {"cookies": []}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        log(f"[douyin_im_bridge] cookie.json 读取失败: {e}")
        return {"cookies": []}


def inject_cookies_to_page(page: Any, cookie_data: dict[str, Any]) -> int:
    """通过 CDP Network.setCookies 批量注入 cookies。
    比 document.cookie 可靠：
      - 支持 httpOnly / SameSite=None 等 document.cookie 无法设置的 cookie
      - 不受当前页面 origin 限制（可跨域设置 .douyin.com 的 cookie）
      - 不依赖页面 JS 执行，注入更稳定
    """
    cookies = cookie_data.get("cookies") or []
    if not cookies:
        return 0

    # 构造 CDP Network.setCookies 期望的格式
    cdp_cookies = []
    for c in cookies:
        try:
            name = c.get("name")
            value = c.get("value")
            if not name or value is None:
                continue
            entry: dict[str, Any] = {
                "name": str(name),
                "value": str(value),
                "domain": c.get("domain") or ".douyin.com",
                "path": c.get("path") or "/",
                "secure": bool(c.get("secure", False)),
                "httpOnly": bool(c.get("httpOnly") or c.get("http_only") or False),
            }
            # SameSite: Playwright 用 "Strict/Lax/None"，CDP 用 "Strict"/"Lax"/"None"
            same_site = c.get("sameSite") or c.get("same_site")
            if same_site and isinstance(same_site, str):
                normalized = same_site.strip().capitalize()
                if normalized in ("Strict", "Lax", "None"):
                    entry["sameSite"] = normalized
            # 过期时间
            expires = c.get("expires") or c.get("expirationDate")
            if expires and isinstance(expires, (int, float)) and expires > 0:
                entry["expires"] = float(expires)
            cdp_cookies.append(entry)
        except Exception as e:
            log(f"[douyin_im_bridge] cookie 格式化失败 {c.get('name', '?')}: {e}")

    if not cdp_cookies:
        return 0

    # 批量注入 (一次 CDP 调用，原子性好)
    try:
        page.run_cdp("Network.setCookies", cookies=cdp_cookies)
        log(f"[douyin_im_bridge] Network.setCookies 批量注入 {len(cdp_cookies)} 条")
        return len(cdp_cookies)
    except Exception as bulk_err:
        log(f"[douyin_im_bridge] 批量注入失败 ({bulk_err})，逐条降级注入")

    # 降级：逐条注入
    injected = 0
    for entry in cdp_cookies:
        try:
            page.run_cdp("Network.setCookie", **entry)
            injected += 1
        except Exception as e:
            log(f"[douyin_im_bridge] 单条 cookie 注入失败 {entry.get('name')}: {e}")
    return injected


def _is_real_cdp_endpoint(port: int) -> bool:
    """端口被占用时，进一步确认这是一个真正的 Chrome CDP 调试服务（而非其他程序）。
    通过访问 http://127.0.0.1:<port>/json/version 看是否能拿到 Chrome 的 JSON 描述。
    """
    try:
        import urllib.request
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=2) as resp:
            if resp.status != 200:
                return False
            text = resp.read().decode("utf-8", errors="ignore")
            return "webSocketDebuggerUrl" in text or "Browser" in text
    except Exception:
        return False


def get_cdp_page(cookie_path: str | None = None):
    from DrissionPage import ChromiumOptions, ChromiumPage

    # 提前检查 Chrome 是否安装
    if CHROME_PATH and CHROME_PATH not in ("google-chrome", "chrome") and not os.path.exists(CHROME_PATH):
        raise RuntimeError(
            f"未找到 Google Chrome 浏览器 (探测路径: {CHROME_PATH})。\n"
            f"请到 https://www.google.com/chrome/ 下载安装后重启应用。"
        )

    co = ChromiumOptions()
    co.set_browser_path(CHROME_PATH)

    port_in_use = is_port_in_use(CDP_PORT)
    reuse_existing = port_in_use and _is_real_cdp_endpoint(CDP_PORT)

    # 是否强制 headless（默认关闭，因为抖音 security-sdk 会检测 headless 拒发 web_protect）
    headless_mode = os.environ.get("AUTOCAST_HEADLESS_CHROME", "0").lower() in ("1", "true", "yes")

    if reuse_existing:
        co.set_address(f"127.0.0.1:{CDP_PORT}")
        log(f"[douyin_im_bridge] 接管已有 Chrome CDP: {CDP_PORT}")
    else:
        if port_in_use:
            log(f"[douyin_im_bridge] 端口 {CDP_PORT} 被非 CDP 进程占用，无法启动 Chrome")
            raise RuntimeError(
                f"端口 {CDP_PORT} 已被其他进程占用，无法启动调试用的 Chrome。\n"
                f"请关闭占用该端口的程序后重试。"
            )
        if headless_mode:
            co.headless()
            log("[douyin_im_bridge] 启动 headless Chrome (AUTOCAST_HEADLESS_CHROME=1)")
        else:
            # 非 headless：抖音的 security-sdk 会检测 navigator.webdriver / 屏幕分辨率等
            # 反爬虫信号，headless 模式下不会下发 web_protect 签名密钥。
            log("[douyin_im_bridge] 启动可见 Chrome (security-sdk 需要真实浏览器特征)")
        co.set_argument("--no-sandbox")
        co.set_argument("--disable-blink-features=AutomationControlled")  # 隐藏 webdriver 标识
        co.set_argument(f"--user-data-dir={CHROME_USER_DATA_DIR}")
        co.set_argument(f"--remote-debugging-port={CDP_PORT}")
        co.set_argument("--window-size=1280,800")

    try:
        browser = ChromiumPage(co)
    except Exception as e:
        raise RuntimeError(
            f"启动 Chrome 失败: {e}\n"
            f"Chrome 路径: {CHROME_PATH}\n"
            f"用户数据目录: {CHROME_USER_DATA_DIR}"
        )

    try:
        page = browser.latest_tab
    except Exception:
        page = browser

    # ★ 关键：先打开一个空白页让 CDP attach，再注入 cookie，最后再导航到目标页
    #   原来的顺序是: 导航 → 注入 → 再导航，导致第一次导航没有携带 cookie，
    #   且 document.cookie 设置跨 domain 的 cookie 不可靠。
    try:
        page.get("about:blank")
        time.sleep(0.5)
    except Exception:
        pass

    cookie_data = load_cookie_json(cookie_path)
    total = len(cookie_data.get("cookies") or [])
    injected = inject_cookies_to_page(page, cookie_data)
    log(f"[douyin_im_bridge] Cookie 注入完成: {injected}/{total}")

    # 现在再正式访问 douyin
    try:
        page.get("https://www.douyin.com/")
        time.sleep(1.5)
    except Exception as e:
        log(f"[douyin_im_bridge] 首次导航 douyin.com 失败: {e}")

    return page


def normalize_text(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def normalize_user(raw: dict[str, Any] | None) -> dict[str, Any]:
    raw = raw or {}
    uid = raw.get("uid") or raw.get("id") or raw.get("user_id") or raw.get("userId") or raw.get("uid_str")
    return {
        "uid": normalize_text(uid),
        "sec_uid": normalize_text(raw.get("sec_uid") or raw.get("secUid")),
        "nickname": normalize_text(raw.get("nickname") or raw.get("nick_name") or raw.get("name")) or "未知用户",
        "avatar": raw.get("avatar_thumb", {}).get("url_list", [None])[0]
        if isinstance(raw.get("avatar_thumb"), dict)
        else raw.get("avatar") or raw.get("avatar_url"),
        "raw": raw,
    }


def normalize_message(raw: dict[str, Any], my_uid: str = "") -> dict[str, Any]:
    content_raw = raw.get("content") or raw.get("text") or ""
    content = content_raw
    if isinstance(content_raw, str):
        try:
            parsed = json.loads(content_raw)
            if isinstance(parsed, dict):
                content = parsed.get("text") or parsed.get("content") or content_raw
        except Exception:
            pass
    sender = normalize_text(raw.get("sender") or raw.get("sender_uid") or raw.get("from_uid") or raw.get("from_user_id") or raw.get("uid"))
    return {
        "id": normalize_text(raw.get("server_message_id") or raw.get("message_id") or raw.get("msg_id") or raw.get("id") or raw.get("client_message_id")),
        "conversation_id": normalize_text(raw.get("conversation_id")),
        "conversation_short_id": raw.get("conversation_short_id"),
        "sender": sender,
        "is_self": bool(my_uid and sender == my_uid),
        "message_type": raw.get("message_type") or raw.get("type"),
        "text": normalize_text(content),
        "create_time": raw.get("create_time") or raw.get("created_at") or raw.get("timestamp") or raw.get("server_time"),
        "raw": raw,
    }


def normalize_conversation(raw: dict[str, Any], my_uid: str = "") -> dict[str, Any]:
    conversation_id = normalize_text(raw.get("conversation_id") or raw.get("conversationId") or raw.get("id"))
    short_id = raw.get("conversation_short_id") or raw.get("conversationShortId") or raw.get("short_id")
    ticket = normalize_text(raw.get("ticket"))
    participants = raw.get("participants") or raw.get("users") or raw.get("members") or []
    if isinstance(participants, dict):
        participants = list(participants.values())
    users = [normalize_user(u) for u in participants if isinstance(u, dict)]
    peer = next((u for u in users if u.get("uid") and u.get("uid") != my_uid), users[0] if users else {})
    last_raw = raw.get("last_message") or raw.get("lastMessage") or raw.get("last_msg") or {}
    last_message = normalize_message(last_raw, my_uid) if isinstance(last_raw, dict) else {"text": normalize_text(last_raw)}
    return {
        "conversation_id": conversation_id,
        "conversation_short_id": short_id,
        "ticket": ticket,
        "peer_uid": peer.get("uid") or "",
        "peer_sec_uid": peer.get("sec_uid") or "",
        "peer_nickname": peer.get("nickname") or conversation_id or "未知联系人",
        "peer_avatar": peer.get("avatar"),
        "last_message": last_message,
        "unread_count": raw.get("unread_count") or raw.get("unread") or 0,
        "updated_at": raw.get("updated_at") or raw.get("update_time") or raw.get("sort_order") or raw.get("last_message_create_time"),
        "raw": raw,
    }


def flatten_candidates(obj: Any, key_hints: tuple[str, ...]) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []

    def walk(value: Any, parent_key: str = "") -> None:
        if isinstance(value, dict):
            for k, v in value.items():
                lk = str(k).lower()
                if isinstance(v, list) and any(h in lk for h in key_hints):
                    found.extend([x for x in v if isinstance(x, dict)])
                walk(v, lk)
        elif isinstance(value, list):
            for item in value:
                walk(item, parent_key)

    walk(obj)
    return found


def page_eval_json(page: Any, js: str) -> Any:
    result = page.run_js(js)
    if isinstance(result, str):
        try:
            return json.loads(result)
        except Exception:
            return result
    return result


def cmd_check(args: argparse.Namespace) -> dict[str, Any]:
    auth = load_auth(args)
    cookie_keys = sorted(auth.cookie.keys())
    missing_for_recv = [k for k in ["sessionid", "s_v_web_id"] if not auth.cookie.get(k)]
    missing_for_send = []
    if not auth.private_key:
        missing_for_send.append("keys/ec_privateKey")
    if not auth.ticket:
        missing_for_send.append("web_protect/ticket")
    if not auth.ts_sign:
        missing_for_send.append("web_protect/ts_sign")
    if not auth.client_cert:
        missing_for_send.append("web_protect/client_cert")
    if not auth.cookie.get("s_v_web_id"):
        missing_for_send.append("cookie s_v_web_id")

    # 检查凭证保存时间——超过有效期视为已过期
    saved_at = _read_credentials_saved_at(args.cookie_path)
    credentials_age_hours: float | None = None
    credentials_expired = False
    if saved_at is not None:
        age = datetime.datetime.utcnow() - saved_at
        credentials_age_hours = round(age.total_seconds() / 3600, 1)
        if credentials_age_hours > CREDENTIALS_MAX_AGE_HOURS:
            credentials_expired = True
    elif not missing_for_send:
        # 凭证存在但没有保存时间戳（旧版本），视为需要检测
        credentials_expired = False

    return ok(
        cookie_keys=cookie_keys,
        receive_ready=not missing_for_recv,
        send_ready=not missing_for_send and not credentials_expired,
        missing_for_recv=missing_for_recv,
        missing_for_send=missing_for_send,
        credentials_expired=credentials_expired,
        credentials_age_hours=credentials_age_hours,
        uid=auth.uid,
    )


def cmd_my_uid(args: argparse.Namespace) -> dict[str, Any]:
    auth = load_auth(args)
    client = DouyinIMClient(auth, timeout=args.timeout)
    return ok(uid=client.get_my_uid())


def cmd_user_info(args: argparse.Namespace) -> dict[str, Any]:
    """根据数字 uid 获取用户信息（昵称、头像、sec_uid 等）。"""
    if not args.user_id:
        return fail("缺少 --user-id 参数")
    auth = load_auth(args)
    client = DouyinIMClient(auth, timeout=args.timeout)
    try:
        info = client.get_user_info(args.user_id)
        return ok(**info)
    except Exception as e:
        return fail(f"获取用户信息失败: {e}")


def cmd_contacts(args: argparse.Namespace) -> dict[str, Any]:
    """仅通过 HTTP 获取当前账号 UID，不再使用 CDP 读取联系人列表。"""
    auth = load_auth(args)
    my_uid = str(auth.uid or "")
    if not my_uid:
        try:
            my_uid = str(DouyinIMClient(auth, timeout=args.timeout).get_my_uid())
        except Exception as e:
            log(f"[douyin_im_bridge] HTTP 获取 UID 失败: {e}")
    return ok(uid=my_uid, contacts=[])


def cmd_messages(args: argparse.Namespace) -> dict[str, Any]:
    """历史消息不通过 CDP 读取，返回空列表。实时消息由 WebSocket 监控接收。"""
    auth = load_auth(args)
    my_uid = str(auth.uid or "")
    if not my_uid:
        try:
            my_uid = str(DouyinIMClient(auth, timeout=args.timeout).get_my_uid())
        except Exception:
            pass
    return ok(uid=my_uid, messages=[])


def cmd_create_conversation(args: argparse.Namespace) -> dict[str, Any]:
    auth = load_auth(args)
    client = DouyinIMClient(auth, timeout=args.timeout)
    conversation_id, conversation_short_id, ticket, response = client.create_conversation(int(args.to_user_id))
    return ok(
        conversation_id=conversation_id,
        conversation_short_id=conversation_short_id,
        ticket=ticket,
        response=response,
    )


def cmd_send(args: argparse.Namespace) -> dict[str, Any]:
    auth = load_auth(args)
    client = DouyinIMClient(auth, timeout=args.timeout)
    try:
        if args.to_user_id:
            result = client.send_to_user(int(args.to_user_id), args.content)
        else:
            missing = [name for name in ["conversation_id", "conversation_short_id", "ticket"] if getattr(args, name) in (None, "")]
            if missing:
                raise ValueError("缺少参数: " + ", ".join("--" + x.replace("_", "-") for x in missing))
            result = client.send_msg(args.conversation_id, args.conversation_short_id, args.ticket, args.content)
        return ok(result=result)
    except ValueError as e:
        # require_send_credentials() 抛出的 ValueError — 凭证字段缺失
        msg = str(e)
        if "缺少认证字段" in msg or "缺少参数" in msg:
            return fail(msg) | {"needs_refresh": True}
        raise
    except Exception as e:
        # HTTP 请求失败：401/403 通常意味着凭证过期
        msg = str(e)
        is_auth_error = any(code in msg for code in ("401", "403", "Unauthorized", "Forbidden"))
        if is_auth_error:
            return fail(f"发送失败（鉴权过期，请刷新凭证）: {msg}") | {"needs_refresh": True}
        raise


def cmd_refresh_credentials(args: argparse.Namespace) -> dict[str, Any]:
    """通过 Chrome CDP 重新读取 localStorage 并更新 cookie.json 中的认证字段。

    适用场景：账号登录后 cookie.json 里 storage_state 为空（未捕获到 security-sdk 数据），
    导致发送私信时报「缺少认证字段」。

    抖音 security-sdk 生成 web_protect 的时机：
      - s_sdk_crypt_sdk：只要 SDK 加载就生成（快，访问 /chat 页就有）
      - s_sdk_sign_data_key/web_protect：SDK 完成挑战 + 调用过签名接口才生成
        通常需要：① 加载 chat 模块 JS ② 与会话列表/消息接口交互
        我们通过：① 访问 /chat ② 触发列表加载 ③ 轮询 localStorage 等待密钥落地
    """
    target_keys = [
        "security-sdk/s_sdk_crypt_sdk",
        "security-sdk/s_sdk_sign_data_key/web_protect",
    ]

    # JS：读取所有 localStorage
    read_ls_js = """
    async function() {
        const items = [];
        try {
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                items.push({name: key, value: localStorage.getItem(key)});
            }
        } catch(e) {}
        return JSON.stringify({url: location.href, origin: location.origin, items: items});
    }
    """

    # JS：强行调用多个需要 webProtect 签名的 IM 接口，迫使 security-sdk 生成密钥
    # 不依赖任何 DOM selector（抖音常改类名），纯网络层触发。
    trigger_chat_js = """
    async function() {
        const results = [];
        // 一组需要 webProtect 签名才能成功的 IM 接口（任一返回 200 都说明 SDK 已签名）
        const endpoints = [
            '/aweme/v1/web/im/users/?aid=6383&device_platform=webapp',
            '/aweme/v1/web/im/conversation/?aid=6383&device_platform=webapp',
            '/aweme/v1/web/im/conversations/?aid=6383&device_platform=webapp&offset=0&limit=20',
            '/aweme/v1/web/im/conv/?aid=6383&device_platform=webapp',
            '/aweme/v1/web/im/cookie/check/?aid=6383&device_platform=webapp',
        ];
        for (const ep of endpoints) {
            try {
                const resp = await fetch(ep, {
                    credentials: 'include',
                    headers: {'Accept': 'application/json'},
                });
                results.push(`${ep.split('?')[0]}: ${resp.status}`);
            } catch(e) {
                results.push(`${ep.split('?')[0]}: ERR ${e.message}`);
            }
        }
        // 模拟一次会话列表滚动事件（有些版本会绑事件触发签名）
        try {
            window.dispatchEvent(new Event('scroll'));
            window.dispatchEvent(new Event('focus'));
        } catch(e) {}
        return JSON.stringify(results);
    }
    """

    page = get_cdp_page(args.cookie_path)
    found_items: dict[str, str] = {}

    def collect_localstorage(label: str) -> None:
        try:
            raw = page.run_js(read_ls_js)
            if raw is None:
                log(f"[douyin_im_bridge] refresh_credentials: {label} run_js 返回 None")
                return
            data = json.loads(raw) if isinstance(raw, str) else raw
            if not isinstance(data, dict):
                return
            for item in data.get("items") or []:
                name = item.get("name") if isinstance(item, dict) else None
                value = item.get("value") if isinstance(item, dict) else None
                if name and value:  # 用 value 覆盖（找到非空值时刷新）
                    found_items[name] = str(value)
        except Exception as e:
            log(f"[douyin_im_bridge] refresh_credentials: {label} 读取 localStorage 失败: {e}")

    try:
        # 第一步：先访问主页让 SDK 加载
        try:
            page.get("https://www.douyin.com/")
            time.sleep(3)
            collect_localstorage("home")
        except Exception as e:
            log(f"[douyin_im_bridge] refresh_credentials: home 导航失败: {e}")

        # 第二步：访问 chat 页
        try:
            page.get("https://www.douyin.com/chat")
            time.sleep(4)
            collect_localstorage("chat_initial")
        except Exception as e:
            log(f"[douyin_im_bridge] refresh_credentials: chat 导航失败: {e}")

        # 第三步：如果 web_protect 还没出现，重复"触发签名接口 + 轮询"策略，最多 3 轮
        web_protect_key = "security-sdk/s_sdk_sign_data_key/web_protect"
        for round_idx in range(1, 4):
            if found_items.get(web_protect_key):
                break

            # 触发：调一批需要签名的 IM 接口
            try:
                trigger_result = page.run_js(trigger_chat_js)
                log(f"[douyin_im_bridge] refresh_credentials: round {round_idx} 触发签名接口 → {trigger_result}")
            except Exception as e:
                log(f"[douyin_im_bridge] refresh_credentials: round {round_idx} 触发失败: {e}")

            # 轮询：等 web_protect 出现，每轮最多 10 秒
            deadline = time.time() + 10
            while time.time() < deadline:
                time.sleep(1)
                collect_localstorage(f"round_{round_idx}_poll")
                if found_items.get(web_protect_key):
                    log(f"[douyin_im_bridge] refresh_credentials: ✓ round {round_idx} 拿到 web_protect")
                    break

        # 最终再读一遍
        collect_localstorage("final")
        log(f"[douyin_im_bridge] refresh_credentials: 最终命中 {[k for k in target_keys if k in found_items]}")

    finally:
        try:
            page.quit()
        except Exception:
            pass

    # 把找到的内容写回 cookie.json
    cookie_json_path = cookie_json_path_from_cookie_txt(args.cookie_path)
    if not cookie_json_path:
        return fail("cookie.json 路径不存在，无法更新认证凭证")

    try:
        cookie_data: dict[str, Any] = json.loads(cookie_json_path.read_text(encoding="utf-8"))
    except Exception as e:
        return fail(f"读取 cookie.json 失败: {e}")

    # 更新 storage_state.origins
    storage_state = cookie_data.get("storage_state") or {}
    origins: list[dict[str, Any]] = (storage_state.get("origins") or []) if isinstance(storage_state, dict) else []

    origin_key = "https://www.douyin.com"
    origin_entry = next((o for o in origins if o.get("origin") == origin_key), None)
    if origin_entry is None:
        origin_entry = {"origin": origin_key, "localStorage": []}
        origins.append(origin_entry)

    ls = origin_entry.get("localStorage") or []
    existing_names = {item.get("name") for item in ls}
    for name, value in found_items.items():
        if name in existing_names:
            for item in ls:
                if item.get("name") == name:
                    item["value"] = value
        else:
            ls.append({"name": name, "value": value})
            existing_names.add(name)
    origin_entry["localStorage"] = ls

    cookie_data["storage_state"] = {"origins": origins}
    cookie_data["credentials_saved_at"] = datetime.datetime.utcnow().isoformat()
    try:
        cookie_json_path.write_text(json.dumps(cookie_data, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception as e:
        return fail(f"写入 cookie.json 失败: {e}")

    found_target_keys = [k for k in target_keys if found_items.get(k)]
    missing_keys = [k for k in target_keys if not found_items.get(k)]

    hint = ""
    if "security-sdk/s_sdk_sign_data_key/web_protect" in missing_keys:
        hint = (
            "\n\n说明：web_protect 是抖音 security-sdk 的签名密钥。我们在独立 Chrome 实例里"
            "已经尝试调用了 IM 接口去触发生成，但 security-sdk 仍未下发。"
            "\n常见原因："
            "\n  ① Cookie 已过期（最常见）→ 请重新登录该抖音账号让 App 抓取新 cookie"
            "\n  ② 抖音风控触发 → 切换网络/换 IP 后再试"
            "\n  ③ 弹出的 Chrome 窗口里出现了人机验证 → 手动完成验证，再点刷新凭证"
            "\n  ④ 浏览器被关闭过快 → 让 Chrome 多停留几秒再关闭"
        )

    return ok(
        found_keys=found_target_keys,
        missing_keys=missing_keys,
        send_ready=len(missing_keys) == 0,
        message=f"已更新 {len(found_target_keys)}/{len(target_keys)} 个认证字段" + (
            f"；仍缺少: {missing_keys}" + hint if missing_keys else "；发送凭证完整"
        ),
    )


def emit_jsonl(obj: dict[str, Any]) -> None:
    print(json.dumps(obj, ensure_ascii=False, separators=(",", ":")), flush=True)


def cmd_monitor(args: argparse.Namespace) -> dict[str, Any]:
    auth = load_auth(args)

    # 凭证完整性预检：缺 web_protect/keys 直接返回明确错误，否则 WebSocket 握手会失败导致"启动即停止"
    missing: list[str] = []
    if not getattr(auth, "web_protect", None):
        missing.append("web_protect")
    if not getattr(auth, "keys", None):
        missing.append("keys (s_sdk_crypt_sdk)")
    if missing:
        msg = (
            f"凭证不完整，缺少: {', '.join(missing)}。"
            f"\n请先点「刷新凭证」让 Chrome 重新读取 localStorage，"
            f"或重新登录账号以捕获完整 security-sdk 数据。"
        )
        emit_jsonl({"type": "status", "status": "error", "account": args.account_name or "", "error": msg})
        return fail(msg)

    def on_event(event: dict[str, Any]) -> None:
        emit_jsonl({"type": "im_message", "account": args.account_name or "", "event": event})

    receiver = DouyinMessageReceiver(auth, auto_reconnect=not args.no_reconnect, on_event=on_event)

    def _on_open(_ws):
        log("[douyin_im_bridge] WebSocket connection open")
        emit_jsonl({"type": "status", "status": "connected", "account": args.account_name or ""})

    def _on_close(_ws, code, msg):
        log(f"[douyin_im_bridge] WebSocket closed: code={code}, msg={msg}")
        emit_jsonl({"type": "status", "status": "disconnected", "account": args.account_name or "",
                    "code": code, "message": str(msg or "")})

    def _on_error(_ws, error):
        log(f"[douyin_im_bridge] WebSocket error: {error}")
        emit_jsonl({"type": "status", "status": "error", "account": args.account_name or "", "error": str(error)})

    # 关闭 receiver.py 自带的递归重连（我们改用上层 process 重启策略，避免栈爆 + 反复无效请求）
    receiver.auto_reconnect = False

    # _on_open 真正连上后才把状态切到 running，避免 UI 一闪而过
    original_on_open = _on_open
    def _on_open_wrapped(_ws):
        original_on_open(_ws)
        emit_jsonl({"type": "status", "status": "running", "account": args.account_name or ""})

    receiver.on_open = _on_open_wrapped
    receiver.on_close = _on_close
    receiver.on_error = _on_error

    # 发"starting"让 UI 知道我们在尝试连接（连接通常 1-3 秒）
    emit_jsonl({"type": "status", "status": "starting", "account": args.account_name or ""})
    try:
        receiver.start()
    except Exception as e:
        emit_jsonl({"type": "status", "status": "error", "account": args.account_name or "", "error": str(e)})
        return fail(f"监控运行异常: {e}")

    # start() 返回意味着 WebSocket 断开。如果根本没触发过 on_open（即从未连上），
    # 上面的 on_error 已经 emit 过 error；否则就是正常的服务端断开。
    return ok(status="stopped")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="AutoCast AI 抖音私信桥接脚本")
    parser.add_argument("action", choices=["check", "my_uid", "contacts", "messages", "create_conversation", "send", "monitor", "recv", "refresh_credentials", "user_info"])
    parser.add_argument("--account-name", default="")
    parser.add_argument("--cookie", default="")
    parser.add_argument("--cookie-path", default="")
    parser.add_argument("--web-protect", default="")
    parser.add_argument("--web-protect-path", default="")
    parser.add_argument("--keys", default="")
    parser.add_argument("--keys-path", default="")
    parser.add_argument("--uid", default="")
    parser.add_argument("--timeout", type=int, default=30)
    parser.add_argument("--wait", type=float, default=5.0)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--to-user-id", default="")
    parser.add_argument("--user-id", default="")
    parser.add_argument("--peer-uid", default="")
    parser.add_argument("--conversation-id", default="")
    parser.add_argument("--conversation-short-id", type=int)
    parser.add_argument("--ticket", default="")
    parser.add_argument("--content", default="")
    parser.add_argument("--no-reconnect", action="store_true")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        if args.action == "monitor":
            # monitor 模式：stdout 是 JSONL 事件流，崩溃也要走 JSONL 报错（让 Rust/UI 能解析）
            try:
                result = cmd_monitor(args)
            except Exception as e:
                import traceback
                tb = traceback.format_exc()
                log(f"[douyin_im_bridge] monitor 崩溃: {e}\n{tb}")
                emit_jsonl({
                    "type": "status",
                    "status": "error",
                    "account": args.account_name or "",
                    "error": f"{type(e).__name__}: {e}",
                })
                return 1
            return 0 if result.get("ok") else 1

        # 第三方/底层 print 全部挪到 stderr，stdout 保持最终 JSON 纯净。
        with contextlib.redirect_stdout(sys.stderr):
            if args.action == "check":
                result = cmd_check(args)
            elif args.action == "my_uid":
                result = cmd_my_uid(args)
            elif args.action == "contacts":
                result = cmd_contacts(args)
            elif args.action == "messages":
                result = cmd_messages(args)
            elif args.action == "create_conversation":
                result = cmd_create_conversation(args)
            elif args.action == "send":
                result = cmd_send(args)
            elif args.action == "refresh_credentials":
                result = cmd_refresh_credentials(args)
            elif args.action == "user_info":
                result = cmd_user_info(args)
            elif args.action == "recv":
                result = cmd_monitor(args)
            else:
                result = fail(f"未知 action: {args.action}")
    except Exception as e:
        log(f"[douyin_im_bridge] error: {e}")
        result = fail(str(e))

    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result.get("ok") else 1


if __name__ == "__main__":
    raise SystemExit(main())
