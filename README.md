# AutoCast AI

AutoCast AI 是一个面向抖音内容运营与数据采集的桌面端中控系统。项目基于 Tauri 2 + Vue 3 + Tailwind CSS v4 构建，用 Rust 负责桌面应用、进程调度和本地文件管理，用 Python 脚本负责浏览器自动化、评论采集和直播弹幕监控。

当前版本重点覆盖：抖音账号授权、Cookie 有效性验证、作品/评论/回复采集、采集结果查看、评论 AI 分析原型、抖音直播间实时监控。

## 主要功能

### 1. 抖音账号管理

- 通过 Chrome CDP 接管模式打开/复用本机 Chrome。
- 用户在真实浏览器中手动完成抖音登录。
- 登录完成后采集并保存 Cookie、LocalStorage 和基础用户信息。
- 支持多个抖音账号本地管理、同步、验证和删除。
- 账号验证采用两层策略：
  - L1：读取 cookie.json 的 expires 字段做本地过期预检。
  - L2：通过 CDP 向浏览器注入 Cookie，访问抖音页面并读取 DOM/页面状态判断登录是否有效。

### 2. 抖音评论采集

- 按目标用户 sec_uid 采集数据。
- 支持采集类型：
  - 全部：作品 + 评论 + 回复
  - 仅作品
  - 仅评论
  - 仅回复
- 支持采集数量限制。
- 支持跳过已采集数据，避免重复写入。
- Tauri 前端实时轮询任务进度、状态、统计数据和日志。
- 支持取消正在运行的采集任务。

### 3. 采集结果查看

- 查看已采集用户列表。
- 查看用户作品列表、封面、发布时间和评论数量。
- 查看单个作品下的评论列表。
- 可使用已授权账号在浏览器中打开指定抖音作品。
- 内置一个评论 AI 分析弹窗原型，用于展示舆情摘要、情感分布、热议主题和运营建议。

### 4. 抖音直播监控

- 选择本地已授权账号作为 Cookie 来源。
- 输入直播间 web_rid 后启动实时监控。
- 支持最多 10 个直播间并发监控。
- 实时展示聊天、礼物、点赞、进场等事件。
- 直播事件通过 Python stdout JSONL 桥接到 Tauri event，再由 Vue 前端展示。
- 历史事件写入本地 history.jsonl，重新进入页面时可恢复最近记录。

### 5. AI 视频创作中心 (New!)

- **项目管理**：支持创建和管理多个视频创作项目。
- **AI 脚本生成**：对接 fal.ai (Luma Dream Machine) 等视频生成引擎，支持文生视频、图生视频。
- **素材本地化**：自动将 AI 生成的视频素材下载到本地 `video_studio` 目录进行持久化。
- **FFmpeg 合成**：内置 FFmpeg 运行时，支持多段视频素材的高性能首尾拼接。
- **实时进度**：前端实时显示 AI 生成及视频渲染的详细进度。

### 6. 本地优先的数据管理

- 账号、Cookie、采集结果、直播历史和日志均保存在用户本机。
- 不依赖远端数据库。
- 适合本地桌面端运营工作流和自动化调试。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面端 | Tauri 2 |
| 后端 | Rust 2021、Tokio、Serde、Reqwest、UUID、dirs |
| 前端 | Vue 3、Vite 6、TypeScript、Tailwind CSS v4 |
| UI/图标 | Lucide Vue Next、clsx、tailwind-merge |
| 浏览器自动化 | DrissionPage、Chrome CDP、Playwright 依赖预留 |
| 视频处理 | FFmpeg (静态二进制)、fluent-ffmpeg 架构借鉴 |
| 抖音评论采集 | scripts/DouyinComment 子模块 |
| 抖音直播监控 | scripts/DouyinBarrage |
| 数据存储 | 本地 JSON、JSONL、SQLite (rusqlite/video_studio.db) |

## 项目结构

