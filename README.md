# AutoCast AI

AutoCast AI 是一个面向抖音内容运营与数据采集的**全自包含（Self-contained）**桌面端中控系统。它集成了账号管理、内容采集、AI 创作以及直播间实时监控等功能，旨在通过 AI 自动化技术提升短视频运营效率。

## 🌟 核心特性

- **一键开箱即用**：内置 Python 3.11、Node.js 20、FFmpeg 运行时，打包后无需用户手动配置任何环境变量。
- **跨平台支持**：原生支持 Windows 和 macOS，并针对 Windows 下的子进程渲染做了深度优化（静默运行，无黑框）。
- **账号全生命周期管理**：基于 DrissionPage + Chrome CDP 的浏览器接管模式，实现安全、直观的扫码授权与 Cookie 验证。
- **多维度数据采集**：自动化采集作品、评论、回复，并支持基于 LLM 的评论情感分析与互动策略生成。
- **AI 视频创作中心**：内置视频自动化剪辑、字幕烧录、TTS 合成，支持 fal.ai / 火山引擎等多种 AI 提供商。
- **直播实时监控**：毫秒级捕获直播间弹幕、礼物、入场消息，支持 AI 辅助实时回复生成。

## 🏗️ 架构设计

- **Frontend**: Vue 3 + Vite 6 + Tailwind CSS v4 + Lucide Icons
- **Backend**: Tauri 2 (Rust) - 负责进程调度、本地文件管理及 API 代理。
- **Automation**: Python 3.11 - 负责核心爬虫、加密签名与浏览器自动化。
- **Encryption**: Node.js - 运行 X-Bogus 等加密签名算法。
- **Storage**: SQLite (本地关系数据) + LanceDB (向量数据库) + JSONL (流式日志)。

## 🚀 快速开始

### 1. 环境准备 (开发者)

克隆项目后，首先执行环境准备脚本：

```bash
# 安装 Node 依赖
npm install

# 自动准备 Python, Node, FFmpeg 便携式环境 (重要)
npm run prepare:all
```

### 2. 运行开发版本

```bash
npm run tauri dev
```

### 3. 构建发布包

```bash
# Windows 下将生成 .exe 安装程序 (NSIS)
# macOS 下将生成 .dmg 或 .app
npm run tauri build
```

## 🛠️ Windows 兼容性说明

项目已针对 Windows 进行专项优化：
- **浏览器探测**：自动识别 Chrome (标准/x86/Local) 及 Microsoft Edge。
- **路径处理**：内置自动转义逻辑，完美解决 FFmpeg 滤镜在 Windows 盘符下的路径报错。
- **静默后台**：所有 Python/Node/FFmpeg 进程均在后台静默运行，不会产生控制台窗口干扰。

## 📅 开发计划 (Factory System)

目前正处于「工厂系统」迭代阶段，核心目标是实现多设备主从架构与知识库分层同步。详情请参见 [FACTORY_SYSTEM_PLAN.md](./FACTORY_SYSTEM_PLAN.md)。

## 📄 开源说明

本项目仅供学习与研究使用，请遵守抖音平台相关开发者规范及爬虫使用道德。
