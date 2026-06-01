#!/usr/bin/env python3
"""Function Calling 工具集：Tool 定义 + 内置工具实现 + 默认工具注册。

从 funcall.py 拆分而来，供 FunctionCallingCLI 使用。
"""

import asyncio
import inspect
import json
import os
import re
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

import httpx


# ============================================================
# Tool 定义
# ============================================================

class Tool:
    """可被 AI 调用的工具定义"""

    def __init__(
        self,
        name: str,
        description: str,
        parameters: Dict[str, Any],
        function: Callable[..., Any],
    ) -> None:
        self.name = name
        self.description = description
        self.parameters = parameters
        self.function = function

    def to_openai_format(self) -> Dict[str, Any]:
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            },
        }

    async def execute(self, **kwargs: Any) -> Any:
        if inspect.iscoroutinefunction(self.function):
            return await self.function(**kwargs)
        return self.function(**kwargs)


# ============================================================
# 内置工具实现（不依赖任何后端服务）
# ============================================================

async def read_file(path: str, limit: int = 500) -> Dict[str, Any]:
    """读取文件内容"""
    try:
        p = Path(path).expanduser().resolve()
        if not p.exists():
            return {"error": f"文件不存在: {path}"}
        if not p.is_file():
            return {"error": f"不是文件: {path}"}
        content = p.read_text(encoding="utf-8")
        if len(content) > limit:
            content = content[:limit] + f"\n... (共 {len(content)} 字符，已截断)"
        return {"path": str(p), "size": len(content), "content": content}
    except Exception as e:
        return {"error": str(e)}


async def write_file(path: str, content: str) -> Dict[str, Any]:
    """写入文件内容"""
    try:
        p = Path(path).expanduser().resolve()
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
        return {"path": str(p), "size": len(content), "message": "写入成功"}
    except Exception as e:
        return {"error": str(e)}


async def list_directory(path: str = ".") -> Dict[str, Any]:
    """列出目录内容"""
    try:
        p = Path(path).expanduser().resolve()
        if not p.exists():
            return {"error": f"目录不存在: {path}"}
        if not p.is_dir():
            return {"error": f"不是目录: {path}"}
        items = []
        for item in sorted(p.iterdir()):
            items.append({
                "name": item.name,
                "type": "dir" if item.is_dir() else "file",
                "size": item.stat().st_size if item.is_file() else 0,
            })
        return {"path": str(p), "items": items, "total": len(items)}
    except Exception as e:
        return {"error": str(e)}


async def search_files(pattern: str, path: str = ".", file_glob: Optional[str] = None, limit: int = 50) -> Dict[str, Any]:
    """搜索文件内容"""
    try:
        p = Path(path).expanduser().resolve()
        if not p.exists():
            return {"error": f"搜索路径不存在: {path}"}
        matches = []
        for txt_file in p.rglob(file_glob or "*"):
            if not txt_file.is_file():
                continue
            try:
                content = txt_file.read_text(encoding="utf-8", errors="ignore")
                lines = content.split("\n")
                for i, line in enumerate(lines, 1):
                    if re.search(pattern, line, re.IGNORECASE):
                        matches.append({
                            "file": str(txt_file),
                            "line": i,
                            "content": line.strip()[:200],
                        })
                        if len(matches) >= limit:
                            break
            except Exception:
                continue
            if len(matches) >= limit:
                break
        return {"pattern": pattern, "path": str(p), "matches": matches, "total": len(matches)}
    except Exception as e:
        return {"error": str(e)}


async def run_command(command: str, timeout: int = 60) -> Dict[str, Any]:
    """执行 Shell 命令"""
    try:
        result = subprocess.run(
            command,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "returncode": result.returncode,
            "stdout": result.stdout[:5000],
            "stderr": result.stderr[:2000],
            "command": command,
        }
    except subprocess.TimeoutExpired:
        return {"error": f"命令执行超时（{timeout}秒）", "command": command}
    except Exception as e:
        return {"error": str(e), "command": command}


