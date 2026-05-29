import sys
import json
import argparse
from video_providers import get_provider
from provider_errors import classify_exception

def start_task(provider_name, api_key, prompt, mode="text", **kwargs):
    provider = get_provider(provider_name, api_key, **kwargs)
    if mode == "text":
        task_id = provider.text_to_video(prompt, **kwargs)
    elif mode == "image":
        image_url = kwargs.get("image_url")
        task_id = provider.image_to_video(image_url, prompt, **kwargs)
    else:
        raise ValueError(f"Unknown mode: {mode}")
    
    return {"status": "processing", "task_id": task_id}

def poll_task(provider_name, api_key, task_id, **kwargs):
    provider = get_provider(provider_name, api_key, **kwargs)
    result = provider.poll_task(task_id)
    return result

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["start", "poll"])
    parser.add_argument("--provider", default="fal")
    parser.add_argument("--api-key", required=True)
    parser.add_argument("--prompt")
    parser.add_argument("--mode", default="text")
    parser.add_argument("--task-id")
    parser.add_argument("--ratio", default="9:16")
    parser.add_argument("--image-url")
    parser.add_argument("--base-url")
    parser.add_argument("--model")

    args = parser.parse_args()

    try:
        if args.action == "start":
            res = start_task(
                args.provider, 
                args.api_key, 
                args.prompt, 
                mode=args.mode, 
                ratio=args.ratio,
                image_url=args.image_url,
                base_url=args.base_url,
                model=args.model
            )
        else:
            res = poll_task(
                args.provider, 
                args.api_key, 
                args.task_id,
                base_url=args.base_url,
                model=args.model
            )
        
        print(json.dumps(res))
    except Exception as e:
        err = classify_exception(e)
        out = {"status": "error", **err.to_dict()}
        print(json.dumps(out, ensure_ascii=False))
        sys.exit(1)

if __name__ == "__main__":
    main()
