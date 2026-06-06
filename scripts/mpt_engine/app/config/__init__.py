import os
import sys

from loguru import logger

from app.config import config
from app.utils import utils


def __init_logger():
    # _log_file = utils.storage_dir("logs/server.log")
    _lvl = config.log_level
    root_dir = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
    )

    def format_record(record):
        # 获取日志记录中的文件全路径
        file_path = record["file"].path
        # 将绝对路径转换为相对于项目根目录的路径
        relative_path = os.path.relpath(file_path, root_dir)
        # 更新记录中的文件路径
        record["file"].path = f"./{relative_path}"
        # 返回修改后的格式字符串
        # 您可以根据需要调整这里的格式
        _format = (
            "<green>{time:%Y-%m-%d %H:%M:%S}</> | "
            + "<level>{level}</> | "
            + '"{file.path}:{line}":<blue> {function}</> '
            + "- <level>{message}</>"
            + "\n"
        )
        return _format

    logger.remove()

    # AutoCast AI 改造：日志必须写到 stderr，而不是 stdout。
    # 视频生成进程的 stdout 被 Rust 侧严格解析为 JSONL 进度协议，任何
    # loguru 输出混进 stdout 都会破坏进度解析。这里统一改用 stderr，并关闭
    # 颜色（终端外重定向时 ANSI 转义会污染日志文件）。
    logger.add(
        sys.stderr,
        level=_lvl,
        format=format_record,
        colorize=False,
    )

    # logger.add(
    #     _log_file,
    #     level=_lvl,
    #     format=format_record,
    #     rotation="00:00",
    #     retention="3 days",
    #     backtrace=True,
    #     diagnose=True,
    #     enqueue=True,
    # )


__init_logger()
