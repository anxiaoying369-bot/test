import asyncio
import json
import os
import subprocess
import sys
import websockets
import socket

import shutil
from pathlib import Path

def get_hermes_path():
    # 1. 尝试从 PATH 查找
    path = shutil.which("hermes")
    if path:
        return path
    
    # 2. 尝试常见安装位置
    candidates = []
    if sys.platform == "win32":
        candidates = [
            Path(os.environ.get("LOCALAPPDATA", "")) / "bin" / "hermes.exe",
            Path("C:\\Program Files\\hermes\\hermes.exe"),
        ]
    else:
        candidates = [
            Path.home() / ".local" / "bin" / "hermes",
            Path("/usr/local/bin/hermes"),
        ]
        
    for c in candidates:
        if c.exists():
            return str(c)
            
    return "hermes" # 最后兜底

HERMES_PATH = get_hermes_path()

async def handle_client(websocket):
    print(f"[Bridge] 客户端已连接: {websocket.remote_address}", flush=True)
    
    try:
        # 启动 hermes acp 进程
        proc = await asyncio.create_subprocess_exec(
            HERMES_PATH, "acp",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
    except Exception as e:
        print(f"[Bridge] 启动 hermes acp 失败: {e}", flush=True)
        await websocket.close()
        return

    async def forward_stdout():
        try:
            while True:
                line = await proc.stdout.readline()
                if not line:
                    print("[Bridge] Hermes Stdout 已关闭", flush=True)
                    break
                decoded_line = line.decode().strip()
                if decoded_line:
                    # 如果是 JSON-RPC 响应，直接转发
                    if decoded_line.startswith('{'):
                        print(f"[Hermes -> Client] {decoded_line}", flush=True)
                        await websocket.send(decoded_line)
                    else:
                        # 可能是普通日志
                        print(f"[Hermes Stdout] {decoded_line}", flush=True)
        except Exception as e:
            print(f"[Bridge] Stdout 转发错误: {e}", flush=True)

    async def forward_stderr():
        try:
            while True:
                line = await proc.stderr.readline()
                if not line:
                    break
                # 将 Hermes 的内部日志输出到终端
                err_line = line.decode()
                sys.stderr.write(f"[Hermes Stderr] {err_line}")
                sys.stderr.flush()
        except Exception as e:
            print(f"[Bridge] Stderr 转发错误: {e}", flush=True)

    stdout_task = asyncio.create_task(forward_stdout())
    stderr_task = asyncio.create_task(forward_stderr())

    try:
        async for message in websocket:
            try:
                print(f"[Client -> Hermes] {message}", flush=True)
                proc.stdin.write((message + "\n").encode())
                await proc.stdin.drain()
            except Exception as e:
                print(f"[Bridge] 发送给 Hermes 失败: {e}", flush=True)
    except websockets.exceptions.ConnectionClosed:
        print("[Bridge] 客户端连接已关闭", flush=True)
    finally:
        stdout_task.cancel()
        stderr_task.cancel()
        if proc.returncode is None:
            proc.terminate()
            await proc.wait()

async def main():
    port = int(os.environ.get("HERMES_BRIDGE_PORT", 8000))
    print(f"[Bridge] 正在启动 WebSocket 桥接器: ws://127.0.0.1:{port}", flush=True)
    
    # 强制开启地址重用，防止 Address already in use
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        sock.bind(("127.0.0.1", port))
    except OSError as e:
        print(f"[Bridge] 错误: 端口 {port} 绑定失败: {e}", flush=True)
        sys.exit(1)

    async with websockets.serve(handle_client, sock=sock):
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
