"""图片生成 CLI 入口。

输入：--provider --api-key --prompt --size [--reference-image] [--base-url --model]
输出 stdout JSON：
  - 成功：{"status": "completed", "image_url": "<url 或 data:URL>"}
  - 失败：{"status": "error",     "error": "<message>"}
"""
import sys
import json
import argparse

from image_providers import get_image_provider
from provider_errors import classify_exception


def main():
    parser = argparse.ArgumentParser(description="AutoCast 图片生成桥接")
    parser.add_argument("--provider", default="mock", help="fal / volcengine / openai / mock")
    parser.add_argument("--api-key", default="")
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--size", default="1024x1024", help="如 1024x1024 / 720x1280")
    parser.add_argument("--reference-image", default="", help="参考图本地路径或 URL，可选")
    parser.add_argument("--base-url", default="", help="OpenAI 兼容 Provider 的 base url")
    parser.add_argument("--model", default="", help="OpenAI 兼容 Provider 的模型 id")

    args = parser.parse_args()
    try:
        kwargs = {}
        if args.base_url:
            kwargs["base_url"] = args.base_url
        if args.model:
            kwargs["model"] = args.model

        provider = get_image_provider(args.provider, args.api_key, **kwargs)

        if args.reference_image:
            url = provider.image_to_image(args.reference_image, args.prompt, args.size)
        else:
            url = provider.text_to_image(args.prompt, args.size)

        if not url:
            print(json.dumps({"status": "error", "error": "Provider 返回空 URL"}))
            sys.exit(1)

        print(json.dumps({"status": "completed", "image_url": url}, ensure_ascii=False))
    except Exception as e:
        err = classify_exception(e)
        out = {"status": "error", **err.to_dict()}
        print(json.dumps(out, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
