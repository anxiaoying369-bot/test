# Spreado 功能集成计划

**目标：** 将 Spreado 的多平台账号管理 + 发布能力集成到 AutoCast AI，分阶段实施

**当前状态 vs Spreado 差距分析：**

| 维度 | 现有实现 | Spreado | 改造方向 |
|---|---|---|---|
| 账号存储 | 平面 `cookies/` + `accounts.json` 元数据 | `cookies/{platform}/{name}/account.json` (storage_state) | 迁移存储结构 |
| 多账号 | 每个平台仅 1 个账号 | 每个平台多个账号 | 解除限制 |
| Cookie 验证 | HTTP API 调用（小红书用 `edith.xiaohongshu.com`）| 三层：expires时间戳预检 + positive DOM + negative DOM | 升级为 Spreado 方式 |
| 登录方式 | Python HTTP Server 轮询 | Playwright + StealthBrowser 同方案 | **复用现有登录脚本** |
| 账号 UI | 简陋列表 | 平台卡片 + 账号列表 + 状态徽章 | 全新账号管理页面 |
| 发布功能 | 无 | 各平台 `BasePublisher` 插件 | Phase 2 |

---

## 第一阶段任务（账号管理页面 + 存储结构升级）

### Task 1: 理解现有脚本登录流程

**Objective:** 确认现有 douyin_login.py 和 xiaohongshu_login.py 的二维码获取方式和 storage_state 保存方式

**Files:**
- Read: `scripts/douyin_login.py`
- Read: `scripts/xiaohongshu_login.py`

**Step 1: Read douyin_login.py**
```bash
cat scripts/douyin_login.py
```
重点关注：
- 二维码如何传递给前端（base64 / HTTP / 文件路径）
- storage_state 是否已保存，保存路径
- 登录成功后如何获取 user_info

---

### Task 2: 设计新的 Rust 后端数据结构

**Objective:** 将账号存储从 flat 结构迁移到 Spreado 的嵌套目录结构

**Files:**
- Modify: `src-tauri/src/lib.rs`

**New storage structure:**
```
~/.local/share/AutoCastAI/
└── cookies/
    └── {platform}/           # douyin, xiaohongshu, kuaishou, shipinhao
        └── {account_name}/    # 用户自定义账号名
            ├── account.json   # Playwright storage_state (cookies + origins)
            └── meta.json      # 账号元数据 (platform, created_at, user_id, avatar)
```

**Changes to Rust:**

1. 新增 `AccountMeta` 结构（对应 Spreado 的 `meta.json`）:
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub platform: String,
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

2. 新增 Tauri 命令：
- `list_accounts(platform: Option<String>)` — 按平台列出所有账号（读 meta.json）
- `get_account_meta(platform, account_name)` — 读取单个账号元数据
- `save_account_meta(platform, account_name, meta)` — 保存元数据
- `delete_account(platform, account_name)` — 删除账号（同时删除目录）
- `verify_account(platform, account_name)` — 调用 Python 验证脚本验证 cookie

3. 改造 `save_new_account`：
- 将 cookie 保存为 `cookies/{platform}/{name}/account.json`
- 将 meta 保存为 `cookies/{platform}/{name}/meta.json`
- 删除旧的 flat cookie 文件和 accounts.json 相关逻辑

4. 保留命令（兼容前端）：
- `init_login_session` — 登录流程不变
- `get_login_status` — 登录状态轮询不变

---

### Task 3: 编写 verify_account Python 脚本

**Objective:** 创建类似 Spreado 三层验证的 Python 脚本

**Files:**
- Create: `scripts/verify_account.py`

**Verification logic:**

```python
"""
verify_account.py — 三层 Cookie 验证
Usage: python verify_account.py <platform> <cookie_dir>

Layer 1: 本地 expires 时间戳检查（秒级，无需浏览器）
Layer 2: Playwright positive 检测（authed_selectors 出现 = 有效）
Layer 3: Playwright negative 检测（login_selectors 出现 = 失效）
"""

import asyncio
import json
import time
import sys
from pathlib import Path

# 各平台验证配置
PLATFORM_CONFIG = {
    "douyin": {
        "publish_url": "https://www.douyin.com/creator-micro/content/upload",
        "login_selectors": ['text="手机号登录"', 'text="扫码登录"'],
        "authed_selectors": ["div[class^='container']", "input[placeholder*='作品标题']"],
    },
    "xiaohongshu": {
        "publish_url": "https://creator.xiaohongshu.com/creator-micro/content/upload",
        "login_selectors": ['text="手机号登录"', 'text="扫码登录"'],
        "authed_selectors": ["div.upload-container", "input#title-input"],
    },
}
```

