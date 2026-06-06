#!/usr/bin/env python3
"""
AutoCast AI · MPT 引擎辅助命令（不跑完整生成）。

目前支持：
  terms     根据主题 + 脚本生成素材搜索关键词（Pexels 检索用），输出 JSON 到 stdout。
  preview   用 Edge TTS 合成一段示例音频，供前端试听音色，输出 mp3 路径到 stdout。

由 Rust 命令调用：
    python scripts/mpt_helper.py terms   --params <params.json>
    python scripts/mpt_helper.py preview --params <params.json>
params.json 视命令而定；配置经 MPT_CONFIG 环境变量注入。
"""

import argparse
import json
import os
import sys

_SCRIPTS_DIR = os.path.dirname(os.path.realpath(__file__))
_ENGINE_DIR = os.path.join(_SCRIPTS_DIR, "mpt_engine")
for _p in (_ENGINE_DIR, _SCRIPTS_DIR):
    if _p not in sys.path:
        sys.path.insert(0, _p)


def _emit(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def cmd_terms(data: dict):
    from app.services import llm

    terms = llm.generate_terms(
        video_subject=data.get("video_subject", "") or "",
        video_script=data.get("video_script", "") or "",
        amount=int(data.get("amount", 5) or 5),
    )
    _emit({"status": "ok", "terms": terms})


def _sample_text_for(voice_name: str) -> str:
    # 英文音色用英文示例，其它（中文/粤语/方言）统一用中文示例。
    if voice_name.lower().startswith("en"):
        return "Hello, this is a quick voice preview for your video."
    return "你好，这是配音音色的试听效果，用于预览这条视频的声音。"


def cmd_preview(data: dict):
    import os
    from app.services import voice as voice_service
    from app.utils import utils

    voice_name = (data.get("voice_name") or "zh-CN-XiaoxiaoNeural-Female").strip()
    text = (data.get("text") or "").strip() or _sample_text_for(voice_name)

    out_dir = os.path.join(utils.storage_dir(), "previews")
    os.makedirs(out_dir, exist_ok=True)
    safe = voice_name.replace(":", "_").replace("/", "_")
    out_file = os.path.join(out_dir, f"{safe}.mp3")

    # 同音色已合成过则直接复用缓存，让试听更快。
    if not (os.path.exists(out_file) and os.path.getsize(out_file) > 0):
        sub = voice_service.tts(
            text=text,
            voice_name=voice_service.parse_voice_name(voice_name),
            voice_rate=1.0,
            voice_file=out_file,
        )
        if sub is None or not (os.path.exists(out_file) and os.path.getsize(out_file) > 0):
            raise RuntimeError("配音试听生成失败，请检查网络（Edge TTS 需要联网）。")

    _emit({"status": "ok", "path": out_file})


def main():
    parser = argparse.ArgumentParser(description="AutoCast AI MPT helper")
    parser.add_argument("action", choices=["terms", "preview"])
    parser.add_argument("--params", required=True)
    args = parser.parse_args()

    try:
        with open(args.params, "r", encoding="utf-8") as fp:
            data = json.load(fp)
    except Exception as e:
        _emit({"status": "error", "message": f"读取参数失败: {e}"})
        sys.exit(1)

    try:
        if args.action == "terms":
            cmd_terms(data)
        elif args.action == "preview":
            cmd_preview(data)
    except Exception as e:
        try:
            from provider_errors import classify_exception

            err = classify_exception(e)
            payload = {"status": "error"}
            payload.update(err.to_dict())
            _emit(payload)
        except Exception:
            _emit({"status": "error", "message": str(e)})
        sys.exit(1)


if __name__ == "__main__":
    main()
