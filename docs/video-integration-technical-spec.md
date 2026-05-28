# AutoCast AI 视频集成方案 (Video Integration)

## 1. 架构概览

AutoCast AI 的视频生成与剪辑模块采用 **Vue 3 + Rust + Python** 的三层架构实现。

### 1.1 模块分工
- **前端 (Vue 3)**: `src/components/VideoStudioView.vue`
  - 负责项目管理 UI、脚本编辑、生成任务发起、素材预览及剪辑流程控制。
- **后端 (Rust/Tauri)**: `src-tauri/src/ffmpeg.rs`, `db.rs`, `lib.rs`
  - **FFmpeg 调度**: 负责二进制路径解析、子进程调用、进度实时反馈。
  - **任务系统**: 异步管理 AI 生成及 FFmpeg 渲染任务，状态持久化至 SQLite。
  - **数据持久化**: 使用 `video_studio.db` 管理项目、素材和任务记录。
- **AI 引擎 (Python)**: `scripts/video_providers/`, `video_manager.py`
  - **Provider 抽象**: 统一不同 AI 视频服务（如 fal.ai/Luma）的调用接口。
  - **异步对接**: 负责与云端 AI 服务通信并返回任务 ID。

## 2. 核心技术实现

### 2.1 FFmpeg 运行时
- **分发方式**: 静态二进制文件打包至 `src-tauri/ffmpeg-runtime/`。
- **准备脚本**: `src-tauri/scripts/prepare-ffmpeg.sh` (macOS/Linux) 和 `.ps1` (Windows)。
- **进度解析**: 通过解析 FFmpeg stderr 输出中的 `time=` 和 `speed=` 字段，实时向前端推送 `video-ffmpeg-progress` 事件。

### 2.2 数据库 Schema
- **`video_projects`**: 存储项目元数据及配置 (JSON)。
- **`video_materials`**: 记录本地素材路径及远程源 URL。
- **`video_tasks`**: 追踪生成、剪辑、导出任务的实时状态。

### 2.3 AI Provider 插件化
- 位于 `scripts/video_providers/`。
- 新增 Provider 只需继承 `VideoProvider` 基类并实现 `text_to_video` 和 `poll_task` 接口。

## 3. 开发者指南

### 3.1 环境准备
```bash
# 准备所有运行时（Python + FFmpeg）
npm run prepare:all
```

### 3.2 新增剪辑功能
- 剪辑指令位于 `src-tauri/src/lib.rs`。
- 复杂的 FFmpeg 滤镜链建议在 Rust 端组装后调用 `ffmpeg::run_ffmpeg_with_progress`。

### 3.3 状态流转
1. 前端调用 `video_start_generation`。
2. Rust 记录任务并调用 Python 发起 AI 任务。
3. 前端或 Rust 定时调用 `video_poll_task_status`。
4. 任务完成后，Rust 触发 `video_download_material` 将结果本地化。
