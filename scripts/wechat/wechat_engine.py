#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
微信 4.0+ 聊天读取引擎（完全独立，不依赖 WeFlow 的任何原生库）。

为什么自己实现：WeFlow 的 libwcdb_api 带反盗版保护（InitProtection / api.weflow.top
token / bundle id 校验），脱离其正版 App 加载会返回 -1006 拒绝工作。读取用户**自己**机器上
**自己**的微信库是正当需求，所以这里用公开的 SQLCipher-v4 参数自行解密，不碰那套锁库。

解密参数（已用 HMAC 实测确认，微信 4.0 mac）：
  PBKDF2-HMAC-SHA512 / 256000 轮 / AES-256-CBC / 页 4096 / reserve 80
  salt = 文件前 16 字节；mac_salt = salt 每字节 XOR 0x3a

数据布局（<accountDir>/db_storage/）：
  session/session.db   -> SessionTable（会话列表）
  message/message_N.db -> Msg_<md5(username)>（每会话一张消息表），Name2Id（发送者映射）
  contact/contact.db   -> contact（联系人：remark/nick_name/...）

协议：stdin 每行一个 JSON 请求 {id,type,...}，stdout 每行一个 JSON 响应
{id,result|error}；日志一律走 stderr。常驻进程，由 Rust commands/wechat.rs 拉起。

支持的 type：
  open / get_sessions / get_messages / get_new_messages / resolve_session /
  get_key / get_status
