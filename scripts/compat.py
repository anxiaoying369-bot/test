"""
AutoCast AI — 跨平台兼容工具模块

提供统一的 Chrome 路径探测、数据目录解析、进程信号处理等跨平台函数，
确保脚本在 macOS / Windows / Linux 上行为一致。
"""
from __future__ import annotations

import os
import sys
import json
from pathlib import Path


# ─── 数据目录 ────────────────────────────────────────────────────────────────

def get_data_dir() -> Path:
    """
    返回 AutoCastAI 数据根目录，与 Rust 端 dirs::data_local_dir() 保持严格一致：
      macOS  : ~/Library/Application Support/AutoCastAI
      Windows: %LOCALAPPDATA%\\AutoCastAI
      Linux  : ~/.local/share/AutoCastAI

    优先读取 Rust 启动时注入的环境变量 AUTOCAST_DATA_DIR，确保路径绝对一致。
    """
    env = os.environ.get("AUTOCAST_DATA_DIR")
    if env:
        return Path(env)

    if sys.platform == "win32":
        base = os.environ.get("LOCALAPPDATA") or str(Path.home())
        return Path(base) / "AutoCastAI"
    elif sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "AutoCastAI"
    else:
        xdg = os.environ.get("XDG_DATA_HOME")
        base = xdg if xdg else str(Path.home() / ".local" / "share")
        return Path(base) / "AutoCastAI"


def get_config() -> dict:
    """
    读取 AutoCastAI 配置文 (config.json)。
    如果不存在则返回包含默认值的字典。
    """
    config_path = get_data_dir() / "config.json"
    if config_path.exists():
        try:
            with open(config_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    return {
        "llm": {
            "api_key": "",
            "base_url": "https://api.openai.com/v1",
            "model": "gpt-4o"
        }
    }


# ─── Chrome 路径 ─────────────────────────────────────────────────────────────

def get_chrome_path() -> str:
    """
    返回 Google Chrome 可执行文件路径，按平台自动探测候选路径。
    所有候选均不存在时返回 "chrome"，依赖系统 PATH。
    """
    if sys.platform == "win32":
        candidates = [
            os.path.expandvars(r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"),
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
        for p in candidates:
            if os.path.exists(p):
                return p
        return "chrome"
    elif sys.platform == "darwin":
        p = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        return p if os.path.exists(p) else "google-chrome"
    else:
        for p in (
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
        ):
            if os.path.exists(p):
                return p
        return "google-chrome"


def get_chrome_user_data_dir() -> str:
    """
    返回 Chrome 调试专用 Profile 目录（跨平台）。
    Windows 放在 %LOCALAPPDATA%\\AutoCastAI\\chrome-debug-profile，
    其他平台放在 ~/chrome-debug-profile（保持原有行为）。
    """
    if sys.platform == "win32":
        base = os.environ.get("LOCALAPPDATA") or str(Path.home())
        return str(Path(base) / "AutoCastAI" / "chrome-debug-profile")
    else:
        return str(Path.home() / "chrome-debug-profile")


# ─── 信号处理 ────────────────────────────────────────────────────────────────

def safe_signal(signum: int, handler) -> None:
    """
    跨平台 signal.signal() 包装。
    Windows 上 SIGTERM 的外部触发受限，但注册本身一般成功；
    在子线程中调用或平台不支持时静默忽略 OSError / ValueError。
    """
    import signal as _signal
    try:
        _signal.signal(signum, handler)
    except (OSError, ValueError):
        pass


def safe_kill_self() -> None:
    """
    向自身发送 SIGTERM（Unix）或直接 sys.exit(0)（Windows）。
    Windows 不支持通过 os.kill 发送 SIGTERM，统一用 exit。
    """
    import signal as _signal
    if sys.platform == "win32":
        sys.exit(0)
    else:
        try:
            os.kill(os.getpid(), _signal.SIGTERM)
        except Exception:
            sys.exit(0)
