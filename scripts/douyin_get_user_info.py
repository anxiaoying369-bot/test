#!/usr/bin/env python3
"""
抖音用户信息查询桥接脚本（AutoCast AI 集成版）

复用 AutoCastAI 现有的 cookie + CDP 接管能力，输入任意形式的用户 ID
（sec_uid / uid / short_id / unique_id / 分享短链 / 主页 URL），
通过抖音 Web 端页面提取用户的完整信息。

设计原则:
  1. 优先复用现有工具: douyin_cookie_utils.py 的 cookie 加载、
     verify_account.py 的 CDP 接管模式、douyin_scraper.py 的 stdout 协议
  2. 输出的 user.json 字段名与 douyin_scraper.py 写入的完全一致
     (sec_uid / nickname / avatar_url / follower_count / ...)
  3. 兼容 scrape 流程: 写入 scraper_data/{sec_uid}/data/{sec_uid}/user.json，
     后续 VideoService / CommentService 等可直接消费

用法:
  # 通过 sec_uid 查询（最快）
  python3 douyin_get_user_info.py --cookie-path <cookie.txt> \\
      --user-id "MS4wLjABAAAAxxx"

  # 通过抖音号(unique_id) 查询
  python3 douyin_get_user_info.py --cookie-path <cookie.txt> \\
      --user-id "dh2022"

  # 通过分享短链查询
  python3 douyin_get_user_info.py --cookie-path <cookie.txt> \\
      --user-id "https://v.douyin.com/JfwAoWb/"

  # 通过主页 URL 查询
  python3 douyin_get_user_info.py --cookie-path <cookie.txt> \\
      --user-id "https://www.douyin.com/user/MS4wLjABAAAAxxx"

  # 不写盘，只打印到 stdout
  python3 douyin_get_user_info.py --cookie-path <cookie.txt> \\
      --user-id "MS4wLjABAAAAxxx" --no-save

stdout: JSON
stderr: 日志
"""

import argparse
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Optional, Tuple
from urllib.parse import urlparse, parse_qs

# 把脚本所在目录加入 path，复用 douyin_cookie_utils / compat
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPT_DIR)

from compat import get_data_dir  # noqa: E402


# ============ 日志 ============

def _log(msg: str) -> None:
    """所有日志走 stderr，stdout 留给 JSON 结果。"""
    print(msg, flush=True, file=sys.stderr)


# ============ ID 类型识别 ============

# Base64 风格: 字母+数字+'-'+'_', 长度 30+, 常以 MS4 开头 (抖音 sec_uid 通用前缀)
SEC_UID_PATTERN = re.compile(r"^[A-Za-z0-9_\-]{20,}$")
# 纯数字 (uid 或 short_id 都需要走进一步判定)
NUMERIC_PATTERN = re.compile(r"^\d{6,20}$")
# 抖音号 (unique_id): 字母/数字/下划线/点, 长度 2-30, 用户自设
UNIQUE_ID_PATTERN = re.compile(r"^[A-Za-z0-9_.]{2,30}$")


def classify_user_id(raw: str) -> str:
    """
    分类输入 ID 的类型。
    返回: 'sec_uid' | 'uid_or_short' | 'unique_id' | 'url' | 'unknown'
    """
    raw = raw.strip()
    if not raw:
        return "unknown"

    # 1. URL
    if raw.startswith(("http://", "https://", "v.douyin.com/")):
        return "url"

    # 2. sec_uid (Base64 风格)
    if SEC_UID_PATTERN.match(raw) and not raw.isdigit():
        return "sec_uid"

    # 3. 纯数字 → uid 或 short_id, 进一步判定需要拉一次
    if NUMERIC_PATTERN.match(raw):
        return "uid_or_short"

    # 4. 抖音号
    if UNIQUE_ID_PATTERN.match(raw):
        return "unique_id"

    return "unknown"


