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

from funcall_tools import Tool, get_default_tools


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
