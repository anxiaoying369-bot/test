import sqlite3
import os
import json
import argparse
import sys
import ast
import base64

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from compat import get_data_dir  # noqa: E402

def parse_url(url_str):
    if not url_str:
        return ""
    if url_str.startswith('[') and url_str.endswith(']'):
        try:
            # 尝试解析 Python 列表字符串
            urls = ast.literal_eval(url_str)
            if isinstance(urls, list) and len(urls) > 0:
                return urls[0]
        except:
            pass
    return url_str

def get_image_base64(path):
    if not path or not os.path.exists(path):
        return None
    try:
        with open(path, "rb") as f:
            data = f.read()
            return f"data:image/jpeg;base64,{base64.b64encode(data).decode()}"
    except:
        return None

def get_data_base():
    return str(get_data_dir() / "scraper_data")

def list_scraped_users():
    base = get_data_base()
    if not os.path.exists(base):
        return []
    
    # 尝试读取全局配置以获取一些昵称
    global_nicknames = {}
    try:
        import yaml
        global_config_path = os.path.join(os.path.dirname(__file__), 'DouyinComment', 'config.yaml')
        if os.path.exists(global_config_path):
            with open(global_config_path, 'r', encoding='utf-8') as f:
                config = yaml.safe_load(f)
                for user in config.get('users', []):
                    if user.get('sec_uid') and user.get('nickname'):
                        global_nicknames[user['sec_uid']] = user['nickname']
    except:
        pass

    users = []
    for sec_uid in os.listdir(base):
        path = os.path.join(base, sec_uid)
        if not os.path.isdir(path):
            continue
        
        # 查找 sqlite.db
        db_path = os.path.join(path, 'data', sec_uid, 'sqlite.db')
        if os.path.exists(db_path):
            try:
                conn = sqlite3.connect(db_path)
                conn.row_factory = sqlite3.Row
                cur = conn.cursor()
                # 尝试获取一些基本信息，比如视频数量
                cur.execute("SELECT COUNT(*) as count FROM videos")
                video_count = cur.fetchone()['count']
                
                cur.execute("SELECT COUNT(*) as count FROM comments")
                comment_count = cur.fetchone()['count']
                
                # 尝试读取保存的用户信息
                nickname = global_nicknames.get(sec_uid, sec_uid[:12] + '...')
                user_json_path = os.path.join(path, 'data', sec_uid, 'user.json')
                if os.path.exists(user_json_path):
                    try:
                        with open(user_json_path, 'r', encoding='utf-8') as f:
                            u_data = json.load(f)
                            nickname = u_data.get('nickname', nickname)
                    except:
                        pass

                # 获取最新的视频缩略图作为头像（或者直接用已有的 avatar.jpg）
                avatar_path_local = os.path.join(path, 'data', sec_uid, 'avatar.jpg')
                has_avatar = os.path.exists(avatar_path_local)
                
                users.append({
                    "sec_uid": sec_uid,
                    "nickname": nickname,
                    "video_count": video_count,
                    "comment_count": comment_count,
                    "has_avatar": has_avatar,
                    "avatar_path": os.path.abspath(avatar_path_local) if has_avatar else None,
                    "avatar_data": get_image_base64(avatar_path_local) if has_avatar else None,
                    "last_scrape": os.path.getmtime(db_path)
                })
                conn.close()
            except Exception as e:
                print(f"Error reading {db_path}: {e}", file=sys.stderr)
    
    return sorted(users, key=lambda x: x['last_scrape'], reverse=True)

def get_videos(sec_uid, limit=50, offset=0):
    db_path = os.path.join(get_data_base(), sec_uid, 'data', sec_uid, 'sqlite.db')
    if not os.path.exists(db_path):
        return []
    
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()
    
    cur.execute("""
        SELECT aweme_id, desc, create_time, thumb, 
               (SELECT COUNT(*) FROM comments WHERE aweme_id = v.aweme_id) as comment_count
        FROM videos v
        ORDER BY create_time DESC
        LIMIT ? OFFSET ?
    """, (limit, offset))
    
    videos = []
    for row in cur.fetchall():
        video = dict(row)
        video['aweme_id'] = str(video['aweme_id'])
        video['thumb'] = parse_url(video['thumb'])
        videos.append(video)
        
    conn.close()
    return videos

def get_comments(sec_uid, aweme_id=None, limit=100, offset=0):
    db_path = os.path.join(get_data_base(), sec_uid, 'data', sec_uid, 'sqlite.db')
    if not os.path.exists(db_path):
        return []
    
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()
    
    if aweme_id:
        cur.execute("""
            SELECT * FROM comments 
            WHERE aweme_id = ? 
            ORDER BY create_time DESC 
            LIMIT ? OFFSET ?
        """, (aweme_id, limit, offset))
    else:
        cur.execute("""
            SELECT * FROM comments 
            ORDER BY create_time DESC 
            LIMIT ? OFFSET ?
        """, (limit, offset))
        
    comments = []
    for row in cur.fetchall():
        comment = dict(row)
        comment['aweme_id'] = str(comment['aweme_id'])
        comment['cid'] = str(comment['cid'])
        comment['user_avatar'] = parse_url(comment['user_avatar'])
        comments.append(comment)
        
    conn.close()
    return comments

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["list_users", "get_videos", "get_comments"])
    parser.add_argument("--sec-uid", help="User sec_uid")
    parser.add_argument("--aweme-id", help="Video aweme_id")
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--offset", type=int, default=0)
    
    args = parser.parse_args()
    
    if args.action == "list_users":
        print(json.dumps(list_scraped_users()))
    elif args.action == "get_videos":
        if not args.sec_uid:
            print("Error: --sec-uid is required", file=sys.stderr)
            sys.exit(1)
        print(json.dumps(get_videos(args.sec_uid, args.limit, args.offset)))
    elif args.action == "get_comments":
        if not args.sec_uid:
            print("Error: --sec-uid is required", file=sys.stderr)
            sys.exit(1)
        print(json.dumps(get_comments(args.sec_uid, args.aweme_id, args.limit, args.offset)))

if __name__ == "__main__":
    main()
