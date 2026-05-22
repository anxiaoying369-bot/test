#!/usr/bin/env python3
"""
Function Calling CLI - 独立运行的 AI Function Calling 对话工具

Usage:
    python funcall.py --api-key YOUR_API_KEY --model gpt-4o
    python funcall.py --api-key YOUR_API_KEY --base-url https://api.openai.com/v1 --model gpt-4o
    python funcall.py --api-key YOUR_API_KEY --model gpt-4o --system "你是我的助手"

Examples:
    > 帮我列出当前目录的文件
    > 计算 1+1 等于多少
    > 用一句话总结 /tmp/test.txt 的内容
    > 在当前目录创建一个名为 hello.py 的文件，内容是打印 hello world
"""

import argparse
import asyncio
import inspect
import json
import os
import re
import subprocess
import sys
import textwrap
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


# ============================================================
# Function Calling Service
# ============================================================

class FunctionCallingCLI:
    """轻量级 Function Calling CLI 运行时"""

    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.openai.com/v1",
        model: str = "gpt-4o",
        timeout: float = 60.0,
        system_prompt: Optional[str] = None,
        tools: Optional[List[Tool]] = None,
    ) -> None:
        self.api_key = api_key
        self.base_url = base_url.rstrip("/")
        self.model = model
        self.timeout = timeout
        self.tools = {t.name: t for t in (tools or [])}
        self.conversation: List[Dict[str, Any]] = []
        if system_prompt:
            self.conversation.append({"role": "system", "content": system_prompt})

    def register_tool(self, tool: Tool) -> None:
        self.tools[tool.name] = tool

    def _format_result(self, result: Any) -> str:
        """格式化工具结果为字符串"""
        if isinstance(result, dict) and "error" in result:
            return f"❌ 错误: {result['error']}"
        if isinstance(result, dict):
            lines = []
            for k, v in result.items():
                v_str = str(v)[:500]
                lines.append(f"  {k}: {v_str}")
            return "\n".join(lines)
        return str(result)

    async def chat(self, user_message: str, max_iterations: int = 10) -> Dict[str, Any]:
        """单轮对话，自动执行工具调用"""
        self.conversation.append({"role": "user", "content": user_message})
        conversation = self.conversation.copy()
        iteration = 0
        tool_calls_history: List[Dict[str, Any]] = []

        try:
            while iteration < max_iterations:
                iteration += 1
                tools_definitions = [t.to_openai_format() for t in self.tools.values()]

                async with httpx.AsyncClient(timeout=self.timeout) as client:
                    response = await client.post(
                        f"{self.base_url}/chat/completions",
                        headers={
                            "Authorization": f"Bearer {self.api_key}",
                            "Content-Type": "application/json",
                        },
                        json={
                            "model": self.model,
                            "messages": conversation,
                            "tools": tools_definitions,
                            "tool_choice": "auto",
                        },
                    )

                if response.status_code != 200:
                    error = response.text[:400]
                    return {"success": False, "error": f"API 错误 ({response.status_code}): {error}"}

                result = response.json()
                message = result["choices"][0]["message"]
                conversation.append(message)

                finish_reason = result["choices"][0].get("finish_reason")

                if message.get("content"):
                    assistant_content = message["content"]
                    self.conversation = conversation
                    return {
                        "success": True,
                        "message": assistant_content,
                        "tool_calls": tool_calls_history,
                        "iterations": iteration,
                    }

                if finish_reason == "tool_calls" and "tool_calls" in message:
                    tool_results = []
                    for tool_call in message["tool_calls"]:
                        tool_name = tool_call["function"]["name"]
                        tool_args = json.loads(tool_call["function"]["arguments"])
                        tool_call_id = tool_call["id"]

                        print(f"\n🔧 调用工具: {tool_name}({json.dumps(tool_args, ensure_ascii=False)})")

                        if tool_name not in self.tools:
                            tool_result = {"error": f"未知工具: {tool_name}"}
                        else:
                            tool_result = await self.tools[tool_name].execute(**tool_args)

                        print(f"📤 结果: {self._format_result(tool_result)}")

                        tool_results.append({
                            "tool_call_id": tool_call_id,
                            "role": "tool",
                            "name": tool_name,
                            "content": json.dumps(tool_result, ensure_ascii=False),
                        })
                        tool_calls_history.append({
                            "name": tool_name,
                            "arguments": tool_args,
                            "result": tool_result,
                        })

                    conversation.extend(tool_results)
                    continue

                if finish_reason == "stop":
                    self.conversation = conversation
                    return {
                        "success": True,
                        "message": message.get("content", ""),
                        "tool_calls": tool_calls_history,
                        "iterations": iteration,
                    }

            return {
                "success": True,
                "message": "达到最大迭代次数",
                "tool_calls": tool_calls_history,
                "iterations": iteration,
            }

        except Exception as exc:
            return {"success": False, "error": str(exc), "tool_calls": tool_calls_history}


