#!/usr/bin/env python3
"""
抖音评论采集桥接脚本（AutoCast AI 集成版）

复用 AutoCastAI 已有的 cookie 逻辑（cookie.txt），
调用 DouyinComment 项目的核心采集模块。

用法:
  python3 douyin_scraper.py --cookie-path <cookie.txt> --sec-uid <id> --type <video|comment|reply|all> [options]

stdout: 只输出最终 JSON 结果（供 Rust 解析）
stderr: 所有日志输出
"""

import argparse
import asyncio
import json
import os
import sys
import time
from pathlib import Path

# 把 DouyinComment 项目加入 import 路径
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
# DouyinComment 是 git submodule，位于 scripts/DouyinComment/
DOUYIN_COMMENT_DIR = os.path.join(SCRIPT_DIR, 'DouyinComment')
sys.path.insert(0, os.path.abspath(DOUYIN_COMMENT_DIR))
sys.path.insert(0, SCRIPT_DIR)

from compat import get_data_dir, safe_signal  # noqa: E402


def _log(msg: str):
    """日志输出到 stderr"""
    print(msg, flush=True, file=sys.stderr)


# ============ Cookie 读取：cookie.txt → 原始字符串 ============

def load_cookie_string(cookie_path: str) -> str:
    """
    从 AutoCastAI 的 cookie.txt 读取 cookie 原始字符串。
    cookie.txt 格式: name1=value1; name2=value2（douyin_login.py 保存时已过滤域名）
    """
    path = Path(cookie_path)
    if not path.exists():
        raise FileNotFoundError(f"cookie.txt 不存在: {cookie_path}")

    with open(path, "r", encoding="utf-8") as f:
        cookie_str = f.read().strip()

    if not cookie_str:
        raise ValueError("cookie.txt 内容为空")

    return cookie_str


# ============ 进度状态文件 ============

class ProgressWriter:
    """
    将采集进度写入 JSON 文件，供 Rust 端轮询读取。
    文件路径: <data_dir>/scraper/{task_id}.json
    """

    def __init__(self, task_id: str):
        data_dir = str(get_data_dir() / "scraper")
        os.makedirs(data_dir, exist_ok=True)
        self.path = os.path.join(data_dir, f"{task_id}.json")
        self.data = {
            "task_id": task_id,
            "status": "running",
            "progress": 0,
            "total": 0,
            "current_type": "",
            "current_user": "",
            "stats": {},
            "log_lines": [],
            "started_at": time.time(),
            "finished_at": None,
        }
        self._write()

    def update(self, **kwargs):
        for k, v in kwargs.items():
            if k in self.data:
                self.data[k] = v
        self._write()

    def add_log(self, line: str):
        self.data["log_lines"].append(line)
        # 只保留最近 200 条日志
        if len(self.data["log_lines"]) > 200:
            self.data["log_lines"] = self.data["log_lines"][-200:]
        self._write()

    def finish(self, status: str = "completed", stats: dict = None):
        self.data["status"] = status
        self.data["finished_at"] = time.time()
        if stats:
            self.data["stats"] = stats
        self._write()

    def _write(self):
        try:
            with open(self.path, "w", encoding="utf-8") as f:
                json.dump(self.data, f, ensure_ascii=False)
        except Exception:
            pass


# ============ 自定义日志 Handler：捕获日志写入进度文件 ============

import logging

class ProgressLogHandler(logging.Handler):
    """将 Python logging 输出同步到 ProgressWriter"""

    def __init__(self, progress: ProgressWriter):
        super().__init__()
        self.progress = progress

    def emit(self, record):
        try:
            msg = self.format(record)
            self.progress.add_log(msg)
        except Exception:
            pass


# ============ 采集核心逻辑 ============