def parse_share_url(url: str) -> dict:
    """
    解析抖音分享链接,提取 sec_uid / uid / unique_id / short_id。
    支持:
      - https://v.douyin.com/xxxxxx/  (短链,需要 302 重定向)
      - https://www.iesdouyin.com/share/user/{uid}?sec_uid=xxx
      - https://www.douyin.com/user/{sec_uid}
      - https://www.douyin.com/discover?modal_id=xxx (非用户链接)
    """
    result = {"sec_uid": None, "uid": None, "short_id": None, "unique_id": None, "raw": url}

    # 短链 v.douyin.com 需要展开 (DrissionPage 在外层会处理)
    # 这里假设已经展开 (或返回后会再调一次)
    try:
        u = urlparse(url)
        host = (u.netloc or "").lower()
        path = u.path or ""
        query = parse_qs(u.query)

        # 1. /user/{sec_uid} 形式
        m = re.match(r"^/user/([A-Za-z0-9_\-]+)/?$", path)
        if m and "douyin.com" in host and "iesdouyin" not in host:
            result["sec_uid"] = m.group(1)
            # query 可能有 unique_id
            if "unique_id" in query:
                result["unique_id"] = query["unique_id"][0]

        # 2. /share/user/{uid}?sec_uid=xxx 形式
        m = re.match(r"^/share/user/?$", path)
        if not m:
            m = re.match(r"^/share/user/(\d+)/?$", path)
            if m:
                result["uid"] = m.group(1)
        if "sec_uid" in query:
            result["sec_uid"] = query["sec_uid"][0]

        # 3. 任何带 unique_id 的链接
        if "uniqueId" in query and not result["unique_id"]:
            result["unique_id"] = query["uniqueId"][0]
        if "unique_id" in query and not result["unique_id"]:
            result["unique_id"] = query["unique_id"][0]

    except Exception as e:
        _log(f"[GET_USER] URL 解析失败 (非致命): {e}")

    return result


# ============ Cookie 加载（复用 douyin_scraper 模式）============

def load_cookie_data(cookie_path: str) -> list:
    """
    读取 cookie 文件,返回 list[dict] 格式 (用于 CDP 注入)。

    支持两种格式:
      1. cookie.json (完整 Playwright storage_state 格式,含 cookies 数组)
      2. cookie.txt (name1=val1; name2=val2; 原始字符串)
    """
    path = Path(cookie_path)
    if not path.exists():
        raise FileNotFoundError(f"cookie 文件不存在: {cookie_path}")

    suffix = path.suffix.lower()

    # 1. JSON 格式
    if suffix == ".json":
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, dict) and "cookies" in data:
            cookies = data["cookies"]
        elif isinstance(data, list):
            cookies = data
        else:
            raise ValueError(f"cookie.json 格式异常: {type(data).__name__}")
        # 补 path 字段
        for c in cookies:
            if isinstance(c, dict) and "path" not in c:
                c["path"] = "/"
        return cookies

    # 2. 原始字符串 (cookie.txt)
    raw = path.read_text(encoding="utf-8").strip()
    if not raw:
        raise ValueError("cookie 文件为空")

    cookies = []
    for part in raw.split(";"):
        part = part.strip()
        if not part or "=" not in part:
            continue
        name, _, value = part.partition("=")
        cookies.append({
            "name": name.strip(),
            "value": value.strip(),
            "domain": ".douyin.com",  # 兼容兜底域
            "path": "/",
        })
    return cookies


# ============ 短链展开 ============

def expand_short_url(page, short_url: str) -> str:
    """
    把 v.douyin.com 短链展开成真实 URL。
    用 page.get(short_url, allow_redirects=False) → 读 Location header。
    """
    try:
        # 尝试用 raw CDP Network.fetch 拿 location
        # 兼容性最好的方式是 page.get + 检查最终 url
        page.get(short_url)
        time.sleep(1.5)
        final = page.url or ""
        if final and final != short_url:
            _log(f"[GET_USER] 短链展开: {short_url[:40]}... → {final[:80]}")
            return final
    except Exception as e:
        _log(f"[GET_USER] 短链展开失败: {e}")
    return short_url


# ============ 页面信息提取（核心）============

