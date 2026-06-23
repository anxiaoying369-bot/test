# AutoCast AI

AutoCast AI 是一套面向新媒体与短视频运营的本地桌面端中控系统。它把账号管理、抖音数据采集、企业知识库（RAG）、AI 内容创作、视频生成、发布排期、直播监控、微信消息监控、手机远程控制与 Hermes Agent 智能体网关整合到一个 Tauri 桌面应用中，目标是用 AI 与自动化流程提升内容生产和运营效率。

> 一次安装、开箱即用：应用内置 Python / Node.js / FFmpeg 等便携式运行时，终端用户无需手动配置开发环境。

## 🌟 核心特性

- **本地桌面中控**：基于 Tauri 2 + Vue 3，前端交互轻量，后端负责本地进程调度、文件管理、数据库与自动化任务。
- **开箱即用运行时**：内置 Python 3.11、Node.js 20、FFmpeg，支持 Windows / macOS 打包分发。
- **账号全生命周期管理**：通过 DrissionPage + Chrome CDP 接管模式进行扫码授权、Cookie 验证、登录凭证复用与账号状态管理。
- **抖音用户信息查询**：支持 sec_uid、主页链接、分享短链查询用户卡片，收录后可一键跳转到评论采集。
- **多维度数据采集**：采集作品、评论、回复，并支持采集结果浏览、筛选与基于 LLM 的评论分析。
- **企业知识库（RAG）**：上传 PDF / Word / Excel / TXT / JSON 等资料，自动切片、向量化并为 AI 助理与创作流程提供事实依据。
- **AI 助理对话**：结合知识库进行多轮问答，任务后台执行，切换页面不中断。
- **AI 创作中心**：围绕产品、场景和平台风格生成口播脚本、表演脚本、内容素材与可审计的创作结果。
- **视频创作中心**：集成 MoneyPrinterTurbo 风格的视频生成流程，支持脚本、关键词、配音、字幕、素材检索/本地素材、MoviePy/FFmpeg 拼接与成片导出。
- **发布排期**：支持工作室成片或本地上传视频发布到抖音创作者中心，提供立即发布、定时发布、多账号矩阵分发与后台调度。
- **直播实时监控**：捕获直播间弹幕、礼物、入场等事件，支持 AI 辅助回复建议与观众信息沉淀。
- **微信监控**：支持微信会话监控、联系人/群聊过滤、图片/视频/语音消息读取、语音播放与 SenseVoiceSmall 语音转文字。
- **手机控制**：通过 AutoCast Mobile 连接局域网手机，支持设备列表、截图、实时画面、点击/滑动/按键、无线 ADB 与通话录音同步。
- **Hermes Agent 网关**：内置可选智能体入口，可通过对话调用专业技能和工具，扩展运营与自动化能力。
- **可定制提示词与模型配置**：LLM、Embedding、TTS、视频生成引擎、直播回复、脚本生成、数据分析等配置均可在系统设置中管理。

## 🧩 功能模块总览

| 模块 | 入口视图 | 说明 |
| --- | --- | --- |
| AI 助理对话 | `ChatView` | 结合企业知识库的多轮对话，后台执行不中断 |
| AI 创作中心 | `ContentStudioView` | 生成口播/表演脚本，注入知识库事实与平台风格 |
| 视频创作中心 | `VideoStudioView` | 脚本、关键词、配音、字幕、素材、剪辑与成片导出 |
| 账号管理 | `AccountsView` | 抖音账号扫码登录、Cookie 验证与账号生命周期管理 |
| 用户信息查询 | `UserInfoView` | 查询并收录抖音用户卡片，支持跳转评论采集 |
| 评论采集 | `ScraperView` | 按博主采集作品、评论和回复，任务进度可追踪 |
| 采集结果 & AI 分析 | `ResultsView` | 浏览采集结果，对评论做情感与互动策略分析 |
| 企业知识库 | `KnowledgeBaseView` | 文档上传、切片、向量化索引与片段查看 |
| 直播监控 | `LiveMonitorView` | 实时弹幕、礼物、入场消息捕获与 AI 回复建议 |
| 微信监控 | `WeChatMonitorView` | 微信联系人/群聊监控、媒体解析、语音播放和转写 |
| 手机控制 | `MobileControlView` | 局域网手机接入、截图串流、远程点击/滑动/按键、录音同步 |
| 发布排期 | `PublishView` | 本地/成片视频发布，支持立即/定时与多账号矩阵分发 |
| Hermes Agent | `HermesGatewayView` | 智能体网关，对接外部工具与专业技能 |
| 系统设置 | `SettingsView` | LLM、知识库、TTS、提示词、视频引擎、Hermes 等配置 |

## 🏗️ 技术架构

- **Frontend**：Vue 3 + Vite 6 + TypeScript + Tailwind CSS v4 + Lucide Icons
- **Desktop Runtime**：Tauri 2 (Rust)
- **Backend Orchestration**：Rust commands + Tokio 异步任务调度
- **Automation**：Python 3.11 + DrissionPage + Chrome CDP
- **Encryption / Signing**：Node.js 运行 X-Bogus 等签名相关逻辑
- **Media Pipeline**：FFmpeg / MoviePy / Edge TTS / Whisper 或 Edge 字幕
- **Storage**：SQLite（业务数据）+ LanceDB（向量数据）+ JSON/JSONL（任务与日志）
- **Mobile Gateway**：局域网设备网关 + AutoCast Mobile + ADB / Accessibility 能力
- **Agent Gateway**：Hermes Agent 作为可选扩展入口

