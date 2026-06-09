import json
import time
from .utils import _log

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
        out.avatar_larger  = pick(user, 'avatar_larger', 'avatarLarger',
                                  'avatar_300x300', 'avatar_300X300');
        out.avatar_medium  = pick(user, 'avatar_medium', 'avatarMedium',
                                  'avatar_168x168', 'avatar_168X168');
        out.avatar_thumb   = pick(user, 'avatar_thumb',  'avatarThumb',
                                  'avatar_100x100', 'avatar_100X100');
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
        out.avatar_url = out.avatar_larger || out.avatar_medium || out.avatar_thumb;
        return true;
    }

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

    try {
        for (const vname of ['__INITIAL_STATE__', '__INITIAL_DATA__']) {
            const v = window[vname];
            if (v) {
                const s = typeof v === 'string' ? JSON.parse(v) : v;
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

    try {
        if (window.userData && normalize(window.userData)) {
            out.source_strategy = 'window.userData';
            return JSON.stringify(out);
        }
    } catch(e) {}

    try {
        const nickEl = document.querySelector('[data-e2e="user-info-name"]')
                    || document.querySelector('h1')
                    || document.querySelector('.header-right-name');
        if (nickEl) out.nickname = (nickEl.textContent || '').split('\n')[0].trim();

        const bodyText = document.body.innerText || '';
        const m = bodyText.match(/(?:抖音号|抖音ID|抖音id)[:：]?\s*([A-Za-z0-9_.-]+)/);
        if (m) out.unique_id = m[1];

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
    """
    last_result: dict = {}
    for i in range(attempts):
        try:
            raw = page.run_js(EXTRACT_JS)
            if raw is None:
                _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次: run_js 返回 None")
                time.sleep(1.5)
                continue
            if isinstance(raw, str):
                try:
                    data = json.loads(raw)
                except Exception:
                    _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次: JSON 解析失败")
                    time.sleep(1.5)
                    continue
            elif isinstance(raw, dict):
                data = raw
            else:
                time.sleep(1.5)
                continue

            if data.get("sec_uid") and data.get("nickname"):
                _log(f"[GET_USER] 提取成功 (策略: {data.get('source_strategy')}, 尝试 {i+1}/{attempts})")
                return data
            last_result = data
        except Exception as e:
            _log(f"[GET_USER] 提取第 {i+1}/{attempts} 次异常: {e}")
        time.sleep(1.5)

    _log(f"[GET_USER] 提取失败,返回最后一次结果")
    return last_result