# 完整提取脚本: 优先 _ROUTER_DATA → RENDER_DATA → DOM 兜底
# 提取后返回统一 schema: { sec_uid, uid, short_id, unique_id, nickname, signature, avatar_url, follower_count, ... }
EXTRACT_JS = r"""
async function() {
    const out = {
        sec_uid: null, uid: null, short_id: null, unique_id: null,
        nickname: null, signature: null, avatar_url: null,
        avatar_larger: null, avatar_medium: null, avatar_thumb: null,
        follower_count: null, following_count: null, aweme_count: null,
        favoriting_count: null, total_favorited: null,
        gender: null, birthday: null, location: null, school: null,
        enterprise_verify_reason: null, custom_verify: null,
        verification_type: null, is_verified: null,
        share_url: null, source_strategy: null, raw: null
    };

    function pick(obj, ...keys) {
        if (!obj) return null;
        for (const k of keys) {
            if (obj[k] !== undefined && obj[k] !== null && obj[k] !== "") return obj[k];
        }
        return null;
    }

    function normalize(user) {
        if (!user || typeof user !== 'object') return false;
        out.sec_uid  = pick(user, 'sec_uid', 'secUid', 'secUserId');
        out.uid      = pick(user, 'uid', 'userId', 'id', 'user_id');
        out.short_id = pick(user, 'short_id', 'shortId', 'short_id_str');
        out.unique_id= pick(user, 'unique_id', 'uniqueId', 'unique_id_str');
        out.nickname = pick(user, 'nickname', 'nick_name', 'name');
        out.signature= pick(user, 'signature', 'desc', 'bio');
        out.follower_count   = pick(user, 'follower_count', 'followerCount');
        out.following_count  = pick(user, 'following_count', 'followingCount');
        out.aweme_count      = pick(user, 'aweme_count', 'awemeCount', 'video_count');
        out.favoriting_count = pick(user, 'favoriting_count', 'favoritingCount');
        out.total_favorited  = pick(user, 'total_favorited', 'totalFavorited');
        out.gender      = pick(user, 'gender');
        out.birthday    = pick(user, 'birthday');
        out.location    = pick(user, 'location', 'city', 'ip_location');
        out.school      = pick(user, 'school_name', 'school');
        out.enterprise_verify_reason = pick(user, 'enterprise_verify_reason', 'enterpriseVerifyReason');
        out.custom_verify  = pick(user, 'custom_verify', 'customVerify');
        out.verification_type = pick(user, 'verification_type', 'verificationType');
        out.is_verified  = pick(user, 'is_verified', 'verified', 'isVerified');
        out.share_url    = pick(user, 'share_url', 'shareUrl');
        // 头像: 抖音有 avatar_300x300 / avatar_larger / avatar_medium / avatar_thumb
        out.avatar_larger  = pick(user, 'avatar_larger', 'avatarLarger',
                                  'avatar_300x300', 'avatar_300X300');
        out.avatar_medium  = pick(user, 'avatar_medium', 'avatarMedium',
                                  'avatar_168x168', 'avatar_168X168');
        out.avatar_thumb   = pick(user, 'avatar_thumb',  'avatarThumb',
                                  'avatar_100x100', 'avatar_100X100');
        // 头像 URL 嵌套在 {url_list: [...]} 或 {url_list: [{url: '...'}], width, height}
        if (!out.avatar_larger) {
            const a = user.avatar_larger || user.avatarLarger || user.avatar_300x300;
            if (a) {
                if (typeof a === 'string') out.avatar_larger = a;
                else if (a.url_list && a.url_list.length) {
                    out.avatar_larger = a.url_list[0].url || a.url_list[0];
                }
            }
        }
        if (!out.avatar_medium) {
            const a = user.avatar_medium || user.avatarMedium;
            if (a) {
                if (typeof a === 'string') out.avatar_medium = a;
                else if (a.url_list && a.url_list.length) {
                    out.avatar_medium = a.url_list[0].url || a.url_list[0];
                }
            }
        }
        if (!out.avatar_thumb) {
            const a = user.avatar_thumb || user.avatarThumb;
            if (a) {
                if (typeof a === 'string') out.avatar_thumb = a;
                else if (a.url_list && a.url_list.length) {
                    out.avatar_thumb = a.url_list[0].url || a.url_list[0];
                }
            }
        }
        // 默认头像用最大图
        out.avatar_url = out.avatar_larger || out.avatar_medium || out.avatar_thumb;
        return true;
    }

    // 方式 1: window._ROUTER_DATA (抖音 PC 端最常见)
    try {
        const rd = window._ROUTER_DATA;
        if (rd && rd.loaderData) {
            for (const key in rd.loaderData) {
                const data = rd.loaderData[key];
                if (data && data.userInfo && data.userInfo.user) {
                    if (normalize(data.userInfo.user)) {
                        out.source_strategy = '_ROUTER_DATA.loaderData.userInfo.user';
                        return JSON.stringify(out);
                    }
                }
                if (data && data.user) {
                    if (normalize(data.user)) {
                        out.source_strategy = '_ROUTER_DATA.loaderData.user';
                        return JSON.stringify(out);
                    }
                }
            }
        }
    } catch(e) {}

    // 方式 2: window.RENDER_DATA
    try {
        const rd = window.RENDER_DATA;
        if (rd) {
            const parsed = typeof rd === 'string' ? JSON.parse(rd) : rd;
            if (parsed && parsed.user) {
                if (normalize(parsed.user)) {
                    out.source_strategy = 'RENDER_DATA.user';
                    return JSON.stringify(out);
                }
            }
        }
    } catch(e) {}

    // 方式 3: window.__INITIAL_STATE__ / window.__INITIAL_DATA__
    try {
        for (const vname of ['__INITIAL_STATE__', '__INITIAL_DATA__']) {
            const v = window[vname];
            if (v) {
                const s = typeof v === 'string' ? JSON.parse(v) : v;
                // 递归找 user
                function findUser(o, depth) {
                    if (depth > 5 || !o || typeof o !== 'object') return null;
                    if (o.user && (o.user.sec_uid || o.user.secUid || o.user.nickname)) return o.user;
                    for (const k in o) {
                        const r = findUser(o[k], depth+1);
                        if (r) return r;
                    }
                    return null;
                }
                const u = findUser(s, 0);
                if (u && normalize(u)) {
                    out.source_strategy = vname + '.user';
                    return JSON.stringify(out);
                }
            }
        }
    } catch(e) {}

    // 方式 4: 抖音页面注入的 window.userData (老版本)
    try {
        if (window.userData && normalize(window.userData)) {
            out.source_strategy = 'window.userData';
            return JSON.stringify(out);
        }
    } catch(e) {}

    // 方式 5: DOM 兜底
    try {
        // 昵称
        const nickEl = document.querySelector('[data-e2e="user-info-name"]')
                    || document.querySelector('h1')
                    || document.querySelector('.header-right-name');
        if (nickEl) out.nickname = (nickEl.textContent || '').split('\n')[0].trim();

        // 抖音号
        const bodyText = document.body.innerText || '';
        const m = bodyText.match(/(?:抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)/);
        if (m) out.unique_id = m[1];

        // 头像
        const avatarImg = document.querySelector('#user_detail_element img')
                       || document.querySelector('[data-e2e="user-avatar"] img')
                       || document.querySelector('img[src*="aweme-avatar"]');
        if (avatarImg) {
            const src = avatarImg.getAttribute('src') || '';
            if (src.startsWith('http')) out.avatar_url = src;
        }

        if (out.nickname || out.unique_id) {
            out.source_strategy = 'DOM_fallback';
            return JSON.stringify(out);
        }
    } catch(e) {}

    return JSON.stringify(out);
}
"""


