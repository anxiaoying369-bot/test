import subprocess
import shutil
import os
import sys

def is_aria2_installed():
    return shutil.which("aria2c") is not None

def download_video_aria2(url, output_path, filename):
    if not is_aria2_installed():
        return False
    
    # Use aria2c for high-speed download
    cmd = [
        "aria2c",
        "--console-log-level=warn",
        "-x", "16",       # 16 connections per server
        "-s", "16",       # 16 connections
        "-k", "1M",       # 1M chunk size
        "--dir", output_path,
        "-o", filename,
        url
    ]
    
    try:
        subprocess.run(cmd, check=True, capture_output=True)
        return True
    except Exception:
        return False

def download_batch_aria2(urls_with_names, output_dir):
    """
    urls_with_names: list of (url, filename) tuples
    """
    if not is_aria2_installed():
        return False
    
    # Create input file for aria2c --input-file
    input_file = os.path.join(output_dir, "aria2_input.txt")
    with open(input_file, "w") as f:
        for url, filename in urls_with_names:
            f.write(f"{url}\n")
            f.write(f"  out={filename}\n")
    
    cmd = [
        "aria2c",
        "--input-file", input_file,
        "--dir", output_dir,
        "-x", "16",
        "-s", "16",
        "--console-log-level=warn"
    ]
    
    try:
        subprocess.run(cmd, check=True)
        os.remove(input_file)
        return True
    except Exception:
        if os.path.exists(input_file):
            os.remove(input_file)
        return False
