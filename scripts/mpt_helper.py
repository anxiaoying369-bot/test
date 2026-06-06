#!/usr/bin/env python3
"""
AutoCast AI · MPT 引擎辅助命令（不跑完整生成）。

目前支持：
  terms   根据主题 + 脚本生成素材搜索关键词（Pexels 检索用），输出 JSON 到 stdout。

由 Rust 命令在「关键词」步骤调用：
    python scripts/mpt_helper.py terms --params <params.json>
params.json 至少包含 video_subject / video_script / amount，配置经 MPT_CONFIG 环境变量注入。
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


def main():
    parser = argparse.ArgumentParser(description="AutoCast AI MPT helper")
    parser.add_argument("action", choices=["terms"])
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