async def run_scrape(cookie_str: str, sec_uid: str, scrape_type: str,
                     limit: int, skip_existing: bool, output_dir: str,
                     progress: ProgressWriter):
    """
    调用 DouyinComment 核心模块执行采集。
    不依赖 DouyinComment 的 config.yaml 和 cookie.txt，
    直接传入 cookie 字符串和参数。
    """
    from core.api import DouyinAPI, CookieExpiredError
    from core.logger import logger
    from services.video_service import VideoService
    from services.comment_service import CommentService
    from services.reply_service import ReplyService
    from utils.printer import Config

    # 注册日志 handler（添加到内部的标准 logging.Logger）
    progress_handler = ProgressLogHandler(progress)
    progress_handler.setLevel(logging.INFO)
    progress_handler.setFormatter(logging.Formatter('%(message)s'))
    logger._get_system_logger().addHandler(progress_handler)

    # 重置 DouyinAPI 单例，确保使用新 cookie
    DouyinAPI._instance = None

    # 初始化 API
    api = DouyinAPI(cookie_str)

    # 验证 cookie
    _log("[SCRAPER] 验证 Cookie...")
    progress.update(current_type="验证Cookie")
    try:
        await api.verify_cookie()
        _log("[SCRAPER] Cookie 验证通过")
    except CookieExpiredError as e:
        _log(f"[SCRAPER] Cookie 验证失败: {e}")
        progress.finish(status="cookie_expired", stats={"error": str(e)})
        await api.close()
        return {"status": "cookie_expired", "error": str(e)}

    # 构造用户信息（替代 config.yaml）
    user = {
        'sec_uid': sec_uid,
        'nickname': sec_uid[:12] + '...' if len(sec_uid) > 12 else sec_uid,
        'videos': True,
        'comments': True,
        'replies': True,
    }

    # 更改工作目录到 output_dir（DouyinComment 的 StorageManager 使用相对路径 data/）
    original_dir = os.getcwd()
    data_dir_abs = os.path.abspath(os.path.join(output_dir, 'data', sec_uid))
    os.makedirs(data_dir_abs, exist_ok=True)

    # FieldConfig/Logger 在模块导入时就初始化了，chdir 后就找不到 config.yaml
    # 在 output_dir 创建 symlink 指向真实配置文件，使相对路径查找仍有效
    config_link = os.path.join(output_dir, 'config.yaml')
    real_config = os.path.join(DOUYIN_COMMENT_DIR, 'config.yaml')
    if not os.path.exists(config_link):
        try:
            os.symlink(real_config, config_link)
        except OSError:
            pass  # 忽略（文件系统不支持symlink等）

    os.chdir(output_dir)
    # chdir 后重新加载配置（UserManager 是进程内单例，chdir 前初始化会导致配置路径失效）
    from utils.field_config import UserManager
    UserManager().reload_config()
    _log(f"[SCRAPER] 工作目录: {output_dir}")
    _log(f"[SCRAPER] data_dir: {data_dir_abs}")

    # 配置默认请求延迟
    delay = 1.0

    all_stats = {}

    try:
        # 在采集作品前，尝试获取并保存用户信息
        if scrape_type in ('video', 'all'):
            _log(f"[SCRAPER] ===== 开始采集作品 =====")
            progress.update(current_type="作品", current_user=sec_uid[:12], progress=10)

            service = VideoService(sec_uid, cookie_str)
            # 我们先运行一次 fetch 来拿数据，以此提取用户信息
            raw_videos = await service.fetch(page_size=1, limit=1)
            if raw_videos:
                author = raw_videos[0].get('author', {})
                nickname = author.get('nickname', '未知用户')
                user_info = {
                    'sec_uid': sec_uid,
                    'nickname': nickname,
                    'avatar_url': author.get('avatar_thumb', {}).get('url_list', [None])[0]
                }
                # 保存到用户数据目录
                user_json_path = os.path.join(data_dir_abs, 'user.json')
                with open(user_json_path, 'w', encoding='utf-8') as f:
                    json.dump(user_info, f, ensure_ascii=False, indent=2)
                _log(f"[SCRAPER] 已保存用户信息: {nickname}")
            
            # 接着正式运行采集
            stats = await service.run(delay=delay, limit=limit)
            all_stats['video'] = stats
            _log(f"[SCRAPER] 作品采集完成: {stats}")

            current_progress = 33 if scrape_type == 'all' else 100
            progress.update(
                progress=current_progress,
                stats=all_stats,
            )

        if scrape_type in ('comment', 'all'):
            _log(f"[SCRAPER] ===== 开始采集评论 =====")
            start_p = 35 if scrape_type == 'all' else 10
            progress.update(current_type="评论", current_user=sec_uid[:12], progress=start_p)

            service = CommentService(sec_uid, cookie_str)
            _log(f"[SCRAPER] 正在初始化评论服务...")
            stats = await service.run(delay=delay, limit=limit, skip_existing=skip_existing)
            all_stats['comment'] = stats
            _log(f"[SCRAPER] 评论采集完成: {stats}")

            current_progress = 66 if scrape_type == 'all' else 100
            progress.update(
                progress=current_progress,
                stats=all_stats,
            )

        if scrape_type in ('reply', 'all'):
            _log(f"[SCRAPER] ===== 开始采集回复 =====")
            start_p = 68 if scrape_type == 'all' else 20
            progress.update(current_type="回复", current_user=sec_uid[:12], progress=start_p)

            service = ReplyService(sec_uid, cookie_str)
            _log(f"[SCRAPER] 正在初始化回复服务...")
            stats = await service.run(delay=delay, limit=limit, skip_existing=skip_existing)
            all_stats['reply'] = stats
            _log(f"[SCRAPER] 回复采集完成: {stats}")

            progress.update(progress=100, stats=all_stats)

    except CookieExpiredError as e:
        _log(f"[SCRAPER] Cookie 过期: {e}")
        progress.finish(status="cookie_expired", stats=all_stats)
        return {"status": "cookie_expired", "error": str(e), "stats": all_stats}

    except Exception as e:
        _log(f"[SCRAPER] 采集异常: {e}")
        import traceback
        _log(traceback.format_exc())
        progress.finish(status="error", stats=all_stats)
        return {"status": "error", "error": str(e), "stats": all_stats}

    finally:
        # 恢复工作目录
        os.chdir(original_dir)
        # 清理资源
        try:
            from core.database import SQLiteDatabase
            SQLiteDatabase.close_all()
        except Exception:
            pass
        try:
            from core.downloader import MediaDownloader
            await MediaDownloader.close_all()
        except Exception:
            pass
        await api.close()

    _log(f"[SCRAPER] 全部采集完成")
    progress.finish(status="completed", stats=all_stats)
    return {"status": "completed", "stats": all_stats}