# ============================================================
# CLI 入口
# ============================================================

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Function Calling CLI - AI Function Calling 对话工具",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent("""
            示例:
              python funcall.py --api-key sk-xxx --model gpt-4o
              python funcall.py --api-key sk-xxx --base-url https://api.siliconflow.cn/v1 --model Qwen/Qwen2.5-72B-Instruct
              python funcall.py --api-key sk-xxx --model gpt-4o --system "你是一个代码审查助手"
              echo "列出文件" | python funcall.py --api-key sk-xxx --model gpt-4o

            工具列表:
              - read_file(path, limit=500): 读取文件
              - write_file(path, content): 写入文件
              - list_directory(path="."): 列出目录
              - search_files(pattern, path=".", file_glob=None, limit=50): 搜索文件
              - run_command(command, timeout=60): 执行命令
              - calculate(expression): 数学计算
              - get_time(timezone="Asia/Shanghai"): 获取时间
              - web_search(query, limit=5): 网页搜索
        """),
    )
    parser.add_argument("--api-key", "-k", required=True, help="API Key")
    parser.add_argument("--base-url", "-b", default="https://api.openai.com/v1", help="API 基础 URL (默认: OpenAI)")
    parser.add_argument("--model", "-m", default="gpt-4o", help="模型名称")
    parser.add_argument("--system", "-s", default=None, help="系统提示词")
    parser.add_argument("--timeout", "-t", type=float, default=60.0, help="请求超时秒数 (默认: 60)")
    parser.add_argument("--max-iter", "-i", type=int, default=10, help="最大迭代次数 (默认: 10)")
    parser.add_argument("--session-file", default=None, help="会话历史文件路径")
    return parser


def load_session(path: str) -> List[Dict[str, Any]]:
    """加载历史会话"""
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return []


def save_session(path: str, conversation: List[Dict[str, Any]]) -> None:
    """保存会话历史"""
    try:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(conversation, f, ensure_ascii=False, indent=2)
    except Exception as e:
        print(f"⚠️  保存会话失败: {e}", file=sys.stderr)


async def interactive(cli: FunctionCallingCLI, session_file: Optional[str], max_iterations: int):
    """交互式对话循环"""
    if session_file:
        history = load_session(session_file)
        if history:
            cli.conversation = history
            print(f"📂 已加载历史会话 ({len(history)} 条消息)")

    print("\n" + "=" * 60)
    print("Function Calling CLI")
    print("=" * 60)
    print(f"模型: {cli.model}")
    print(f"工具: {', '.join(cli.tools.keys())}")
    print("输入 exit() 或 quit() 退出\n")

    while True:
        try:
            user_input = input("\n👤 你: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n\n👋 再见!")
            break

        if not user_input:
            continue
        if user_input.lower() in ("exit()", "quit()", "exit", "quit"):
            print("👋 再见!")
            break

        print()
        result = await cli.chat(user_input, max_iterations=max_iterations)

        if result.get("success"):
            if result.get("tool_calls"):
                print(f"\n🤖 AI: 已执行 {len(result['tool_calls'])} 个工具调用")
            if result.get("message"):
                print(f"\n🤖 AI: {result['message']}")
        else:
            print(f"\n❌ 错误: {result.get('error')}")

        if session_file:
            save_session(session_file, cli.conversation)


async def main():
    parser = build_parser()
    args = parser.parse_args()

    tools = get_default_tools()

    cli = FunctionCallingCLI(
        api_key=args.api_key,
        base_url=args.base_url,
        model=args.model,
        timeout=args.timeout,
        system_prompt=args.system,
        tools=tools,
    )

    # 非交互式模式（从 stdin 读取单条消息）
    if not sys.stdin.isatty():
        user_input = sys.stdin.read().strip()
        if user_input:
            result = await cli.chat(user_input, max_iterations=args.max_iter)
            if result.get("success"):
                print(result.get("message", ""))
            else:
                print(f"错误: {result.get('error')}", file=sys.stderr)
                sys.exit(1)
        return

    await interactive(cli, args.session_file, args.max_iter)


if __name__ == "__main__":
    asyncio.run(main())
