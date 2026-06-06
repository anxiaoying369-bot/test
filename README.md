# AutoCast AI

AutoCast AI 是一个面向抖音内容运营与数据采集的 **全自包含（Self-contained）** 桌面端中控系统。它将账号管理、数据采集、企业知识库（RAG）、AI 创作与视频生产、直播间实时监控整合到一个桌面应用中，旨在通过 AI 自动化提升短视频运营效率。

> 一次安装、开箱即用：内置 Python / Node.js / FFmpeg 便携式运行时，终端用户无需配置任何环境。

## 🌟 核心特性

- **一键开箱即用**：内置 Python 3.11、Node.js 20、FFmpeg 运行时，打包后无需用户手动配置环境变量。
- **跨平台支持**：原生支持 Windows（NSIS 安装包）和 macOS（.dmg / .app），针对 Windows 子进程做了静默运行优化（无黑框）。
- **账号全生命周期管理**：基于 DrissionPage + Chrome CDP 的浏览器接管模式，实现安全直观的扫码授权与 Cookie 验证。
- **多维度数据采集**：自动采集作品、评论、回复，支持基于 LLM 的评论情感分析与互动策略生成。
- **企业知识库（RAG）**：上传 PDF / Word / Excel / TXT / JSON，自动切片向量化，为 AI 创作与对话提供专业背景事实。
- **AI 助理对话**：结合企业知识库的多轮对话，任务在后台执行，切换页面不中断。
- **AI 视频创作中心（MoneyPrinterTurbo 引擎）**：输入主题/产品 → AI 生成口播脚本 → 关键词 → Edge TTS 配音 → 字幕（Edge/Whisper）→ 自动下载 Pexels 免版权素材或使用本地素材 → MoviePy/FFmpeg 拼接 + 字幕烧录 + 背景音乐，一键成片。
- **可定制提示词**：直播回复、脚本生成、数据分析等提示词均可在设置页编辑，并支持一键恢复默认。
- **TTS 语气与声调标注**：可自定义服务商支持的 Prosody Tags，LLM 生成「表演脚本」时严格遵循，提升配音表现力。
- **直播实时监控**：毫秒级捕获直播间弹幕、礼物、入场消息，支持 AI 辅助实时回复生成。

## 🧩 功能模块总览

| 模块 | 入口视图 | 说明 |
| --- | --- | --- |
| 账号管理 | `AccountsView` | 多平台账号扫码登录、Cookie 验证与生命周期管理 |
| 评论采集 | `ScraperView` | 按博主采集作品 / 评论 / 回复，后台任务进度可追踪 |
| 采集结果 & AI 分析 | `ResultsView` | 浏览采集数据，对单条作品评论做 AI 情感与策略分析 |
| 企业知识库 | `KnowledgeBaseView` | 文档上传、向量化索引、切片查看（LanceDB） |
| AI 助理 | `ChatView` | 结合知识库的多轮对话，后台执行不中断 |
| 视频创作中心 | `ContentStudioView` | 口播 / 表演脚本生成，注入知识库事实与平台风格 |
| 视频工作室 | `VideoStudioView` | 四步成片：脚本 → 关键词 → 参数 → 生成。MoneyPrinterTurbo 引擎（Edge TTS + 字幕 + Pexels/本地素材 + FFmpeg 拼接） |
| 直播监控 | `LiveMonitorView` | 实时弹幕 / 礼物捕获与 AI 回复建议 |
| Hermes 网关 | `HermesGatewayView` | 可选的智能体网关，扩展技能与工具 |
| 系统设置 | `SettingsView` | LLM / 知识库 / TTS 配置与各类提示词管理 |

## 🏗️ 架构设计

- **Frontend**：Vue 3 + Vite 6 + Tailwind CSS v4 + Lucide Icons
- **Backend**：Tauri 2 (Rust) —— 负责进程调度、本地文件管理及 API 代理
- **Automation**：Python 3.11 —— 核心爬虫、加密签名与浏览器自动化
- **Encryption**：Node.js —— 运行 X-Bogus 等加密签名算法
- **Storage**：SQLite（本地关系数据）+ LanceDB（向量数据库）+ JSONL（流式日志）

## 🚀 快速开始

### 1. 环境准备（开发者）

```bash
# 安装 Node 依赖
npm install

# 自动准备 Python / Node / FFmpeg 便携式运行时（重要，首次必跑）
npm run prepare:all
```

> `prepare:all` 会下载并解压三套便携式运行时到 `src-tauri/{python,node,ffmpeg}-runtime/<platform>/`。
> 也可单独执行 `npm run prepare:python` / `prepare:node` / `prepare:ffmpeg`。

### 2. 运行开发版本

```bash
npm run tauri dev
```

### 3. 构建发布包

```bash
# Windows：生成 .exe 安装程序 (NSIS)
# macOS ：生成 .dmg 与 .app
npm run tauri build
```

#### macOS 打包提示：清理残留 DMG 卷

若打包在 `bundle_dmg.sh` 处失败，通常是上一次中断的打包在 `/Volumes` 下残留了挂载卷导致冲突。清理后重试即可：

```bash
# 卸载所有残留的 dmg.* 卷
for v in $(ls /Volumes/ | grep -E '^dmg\.'); do hdiutil detach "/Volumes/$v" -force; done
# 删除残留的临时 dmg
rm -f src-tauri/target/release/bundle/macos/rw.*.dmg
```

## ⚙️ 配置说明

首次使用请在「系统设置」中完成以下配置：

- **AI 模型（LLM）**：`api_key`、`base_url`、`model`，用于评论分析、AI 助理、脚本生成等。
- **知识库嵌入**：可单独配置 `kb_api_key` / `kb_base_url` / `embedding_model`（默认 `text-embedding-3-small`）；留空则回退使用主 LLM 配置。
- **TTS 语音合成**：选择 `tts_provider`（openai / minimax / volcengine / mock）并填写对应密钥、音色与语速。
- **视频生成引擎（MoneyPrinterTurbo）**：在「系统设置 → 视频生成引擎」中填写 **Pexels API Key**（[免费申请](https://www.pexels.com/api/)，用于在线素材检索；仅用本地素材可留空），并选择默认 Edge 配音音色与字幕方式（Edge 免费默认 / Whisper 更精准但需下模型）。脚本与关键词生成复用上方「AI 模型（LLM）」配置。
- **提示词**：直播回复、脚本生成、数据分析提示词均可自定义，并可点击「恢复默认」还原。
- **TTS 语气与声调标注（Prosody Tags）**：填入服务商支持的标签后，LLM 在生成「表演脚本」时会严格且仅使用这些标注；留空则不添加任何语气/声调标注。

## 🛠️ Windows 兼容性说明

项目已针对 Windows 做专项优化：

- **浏览器探测**：自动识别 Chrome（标准 / x86 / Local）及 Microsoft Edge。
- **路径处理**：内置自动转义逻辑，解决 FFmpeg 滤镜在 Windows 盘符下的路径报错。
- **静默后台**：所有 Python / Node / FFmpeg 进程在后台静默运行，不产生控制台窗口。
- **平台化资源打包**：通过 `tauri.windows.conf.json` 注入 Windows 专属运行时资源路径，避免跨平台 glob 校验失败。

## 📅 开发计划

下一阶段规划详见 [VERSION_0.2.0_PLAN.md](./VERSION_0.2.0_PLAN.md)。

## 📄 开源说明

本项目仅供学习与研究使用，请遵守抖音平台相关开发者规范及爬虫使用道德。