async def calculate(expression: str) -> Dict[str, Any]:
    """计算数学表达式"""
    try:
        # 安全计算：只允许数字和基本运算符
        safe_expr = re.sub(r"[^0-9+\-*/().e\s]", "", expression)
        result = eval(safe_expr)  # nosec
        return {"expression": expression, "result": result}
    except Exception as e:
        return {"error": f"计算错误: {e}", "expression": expression}


async def get_time(timezone: str = "Asia/Shanghai") -> Dict[str, Any]:
    """获取当前时间"""
    try:
        now = datetime.now()
        return {
            "datetime": now.isoformat(),
            "timestamp": int(now.timestamp()),
            "timezone": timezone,
        }
    except Exception as e:
        return {"error": str(e)}


async def web_search(query: str, limit: int = 5) -> Dict[str, Any]:
    """搜索网页（需要配置 SEARCH_API_KEY）"""
    api_key = os.getenv("SEARCH_API_KEY")
    if not api_key:
        return {"error": "SEARCH_API_KEY 环境变量未设置，请设置搜索 API key"}

    try:
        async with httpx.AsyncClient(timeout=15.0) as client:
            response = await client.get(
                "https://api.search.code-search.tech/v1/search",
                params={"q": query, "limit": limit},
                headers={"Authorization": f"Bearer {api_key}"},
            )
            data = response.json()
            return {"query": query, "results": data.get("results", []), "total": len(data.get("results", []))}
    except Exception as e:
        return {"error": str(e)}


# ============================================================
# 注册所有工具
# ============================================================

def get_default_tools() -> List[Tool]:
    return [
        Tool(
            name="read_file",
            description="读取文件内容，支持大文件截断",
            parameters={
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "limit": {"type": "integer", "description": "最大读取字符数，默认 500", "default": 500},
                },
                "required": ["path"],
            },
            function=read_file,
        ),
        Tool(
            name="write_file",
            description="创建或覆写文件内容",
            parameters={
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "content": {"type": "string", "description": "文件内容"},
                },
                "required": ["path", "content"],
            },
            function=write_file,
        ),
        Tool(
            name="list_directory",
            description="列出目录下的文件和子目录",
            parameters={
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "目录路径，默认为当前目录", "default": "."},
                },
                "required": [],
            },
            function=list_directory,
        ),
        Tool(
            name="search_files",
            description="在目录中递归搜索文件内容（支持正则）",
            parameters={
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "搜索关键词或正则表达式"},
                    "path": {"type": "string", "description": "搜索目录，默认为当前目录", "default": "."},
                    "file_glob": {"type": "string", "description": "文件类型过滤，如 *.py", "default": None},
                    "limit": {"type": "integer", "description": "最大结果数，默认 50", "default": 50},
                },
                "required": ["pattern"],
            },
            function=search_files,
        ),
        Tool(
            name="run_command",
            description="执行 Shell 命令",
            parameters={
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "要执行的命令"},
                    "timeout": {"type": "integer", "description": "超时秒数，默认 60", "default": 60},
                },
                "required": ["command"],
            },
            function=run_command,
        ),
        Tool(
            name="calculate",
            description="计算数学表达式",
            parameters={
                "type": "object",
                "properties": {
                    "expression": {"type": "string", "description": "数学表达式，如 1+1 或 2**10"},
                },
                "required": ["expression"],
            },
            function=calculate,
        ),
        Tool(
            name="get_time",
            description="获取当前时间",
            parameters={
                "type": "object",
                "properties": {
                    "timezone": {"type": "string", "description": "时区，默认 Asia/Shanghai", "default": "Asia/Shanghai"},
                },
                "required": [],
            },
            function=get_time,
        ),
        Tool(
            name="web_search",
            description="搜索网页（需要 SEARCH_API_KEY 环境变量）",
            parameters={
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "搜索关键词"},
                    "limit": {"type": "integer", "description": "结果数量，默认 5", "default": 5},
                },
                "required": ["query"],
            },
            function=web_search,
        ),
    ]