# ============ 入口 ============

def main():
    parser = argparse.ArgumentParser(description="抖音评论采集（AutoCast AI 集成版）")
    parser.add_argument("--task-id", required=True, help="任务 ID")
    parser.add_argument("--cookie-path", required=True, help="cookie.txt 路径")
    parser.add_argument("--sec-uid", required=True, help="目标用户 sec_uid")
    parser.add_argument("--type", default="all", choices=["video", "comment", "reply", "all"],
                        help="采集类型（默认 all）")
    parser.add_argument("--limit", type=int, default=0, help="限制采集数量（0=不限制）")
    parser.add_argument("--skip-existing", action="store_true", help="跳过已采集数据")
    parser.add_argument("--output-dir", default="", help="输出目录（默认自动生成）")
    args = parser.parse_args()

    _log(f"[SCRAPER] 启动参数: task_id={args.task_id}, sec_uid={args.sec_uid[:20]}..., type={args.type}")
    _log(f"[SCRAPER]   limit={args.limit}, skip_existing={args.skip_existing}")

    # 初始化进度
    progress = ProgressWriter(args.task_id)

    # 信号处理：确保取消时更新状态
    def handle_signal(sig, frame):
        _log(f"[SCRAPER] 收到信号 {sig}，正在取消任务...")
        progress.finish(status="cancelled")
        sys.exit(1)

    import signal
    safe_signal(signal.SIGTERM, handle_signal)
    safe_signal(signal.SIGINT, handle_signal)

    # 加载 cookie
    try:
        cookie_str = load_cookie_string(args.cookie_path)
        _log(f"[SCRAPER] 加载 cookie 成功，长度={len(cookie_str)}")
    except Exception as e:
        _log(f"[SCRAPER] 加载 cookie 失败: {e}")
        progress.finish(status="error", stats={"error": str(e)})
        result = {"status": "error", "error": f"加载 cookie 失败: {e}"}
        print(json.dumps(result, ensure_ascii=False))
        sys.exit(1)

    # 确定输出目录
    if not args.output_dir:
        output_dir = str(get_data_dir() / "scraper_data" / args.sec_uid)
    else:
        output_dir = args.output_dir

    _log(f"[SCRAPER] 数据输出到: {output_dir}")

    # 执行采集
    try:
        result = asyncio.run(run_scrape(
            cookie_str=cookie_str,
            sec_uid=args.sec_uid,
            scrape_type=args.type,
            limit=args.limit,
            skip_existing=args.skip_existing,
            output_dir=output_dir,
            progress=progress,
        ))
    except Exception as e:
        _log(f"[SCRAPER] 严重错误: {e}")
        progress.finish(status="error", stats={"error": str(e)})
        result = {"status": "error", "error": str(e)}

    # stdout 只输出最终 JSON
    print(json.dumps(result, ensure_ascii=False))
    sys.exit(0 if result.get("status") == "completed" else 1)


if __name__ == "__main__":
    main()
