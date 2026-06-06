"""
AutoCast AI 适配层：替换 MoneyPrinterTurbo 原本基于 config.toml 的配置加载。

原 MPT 在导入 `app.config` 时会读取项目根目录的 `config.toml`。在 AutoCast AI 里，
视频生成由 Rust 命令 `video_mpt_generate` 拉起 `scripts/mpt_generate.py`，所有配置
（LLM 凭据、Pexels Key、FFmpeg 路径、whisper 设置、字幕默认值等）通过环境变量
`MPT_CONFIG`（一段 JSON）注入。本模块据此构造出与原 MPT 同结构的
`app / whisper / proxy / ui` 等模块级对象，使上游 services 代码几乎零改动即可运行。

注入约定（由 Rust 侧 commands/video_studio 写入）：
    MPT_CONFIG = {
        "app":     { "llm_provider": "openai", "openai_api_key": ..., "openai_base_url": ...,
                     "openai_model_name": ..., "pexels_api_keys": [...], "subtitle_provider": "edge",
                     "video_source": "pexels", "tls_verify": true, ... },
        "whisper": { "model_size": "large-v3", "device": "cpu", "compute_type": "int8" },
        "proxy":   {},            # 可为空，requests 接受 None/{}
        "ui":      { "subtitle_position": "bottom", "custom_position": 70.0, "hide_log": true },
        "log_level": "INFO"
    }
另外 `IMAGEIO_FFMPEG_EXE`（打包 FFmpeg 路径）和 `MPT_STORAGE_DIR`（任务输出目录）
也由 Rust 直接以环境变量注入。
"""

import json
import os

# 兼容原 MPT 里少数地方仍会引用 config.config_file（仅用于错误信息展示）。
root_dir = os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.realpath(__file__))))
)
config_file = os.path.join(root_dir, "config.toml")


def _load_injected_config() -> dict:
    raw = os.environ.get("MPT_CONFIG")
    if not raw:
        return {}
    try:
        data = json.loads(raw)
        if isinstance(data, dict):
            return data
    except Exception:
        # 配置注入异常时不应让整个引擎在 import 阶段崩溃，交由后续具体调用
        # （如 LLM/Pexels Key 缺失）抛出更可读的错误。
        pass
    return {}


_cfg = _load_injected_config()

app = _cfg.get("app", {}) or {}
whisper = _cfg.get("whisper", {}) or {
    "model_size": "large-v3",
    "device": "cpu",
    "compute_type": "int8",
}
# requests 的 proxies 参数接受 None 或 dict；默认不走代理。
proxy = _cfg.get("proxy", None)
ui = _cfg.get(
    "ui",
    {
        "subtitle_position": "bottom",
        "custom_position": 70.0,
        "hide_log": True,
    },
)

log_level = _cfg.get("log_level", "INFO")
project_name = "AutoCast AI · MPT Engine"
project_version = "1.0.0"

# FFmpeg 路径：优先沿用 Rust 注入的 IMAGEIO_FFMPEG_EXE（MoviePy/imageio 约定）。
# 如果调用方在 app 配置里另外给了 ffmpeg_path，也兼容设置到环境变量。
_ffmpeg_path = app.get("ffmpeg_path", "")
if _ffmpeg_path and os.path.isfile(_ffmpeg_path):
    os.environ.setdefault("IMAGEIO_FFMPEG_EXE", _ffmpeg_path)


def save_config():
    # AutoCast AI 不把配置写回文件（配置由桌面端设置统一管理），保留空实现以兼容
    # 任何历史调用方。
    return
