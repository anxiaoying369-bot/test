# AutoCast AI - 智播客系统

基于 Tauri 2.0 (Rust) + Vue 3 (Vite) + Tailwind CSS v4 构建的自动化播客/新媒体矩阵桌面端中控系统。

## 功能特性

### 平台登录与账号管理
- **抖音登录** — DrissionPage + Chrome CDP 接管模式（复用本地已打开的 Chrome，--remote-debugging-port=9222）
- **本地 Cookie 持久化** — 登录后自动采集并保存到 `~/Library/Application Support/AutoCastAI/cookies/`
- **多账号管理** — 账号列表、状态查看、一键验证、删除

### 数据采集
- **抖音爬虫** — 支持按 SECUID 采集：全部（作品+评论+回复）、仅作品、仅评论、仅回复
- **跳过已有数据** — 可选跳过已存在的视频/评论，避免重复采集
- **实时进度跟踪** — 任务队列、百分比进度、动态日志流

### 架构设计
- **Rust Tauri 后端** — 负责窗口管理、文件系统、进程调度、HTTP API Server
- **Vue 3 前端** — 三栏式控制台：导航 / 任务队列 / Agent 日志
- **Python 自动化脚本** — 浏览器操控、数据爬取、业务逻辑（通过 Tauri Shell 调用）
- **DouyinComment 子模块** — 独立的评论自动化 Python 项目（git submodule）

---

## 技术栈

### 前端
| 类别 | 技术 | 版本 |
| :--- | :--- | :--- |
| 框架 | Vue 3 + Vite | ^3.5.x / ^6.x |
| UI 框架 | Tailwind CSS v4 + @tailwindcss/vite | ^4.x |
| 组件库 | Shadcn-Vue (基于 Radix UI) | latest |
| 图标 | Lucide-Vue-Next | latest |
| 类名合并 | clsx + tailwind-merge | latest |

### 后端
| 类别 | 技术 | 版本 |
| :--- | :--- | :--- |
| 跨端框架 | Tauri 2.0 | ^2.x |
| 语言 | Rust | 1.75+ |
| HTTP Server | Rust (tonic-web / warp) | - |
| 浏览器自动化 | DrissionPage / Playwright | latest |

### 自动化脚本
| 脚本 | 技术 | 用途 |
| :--- | :--- | :--- |
| `douyin_login.py` | DrissionPage + CDP | 抖音登录 + Cookie 采集 |
| `douyin_scraper.py` | DrissionPage | 抖音数据爬取 |
| `verify_account.py` | HTTP API | 账号有效性验证 |
| `DouyinComment/` | Python | 抖音评论自动化（submodule） |

---

## 项目结构

```
podcast/
├── src/                          # Vue 前端源码
│   ├── App.vue                   # 主应用组件（账号管理+登录流程）
│   ├── main.ts                   # 应用入口
│   ├── components/
│   │   └── ScraperView.vue       # 抖音爬虫控制台
│   └── lib/
│       └── utils.ts              # Shadcn-Vue 工具函数
├── src-tauri/                    # Rust 后端源码
│   ├── src/
│   │   └── lib.rs                # Tauri 命令（账号管理、登录会话、爬虫任务）
│   ├── Cargo.toml
│   └── tauri.conf.json
├── scripts/                      # Python 自动化脚本
│   ├── douyin_login.py           # DrissionPage CDP 登录
│   ├── douyin_scraper.py         # 抖音数据采集
│   ├── verify_account.py         # 账号验证
│   ├── funcall.py                # 函数调用封装
│   └── DouyinComment/            # 评论自动化子模块 (submodule)
├── package.json
├── vite.config.ts
└── README.md
```

---

## 开发指南

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 生产构建

```bash
npm run build
```

### Tailwind CSS v4 Dark Mode

确保在 `src/main.ts` 中显式设置 dark class：

```ts
document.documentElement.classList.add('dark')
```

---

## 配置与数据路径

| 类型 | 路径 |
| :--- | :--- |
| Cookie 存储 | `~/Library/Application Support/AutoCastAI/cookies/{platform}_{user_id}.json` |
| 账号数据 | `~/Library/Application Support/AutoCastAI/accounts.json` |
| 登录二维码 | `~/Library/Application Support/AutoCastAI/login_sessions/qrcodes/` |
| 应用日志 | `~/Library/Application Support/AutoCastAI/logs/` |

---

## 组件库使用 (Shadcn-Vue)

```bash
npx shadcn-vue@latest add [component-name]
```

常用组件：Button、Card、Input、Badge、Dialog、ScrollArea、Select

---

## 后续规划

- [ ] 小红书平台自动化发布
- [ ] 微信私域客服自动化
- [ ] Obsidian 知识库集成
- [ ] 多 Agent 并行协作