```text
podcast/
├── README.md
├── docs/                               # 方案规划与技术文档
├── scripts/
│   ├── video_providers/                # AI 视频生成 Provider 抽象
│   ├── video_manager.py                # 视频生成任务 Python 调度脚本
│   └── ...                             # 其他采集与监控脚本
├── src/
│   ├── components/
│   │   ├── VideoStudioView.vue         # 视频创作中心 (New!)
│   │   └── ...
├── src-tauri/
│   ├── ffmpeg-runtime/                 # 内置 FFmpeg 二进制目录
│   ├── scripts/                        # 环境准备脚本 (Python/FFmpeg)
│   ├── src/
│   │   ├── ffmpeg.rs                   # FFmpeg 调度模块
│   │   ├── db.rs                       # SQLite 数据库初始化
│   │   └── lib.rs                      # 主指令入口
│   └── ...
└── ...
```

## 本地数据路径

macOS 下数据默认保存在：

```text
~/Library/Application Support/AutoCastAI/
```

常用子路径：

| 数据 | 路径 |
| --- | --- |
| 账号索引 | `~/Library/Application Support/AutoCastAI/accounts.json` |
| 账号 Cookie | `~/Library/Application Support/AutoCastAI/cookies/douyin/{account_name}/` |
| 登录/采集日志 | `~/Library/Application Support/AutoCastAI/logs/` |
| 采集任务进度 | `~/Library/Application Support/AutoCastAI/scraper/{task_id}.json` |
| 采集结果数据 | `~/Library/Application Support/AutoCastAI/scraper_data/` |
| 直播历史 | `~/Library/Application Support/AutoCastAI/live_data/{room_id}/history.jsonl` |

单个账号目录通常包含：

```text
cookie.txt      # 原始 Cookie 字符串，供采集/直播模块使用
cookie.json     # 结构化 Cookie，供 CDP 注入与验证使用
meta.json       # 用户昵称、抖音号、头像等基础信息
```

## 环境要求

- macOS。
- Node.js 和 npm。
- Rust 工具链，用于 Tauri 后端构建。
- Python 3。
- Google Chrome，默认路径：

```text
/Applications/Google Chrome.app/Contents/MacOS/Google Chrome
```

- Python 依赖：
  - DrissionPage
  - DouyinComment 的依赖
  - DouyinBarrage 的依赖

## 安装依赖

### 1. 克隆子模块

```bash
git submodule update --init --recursive
```

当前仓库包含 `scripts/DouyinComment` 子模块。`scripts/DouyinBarrage` 作为项目内模块使用，如果它来自独立仓库，请确保目录内容完整。

### 2. 安装前端依赖

```bash
npm install
```

### 3. 安装 Python 依赖

```bash
python3 -m pip install DrissionPage
python3 -m pip install -r scripts/DouyinComment/requirements.txt
python3 -m pip install -r scripts/DouyinBarrage/requirements.txt
```

如需使用 Playwright 相关能力，可额外安装浏览器依赖：

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

### 4. 安装 Tauri CLI

本项目已在 devDependencies 中声明 `@tauri-apps/cli`，可通过 npm scripts 调用：

```bash
npm run tauri -- --version
```

## 开发运行

```bash
npm run tauri dev
```

Tauri 会先启动 Vite 开发服务，默认端口为 1420，然后打开桌面窗口。

也可以只启动前端开发服务：

```bash
npm run dev
```

## 生产构建

前端类型检查和构建：

```bash
npm run build
```

构建 Tauri 桌面应用：

```bash
npm run tauri build
```

## 使用流程

### 账号授权

1. 打开应用，进入「账号管理」。
2. 点击「新增授权」。
3. 应用会连接或启动带有 `--remote-debugging-port=9222` 的 Chrome。
4. 在 Chrome 中手动完成抖音登录。
5. 回到应用，输入账号名称，点击「我已登录完成」。
6. 应用会保存 Cookie 和账号信息到本地。
7. 点击「验证」可检查 Cookie 是否仍然有效。

Chrome CDP 默认端口为 9222。脚本会优先复用已有端口；如果端口未占用，会用如下用户数据目录启动 Chrome：

```text
~/chrome-debug-profile
```

### 评论采集

1. 进入「评论采集」。
2. 选择已授权的抖音账号。
3. 输入目标用户的 sec_uid。
4. 选择采集类型和数量限制。
5. 点击「开始采集」。
6. 在界面中查看进度、统计和实时日志。
7. 如 Cookie 过期，回到「账号管理」重新授权。