## 📁 目录结构

```text
.
├── src/                         # Vue 前端
│   ├── components/              # 各业务页面与组件
│   ├── composables/             # 前端状态与 Tauri command 封装
│   ├── lib/                     # 通用前端工具
│   └── types/                   # TypeScript 类型
├── src-tauri/                   # Tauri / Rust 后端
│   ├── src/commands/            # Tauri commands：账号、采集、发布、手机、微信等
│   ├── src/device_gateway.rs    # 手机设备网关
│   ├── src/device_manager.rs    # 手机设备状态管理
│   ├── python-runtime/          # 便携式 Python 运行时（prepare 后生成）
│   ├── node-runtime/            # 便携式 Node.js 运行时（prepare 后生成）
│   └── ffmpeg-runtime/          # 便携式 FFmpeg 运行时（prepare 后生成）
├── scripts/                     # Python / Node 自动化脚本与视频生成引擎
│   ├── DouyinComment/           # 抖音评论采集相关脚本
│   ├── DouyinBarrage/           # 抖音直播弹幕相关脚本
│   ├── wechat/                  # 微信监控与媒体解析脚本
│   └── mpt_engine/              # 视频生成引擎资源
├── autocast-mobile/             # 手机端配套应用/服务相关代码
├── package.json                 # 前端依赖与 npm scripts
└── README.md
```

## 🚀 快速开始

### 1. 安装 Node 依赖

```bash
npm install
```

### 2. 准备便携式运行时（首次必跑）

```bash
npm run prepare:all
```

`prepare:all` 会下载并解压 Python / Node.js / FFmpeg 到 `src-tauri/{python,node,ffmpeg}-runtime/<platform>/`。

也可以按需单独执行：

```bash
npm run prepare:python
npm run prepare:node
npm run prepare:ffmpeg
```

### 3. 运行开发版本

```bash
npm run tauri dev
```

### 4. 构建发布包

```bash
npm run tauri build
```

构建目标：

- Windows：NSIS `.exe` 安装程序
- macOS：`.dmg` 与 `.app`

## ⚙️ 配置说明

首次使用请在「系统设置」中完成以下配置：

- **AI 模型（LLM）**：填写 `api_key`、`base_url`、`model`，用于 AI 助理、评论分析、脚本生成等。
- **知识库嵌入模型**：可单独配置 `kb_api_key`、`kb_base_url`、`embedding_model`；留空时回退使用主 LLM 配置。
- **TTS 语音合成**：选择 OpenAI / MiniMax / Volcengine / Mock 等服务商，并配置密钥、音色、语速等参数。
- **视频生成引擎**：配置 Pexels API Key、默认 Edge 配音音色、字幕方式、素材来源等。
- **提示词管理**：直播回复、脚本生成、数据分析、创作中心等提示词可编辑，并支持恢复默认。
- **Prosody Tags**：可配置服务商支持的语气/声调标签，让表演脚本生成时严格按可用标签输出。
- **Hermes Agent**：按需配置 Hermes 网关，用于扩展智能体工具与技能。
- **手机控制**：手机端需与电脑处于同一局域网，并开启 AutoCast Mobile 与必要的无障碍/ADB 权限。

## 🧪 常用开发命令

```bash
# 前端开发服务器
npm run dev

# 类型检查并构建前端
npm run build

# Tauri 开发模式
npm run tauri dev

# Tauri 打包
npm run tauri build

# 准备所有便携式运行时
npm run prepare:all
```

## 🛠️ Windows 兼容性说明

项目已针对 Windows 做专项优化：

- **浏览器探测**：自动识别 Chrome（标准 / x86 / Local）及 Microsoft Edge。
- **静默后台**：Python / Node / FFmpeg 子进程后台静默运行，避免弹出黑色控制台窗口。
- **路径处理**：对 FFmpeg 滤镜路径、Windows 盘符与中文路径做兼容处理。
- **资源打包**：通过 Tauri bundle resources 注入脚本、运行时、视频引擎资源与微信/抖音相关脚本。
- **老设备兼容**：Windows 工控机或旧 CPU 环境需注意 Python 依赖的指令集兼容性。

## 🍎 macOS 打包提示

若打包在 `bundle_dmg.sh` 处失败，通常是上一次中断的打包在 `/Volumes` 下残留了挂载卷。清理后重试：

```bash
for v in $(ls /Volumes/ | grep -E '^dmg\.'); do hdiutil detach "/Volumes/$v" -force; done
rm -f src-tauri/target/release/bundle/macos/rw.*.dmg
```

## 🔐 数据与隐私

- 应用以本地桌面端为主，账号凭证、采集结果、知识库、任务记录等默认存储在本机应用数据目录。
- 抖音、微信、手机控制等能力依赖用户主动授权、扫码登录或本机/局域网设备连接。
- 请妥善保管 API Key、Cookie、模型密钥和本地数据库文件。

## 📄 使用声明

本项目仅供学习、研究与合规的内部运营自动化使用。请遵守目标平台服务条款、开发者规范、数据合规要求与爬虫使用边界，不得用于未授权的数据获取、骚扰、垃圾营销或其他违规场景。