def extract_user_from_page(page, attempts: int = 3) -> dict:
    """
    从当前页面提取用户信息,失败时多次重试。
    抖音 SPA 经常初次加载拿到的是骨架屏,要 sleep + 重试。
    """
    last_result: dict = {}
    for i in range(attempts):
        try:
            raw = page.run_js(EXTRACT_JS)
            if raw is None:
                _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次: run_js 返回 None")
                time.sleep(1.5)
                continue
            # run_js 异步函数会返回 JSON 字符串,先 parse
            if isinstance(raw, str):
                try:
                    data = json.loads(raw)
                except Exception:
                    _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次: JSON 解析失败 raw={raw[:120]}")
                    time.sleep(1.5)
                    continue
            elif isinstance(raw, dict):
                data = raw
            else:
                time.sleep(1.5)
                continue

            # 至少要 sec_uid + nickname 才算成功
            if data.get("sec_uid") and data.get("nickname"):
                _log(f"[GET_USER] 提取成功 (策略: {data.get('source_strategy')}, 尝试 {i+1}/{attempts})")
                return data
            last_result = data
        except Exception as e:
            _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次异常: {e}")
        time.sleep(1.5)

    _log(f"[GET_USER] 提取失败,返回最后一次结果 (字段数: {sum(1 for v in last_result.values() if v)})")
    return last_result


