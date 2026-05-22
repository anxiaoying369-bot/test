# AutoCast AI - 智播客系统

基于 Tauri 2.0 (Rust) + Vue 3 (Vite) + Tailwind CSS v4 构建的自动化播客/新媒体矩阵桌面端中控系统。

## 🎨 UI 规范 (UI Specification)

本系统采用极客暗黑风格设计，旨在有效缓解长时间挂机盯盘的视觉疲劳。

### 1. 色彩规范 (Color Palette)

| 语义角色 | 颜色名称 | Hex 编码 | Tailwind 类名 | 适用场景 |
| :--- | :--- | :--- | :--- | :--- |
| **应用背景** | Deep Obsidian | `#030712` | `bg-gray-950` | 软件底层大背景 |
| **卡片背景** | Charcoal Black | `#111827` | `bg-gray-900` | 模块区块、任务卡片 |
| **主色调/行动点** | Electric Neon Blue | `#2563EB` | `bg-blue-600` | 主按钮、选中状态、触发动作 |
| **排队状态** | Amber Glow | `#D97706` | `text-amber-500` | 队列等待中状态提示 |
| **执行状态** | Matrix Cyber Green | `#10B981` | `text-emerald-500` | 正在运行、成功、微信新消息 |
| **错误状态** | Crimson Red | `#EF4444` | `text-red-500` | 任务失败、风控警告 |
| **主文字** | Pure Platinum | `#F9FAFB` | `text-gray-50` | 标题、核心标签 |
| **次要文字** | Muted Slate | `#9CA3AF` | `text-gray-400` | 参数说明、未选中导航、时间戳 |

### 2. 字体与排版 (Typography)

* **系统无衬线字体**：`font-sans` (Inter, system-ui, sans-serif)，保证全平台界面渲染锐利。
* **等宽字体（核心）**：`font-mono` (Fira Code, JetBrains Mono, monospace)，专门用于右侧日志面板，确保代码、路径、时间戳对齐，极具专业感。

### 3. 布局尺寸 (Layout Sizes)

* **窗口视口**：固定或推荐最小尺寸 **1280 × 800** 像素（黄金商用工作台比例）。
* **圆角规范**：外层大卡片使用 `rounded-xl` (12px)，内部按钮/输入框使用 `rounded-lg` (8px)。

---

## 🖥️ 页面视觉布局

整个界面采用经典三栏式联动布局，从左到右代表"控制、执行、监控"的逻辑。

### 🧭 1. 左侧栏：全局导航与设备状态 (Width: 20%)

* **背景**：比主背景略深，带有 `border-r border-gray-800` 的细边框分割线。
* **顶部区域**：Logo & 品牌，流线型无衬线字体 **AutoCast AI**，右侧带有一个绿色的闪烁微标 **● Online**。
* **中部导航区**（垂直通栏排列）：

  | 导航项 | 状态 |
  | :--- | :--- |
  | `[🎬 视频发布矩阵]` | 激活状态（带有左侧蓝色高亮粗条，背景微亮） |
  | `[💬 私域微信客服]` | 未激活状态（Slate 灰色，鼠标悬停时变为白字） |
  | `[📦 Obsidian 知识库]` | 未激活状态 |
  | `[⚙️ 系统全局设置]` | 底部固定 |

* **底部设备面板**：一个精致的灰色卡片，显示 `📱 本地仿真器 #1`。动态显示无障碍辅助 App 的连接状态：`IP: 127.0.0.1:5555`。

### 📊 2. 中间栏：工作流任务队列中控台 (Width: 40%)

这是操盘手最核心的交互区。

* **顶部操控区**：包含一个优雅的深色输入框（带 Focus 时的蓝色外发光边框），提示词为：`"输入搞钱关键词 (如: 抹茶欧包)..."`。右侧是一个亮蓝色的实体按钮 `[ + 投放新任务 ]`。

* **中部队列列表**（垂直滚动，支持排队、插队可视化）：

  | 任务类型 | 边框效果 | 内容示例 |
  | :--- | :--- | :--- |
  | 执行中卡片（居顶） | 淡淡的绿色呼吸晕染效果 | `#Task-8821 正在剪辑视频...`，右侧带有一个带百分比的进度条（如 68%）和一串不停变化的动态小图标 |
  | 高优先级插队卡片 | 闪烁的红色惊叹号 | `[⚡ 优先响应] 自动回复: 客户 张步步`，排在所有普通生成任务前面 |
  | 等待中卡片队列 | 暗灰色卡片，右上角标有黄色微标 | `排队中`，内容为 `关键词: 肉松小贝`、`关键词: 提拉米苏` |

