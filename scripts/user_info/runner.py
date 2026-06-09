import time
from .utils import _log, load_cookie_data, classify_user_id, expand_short_url, parse_share_url, build_user_home_url, save_user_json
from .resolver import resolve_id_to_sec_uid
from .extractor import extract_user_from_page

def run(cookie_path: str, user_id_input: str, save: bool = True, headless: bool = False) -> dict:
    _log(f"[GET_USER] 加载 cookie: {cookie_path}")
    cookies = load_cookie_data(cookie_path)
    _log(f"[GET_USER] cookie 数量: {len(cookies)}")

    from DrissionPage import ChromiumPage, ChromiumOptions
    co = ChromiumOptions()
    if headless:
        co.headless()

    page = None
    try:
        page = ChromiumPage(co)
        page = page.latest_tab
    except Exception as e:
        _log(f"[GET_USER] 启动浏览器失败: {e}")
        return {"status": "error", "error": f"启动浏览器失败: {e}"}

    try:
        _log("[GET_USER] Step 1: 访问 douyin.com 建立上下文")
        try:
            page.get("https://www.douyin.com/")
            time.sleep(2)
        except Exception as e:
            _log(f"[GET_USER] 访问首页失败 (非致命): {e}")

        _log(f"[GET_USER] Step 2: 注入 {len(cookies)} 条 cookie")
        injected = 0
        for c in cookies:
            try:
                name = c.get("name")
                value = c.get("value")
                if not name or value is None:
                    continue
                domain = c.get("domain", ".douyin.com")
                if not domain.startswith("."):
                    domain = "." + domain
                path = c.get("path", "/")

                if c.get("httpOnly"):
                    try:
                        page.run_cdp("Network.setCookie", **{
                            "name": name,
                            "value": str(value),
                            "domain": domain,
                            "path": path,
                            "secure": bool(c.get("secure", False)),
                            "httpOnly": True,
                        })
                        injected += 1
                    except Exception:
                        pass
                    continue

                cookie_str = f"{name}={value}; domain={domain}; path={path}"
                if c.get("secure"):
                    cookie_str += "; secure"
                page.run_js(f"document.cookie = '{cookie_str}'")
                injected += 1
            except Exception:
                continue
        _log(f"[GET_USER] 注入成功 {injected}/{len(cookies)}")

        kind = classify_user_id(user_id_input)
        _log(f"[GET_USER] 输入 ID 分类: {kind} (raw={user_id_input[:60]})")

        resolved_meta = None
        if kind == "url":
            url = user_id_input
            if "v.douyin.com" in url:
                url = expand_short_url(page, url)
            parsed = parse_share_url(url)
            sec_uid = parsed.get("sec_uid")
            if not sec_uid:
                return {"status": "error", "error": f"无法从 URL 解析 sec_uid: {url}"}
            resolved_meta = {
                "uid": parsed.get("uid"),
                "unique_id": parsed.get("unique_id"),
                "short_id": parsed.get("short_id"),
            }
        elif kind == "sec_uid":
            sec_uid = user_id_input
        elif kind in ("uid_or_short", "unique_id"):
            sec_uid, resolved_meta = resolve_id_to_sec_uid(page, user_id_input, kind)
            if not sec_uid:
                return {
                    "status": "error",
                    "error": f"无法把 {kind}={user_id_input} 转换为 sec_uid",
                }
        else:
            return {"status": "error", "error": f"无法识别 ID 格式: {user_id_input}"}

        _log(f"[GET_USER] 锁定 sec_uid: {sec_uid}")
        home_url = build_user_home_url(sec_uid)
        _log(f"[GET_USER] Step 3: 访问主页 {home_url}")
        try:
            page.get(home_url)
            time.sleep(4)
        except Exception as e:
            _log(f"[GET_USER] 访问主页失败: {e}")
            return {"status": "error", "error": f"访问主页失败: {e}", "sec_uid": sec_uid}

        user_data = extract_user_from_page(page, attempts=4)
        if resolved_meta:
            for k, v in resolved_meta.items():
                if v and not user_data.get(k):
                    user_data[k] = v

        if not user_data.get("sec_uid"):
            user_data["sec_uid"] = sec_uid

        user_json_path = None
        if save and user_data.get("sec_uid"):
            try:
                user_json_path = save_user_json(sec_uid, user_data)
            except Exception as e:
                _log(f"[GET_USER] 写盘失败: {e}")

        return {
            "status": "ok" if user_data.get("nickname") else "partial",
            "user": user_data,
            "sec_uid": sec_uid,
            "input_id": user_id_input,
            "input_kind": kind,
            "user_json_path": user_json_path,
        }

    except Exception as e:
        _log(f"[GET_USER] 主流程异常: {e}")
        return {"status": "error", "error": str(e), "input_id": user_id_input}
    finally:
        try:
            page.quit()
        except Exception:
            pass