# ============ UID 数字 → SEC_UID 反查 ============

# 当用户输入 uid/short_id/unique_id 时,需要先在页面里找到对应的 sec_uid
# 策略: 用搜索 API (通过 fetch + cookie),或者直接访问抖音搜索页

# 这一段较复杂,本脚本默认需要 sec_uid 形式;uid/short_id/unique_id 走 CDP 方式打开对应页面

# uid → 个人主页 URL: https://www.douyin.com/user/{sec_uid} 不行,需要 sec_uid
# 但可以试: https://www.douyin.com/user/uid_{uid}? 不行,只有 sec_uid 形式
# 所以 uid/short_id/unique_id 都要先转 sec_uid,这里通过搜索 API 实现

# 抖音搜索用户 API (不需要 X-Bogus 签名的搜索)
# 实际: https://www.douyin.com/search/{keyword}/?type=user
SEARCH_USER_JS = r"""
async function(keyword) {
    try {
        const r = await fetch(
            `https://www.douyin.com/aweme/v1/web/search/user/?keyword=${encodeURIComponent(keyword)}&count=10&type=1`,
            {
                method: 'GET',
                credentials: 'include',
                headers: {
                    'Accept': 'application/json',
                    'Referer': 'https://www.douyin.com/search/' + encodeURIComponent(keyword) + '/?type=user'
                }
            }
        );
        if (!r.ok) return JSON.stringify({error: 'http_' + r.status});
        const data = await r.json();
        const users = (data.user_list || data.users || []).map(u => ({
            sec_uid: u.sec_uid || u.secUid,
            uid: u.uid || u.user_id,
            unique_id: u.unique_id || u.uniqueId,
            short_id: u.short_id || u.shortId,
            nickname: u.nickname
        }));
        return JSON.stringify({users: users});
    } catch(e) {
        return JSON.stringify({error: e.message});
    }
}
"""


def resolve_id_to_sec_uid(page, user_id: str, kind: str) -> Tuple[Optional[str], Optional[dict]]:
    """
    把 uid / short_id / unique_id 转换为 sec_uid。
    通过抖音搜索 API (在 page 上下文里跑,自动带 cookie)。
    返回: (sec_uid, 搜索结果第一条的元信息)
    """
    if kind == "sec_uid":
        return user_id, None

    if kind == "uid_or_short":
        # uid 和 short_id 都是纯数字,搜索时用 uid 当 keyword
        # 实际上抖音搜索 API 不接受纯数字搜索出具体用户,但可以试 unique_id 形式
        # 走捷径: 数字尝试当 short_id,搜索结果可能拿到
        keyword = user_id
    elif kind == "unique_id":
        keyword = user_id.lstrip("@")
    else:
        return None, None

    _log(f"[GET_USER] 反查 {kind}={keyword} → sec_uid, 走抖音搜索 API")
    try:
        raw = page.run_js(SEARCH_USER_JS, keyword)
        if raw is None:
            return None, None
        data = json.loads(raw) if isinstance(raw, str) else raw
        if "error" in data:
            _log(f"[GET_USER] 搜索 API 失败: {data['error']}")
            return None, None
        users = data.get("users", []) or []
        if not users:
            _log(f"[GET_USER] 搜索 API 无结果")
            return None, None
        # 优先精确匹配
        for u in users:
            if kind == "uid_or_short" and str(u.get("uid") or "") == user_id:
                return u.get("sec_uid"), u
            if kind == "unique_id" and u.get("unique_id") == keyword.lstrip("@"):
                return u.get("sec_uid"), u
        # 退化: 取第一个
        first = users[0]
        _log(f"[GET_USER] 未精确匹配,使用第一个: {first.get('nickname')}")
        return first.get("sec_uid"), first
    except Exception as e:
        _log(f"[GET_USER] 反查异常: {e}")
        return None, None