"""
import sys
import os
import re
import io
import json
import glob
import time
import wave
import base64
import hashlib
import sqlite3
import tempfile
import threading
import subprocess

try:
    from Crypto.Cipher import AES
except Exception as e:  # pragma: no cover
    sys.stderr.write("[wechat-engine] 缺少 pycryptodome: %s\n" % e)
    raise

PAGE = 4096
ITER = 256000
RESERVE = 80  # IV(16) + HMAC-SHA512(64)
SQLITE_HEADER = b"SQLite format 3\x00"


def log(*args):
    sys.stderr.write("[wechat-engine] " + " ".join(str(a) for a in args) + "\n")
    sys.stderr.flush()


def send(obj):
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()


# ---------------------------------------------------------------------------
# 消息类型 → 文本占位
# ---------------------------------------------------------------------------
TYPE_PLACEHOLDER = {
    3: "[图片]",
    34: "[语音]",
    42: "[名片]",
    43: "[视频]",
    47: "[表情]",
    48: "[位置]",
    49: "[链接/文件]",
    50: "[音视频通话]",
    10000: "[系统消息]",
    10002: "[系统消息]",
}


# 微信 4.0 图片加密 .dat 的 V2 头（magic）。
V2_MAGIC = bytes([0x07, 0x08, 0x56, 0x32, 0x08, 0x07])


def _img_magic_kind(b: bytes):
    if b[:3] == b"\xff\xd8\xff":
        return "image/jpeg"
    if b[:4] == b"\x89PNG":
        return "image/png"
    if b[:3] == b"GIF":
        return "image/gif"
    if b[:4] == b"RIFF":
        return "image/webp"
    if b[:4] == b"wxgf":
        return "wxgf"
    return None


def clean_own_wxid(account_basename: str) -> str:
    """从账号目录名推断自己的 wxid（wxid_xxx_yyyy -> wxid_xxx）。"""
    name = account_basename.strip()
    m = re.match(r"^(wxid_[^_]+)", name, re.I)
    if m:
        return m.group(1)
    m = re.match(r"^(.+)_([a-zA-Z0-9]{4})$", name)
    return m.group(1) if m else name


class Decryptor:
    """按 mtime 缓存的逐页解密器：把加密 db 解成明文 SQLite 临时文件后复用。"""

    def __init__(self, key_hex: str, cache_dir: str):
        self.key = bytes.fromhex(key_hex)
        self.cache_dir = cache_dir
        os.makedirs(cache_dir, exist_ok=True)
        # src_path -> (mtime, size, plain_path)
        self._cache = {}

    def verify(self, src: str) -> bool:
        """用第 1 页 HMAC 校验 key 是否正确。"""
        with open(src, "rb") as f:
            page = f.read(PAGE)
        if len(page) < PAGE:
            return False
        salt = page[:16]
        enc_key = hashlib.pbkdf2_hmac("sha512", self.key, salt, ITER, 32)
        mac_salt = bytes(b ^ 0x3A for b in salt)
        mac_key = hashlib.pbkdf2_hmac("sha512", enc_key, mac_salt, 2, 32)
        end = PAGE - RESERVE
        body_iv = page[16:end + 16]
        stored = page[end + 16:end + 16 + 64]
        import hmac as _hmac
        calc = _hmac.new(mac_key, body_iv + (1).to_bytes(4, "little"), hashlib.sha512).digest()
        return calc == stored

    def decrypt(self, src: str) -> str:
        """返回 src 对应的明文 SQLite 文件路径（带 mtime 缓存）。"""
        st = os.stat(src)
        cached = self._cache.get(src)
        if cached and cached[0] == st.st_mtime and cached[1] == st.st_size and os.path.exists(cached[2]):
            return cached[2]

        with open(src, "rb") as f:
            data = f.read()
        salt = data[:16]
        enc_key = hashlib.pbkdf2_hmac("sha512", self.key, salt, ITER, 32)
        out = bytearray()
        n = len(data) // PAGE
        for i in range(n):
            pg = data[i * PAGE:(i + 1) * PAGE]
            off = 16 if i == 0 else 0
            iv = pg[PAGE - RESERVE:PAGE - RESERVE + 16]
            try:
                dec = AES.new(enc_key, AES.MODE_CBC, iv).decrypt(pg[off:PAGE - RESERVE])
            except Exception:
                dec = b"\x00" * (PAGE - RESERVE - off)
            out += (SQLITE_HEADER + dec if i == 0 else dec)
            out += pg[PAGE - RESERVE:]  # reserve 尾部原样保留

        digest = hashlib.md5(src.encode("utf-8")).hexdigest()
        plain = os.path.join(self.cache_dir, digest + ".db")
        with open(plain, "wb") as f:
            f.write(out)
        self._cache[src] = (st.st_mtime, st.st_size, plain)
        return plain

    def open_ro(self, src: str) -> sqlite3.Connection:
        plain = self.decrypt(src)
        con = sqlite3.connect(plain)
        con.text_factory = lambda b: b.decode("utf-8", "replace") if isinstance(b, bytes) else b
        return con


class Engine:
    def __init__(self):
        self.account_dir = None
        self.key_hex = None
        self.own_wxid = None
        self.dec = None
        self.cache_dir = os.path.join(
            os.environ.get("WECHAT_USER_DATA", os.path.join(tempfile.gettempdir(), "autocast_wechat")),
            "plain",
        )
        # username -> (message_db_path, table_name)
        self._msg_loc = {}
        self._contact_cache = None  # (mtime, dict)
        self._lock = threading.Lock()

    # ---- 路径 ----
    @property
    def storage(self):
        return os.path.join(self.account_dir, "db_storage")

    def session_db(self):
        return os.path.join(self.storage, "session", "session.db")

    def contact_db(self):
        return os.path.join(self.storage, "contact", "contact.db")

    def message_dbs(self):
        return sorted(glob.glob(os.path.join(self.storage, "message", "message_*.db")))

    # ---- 连接 ----
    def open(self, account_dir, hex_key):
        self.account_dir = account_dir.rstrip("/\\")
        self.key_hex = hex_key.strip()
        self.own_wxid = clean_own_wxid(os.path.basename(self.account_dir))
        self.dec = Decryptor(self.key_hex, self.cache_dir)
        self._msg_loc.clear()
        self._contact_cache = None
        sdb = self.session_db()
        if not os.path.exists(sdb):
            return {"success": False, "error": "未找到 session.db: " + sdb}
        if not self.dec.verify(sdb):
            return {"success": False, "error": "密钥校验失败（HMAC 不匹配），密钥或账号目录不正确"}
        return {"success": True, "ownWxid": self.own_wxid}

    # ---- 联系人显示名 ----
    def contact_map(self):
        cdb = self.contact_db()
        st = os.stat(cdb)
        if self._contact_cache and self._contact_cache[0] == st.st_mtime:
            return self._contact_cache[1]
        m = {}
        con = self.dec.open_ro(cdb)
        try:
            for username, remark, nick in con.execute(
                "SELECT username, remark, nick_name FROM contact"
            ):
                name = (remark or "").strip() or (nick or "").strip() or username
                m[username] = name
        finally:
            con.close()
        self._contact_cache = (st.st_mtime, m)
        return m

    def display_name(self, username, cmap=None):
        cmap = cmap if cmap is not None else self.contact_map()
        return cmap.get(username, username)

    # ---- 会话列表 ----
    def get_sessions(self):
        con = self.dec.open_ro(self.session_db())
        cmap = self.contact_map()
        out = []
        try:
            rows = con.execute(
                "SELECT username, summary, last_timestamp, sort_timestamp, unread_count, "
                "last_sender_display_name FROM SessionTable ORDER BY sort_timestamp DESC"
            ).fetchall()
        finally:
            con.close()
        for username, summary, last_ts, sort_ts, unread, last_sender in rows:
            if not username:
                continue
            is_group = username.endswith("@chatroom")
            out.append({
                "username": username,
                "displayName": self.display_name(username, cmap),
                "summary": summary or "",
                "lastTimestamp": last_ts or sort_ts or 0,
                "unreadCount": unread or 0,
                "isGroup": is_group,
            })
        return {"sessions": out}

    # ---- 通讯录（好友 + 群聊） ----
    def get_contacts(self):
        """从 contact.db 读好友与群聊（不是最近会话）。
        分类依据（已实测）：local_type=1 且非 gh_/非群 = 好友；username 以 @chatroom 结尾 = 群聊；
        local_type=3 是群成员/陌生人，排除；gh_ 公众号排除。"""
        con = self.dec.open_ro(self.contact_db())
        friends, groups = [], []
        try:
            rows = con.execute(
                "SELECT username, local_type, remark, nick_name, alias FROM contact"
            ).fetchall()
        finally:
            con.close()
        for username, ltype, remark, nick, alias in rows:
            if not username:
                continue
            if username.endswith("@chatroom"):
                name = (nick or "").strip() or "群聊"
                groups.append({
                    "username": username, "displayName": name,
                    "isGroup": True, "category": "group",
                })
            elif ltype == 1 and not username.startswith("gh_"):
                name = ((remark or "").strip() or (nick or "").strip()
                        or (alias or "").strip() or username)
                friends.append({
                    "username": username, "displayName": name,
                    "isGroup": False, "category": "friend",
                })
        friends.sort(key=lambda c: c["displayName"])
        groups.sort(key=lambda c: c["displayName"])
        return {
            "contacts": friends + groups,
            "friendCount": len(friends),
            "groupCount": len(groups),
        }

    # ---- 定位某会话的消息表 ----
    def locate_message_table(self, username):
        if username in self._msg_loc:
            loc = self._msg_loc[username]
            if loc and os.path.exists(loc[0]):
                return loc
        table = "Msg_" + hashlib.md5(username.encode("utf-8")).hexdigest()
        for mdb in self.message_dbs():
            con = self.dec.open_ro(mdb)
            try:
                hit = con.execute(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (table,)
                ).fetchone()
            finally:
                con.close()
            if hit:
                self._msg_loc[username] = (mdb, table)
                return mdb, table
        self._msg_loc[username] = None
        return None

    def _name2id(self, con):
        return {rid: uname for rid, uname in con.execute("SELECT rowid, user_name FROM Name2Id")}

    @staticmethod
    def _content_text(content):
        """message_content 可能是明文 str，也可能是 WCDB zstd 压缩的 BLOB（魔数 28 b5 2f fd）。"""
        if isinstance(content, str):
            return content
        if isinstance(content, (bytes, bytearray)):
            b = bytes(content)
            if b[:4] == b"\x28\xb5\x2f\xfd":  # zstd magic
                try:
                    import zstandard as zstd
                    b = zstd.ZstdDecompressor().decompress(b, max_output_size=16 << 20)
                except Exception:
                    pass
            return b.decode("utf-8", "replace")
        return ""

    def _row_to_msg(self, row, n2i, is_group, cmap):
        local_id, server_id, ltype, sort_seq, sender_id, create_time, content = row
        sender = n2i.get(sender_id, "")
        text = self._content_text(content)
        # 群聊文本前缀 "wxid:\n正文"
        if is_group and ltype == 1 and text:
            idx = text.find(":\n")
            if 0 < idx < 64:
                sender = sender or text[:idx]
                text = text[idx + 2:]
        if ltype != 1:
            text = TYPE_PLACEHOLDER.get(ltype, "[消息]")
        return {
            "localId": local_id,
            "svrId": server_id,
            "localType": ltype,
            "createTime": create_time,
            "senderUsername": sender,
            "senderName": self.display_name(sender, cmap) if sender else "",
            "content": text,
            "isSender": 1 if sender and sender == self.own_wxid else 0,
        }

    def get_messages(self, username, limit=100, offset=0):
        loc = self.locate_message_table(username)
        if not loc:
            return {"messages": []}
        mdb, table = loc
        is_group = username.endswith("@chatroom")
        cmap = self.contact_map()
        con = self.dec.open_ro(mdb)
        try:
            n2i = self._name2id(con)
            rows = con.execute(
                f"SELECT local_id, server_id, local_type, sort_seq, real_sender_id, "
                f"create_time, message_content FROM '{table}' "
                f"ORDER BY sort_seq DESC LIMIT ? OFFSET ?", (limit, offset)
            ).fetchall()
        finally:
            con.close()
        msgs = [self._row_to_msg(r, n2i, is_group, cmap) for r in rows]
        return {"messages": msgs}

    def get_new_messages(self, username, min_time, limit=50):
        loc = self.locate_message_table(username)
        if not loc:
            return {"messages": []}
        mdb, table = loc
        is_group = username.endswith("@chatroom")
        cmap = self.contact_map()
        con = self.dec.open_ro(mdb)
        try:
            n2i = self._name2id(con)
            rows = con.execute(
                f"SELECT local_id, server_id, local_type, sort_seq, real_sender_id, "
                f"create_time, message_content FROM '{table}' "
                f"WHERE create_time > ? ORDER BY create_time ASC LIMIT ?", (min_time, limit)
            ).fetchall()
        finally:
            con.close()
        msgs = [self._row_to_msg(r, n2i, is_group, cmap) for r in rows]
        return {"messages": msgs}

    # ---- 语音（SILK_V3 → WAV）----
    def media_dbs(self):
        return sorted(glob.glob(os.path.join(self.storage, "message", "media_*.db")))

    @staticmethod
    def _silk_to_wav(silk: bytes) -> bytes:
        import pilk
        # 微信 voice_data 常带 1 字节前缀（\x02），定位到 #!SILK_V3 头
        idx = silk.find(b"#!SILK_V3")
        if idx > 0:
            silk = silk[idx:]
        sp = tempfile.mktemp(suffix=".silk")
        pcmp = tempfile.mktemp(suffix=".pcm")
        try:
            with open(sp, "wb") as f:
                f.write(silk)
            pilk.decode(sp, pcmp, pcm_rate=24000)
            with open(pcmp, "rb") as f:
                pcm = f.read()
        finally:
            for x in (sp, pcmp):
                try:
                    os.remove(x)
                except OSError:
                    pass
        buf = io.BytesIO()
        w = wave.open(buf, "wb")
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(24000)
        w.writeframes(pcm)
        w.close()
        return buf.getvalue()

    def get_voice(self, session_username, svr_id=0, local_id=0, create_time=0):
        """从 media_*.db 的 VoiceInfo 取 SILK 语音并解码成 base64 WAV。
        关联：chat_name_id=Name2Id(session)，再用 svr_id（优先）或 create_time 命中具体消息。"""
        for mdb in self.media_dbs():
            con = self.dec.open_ro(mdb)
            try:
                if not con.execute(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='VoiceInfo'"
                ).fetchone():
                    continue
                n2i = {u: r for r, u in con.execute("SELECT rowid, user_name FROM Name2Id")}
                cid = n2i.get(session_username)
                if cid is None:
                    continue
                row = None
                if svr_id:
                    row = con.execute(
                        "SELECT voice_data FROM VoiceInfo WHERE chat_name_id=? AND svr_id=?",
                        (cid, svr_id)).fetchone()
                if not row and create_time:
                    row = con.execute(
                        "SELECT voice_data FROM VoiceInfo WHERE chat_name_id=? AND create_time=?",
                        (cid, create_time)).fetchone()
                if not row and local_id:
                    row = con.execute(
                        "SELECT voice_data FROM VoiceInfo WHERE chat_name_id=? AND local_id=?",
                        (cid, local_id)).fetchone()
                if row and row[0]:
                    wav = self._silk_to_wav(bytes(row[0]))
                    return {"ok": True, "mime": "audio/wav",
                            "base64": base64.b64encode(wav).decode("ascii")}
            finally:
                con.close()
        return {"ok": False, "error": "未找到语音数据"}

    # ---- 媒体（视频缩略图 = 明文 jpg；图片原图留待 phase 2）----
    @staticmethod
    def _md5_from_packed(packed: bytes):
        m = re.search(rb"[0-9a-f]{32}", packed or b"")
        return m.group(0).decode("ascii") if m else None

    def get_media(self, session_username, local_id, local_type, svr_id=0, create_time=0):
        loc = self.locate_message_table(session_username)
        if not loc:
            return {"ok": False, "error": "未定位到消息表"}
        mdb, table = loc
        con = self.dec.open_ro(mdb)
        try:
            row = con.execute(
                f"SELECT packed_info_data FROM '{table}' WHERE local_id=?", (local_id,)
            ).fetchone()
        finally:
            con.close()
        md5 = self._md5_from_packed(bytes(row[0])) if (row and row[0]) else None
        if not md5:
            return {"ok": False, "error": "无媒体 md5"}
        if local_type == 43:  # 视频缩略图：磁盘明文 jpg
            hits = glob.glob(
                os.path.join(self.account_dir, "msg", "video", "**", md5 + "_thumb.jpg"),
                recursive=True)
            if hits:
                with open(hits[0], "rb") as f:
                    data = f.read()
                return {"ok": True, "mime": "image/jpeg",
                        "base64": base64.b64encode(data).decode("ascii")}
            return {"ok": False, "error": "未找到视频缩略图"}
        if local_type == 3:  # 图片：默认返回缩略图（内联）
            return self.get_image(session_username, local_id, want_full=False)
        return {"ok": False, "error": "该媒体类型暂未支持"}

    # ---- 图片：磁盘推导 image key（XOR+AES，无需内存扫描）----
    def _kvcomm_dirs(self):
        home = os.path.expanduser("~")
        dirs = []
        if sys.platform == "darwin":
            dirs.append(os.path.join(home, "Library", "Containers", "com.tencent.xinWeChat",
                                     "Data", "Documents", "app_data", "net", "kvcomm"))
        # 由账号目录推导：.../xwechat_files -> .../app_data/net/kvcomm
        norm = (self.account_dir or "").replace("\\", "/").rstrip("/")
        idx = norm.find("/xwechat_files")
        if idx >= 0:
            dirs.append(norm[:idx] + "/app_data/net/kvcomm")
        return [d for d in dirs if os.path.isdir(d)]

    def _collect_codes(self):
        codes = set()
        for d in self._kvcomm_dirs():
            try:
                for fn in os.listdir(d):
                    m = re.match(r"^key_(\d+)_", fn)
                    if m:
                        codes.add(int(m.group(1)))
            except OSError:
                pass
        return codes

    def image_keys(self):
        """返回 (aes_key_bytes, xor_key_int) 或 None。结果缓存。
        算法（移植自 WeFlow，全程磁盘）：kvcomm 文件名取 code；_t.dat 尾部反推 xorKey；
        aesKey=md5(str(code)+wxid)[:16]，用模板密文 AES 验证图片 magic。"""
        if hasattr(self, "_img_keys_cache"):
            return self._img_keys_cache

        result = None
        try:
            # 模板：收集 _t.dat（V2），从尾部 2 字节反推 xorKey，并取 [0xF:0x1F] 作 AES 模板
            tdats = glob.glob(os.path.join(self.account_dir, "msg", "**", "*_t.dat"), recursive=True)
            tdats.sort(key=lambda f: -(os.path.getmtime(f) if os.path.exists(f) else 0))
            tail_counts = {}
            template = None
            for f in tdats[:48]:
                try:
                    d = open(f, "rb").read()
                except OSError:
                    continue
                if d[:6] == V2_MAGIC and len(d) >= 0x1F:
                    tail_counts[(d[-2], d[-1])] = tail_counts.get((d[-2], d[-1]), 0) + 1
                    if template is None:
                        template = d[0xF:0x1F]
            xor_key = None
            best = 0
            for (x, y), c in tail_counts.items():
                if c > best:
                    k = x ^ 0xFF
                    if k == (y ^ 0xD9):  # 明文尾 FF D9（JPEG EOI）
                        best, xor_key = c, k

            if template is not None and xor_key is not None:
                codes = self._collect_codes()
                wxids = [self.own_wxid, os.path.basename(self.account_dir or "")]
                for wxid in wxids:
                    for code in codes:
                        ak = hashlib.md5((str(code) + wxid).encode()).hexdigest()[:16].encode()
                        dec = AES.new(ak, AES.MODE_ECB).decrypt(template)
                        if _img_magic_kind(dec):
                            result = (ak, xor_key)
                            break
                    if result:
                        break
        except Exception as e:
            log("image_keys error:", e)

        self._img_keys_cache = result
        return result

    def _decrypt_dat(self, path):
        keys = self.image_keys()
        d = open(path, "rb").read()
        if d[:6] != V2_MAGIC:
            # 非 V2：可能已是明文图片
            return d if _img_magic_kind(d) else None
        if not keys:
            return None
        aes_key, xor_key = keys
        import struct
        aes_size = struct.unpack("<I", d[6:10])[0]
        payload = d[0xF:]
        # AES 段必须按 16 字节块对齐：header 的 aes_size 不一定是 16 的倍数，且高清原图常被
        # 微信懒下载只下了一半（payload 截断）。两种情况都要把 aes_size 下取整到块边界，
        # 否则 AES-ECB 会报 "Data must be aligned" 或解密错位导致花屏。
        aes_size = min(aes_size, len(payload))
        aes_size -= aes_size % 16
        head = AES.new(aes_key, AES.MODE_ECB).decrypt(payload[:aes_size]) if aes_size else b""
        tail = bytes(b ^ xor_key for b in payload[aes_size:])
        return head + tail

    @staticmethod
    def _extract_nalus(buf):
        """提取 annexb HEVC NALU（兼容 3/4 字节起始码，丢弃 forbidden_zero_bit 异常单元）。"""
        starts = []
        i, n = 4, len(buf)
        while i < n - 3:
            if buf[i] == 0 and buf[i + 1] == 0 and buf[i + 2] == 0 and buf[i + 3] == 1:
                starts.append((i, 4)); i += 4; continue
            if buf[i] == 0 and buf[i + 1] == 0 and buf[i + 2] == 1:
                starts.append((i, 3)); i += 3; continue
            i += 1
        units = []
        for idx, (s, pl) in enumerate(starts):
            e = starts[idx + 1][0] if idx + 1 < len(starts) else n
            pay = buf[s + pl:e]
            if len(pay) >= 2 and (pay[0] & 0x80) == 0:
                units.append(pay)
        return units

    def _ffmpeg_hevc_to_jpg(self, ffmpeg, units):
        """把一组 NALU 重建为 annexb 流并用 ffmpeg 取第一帧 jpg。多帧偏移做兜底尝试。"""
        if not units:
            return None
        annexb = b"".join(b"\x00\x00\x00\x01" + u for u in units)
        inp = tempfile.mktemp(suffix=".hevc")
        out = tempfile.mktemp(suffix=".jpg")
        try:
            with open(inp, "wb") as f:
                f.write(annexb)
            for extra in ([], ["-vf", "select=eq(n\\,1)"], ["-vf", "select=eq(n\\,5)"]):
                try:
                    if os.path.exists(out):
                        os.remove(out)
                except OSError:
                    pass
                cmd = [ffmpeg, "-y", "-loglevel", "error", "-f", "hevc", "-i", inp]
                cmd += extra + ["-frames:v", "1", out]
                try:
                    r = subprocess.run(cmd, capture_output=True, timeout=30)
                except Exception:
                    continue
                if r.returncode == 0 and os.path.exists(out) and os.path.getsize(out) > 1000:
                    with open(out, "rb") as f:
                        return f.read()
            return None
        finally:
            for x in (inp, out):
                try:
                    os.remove(x)
                except OSError:
                    pass

    def _wxgf_to_jpg(self, buf):
        """wxgf 容器（内含 HEVC）→ jpg。先找内嵌 jpeg/png，否则提取 NALU、按 VPS 分组成多个
        候选流分别交 ffmpeg（兼容实况/多帧），取第一个成功的。"""
        # 内嵌传统图片
        for i in range(4, min(len(buf) - 12, 4096)):
            if buf[i] == 0xFF and buf[i + 1] == 0xD8 and buf[i + 2] == 0xFF:
                return "image/jpeg", buf[i:]
            if buf[i] == 0x89 and buf[i + 1] == 0x50 and buf[i + 2] == 0x4E and buf[i + 3] == 0x47:
                return "image/png", buf[i:]
        ffmpeg = os.environ.get("WECHAT_FFMPEG")
        if not ffmpeg or not os.path.exists(ffmpeg):
            return None
        units = self._extract_nalus(buf)
        if not units:
            return None

        # 候选 1：整条流
        candidates = [units]
        # 候选 2..：按 VPS(type=32) 分组，取每个含 VCL 帧的组
        vps_idx = [i for i, u in enumerate(units) if ((u[0] >> 1) & 0x3F) == 32]
        for gi in range(len(vps_idx)):
            start = vps_idx[gi]
            end = vps_idx[gi + 1] if gi + 1 < len(vps_idx) else len(units)
            group = units[start:end]
            if any(((u[0] >> 1) & 0x3F) in (19, 20, 21, 1, 0) for u in group):
                candidates.append(group)

        for cand in candidates:
            jpg = self._ffmpeg_hevc_to_jpg(ffmpeg, cand)
            if jpg:
                return "image/jpeg", jpg
        return None

    @staticmethod
    def _image_complete(mime, data):
        """校验解码后的图片是否完整（关键：微信高清原图常懒下载，磁盘 .dat 可能只下了一半，
        解出来是截断 JPEG → 顶部正常其余花屏）。"""
        if mime == "image/jpeg":
            return data[:3] == b"\xff\xd8\xff" and data.rstrip(b"\x00")[-2:] == b"\xff\xd9"
        if mime == "image/png":
            return data[:8] == b"\x89PNG\r\n\x1a\n" and b"IEND" in data[-16:]
        return len(data) > 64  # gif/webp 等：长度兜底

    def _dat_to_image(self, path):
        """解密单个 .dat 并转成 (mime, bytes)；不完整/无法解码返回 None（交由上层回退候选）。"""
        try:
            dec = self._decrypt_dat(path)
        except Exception as e:
            log("decrypt dat error:", path, e)
            return None
        if not dec:
            return None
        kind = _img_magic_kind(dec)
        if kind == "wxgf":
            return self._wxgf_to_jpg(dec)  # ffmpeg 失败(含截断 HEVC)会返回 None
        if kind:
            return (kind, dec) if self._image_complete(kind, dec) else None
        return None

    def _find_dats(self, md5, want_full):
        """返回候选 .dat 路径（按优先级）。大图优先高清/原图，再回退缩略图；缩略图模式只用 _t.dat。"""
        base = os.path.join(self.account_dir, "msg")

        def find(suffix):
            hits = glob.glob(os.path.join(base, "**", md5 + suffix), recursive=True)
            if suffix == ".dat":
                hits = [h for h in hits if not h.endswith("_t.dat") and not h.endswith("_h.dat")]
            return hits

        thumb = find("_t.dat")
        if not want_full:
            return thumb
        ordered = []
        for suffix in (".dat", "_h.dat"):
            ordered += find(suffix)
        ordered += thumb  # 高清没下全时回退缩略图（完整、低清但不花屏）
        return ordered

    def get_image(self, session_username, local_id, want_full=False):
        if not self.image_keys():
            return {"ok": False, "error": "未能推导图片密钥（kvcomm code 缺失或微信未产生过图片）"}
        loc = self.locate_message_table(session_username)
        if not loc:
            return {"ok": False, "error": "未定位到消息表"}
        mdb, table = loc
        con = self.dec.open_ro(mdb)
        try:
            row = con.execute(
                f"SELECT packed_info_data FROM '{table}' WHERE local_id=?", (local_id,)
            ).fetchone()
        finally:
            con.close()
        md5 = self._md5_from_packed(bytes(row[0])) if (row and row[0]) else None
        if not md5:
            return {"ok": False, "error": "无图片 md5"}
        # 依次尝试候选，返回第一个**完整**解码的图片
        for path in self._find_dats(md5, want_full):
            img = self._dat_to_image(path)
            if img:
                mime, data = img
                return {"ok": True, "mime": mime,
                        "base64": base64.b64encode(data).decode("ascii"),
                        "fallbackThumb": want_full and path.endswith("_t.dat")}
        return {"ok": False, "error": "图片未下全或解码失败（原图可能尚未下载，请在微信里打开一次）"}

    def get_video_path(self, session_username, local_id):
        loc = self.locate_message_table(session_username)
        if not loc:
            return {"ok": False, "error": "未定位到消息表"}
        mdb, table = loc
        con = self.dec.open_ro(mdb)
        try:
            row = con.execute(
                f"SELECT packed_info_data FROM '{table}' WHERE local_id=?", (local_id,)
            ).fetchone()
        finally:
            con.close()
        md5 = self._md5_from_packed(bytes(row[0])) if (row and row[0]) else None
        if not md5:
            return {"ok": False, "error": "无视频 md5"}
        # 原视频可能在 video 下，也可能其它命名；放宽到整个 msg 目录搜 <md5>*.mp4
        for pat in (os.path.join(self.account_dir, "msg", "video", "**", md5 + ".mp4"),
                    os.path.join(self.account_dir, "msg", "**", md5 + "*.mp4")):
            hits = glob.glob(pat, recursive=True)
            if hits:
                return {"ok": True, "path": hits[0]}
        return {"ok": False, "error": "原视频尚未下载到本地，请先在微信里播放一次该视频后再试"}

    def resolve_session(self, keyword):
        kw = (keyword or "").strip()
        for s in self.get_sessions()["sessions"]:
            if kw and kw in (s["displayName"] or ""):
                return {"found": True, "sessionId": s["username"], "displayName": s["displayName"]}
        return {"found": False}


# ---------------------------------------------------------------------------
# 账号目录自动发现
# ---------------------------------------------------------------------------
def discover_accounts():
    home = os.path.expanduser("~")
    roots = []
    if sys.platform == "darwin":
        roots.append(os.path.join(home, "Library", "Containers", "com.tencent.xinWeChat",
                                  "Data", "Documents", "xwechat_files"))
        roots.append(os.path.join(home, "Documents", "xwechat_files"))
    elif sys.platform == "win32":
        roots.append(os.path.join(home, "Documents", "xwechat_files"))
        if os.environ.get("USERPROFILE"):
            roots.append(os.path.join(os.environ["USERPROFILE"], "Documents", "xwechat_files"))
    else:
        roots.append(os.path.join(home, "Documents", "xwechat_files"))

    seen, accounts = set(), []
    for root in roots:
        if root in seen or not os.path.isdir(root):
            continue
        seen.add(root)
        for entry in os.listdir(root):
            full = os.path.join(root, entry)
            if not os.path.isdir(full):
                continue
            has_db = os.path.isdir(os.path.join(full, "db_storage"))
            if not has_db:
                continue
            has_session = (
                os.path.exists(os.path.join(full, "db_storage", "session", "session.db"))
                or os.path.exists(os.path.join(full, "db_storage", "session.db"))
            )
            try:
                mtime = os.path.getmtime(full)
            except OSError:
                mtime = 0
            accounts.append({"accountDir": full, "wxid": entry, "hasSession": has_session, "mtime": mtime})
    accounts.sort(key=lambda a: (not a["hasSession"], -a["mtime"]))
    return accounts


# ---------------------------------------------------------------------------
# 密钥自动提取
# ---------------------------------------------------------------------------
def _resources_root():
    return os.environ.get("WCDB_RESOURCES_PATH") or os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        "src-tauri", "resources", "wechat")


def get_key_mac(timeout_ms=180000):
    root = _resources_root()
    base = os.path.join(root, "key", "macos", "universal")
    helper = os.path.join(base, "xkey_helper")
    if not os.path.exists(helper):
        return {"success": False, "error": "xkey_helper 未找到: " + helper}
    # SIP 检查
    try:
        sip = subprocess.run(["/usr/bin/csrutil", "status"], capture_output=True, text=True, timeout=10)
        if "enabled" in sip.stdout.lower():
            return {"success": False, "error": "SIP（系统完整性保护）已开启，需进入恢复模式执行 csrutil disable 后重试。"}
    except Exception:
        pass
    # 找微信 pid
    pid = 0
    for args in (["/usr/bin/pgrep", "-x", "WeChat"],
                 ["/usr/bin/pgrep", "-f", "WeChat.app/Contents/MacOS/WeChat"]):
        try:
            r = subprocess.run(args, capture_output=True, text=True, timeout=10)
            ids = [int(x) for x in r.stdout.split() if x.strip().isdigit()]
            if ids:
                pid = max(ids)
                break
        except Exception:
            pass
    if not pid:
        return {"success": False, "error": "未找到微信进程，请先启动并登录微信（4.0+）"}

    wait_ms = max(timeout_ms, 30000)
    timeout_sec = wait_ms // 1000 + 30
    artifacts = [os.path.join(base, n) for n in
                 ("xkey_helper", "image_scan_helper", "xkey_helper_macos", "libwx_key.dylib")
                 if os.path.exists(os.path.join(base, n))]

    def sq(p):
        return "'" + p.replace("'", "'\\''") + "'"

    chmod = ("/bin/chmod +x " + " ".join(sq(a) for a in artifacts)) if artifacts else ""
    run = "%s %d %d" % (sq(helper), pid, wait_ms)
    priv = (chmod + " && " + run) if chmod else run
    script_lines = [
        "set cmd to " + json.dumps(priv),
        "set timeoutSec to %d" % timeout_sec,
        "try",
        "with timeout of timeoutSec seconds",
        'set outText to do shell script (cmd & " 2>&1") with administrator privileges',
        "end timeout",
        'return "WF_OK::" & outText',
        "on error errMsg number errNum",
        'return "WF_ERR::" & errNum & "::" & errMsg',
        "end try",
    ]
    cmd = ["/usr/bin/osascript"]
    for l in script_lines:
        cmd += ["-e", l]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=wait_ms / 1000 + 20)
        out = r.stdout
    except subprocess.TimeoutExpired:
        return {"success": False, "error": "提取超时，请保持微信前台并操作一次会话后重试。"}
    joined = "\n".join(l.strip() for l in out.splitlines() if l.strip())
    if joined.startswith("WF_ERR::"):
        parts = joined.split("::")
        if parts[1] == "-128" or "User canceled" in (parts[2] if len(parts) > 2 else ""):
            return {"success": False, "error": "已取消管理员授权"}
        return {"success": False, "error": "helper 执行失败：" + (parts[2] if len(parts) > 2 else "unknown")}
    body = joined[len("WF_OK::"):] if joined.startswith("WF_OK::") else joined
    # 解析 JSON {success,key} 或裸 64 位 hex
    key = None
    for m in re.finditer(r"\{[^{}]*\}", body):
        try:
            obj = json.loads(m.group(0))
        except Exception:
            continue
        if obj.get("success") and isinstance(obj.get("key"), str):
            key = obj["key"]
            break
        if isinstance(obj.get("result"), str):
            key = obj["result"]
    if not key:
        m = re.search(r"\b[0-9a-fA-F]{64}\b", body)
        key = m.group(0) if m else None
    if key and len(key) == 64:
        return {"success": True, "key": key}
    return {"success": False, "error": "未能从 helper 输出解析出密钥。"}


def get_key_win(timeout_ms=120000):
    import ctypes
    root = _resources_root()
    arch = "arm64" if "arm" in (os.environ.get("PROCESSOR_ARCHITECTURE", "").lower()) else "x64"
    for cand in (os.path.join(root, "key", "win32", arch, "wx_key.dll"),
                 os.path.join(root, "key", "win32", "x64", "wx_key.dll")):
        if os.path.exists(cand):
            dll_path = cand
            break
    else:
        return {"success": False, "error": "wx_key.dll 未找到"}
    # 找微信 pid
    pid = 0
    for name in ("Weixin.exe", "WeChat.exe"):
        try:
            r = subprocess.run(["tasklist", "/FI", "IMAGENAME eq " + name, "/FO", "CSV", "/NH"],
                               capture_output=True, text=True, timeout=10)
            for line in r.stdout.splitlines():
                m = re.match(r'^"[^"]+","(\d+)"', line)
                if m:
                    pid = max(pid, int(m.group(1)))
        except Exception:
            pass
    if not pid:
        return {"success": False, "error": "未找到微信进程，请先启动并登录微信（4.0+）"}
    try:
        lib = ctypes.WinDLL(dll_path)
        lib.InitializeHook.restype = ctypes.c_bool
        lib.InitializeHook.argtypes = [ctypes.c_uint32]
        lib.PollKeyData.restype = ctypes.c_bool
        lib.PollKeyData.argtypes = [ctypes.c_char_p, ctypes.c_int]
        lib.CleanupHook.restype = ctypes.c_bool
        if not lib.InitializeHook(pid):
            return {"success": False, "error": "InitializeHook 失败：请以管理员身份运行并关闭安全软件拦截。"}
        try:
            deadline = time.time() + max(timeout_ms, 30000) / 1000
            buf = ctypes.create_string_buffer(128)
            while time.time() < deadline:
                if lib.PollKeyData(buf, 128):
                    key = buf.value.decode("utf-8", "ignore").strip()
                    if len(key) == 64:
                        return {"success": True, "key": key}
                time.sleep(0.12)
            return {"success": False, "error": "获取密钥超时。"}
        finally:
            lib.CleanupHook()
    except Exception as e:
        return {"success": False, "error": "加载 wx_key.dll 失败：%s" % e}


def get_key(timeout_ms=180000):
    account = discover_accounts()
    best = account[0] if account else None
    if sys.platform == "darwin":
        res = get_key_mac(timeout_ms)
    elif sys.platform == "win32":
        res = get_key_win(timeout_ms)
    else:
        res = {"success": False, "error": "当前平台暂不支持自动提取密钥，请手动填写。"}
    res["accountDir"] = best["accountDir"] if best else None
    res["wxid"] = best["wxid"] if best else None
    if res.get("success"):
        res["hexKey"] = res.pop("key", None)
    return res


# ---------------------------------------------------------------------------
# 主循环
# ---------------------------------------------------------------------------
def main():
    engine = Engine()
    log("ready; resources=" + _resources_root())
    send({"id": 0, "result": {"ready": True}})

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception as e:
            log("invalid json:", str(e))
            continue
        rid = req.get("id")
        typ = req.get("type")
        try:
            if typ == "open":
                result = engine.open(req["accountDir"], req["hexKey"])
            elif typ == "get_sessions":
                result = engine.get_sessions()
            elif typ == "get_contacts":
                result = engine.get_contacts()
            elif typ == "get_messages":
                result = engine.get_messages(req["sessionId"], req.get("limit", 100), req.get("offset", 0))
            elif typ == "get_new_messages":
                result = engine.get_new_messages(req["sessionId"], req.get("minTime", 0), req.get("limit", 50))
            elif typ == "resolve_session":
                result = engine.resolve_session(req.get("keyword", ""))
            elif typ == "get_voice":
                result = engine.get_voice(req["sessionId"], req.get("svrId", 0),
                                          req.get("localId", 0), req.get("createTime", 0))
            elif typ == "get_media":
                result = engine.get_media(req["sessionId"], req.get("localId", 0),
                                          req.get("localType", 0), req.get("svrId", 0),
                                          req.get("createTime", 0))
            elif typ == "get_image":
                result = engine.get_image(req["sessionId"], req.get("localId", 0),
                                          req.get("wantFull", False))
            elif typ == "get_video":
                result = engine.get_video_path(req["sessionId"], req.get("localId", 0))
            elif typ == "get_key":
                result = get_key(req.get("timeoutMs", 180000))
            elif typ == "account_list":
                result = {"accounts": discover_accounts()}
            elif typ == "get_status":
                result = {"connected": engine.dec is not None, "ownWxid": engine.own_wxid,
                          "accountDir": engine.account_dir}
            else:
                send({"id": rid, "error": "unknown type: %s" % typ})
                continue
            send({"id": rid, "result": result})
        except Exception as e:
            import traceback
            log("handle error:", traceback.format_exc())
            send({"id": rid, "error": str(e)})


if __name__ == "__main__":
    main()
