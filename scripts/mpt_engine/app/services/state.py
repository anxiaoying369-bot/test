"""
AutoCast AI 适配层：把 MoneyPrinterTurbo 的任务状态管理改为 JSONL 进度流。

原 MPT 用内存 / Redis 保存任务状态，供 WebUI / API 轮询。在 AutoCast AI 里，
视频生成是 Rust 拉起的一次性子进程，进度通过 stdout 的 JSONL 实时回传给 Rust，
再由 Rust `emit` 给前端。因此这里用 `JsonlState` 替换原来的 Memory/Redis 实现：
每次 `update_task` 都向 stdout 打印一行进度 JSON 并 flush。

stdout 协议（每行一个 JSON 对象）：
    {"event": "progress", "state": 4, "progress": 30}
非进度的业务日志一律走 stderr（见 app/config/__init__.py），不污染本协议。
"""

import json
import sys

from app.models import const


def _emit(payload: dict) -> None:
    try:
        sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
        sys.stdout.flush()
    except Exception:
        # 进度回传失败不应中断视频生成主流程。
        pass


class JsonlState:
    """把任务进度以 JSONL 形式写到 stdout 的最小状态实现。"""

    def __init__(self):
        self._tasks = {}

    def update_task(
        self,
        task_id: str,
        state: int = const.TASK_STATE_PROCESSING,
        progress: int = 0,
        **kwargs,
    ):
        progress = int(progress)
        if progress > 100:
            progress = 100

        self._tasks[task_id] = {
            "task_id": task_id,
            "state": state,
            "progress": progress,
            **kwargs,
        }

        # 仅回传轻量进度字段；script / videos 等大字段不进度流，由编排脚本
        # 在结束时用 task.start() 的返回值统一输出 done 行。
        _emit({"event": "progress", "state": state, "progress": progress})

    def get_task(self, task_id: str):
        return self._tasks.get(task_id, None)

    def delete_task(self, task_id: str):
        self._tasks.pop(task_id, None)

    def get_all_tasks(self, page: int = 1, page_size: int = 20):
        tasks = list(self._tasks.values())
        return tasks, len(tasks)


# 全局状态对象，task.py 通过 `from app.services import state as sm; sm.state...` 使用。
state = JsonlState()