# ============ 主页 URL 构造 ============

def build_user_home_url(sec_uid: str) -> str:
    """构造抖音用户主页 URL"""
    return f"https://www.douyin.com/user/{sec_uid}"


# ============ user.json 落盘 ============

def save_user_json(sec_uid: str, user_data: dict, data_dir_base: Optional[str] = None) -> str:
    """
    把用户信息写到 scraper_data/{sec_uid}/data/{sec_uid}/user.json。
    与 douyin_scraper.py 写入的 user.json 字段对齐 (下游 VideoService 等可消费)。
    """
    if data_dir_base:
        base = Path(data_dir_base)
    else:
        base = get_data_dir() / "scraper_data"

    out_path = base / sec_uid / "data" / sec_uid / "user.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # 字段对齐 douyin_scraper.py:204-238
    # scraper 只写 sec_uid / nickname / avatar_url;我们补全更多
    payload = {
        "sec_uid": sec_uid,
        "uid": user_data.get("uid"),
        "short_id": user_data.get("short_id"),
        "unique_id": user_data.get("unique_id"),
        "nickname": user_data.get("nickname"),
        "signature": user_data.get("signature"),
        "avatar_url": user_data.get("avatar_url"),
        "avatar_larger": user_data.get("avatar_larger"),
        "avatar_medium": user_data.get("avatar_medium"),
        "avatar_thumb": user_data.get("avatar_thumb"),
        "follower_count": user_data.get("follower_count"),
        "following_count": user_data.get("following_count"),
        "aweme_count": user_data.get("aweme_count"),
        "favoriting_count": user_data.get("favoriting_count"),
        "total_favorited": user_data.get("total_favorited"),
        "gender": user_data.get("gender"),
        "location": user_data.get("location"),
        "school": user_data.get("school"),
        "enterprise_verify_reason": user_data.get("enterprise_verify_reason"),
        "custom_verify": user_data.get("custom_verify"),
        "verification_type": user_data.get("verification_type"),
        "is_verified": user_data.get("is_verified"),
        "share_url": user_data.get("share_url"),
        "queried_at": int(time.time()),
        "source_strategy": user_data.get("source_strategy"),
    }
    # 移除空字段,保持 JSON 干净
    payload = {k: v for k, v in payload.items() if v is not None and v != ""}

    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(payload, f, ensure_ascii=False, indent=2)

    _log(f"[GET_USER] user.json 已写入: {out_path}")
    return str(out_path)


# ============ 主流程 ============