Output: JSON `{"status": "valid"|"expired"|"error", "method": "expires|authed_dom|login_dom|same_domain|no_login_dom", "user_id": "...", "name": "...", "avatar": "..."}`

---

### Task 4: 改造 Vue 账号管理页面

**Objective:** 全新的账号管理 UI，参考 Spreado 的设计

**Files:**
- Modify: `src/App.vue`

**New UI Design:**

```
┌─────────────────────────────────────────────────────────────┐
│  [AI对话]  [账号管理]  [发布控制台]                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  账号管理                                    [🔄 刷新]       │
│                                                             │
│  ┌─ 平台 ──────────────────────────────────────────────┐   │
│  │ 🎵 抖音                         [+ 新增授权] [✓ 验证]│   │
│  │ ┌────────────────────────────────────────────────┐ │   │
│  │ │ 👤 我的大号  ● 有效  2024-01-15  [验证] [删除]  │ │   │
│  │ └────────────────────────────────────────────────┘ │   │
│  │ ┌────────────────────────────────────────────────┐ │   │
│  │ │ 👤 小号  ● 已失效  2024-02-20  [重新登录] [删除]│ │   │
│  │ └────────────────────────────────────────────────┘ │   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ 平台 ──────────────────────────────────────────────┐   │
│  │ 📕 小红书                       [+ 新增授权] [✓ 验证]│   │
│  │ ...                                                   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ 平台 ──────────────────────────────────────────────┐   │
│  │ ⚡ 快手                           [+ 新增授权] [✓ 验证]│   │
│  │ ...                                                   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Features:**
- 每个平台一个可折叠卡片
- 平台内多账号并列显示
- 状态徽章：● 有效 / ○ 已失效 / ◐ 验证中
- 每账号操作：验证 / 删除
- 全局操作：按平台验证全部账号
- 登录弹窗复用现有实现

---

### Task 5: 实现多账号登录（复用现有脚本）

**Objective:** 登录成功后，让用户输入账号名（用于多账号隔离）

**Files:**
- Modify: `src/App.vue` (login modal 部分)
- Modify: `src-tauri/src/lib.rs` (save_new_account 适配新存储)

**Flow:**
1. 登录成功 → 弹出输入框"请输入账号名称"
2. Rust `save_new_account` 接收 `account_name` 参数
3. 保存到 `cookies/{platform}/{account_name}/account.json`
4. 前端刷新账号列表

---

### Task 6: Tauri 配置更新

**Objective:** 确保 Tauri 允许必要的文件读写权限

**Files:**
- Check: `src-tauri/capabilities/default.json`
- Check: `src-tauri/tauri.conf.json` (window size: 1280x800)

---

### Task 7: 验证完整流程

**Objective:** 端到端测试：登录 → 保存 → 验证 → 显示

**Test commands:**
```bash
cd /Users/make/project/podcast
npm run tauri dev
```

**Verification checklist:**
- [ ] 抖音扫码登录成功，二维码不自动关闭（浏览器保持打开）
- [ ] 登录后弹出账号名输入框
- [ ] 账号保存到 `~/.local/share/AutoCastAI/cookies/douyin/{name}/`
- [ ] 账号列表显示平台图标、账号名、状态徽章
- [ ] 点击验证按钮，3 秒内显示验证结果
- [ ] 删除账号后，目录被清理

---

## 第二阶段任务（发布控制台）— 规划预览

```
发布控制台
├── 素材选择（视频文件拖拽）
├── 平台选择（多选 checkbox）
├── 账号选择（每个平台的下拉）
├── 标题 / 描述 / 标签输入
├── 定时发布（datetime picker）
├── 发布预览摘要
└── 发布按钮
```

**核心依赖：**
- Spreado 的 `BaseUploader.login_flow()` / `upload_video_flow()` 
- 各平台特有的 `_upload_video()` 实现（可参考 Spreado 源码）
- StealthBrowser 多浏览器通道

---

## 技术风险 & 应对

| 风险 | 缓解方案 |
|---|---|
| Spreado 的 DOM selector 可能随平台改版失效 | `scripts/verify_selectors.py` 定期检查 + 前端告警 |
| Playwright 浏览器占用资源 | headless 验证，每账号最多并发 1 个 |
| 多账号 cookie 冲突 | 每个账号独立的 storage_state 文件 + 独立浏览器 context |
| storage_state 格式差异 | 统一使用 Playwright 的 `context.storage_state()` 输出格式 |
