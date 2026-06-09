import os
import sys
from modelscope import snapshot_download

def download_sensevoice():
    model_dir = os.path.join("src-tauri", "resources", "models", "SenseVoiceSmall")
    print(f"Downloading SenseVoiceSmall to {model_dir}...")
    
    # 下载模型到指定目录
    # local_dir 会自动创建
    snapshot_download(
        'iic/SenseVoiceSmall',
        local_dir=model_dir
    )
    print("Download complete.")

if __name__ == "__main__":
    try:
        download_sensevoice()
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