def run(cookie_path: str, user_id_input: str, save: bool = True,
        headless: bool = False) -> dict:
    """
    主流程:
      1. 加载 cookie
      2. 启动 / 接管 Chrome (CDP)
      3. 解析 user_id 类型
      4. 必要时反查 sec_uid
      5. 访问用户主页,提取信息
      6. 写盘 + 返回 JSON
    """
    # 1. 加载 cookie
    _log(f"[GET_USER] 加载 cookie: {cookie_path}")
    cookies = load_cookie_data(cookie_path)
    _log(f"[GET_USER] cookie 数量: {len(cookies)}")

    # 2. 启动 / 接管 Chrome
    from DrissionPage import ChromiumPage, ChromiumOptions
    co = ChromiumOptions()

    # 跟 verify_account.py 一致的接管策略
    CDP_PORT = 9222
    page = None
    try:
        page = ChromiumPage(co)
        page = page.latest_tab
    except Exception as e:
        _log(f"[GET_USER] 启动浏览器失败: {e}")
        return {"status": "error", "error": f"启动浏览器失败: {e}"}

    try:
        # 3. 建立 douyin.com 上下文,注入 cookie
        _log("[GET_USER] Step 1: 访问 douyin.com 建立上下文")
        try:
            page.get("https://www.douyin.com/")
            time.sleep(2)
        except Exception as e:
            _log(f"[GET_USER] 访问首页失败 (非致命): {e}")

        # 注入 cookie
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

        # 4. 解析 ID 类型
        kind = classify_user_id(user_id_input)
        _log(f"[GET_USER] 输入 ID 分类: {kind} (raw={user_id_input[:60]})")

        resolved_meta = None  # 反查时拿到的元信息 (uid/unique_id/short_id)

        if kind == "url":
            url = user_id_input
            if "v.douyin.com" in url:
                url = expand_short_url(page, url)
            parsed = parse_share_url(url)
            sec_uid = parsed.get("sec_uid")
            if not sec_uid:
                _log(f"[GET_USER] URL 未解析出 sec_uid: {url}")
                return {"status": "error", "error": f"无法从 URL 解析 sec_uid: {url}"}
            # 把 URL 里的其他信息也带上
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
                    "error": f"无法把 {kind}={user_id_input} 转换为 sec_uid (搜索 API 失败或无结果)",
                }
        else:
            return {
                "status": "error",
                "error": f"无法识别 ID 格式: {user_id_input}",
            }

        _log(f"[GET_USER] 锁定 sec_uid: {sec_uid}")

        # 5. 访问用户主页
        home_url = build_user_home_url(sec_uid)
        _log(f"[GET_USER] Step 3: 访问主页 {home_url}")
        try:
            page.get(home_url)
            time.sleep(4)  # 等 SPA 加载
        except Exception as e:
            _log(f"[GET_USER] 访问主页失败: {e}")
            return {"status": "error", "error": f"访问主页失败: {e}", "sec_uid": sec_uid}

        # 6. 提取用户信息
        user_data = extract_user_from_page(page, attempts=4)

        # 把反查拿到的元信息合并
        if resolved_meta:
            for k, v in resolved_meta.items():
                if v and not user_data.get(k):
                    user_data[k] = v

        # 检查核心字段
        if not user_data.get("sec_uid"):
            user_data["sec_uid"] = sec_uid
        if not user_data.get("nickname"):
            _log("[GET_USER] 警告: 未提取到 nickname,可能 cookie 已失效或用户不存在")

        # 7. 写盘
        user_json_path = None
        if save and user_data.get("sec_uid"):
            try:
                user_json_path = save_user_json(sec_uid, user_data)
            except Exception as e:
                _log(f"[GET_USER] 写盘失败 (非致命): {e}")

        # 8. 返回结果
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


# ============ CLI ============

def main():
    parser = argparse.ArgumentParser(
        description="抖音用户信息查询 (支持 sec_uid / uid / short_id / unique_id / 分享链接 / 主页 URL)"
    )
    parser.add_argument("--cookie-path", required=True,
                        help="cookie 文件路径 (.json 优先, .txt 也支持)")
    parser.add_argument("--user-id", required=True,
                        help="用户 ID (sec_uid / uid / short_id / unique_id / 分享短链 / 主页 URL)")
    parser.add_argument("--no-save", action="store_true",
                        help="不写 user.json, 只打印到 stdout")
    args = parser.parse_args()

    result = run(
        cookie_path=args.cookie_path,
        user_id_input=args.user_id,
        save=not args.no_save,
    )

    # stdout 只输出 JSON
    print(json.dumps(result, ensure_ascii=False, indent=2))
    sys.exit(0 if result.get("status") in ("ok", "partial") else 1)


if __name__ == "__main__":
    main()
