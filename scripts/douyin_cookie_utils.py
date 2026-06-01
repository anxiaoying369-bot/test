#!/usr/bin/env python3
"""抖音登录辅助：用户信息提取 + Cookie/localStorage 抓取与落盘。

从 douyin_login.py 拆分而来，均为无副作用的纯函数（只依赖传入的 page / 参数），
不涉及登录脚本的全局状态。
"""

import json
import os
import re
import time
from typing import Optional, Any


# ============ 用户信息提取 ============

def extract_user_info(page):
    user_name, user_id, avatar = None, None, None
    try:
        try:
            page.get("https://www.douyin.com/user/self")
            time.sleep(3)
        except Exception as e:
            print(f"[DY] navigate user/self failed (non-fatal): {e}", flush=True)

        # 尝试从 JS 全局变量提取（_ROUTER_DATA / NEXT_DATA 等）
        try:
            js_info = page.run_js("""() => {
                // 方式1: _ROUTER_DATA（抖音 PC 端常见）
                if (window._ROUTER_DATA?.loaderData) {
                    for (let key in window._ROUTER_DATA.loaderData) {
                        const data = window._ROUTER_DATA.loaderData[key];
                        if (data?.user) return data.user;
                        if (data?.userInfo?.user) return data.userInfo.user;
                    }
                }
                // 方式2: __NEXT_DATA__
                try {
                    const nd = JSON.parse(document.getElementById('__NEXT_DATA__')?.textContent || 'null');
                    const u = nd?.props?.pageProps?.userInfo?.user || nd?.props?.pageProps?.user;
                    if (u) return u;
                } catch(e) {}
                // 方式3: window.userData
                if (window.userData) return window.userData;
                return null;
            }""")
            if js_info and isinstance(js_info, dict):
                user_id = (js_info.get("uniqueId") or js_info.get("unique_id")
                           or js_info.get("userId") or js_info.get("uid"))
                user_name = js_info.get("nickname") or js_info.get("name")
                # 头像优先取大图
                avatar = (js_info.get("avatarLarger") or js_info.get("avatarMedium")
                          or js_info.get("avatarThumb") or js_info.get("avatarUrl")
                          or js_info.get("avatar_url") or js_info.get("avatar_300x300"))
                print(f"[DY] js extract: uid={user_id} name={user_name} avatar={'yes' if avatar else 'no'}", flush=True)
        except Exception as e:
            print(f"[DY] js extract failed: {e}", flush=True)

        # 从页面文本提取抖音号
        if not user_id:
            try:
                body_text = page.html or ""
                m = re.search(r"(抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)", body_text)
                if m:
                    user_id = m.group(2)
            except Exception:
                pass

        # 从 DOM 提取昵称
        if not user_name:
            for sel in ['[data-e2e="user-info-name"]', 'h1', '.header-right-name']:
                try:
                    el = page.ele(f"css:{sel}", timeout=2)
                    if el:
                        user_name = (el.text or "").strip().split("\n")[0]
                        if user_name:
                            break
                except Exception:
                    pass

        # 从 DOM 提取头像（按优先级逐一尝试）
        if not avatar:
            avatar_selectors = [
                # 精确匹配个人主页头像区域（XPath //*[@id="user_detail_element"]/div/div[2]/div[1]/span/img）
                "css:#user_detail_element img",
                # data-e2e 属性
                'css:[data-e2e="user-avatar"] img',
                'css:[data-e2e="avatar"] img',
                # 常见 class 片段
                "css:div[class*='avatar'] img",
                "css:.semi-avatar img",
                # src 包含 aweme-avatar 路径
                "css:img[src*='aweme-avatar']",
            ]
            for sel in avatar_selectors:
                try:
                    el = page.ele(sel, timeout=2)
                    if el:
                        src = el.attr("src") or ""
                        if src and src.startswith("http"):
                            avatar = src
                            print(f"[DY] avatar found via '{sel}': {avatar[:80]}...", flush=True)
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


def get_cookies_from_page(page) -> list:
    """从 DrissionPage 获取 Cookie，统一为 list[dict] 格式"""
    try:
        raw = page.cookies()
        if isinstance(raw, list):
            # 确保每个 cookie 都有 path 字段（兼容 Playwright storage_state 格式）
            for c in raw:
                if isinstance(c, dict) and "path" not in c:
                    c["path"] = "/"
            return raw
        if isinstance(raw, dict):
            if "path" not in raw:
                raw["path"] = "/"
            return [raw]
    except Exception as e:
        print(f"[DY] get_cookies failed: {e}", flush=True)
    return []


def get_storage_state(page) -> Optional[dict]:
    """从 DrissionPage 获取 localStorage 等存储状态。

    注意：extract_user_info() 会跳到 /user/self。抖音 security-sdk 的 IM 发送认证材料
    可能只在首页/私信页初始化，所以这里不能只读当前页 localStorage；要额外访问
    www.douyin.com 和 /chat，把目标 localStorage key 保存下来。
    """
    targets = [
        "security-sdk/s_sdk_crypt_sdk",
        "security-sdk/s_sdk_sign_data_key/web_protect",
    ]
    origins_by_url: dict[str, dict[str, Any]] = {}

    def read_current_page(label: str) -> None:
        try:
            # 使用 async function + JSON.stringify，与 DrissionPage 兼容性最好
            # （DrissionPage 会自动调用 async function，并将返回值序列化为字符串）
            raw = page.run_js("""
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
            """)
            if raw is None:
                print(f"[DY] localStorage scan {label}: run_js 返回 None（页面可能未完全加载）", flush=True)
                return
            result = json.loads(raw) if isinstance(raw, str) else raw
            if not isinstance(result, dict):
                print(f"[DY] localStorage scan {label}: 意外结果类型 type={type(raw).__name__} val={str(raw)[:80]}", flush=True)
                return
            items = result.get("items") or []
            origin = result.get("origin") or "https://www.douyin.com"
            if origin not in origins_by_url:
                origins_by_url[origin] = {"origin": origin, "localStorage": []}
            known = {x.get("name") for x in origins_by_url[origin]["localStorage"]}
            for item in items:
                name = item.get("name") if isinstance(item, dict) else None
                if name and name not in known:
                    origins_by_url[origin]["localStorage"].append({
                        "name": name,
                        "value": item.get("value"),
                    })
                    known.add(name)
            found = sorted(name for name in targets if name in known)
            print(f"[DY] localStorage scan {label}: origin={origin}, count={len(items)}, auth_keys={found}", flush=True)
        except Exception as e:
            print(f"[DY] localStorage scan {label} failed: {e}", flush=True)

    read_current_page("current")

    # 主动触发 security-sdk 初始化；只发生在登录保存阶段，不发生在私信发送动作里。
    original_url = ""
    try:
        original_url = page.url or ""
    except Exception:
        pass
    for label, url in [("home", "https://www.douyin.com/"), ("chat", "https://www.douyin.com/chat")]:
        try:
            page.get(url)
            time.sleep(5)
            read_current_page(label)
        except Exception as e:
            print(f"[DY] navigate {label} for localStorage failed: {e}", flush=True)

    if original_url and original_url.startswith("http"):
        try:
            page.get(original_url)
            time.sleep(1)
        except Exception:
            pass

    origins = list(origins_by_url.values())
    if origins:
        all_names = {item.get("name") for origin in origins for item in origin.get("localStorage") or []}
        missing = [name for name in targets if name not in all_names]
        if missing:
            print(f"[DY] auth localStorage missing after scan: {missing}", flush=True)
        return {"origins": origins}
    return None


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
