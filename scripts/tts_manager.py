"""TTS 桥接 CLI。

子命令：
  list-voices  列出当前 Provider 的可用音色
  synthesize   合成语音到 output 文件

输出：
  list-voices  → {"voices": [{...}, ...]}
  synthesize   → {"status": "completed", "audio_path": "..."} | {"status": "error", "error": "..."}
"""
import argparse
import json
import sys

from tts_providers import get_tts_provider
from provider_errors import classify_exception


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["list-voices", "synthesize"])
    parser.add_argument("--provider", default="mock")
    parser.add_argument("--api-key", default="")
    parser.add_argument("--base-url", default="")
    parser.add_argument("--model", default="")
    parser.add_argument("--text", default="")
    parser.add_argument("--voice", default="")
    parser.add_argument("--speed", type=float, default=1.0)
    parser.add_argument("--output", default="")

    args = parser.parse_args()
    try:
        kwargs = {}
        if args.base_url: kwargs["base_url"] = args.base_url
        if args.model:    kwargs["model"] = args.model

        provider = get_tts_provider(args.provider, args.api_key, **kwargs)

        if args.action == "list-voices":
            print(json.dumps({"voices": provider.list_voices()}, ensure_ascii=False))
            return

        if not args.text.strip():
            print(json.dumps({"status": "error", "error": "text 不能为空"}, ensure_ascii=False))
            sys.exit(1)
        if not args.output:
            print(json.dumps({"status": "error", "error": "output 必填"}, ensure_ascii=False))
            sys.exit(1)
        voice = args.voice or (provider.list_voices()[0]["id"] if provider.list_voices() else "")

        path = provider.synthesize(args.text, voice_id=voice, speed=args.speed, output_path=args.output)
        print(json.dumps({"status": "completed", "audio_path": path}, ensure_ascii=False))

    except Exception as e:
        err = classify_exception(e)
        out = {"status": "error", **err.to_dict()}
        print(json.dumps(out, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