### 查看采集结果

1. 进入「采集结果」。
2. 从用户列表选择一个已采集用户。
3. 查看该用户作品列表。
4. 点击作品进入评论列表。
5. 可点击作品 ID 使用浏览器打开原视频。
6. 可点击「AI 分析」查看当前原型生成的分析报告。

### 直播监控

1. 进入「直播监控」。
2. 选择一个抖音账号作为 Cookie 来源。
3. 输入直播间 ID，即抖音直播 URL 中的 web_rid，例如 `https://live.douyin.com/{web_rid}`。
4. 点击启动按钮开始监控。
5. 实时查看聊天、礼物、点赞和进场事件。
6. 可同时添加多个直播间，最多 10 个。

## Tauri 命令概览

后端命令集中在 `src-tauri/src/lib.rs`：

| 命令 | 说明 |
| --- | --- |
| `list_accounts` | 列出账号，并同步本地 Cookie 目录中的账号 |
| `sync_local_accounts` | 扫描本地 Cookie 目录并补全账号索引 |
| `init_login_session` | 启动登录 Python 服务 |
| `finish_login` | 通知 Python 抓取并保存 Cookie |
| `cleanup_login_session` | 清理登录会话进程 |
| `verify_account` | 验证账号 Cookie 是否有效 |
| `delete_account` | 删除账号及本地 Cookie 目录 |
| `start_scrape` | 启动采集任务 |
| `get_scrape_progress` | 读取采集进度文件 |
| `cancel_scrape` | 取消采集任务 |
| `get_current_task` | 获取当前采集任务 ID |
| `clear_current_task` | 清除当前采集任务状态 |
| `list_scraped_users` | 查询已采集用户 |
| `get_scraped_videos` | 查询用户作品 |
| `get_scraped_comments` | 查询作品评论 |
| `open_video_in_browser` | 注入 Cookie 并打开作品页面 |
| `start_live_monitor` | 启动直播监控进程 |
| `stop_live_monitor` | 停止直播监控进程 |
| `get_active_monitors` | 获取当前活跃直播监控列表 |
| `get_live_history` | 读取直播间历史事件 |

## 调试说明

### Chrome CDP 连接失败

检查 9222 端口是否被占用：

```bash
lsof -i :9222
```

如需手动启动 Chrome CDP：

```bash
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
  --remote-debugging-port=9222 \
  --user-data-dir=$HOME/chrome-debug-profile
```

### Python 依赖缺失

如果登录、验证、采集或直播监控失败，先确认依赖已安装：

```bash
python3 -m pip show DrissionPage
python3 -m pip install -r scripts/DouyinComment/requirements.txt
python3 -m pip install -r scripts/DouyinBarrage/requirements.txt
```

### 查看应用日志

```bash
open "$HOME/Library/Application Support/AutoCastAI/logs"
```

开发模式下，部分 Python stderr 会由 Rust 转发到 `npm run tauri dev` 控制台，方便直接排查。

### 清理本地数据

谨慎执行，以下命令会删除本应用的本地账号、Cookie、采集数据和日志：

```bash
rm -rf "$HOME/Library/Application Support/AutoCastAI"
```

## 当前限制与后续方向

- 目前主流程以抖音为核心，其他平台尚未接入完整 UI 和自动化链路。
- 「AI 助理对话」页面仍在开发中。
- 「AI 分析」当前是前端原型展示，后续可接入真实 LLM 分析评论内容。
- Playwright 依赖已在前端 package 中出现，但当前抖音账号验证优先使用 DrissionPage + Chrome CDP。
- 直播监控依赖 DouyinBarrage 的内部实现和抖音 Web 接口状态，接口变化时可能需要同步调整。

## 开发约定

- 前端页面和组件位于 `src/`。
- Rust/Tauri 命令统一注册在 `src-tauri/src/lib.rs`。
- Python 脚本 stdout 如果要被 Rust 解析，应只输出 JSON；日志优先输出到 stderr。
- 账号验证不要走 HTTP API 探测，优先使用 Chrome CDP 注入 Cookie 并读取页面状态。
- 本地敏感数据不要提交到仓库，包括 Cookie、账号文件、日志和采集结果。