### 💻 3. 右侧栏：Agent 脑核控制台与本地日志 (Width: 40%)

纯粹的黑客极客风，满足用户对"AI 正在自动帮我干活"的视觉爽感。

* **背景**：全黑背景（`bg-black`），内部文字全为纯绿色或白色的等宽字体（`font-mono`）。
* **顶部状态栏**：实时显示当前活跃的 Agent 角色：`当前大脑: [ 🎬 导演/剪辑智能体 ]`。

* **滚动日志区**（核心）：模拟终端的实时滚动流，每一行日志都有精准的灰色时间戳：

  ```
  [09:42:01] [INFO] Workspace initialized at AppData/Roaming/autocast/xhs_writer
  [09:42:03] [OCR] Crawled 15 top notes from Xiaohongshu successfully.
  [09:42:06] [LLM] Thinking: Analyzing audience pain points for "Matcha Bread"...
  [09:42:10] [AIGC] Triggered Stable Diffusion API. Generating merchant-grade images...
  [09:42:15] [RPA] ADB Command Executed: adb shell input tap 950 1850 (Click Send)
  ```

* **微信消息高亮**：当有微信消息进来时，日志流中会突显亮黄色高亮块：

  ```
  [09:43:00] [WECHAT] New message from [张步步]: "系统怎么卖？"
  [09:43:01] [AGENT] Loaded Obsidian Vault context: 📦 品牌产品知识库/价格表.md
  ```

---

## 🛠️ 技术栈与依赖

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
| 浏览器引擎 | WRY (WebView) | Tauri 内置 |

### 自动化

| 类别 | 技术 | 用途 |
| :--- | :--- | :--- |
| 浏览器自动化 | Playwright | 平台扫码登录、Cookie 采集 |
| RPA 控制 | ADB (Android Debug Bridge) | 模拟器操控 |

---

## 📁 项目结构

```
podcast/
├── src/                          # Vue 前端源码
│   ├── App.vue                   # 主应用组件
│   ├── main.ts                   # 应用入口
│   ├── lib/
│   │   └── utils.ts               # Shadcn-Vue 工具函数
│   └── assets/                   # 静态资源
├── src-tauri/                    # Rust 后端源码
│   ├── src/
│   │   └── lib.rs                # Tauri 命令定义
│   ├── scripts/                  # Node.js 自动化脚本
│   │   └── platform_login.ts      # 平台扫码登录脚本
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置
├── components.json                # Shadcn-Vue 组件配置
├── vite.config.ts                # Vite 配置（含 Tailwind v4 插件）
└── package.json                   # Node.js 依赖
```

---

## ⚙️ 开发指南

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

## 📝 组件库使用 (Shadcn-Vue)

### 添加组件

```bash
npx shadcn-vue@latest add [component-name]
```

### 可用组件

| 组件 | 用途 |
| :--- | :--- |
| Button | 主按钮、行动点 |
| Card | 任务卡片、模块区块 |
| Input | 关键词输入框 |
| Badge | 状态标签（排队中、执行中、失败） |
| Dialog | 扫码登录弹窗 |
| ScrollArea | 日志滚动区域 |

---

## 🔧 配置说明

### 窗口配置 (`src-tauri/tauri.conf.json`)

```json
{
  "app": {
    "windows": [{
      "title": "AutoCast AI",
      "width": 1280,
      "height": 800,
      "resizable": false
    }]
  }
}
```

### Cookie 存储路径

```
~/Library/Application Support/AutoCastAI/cookies/{platform}_{user_id}.json
```

### 账号数据路径

```
~/Library/Application Support/AutoCastAI/accounts.json
```

### 会话二维码路径

```
~/Library/Application Support/AutoCastAI/login_sessions/qrcodes/
```

---

## 🚀 后续规划

- [ ] 小红书平台自动化发布
- [ ] 抖音平台自动化发布
- [ ] 微信私域客服自动化
- [ ] Obsidian 知识库集成
- [ ] 多 Agent 并行协作
