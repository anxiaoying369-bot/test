#!/usr/bin/env python3
"""
AutoCast AI 视频生成编排入口（MoneyPrinterTurbo 引擎）。

由 Rust 命令 `video_mpt_generate` 拉起：
    python scripts/mpt_generate.py --params <params.json>

职责：
  1. 把前端/Rust 传来的参数 JSON 映射为 MPT 的 `VideoParams`；
  2. 调用 `app.services.task.start()` 跑完整管线
     （脚本→关键词→TTS→字幕→下载/校验素材→拼接→字幕烧录+BGM→成片）；
  3. 进度由 `app.services.state.JsonlState` 实时打到 stdout（JSONL）；
  4. 结束时在 stdout 打印一行 done / error 结果，并以对应退出码退出。

约定：
  - 所有业务日志走 stderr（见 app/config/__init__.py），stdout 只承载 JSONL 协议。
  - 配置（LLM/Pexels/whisper/FFmpeg/存储目录）由 Rust 通过环境变量
    MPT_CONFIG / IMAGEIO_FFMPEG_EXE / MPT_STORAGE_DIR 注入（见 app/config/config.py）。
"""

import argparse
import json
import os
import sys

# 让 `app` 包（scripts/mpt_engine/app）与同级脚本（provider_errors）可被导入。
_SCRIPTS_DIR = os.path.dirname(os.path.realpath(__file__))
_ENGINE_DIR = os.path.join(_SCRIPTS_DIR, "mpt_engine")
for _p in (_ENGINE_DIR, _SCRIPTS_DIR):
    if _p not in sys.path:
        sys.path.insert(0, _p)


def _emit(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def _build_params(data: dict):
    """把入参 dict 映射为 MPT VideoParams。"""
    from app.models.schema import MaterialInfo, VideoParams

    video_source = (data.get("video_source") or "pexels").strip()

    # 本地素材：构造 MaterialInfo 列表；素材文件需位于 MPT_STORAGE_DIR/local_videos 下
    # （Rust 侧会把用户选择的本地素材复制进该目录后再传入文件名/路径）。
    materials = None
    if video_source == "local":
        materials = [
            MaterialInfo(provider="local", url=str(u))
            for u in (data.get("video_materials") or [])
            if str(u).strip()
        ]

    params = VideoParams(
        video_subject=data.get("video_subject", "") or "",
        video_script=data.get("video_script", "") or "",
        video_terms=data.get("video_terms") or None,
        video_aspect=data.get("video_aspect", "9:16") or "9:16",
        video_concat_mode=data.get("video_concat_mode", "random") or "random",
        video_transition_mode=data.get("video_transition_mode") or None,
        video_clip_duration=int(data.get("video_clip_duration", 5) or 5),
        video_count=int(data.get("video_count", 1) or 1),
        video_source=video_source,
        video_materials=materials,
        video_language=data.get("video_language", "") or "",
        voice_name=data.get("voice_name", "") or "",
        voice_volume=float(data.get("voice_volume", 1.0) or 1.0),
        voice_rate=float(data.get("voice_rate", 1.0) or 1.0),
        bgm_type=data.get("bgm_type", "random") if data.get("bgm_type") is not None else "random",
        bgm_file=data.get("bgm_file", "") or "",
        bgm_volume=float(data.get("bgm_volume", 0.2) or 0.2),
        subtitle_enabled=bool(data.get("subtitle_enabled", True)),
        subtitle_position=data.get("subtitle_position", "bottom") or "bottom",
        custom_position=float(data.get("custom_position", 70.0) or 70.0),
        font_name=data.get("font_name", "STHeitiMedium.ttc") or "STHeitiMedium.ttc",
        text_fore_color=data.get("text_fore_color", "#FFFFFF") or "#FFFFFF",
        text_background_color=data.get("text_background_color", True),
        font_size=int(data.get("font_size", 60) or 60),
        stroke_color=data.get("stroke_color", "#000000") or "#000000",
        stroke_width=float(data.get("stroke_width", 1.5) or 1.5),
        n_threads=int(data.get("n_threads", 2) or 2),
        paragraph_number=int(data.get("paragraph_number", 1) or 1),
    )
    return params


def main():
    parser = argparse.ArgumentParser(description="AutoCast AI MPT video generator")
    parser.add_argument("--params", required=True, help="path to params JSON file")
    parser.add_argument("--task-id", default=None, help="task id (default: random uuid)")
    args = parser.parse_args()

    try:
        with open(args.params, "r", encoding="utf-8") as fp:
            data = json.load(fp)
    except Exception as e:
        _emit({"event": "error", "status": "error", "message": f"读取参数失败: {e}"})
        sys.exit(1)

    # 延迟导入：保证 sys.path 与环境变量（MPT_CONFIG 等）已就绪后再触发 app 包初始化。
    try:
        from app.services import task as task_service
        from app.utils import utils
    except Exception as e:
        _emit({"event": "error", "status": "error", "message": f"引擎初始化失败: {e}"})
        sys.exit(1)

    task_id = args.task_id or utils.get_uuid(remove_hyphen=True)

    try:
        params = _build_params(data)
    except Exception as e:
        _emit({"event": "error", "status": "error", "message": f"参数解析失败: {e}"})
        sys.exit(1)

    _emit({"event": "start", "task_id": task_id})

    try:
        result = task_service.start(task_id=task_id, params=params, stop_at="video")
    except Exception as e:
        # 复用项目统一的错误归类，给前端更可读的提示。
        try:
            from provider_errors import classify_exception

            err = classify_exception(e)
            payload = {"event": "error", "status": "error", "task_id": task_id}
            payload.update(err.to_dict())
            _emit(payload)
        except Exception:
            _emit(
                {
                    "event": "error",
                    "status": "error",
                    "task_id": task_id,
                    "message": str(e),
                }
            )
        sys.exit(1)

    if not result or not result.get("videos"):
        _emit(
            {
                "event": "error",
                "status": "error",
                "task_id": task_id,
                "message": "视频生成失败：未产出成片，请检查日志（素材/LLM/网络）。",
            }
        )
        sys.exit(1)

    _emit(
        {
            "event": "done",
            "status": "done",
            "task_id": task_id,
            "videos": result.get("videos", []),
            "combined_videos": result.get("combined_videos", []),
            "audio_file": result.get("audio_file"),
            "subtitle_path": result.get("subtitle_path"),
            "script": result.get("script"),
            "terms": result.get("terms"),
        }
    )


if __name__ == "__main__":
    main()
