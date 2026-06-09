import json
from typing import Optional, Tuple
from .utils import _log

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
    """
    if kind == "sec_uid":
        return user_id, None

    if kind == "uid_or_short":
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
        for u in users:
            if kind == "uid_or_short" and str(u.get("uid") or "") == user_id:
                return u.get("sec_uid"), u
            if kind == "unique_id" and u.get("unique_id") == keyword.lstrip("@"):
                return u.get("sec_uid"), u
        first = users[0]
        _log(f"[GET_USER] 未精确匹配,使用第一个: {first.get('nickname')}")
        return first.get("sec_uid"), first
    except Exception as e:
        _log(f"[GET_USER] 反查异常: {e}")
        return None, None
