import sys
import os
import json
import subprocess

def create_marketing_video(prompt: str, ratio: str = "9:16", project_id: str = None):
    """
    Hermes Skill 工具实现：调用 AutoCast AI 核心引擎发起生成。
    """
    # 查找项目根目录（假设 skill 在 scripts/hermes_skills/video-producer/）
    scripts_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    manager_py = os.path.join(scripts_dir, "video_manager.py")
    
    # 鉴权 Key 应由环境变量或配置注入，这里假设系统已配置
    # 注意：在真实的 Hermes Skill 环境中，可能需要通过特定方式读取主应用的配置
    # 暂时使用 mock provider 进行链路演示
    
    cmd = [
        sys.executable, manager_py, "start",
        "--provider", "mock",
        "--api-key", "sk-mock-key",
        "--prompt", prompt,
        "--ratio", ratio
    ]
    
    try:
        res = subprocess.check_output(cmd, stderr=subprocess.STDOUT)
        data = json.loads(res)
        return f"✅ 视频生成任务已发起！\n任务 ID: {data['task_id']}\n状态: 正在处理中...\n您可以稍后在 'AI 视频创作中心' 查看进度。"
    except Exception as e:
        return f"❌ 发起生成失败: {str(e)}"

if __name__ == "__main__":
    # 处理 Hermes 的 JSON-RPC 调用
    # 简化实现，仅做工具导出逻辑说明
    pass
