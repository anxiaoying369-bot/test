# 工厂系统 实施计划与进度单（Factory System）

> **这份文档的用法**
> - 它是工厂系统的「总规划 + 进度追踪单」。任何人（或 AI）接手时，先读「§0 进度总览」知道做到哪、下一步做什么。
> - 每完成一项任务，把对应的 `- [ ]` 改成 `- [x]`，并更新「§0 进度总览」的指针。
> - 「怎么做」一栏给出涉及的文件与思路，照着做即可。
> - 决策已在需求讨论中锁定（见 §2），实现时不要随意更改；如需变更，先更新本文件。

---

## 0. 进度总览（每次改动后更新这里）

- **当前阶段**：P0 已完成 → 准备开始 **P1**
- **下一步要做**：`P1-1` 在 `src-tauri` 建 `factory.db` 与表结构（见 §5 · P1）
- **阶段勾选**
  - [x] **P0** 工厂系统页 + 人员/库存前端骨架
  - [ ] **P1** 工厂数据落库（人员/库存持久化 + 管理员身份）
  - [ ] **P2** 主系统服务（HTTP API + 密钥签发 + 审计）
  - [ ] **P3** 副系统接入（密钥认证 + 命令层代理 + 留痕）
  - [ ] **P4** 知识库分层（公司库 / 个人库 作用域）
  - [ ] **P5** 知识库同步（公司库下行 + 个人库双向 + 本地合并检索）
  - [ ] **P6** 审计查询页 + 同步状态 UI + 断网降级

---

## 1. 最终功能清单（做完后系统能做到什么）

### A. 系统模式（设置页切换）
- 设置页「工厂系统」分区切换 **主系统 / 副系统**。
- 主模式：启停内置服务、配置端口、生成/查看 API Key 状态、查看已连接副系统。
- 副模式：填写主系统地址 + 密钥、连接测试、显示连接状态。

### B. 工厂业务模块
- **人员管理**：增删改查、持久化；（主系统）对人员签发 / 重置密钥。
- **库存管理**：增删改查、持久化、低于安全库存预警。
- 预留后续模块扩展位（`/api/<module>`、新子页面）。

### C. 身份与密钥
- 主系统按人员**签发单把有效密钥**（明文仅显示一次，库内只存哈希）。
- 密钥**绑定人员**；可**重置**（旧密钥立即失效，但历史操作仍可追溯到该人员）。
- 副系统用密钥认证 → 解析出 `person_id`。

### D. 审计留痕
- 记录**写操作 + 登入 + 知识库检索（敏感读）**。
- 主系统本机操作归属一个「管理员」人员身份。
- 提供**审计日志查询页**（按人员/动作/时间/来源筛选）。

### E. 知识库分层与同步
- **集中摄取**：解析/切片/向量化/存原件全部在主系统；副系统**上传原文件**触发入库；原文件集中存于主系统，任意设备可下载。
- **公司知识库**：主系统权威；副系统每次打开**下行同步到本地只读镜像**。**增/改/删仅在主系统进行，副系统只读**。
- **个人知识库**：每人一份，**主系统为中心**；副系统发起增/删 → 主系统处理 → 该人各设备拉取同步；本地保留副本供检索。
- 副系统**检索全程本地**（最快）；RAG 默认 **个人 + 公司合并**、**公司库优先**。
- 同步以 **`factory.db` 的 `kb_metadata`** 为真相源，按整文档粒度下发向量切片 + 原文件。
- 知识库管理页按作用域区分：公司库（主系统可编辑 / 副系统只读）、个人库（拥有人可编辑）。

### F. 安全与健壮性
- **两层鉴权**：设备准入密钥 + 用户登入密钥；公共设备每次/切换操作人需登入。
- 密钥**被吊销**：联网即**清空本地公司库镜像**；**离线租约 TTL** 到期锁定本地库。
- 副系统断网降级提示；同步状态 / 进度 UI。
- embedding 仅在主系统计算（向量直传、免重嵌）；改 embedding → KB 升「指纹」，副系统检测不一致即清空重拉。

---

## 2. 架构决策（已锁定，勿擅改）

| 决策点 | 结论 |
|---|---|
| 部署形态 | 同一 app，设置页切「主/副」模式 |
| 事务数据（人员/库存） | 副系统不本地存，读写**代理到主系统**（单一数据源） |
| 连接 / 鉴权 | 副系统**手动填主系统地址**；**两层鉴权**：①**设备准入密钥**（证明合法设备）②**用户登入密钥**（=个人身份）。请求带「设备 Token + 用户会话 Token」 |
| 设备模式 | **同时支持**：**个人设备**（记住登入、无感）/ **公共设备**（每次或切换操作人时需员工登入）。审计始终记**实际登入的人**，而非设备 |
| 用户密钥 | 每人**单把有效**；主系统按人员签发、绑人员表；可重置（旧失效、历史可追溯）；**明文仅显示一次，存哈希** |
| 权限 | 人员/密钥表**预留字段，本期不校验** |
| 审计 | 写操作 + 登入 + KB 检索；主系统本机=管理员身份 |
| **知识库摄取（集中）** | **解析/切片/向量化/存原件全部在主系统**：副系统上传原文件 → 主系统处理入库；embedding 只在主系统算（指纹天然一致）。个人库新增需主系统在线 |
| 原文件存储 | 主系统集中存 `uploads/<owner>/<doc_id>.<ext>`；任意设备登入后可下载原件（解决换机看不到 PDF 原件） |
| 公司知识库编辑权 | **增/改/删仅限主系统**；副系统对公司库**只读**（无上传/编辑/删除入口） |
| 公司知识库 | 主权威；副系统**打开时下行同步到本地、本地检索（只读）** |
| 个人知识库 | **主系统为中心**：副系统发起增/删 → 主系统处理存储 → 该人**各设备拉取**同步（基本无 peer 冲突，`updated_at` 仅兜底）；副系统本地保留副本供检索 |
| **KB 同步真相源** | **`factory.db` 的 `kb_metadata` 表**（**不让 LanceDB 负责同步**）；按**整文档**粒度：更新=删旧文档全部切片+插新切片（不按 chunk_id 对应）；删除=元数据打标，各端按 `doc_id` 清本地 LanceDB |
| KB 同步内容 | 元数据先行（diff）→ 再下发**向量切片 + 原文件**（向量直传，免重嵌） |
| 检索 / 审计取向 | **本地检索优先**（快/可离线）；公司库检索审计**尽力而为**（客户端可绕过，见「§8 已接受风险」）；密钥被吊销→**联网即清空本地镜像** + **离线租约 TTL** 到期锁定；提供**「强制在线检索」开关**给高安全部署 |
| RAG 检索范围 | 默认**个人 + 公司合并**，副系统本地执行 |
| RAG 冲突优先级 | **公司库优先**（去重保留公司份；提示词标来源 + 冲突以公司库为准） |
| HTTP 服务 | 主系统内嵌 **axum**（与 tokio 同源），与 Tauri 同进程 |
| 库存变更 | **入出库流水（增量）**：每次变动记一条 `inventory_txn(+N/-N)`，数量由流水维护；多终端并发不丢更新、可追溯 |
| 人员离职/停用 | 状态置离职 → **自动吊销其密钥**（副系统无法再登入）；**个人库保留**在主系统，管理员可查/移交；历史留痕不变 |
| 副系统离线写 | 事务数据（人员/库存）**离线直接报错**（单一数据源，不脱机写）；个人库写本地可离线、联网再同步 |
| 主系统备份 | 内置 **一键导出 + 定时本地备份**（factory.db + 知识库） |
| 传输安全 | 局域网 + Bearer API Key（明文）；后续可选自签 TLS。副系统本机密钥存配置文件 |
| 版本兼容 | `/health` 返回 API 版本；主/副不兼容时提示升级 |
| embedding 变更 | 主系统改 embedding 配置 = KB 升「版本号」，副系统检测不一致即全量重拉/重嵌 |

---

## 3. 数据模型（参考）

### 3.1 `factory.db`（主系统，rusqlite，新文件与 `video_studio.db` 并列）
```sql
CREATE TABLE personnel (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  emp_no TEXT UNIQUE,
  dept TEXT,
  role TEXT,
  perm TEXT,                 -- 预留权限(JSON)，本期不校验
  is_admin INTEGER DEFAULT 0,-- 主系统本机操作归属的管理员
  status TEXT DEFAULT '在职',
  created_at INTEGER, updated_at INTEGER
);

CREATE TABLE inventory (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL, sku TEXT UNIQUE, category TEXT,
  quantity INTEGER DEFAULT 0,  -- 由 inventory_txn 流水维护(每次变动同事务 += delta)，不直接覆盖
  unit TEXT, safety_stock INTEGER DEFAULT 0,
  created_at INTEGER, updated_at INTEGER
);

CREATE TABLE inventory_txn (   -- 入出库流水(增量)，数量变更的唯一来源
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL,
  delta INTEGER NOT NULL,     -- 入库为正 / 出库为负
  reason TEXT,                -- 入库/出库/盘点调整...
  operator_id TEXT,           -- person_id，谁操作的
  created_at INTEGER
);

CREATE TABLE api_keys (        -- 用户登入密钥（=个人身份）
  id TEXT PRIMARY KEY,
  person_id TEXT NOT NULL,
  key_hash TEXT NOT NULL,     -- 只存哈希
  key_prefix TEXT,            -- 形如 fk_ZW1001_ 便于识别归属
  status TEXT DEFAULT 'active',-- active | revoked
  created_at INTEGER, revoked_at INTEGER, last_used_at INTEGER
);  -- 每人同时仅 1 把 active；revoked 历史行永不删除（用于追溯）

CREATE TABLE device_keys (     -- 设备准入密钥（证明合法副系统设备，与用户身份分离）
  id TEXT PRIMARY KEY,
  name TEXT,                  -- 设备名/位置，如「车间A平板」
  mode TEXT DEFAULT 'personal',-- personal | shared(公共设备，操作需用户登入)
  key_hash TEXT NOT NULL,
  status TEXT DEFAULT 'active',-- active | revoked
  last_seen_at INTEGER, created_at INTEGER, revoked_at INTEGER
);

CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  person_id TEXT,             -- 追溯锚点，永不变
  api_key_id TEXT,           -- 当时用的密钥（可空）
  action TEXT, resource_type TEXT, resource_id TEXT, detail TEXT,
  source TEXT,               -- master | slave
  source_ip TEXT, result TEXT, created_at INTEGER
);
```

### 3.2 知识库（同步真相源在 SQLite，向量在 LanceDB，原件在文件系统）
```sql
-- factory.db：文档级元数据 = 同步真相源（不让 LanceDB 负责同步）
CREATE TABLE kb_metadata (
  doc_id TEXT PRIMARY KEY,
  scope TEXT,                 -- company | personal
  owner_id TEXT,             -- personal 时 = person_id
  source_name TEXT,          -- 原文件名
  file_path TEXT,            -- uploads/ 下原件相对路径
  content_hash TEXT,         -- 原件内容哈希(判断是否变更)
  embedding_fingerprint TEXT,-- 该文档向量所用 embedding 指纹
  chunk_count INTEGER,
  updated_at INTEGER,
  deleted INTEGER DEFAULT 0  -- 软删：传播删除用
);
CREATE TABLE kb_sync_state (person_id TEXT, scope TEXT, last_sync_at INTEGER);
```
```
-- LanceDB：仅存切片向量，按 doc_id 归组（更新/删除以整文档为单位按 doc_id 操作）
kb_chunks(doc_id, chunk_idx, scope, owner_id, text, embedding)
-- 文件系统：主系统 uploads/<owner>/<doc_id>.<ext> 存原件
```
> 同步流程：先比对 `kb_metadata`（按 `updated_at`/`content_hash`/`deleted`）→ 对变更文档下发其全部新切片（替换）+ 原文件；对删除文档下发删除指令（各端按 `doc_id` 清 LanceDB + 删本地原件）。

### 3.3 配置（`AppConfig` 增加 `factory`）
```
config.factory = {
  mode: 'master' | 'slave',          // 默认 master
  server_port: 8765,                 // 主模式
  master_url: '',                    // 副模式 http://ip:port
  device_key: '',                    // 副模式：本机设备准入密钥（明文，仅本机）
  device_mode: 'personal'|'shared',  // 副模式：个人/公共设备
  remember_login: true,              // 个人设备可记住用户登入；公共设备建议 false
  force_online_search: false,        // 高安全部署：强制每次检索走主系统(放弃本地/离线)
  offline_lease_days: 7,             // 离线租约：超过未成功联网鉴权则锁本地库
  embedding_fingerprint: ''          // 副模式：本地镜像所用 embedding 指纹(与主系统比对)
}
// 注：用户登入会话 Token 不落配置文件，保存在内存/安全存储；device_key 才是本机持久持有的那把

```

---

## 4. API 契约 & 命令清单（参考）

### 4.1 主系统 HTTP API（axum，Bearer API Key）
| 方法 | 路径 | 说明 | 审计 |
|---|---|---|---|
| GET | `/health` | 健康/API 版本/embedding 指纹 | - |
| POST | `/device/auth` | **设备准入**：验设备密钥 → 设备 Token | - |
| POST | `/auth/login` | **用户登入**：验用户密钥（需先过设备准入）→ 会话 Token + person 概要 | 登入 |
| GET/POST | `/api/personnel` | 列表(?q=)/新增 | 写 |
| PUT/DELETE | `/api/personnel/:id` | 改/删（置离职=自动吊销其密钥） | 写 |
| GET/POST | `/api/inventory` | 列表/新增 | 写 |
| PUT/DELETE | `/api/inventory/:id` | 改/删 | 写 |
| POST | `/api/inventory/:id/txn` | 入出库（增量 delta） | 写 |
| GET | `/api/kb/changes?scope=&since=` | KB 元数据增量（公司=下行；个人=本人）→ 返回变更/删除文档清单 | - |
| GET | `/api/kb/doc/:doc_id/chunks` | 拉某文档的向量切片（同步用） | - |
| POST | `/api/kb/ingest` | **集中摄取**：上传原文件(+scope) → 主系统解析/切片/向量化/存件 → 返回 doc_id | 写 |
| DELETE | `/api/kb/doc/:doc_id` | 删除文档（软删，传播） | 写 |
| GET | `/api/kb/file/:doc_id` | 下载原文件 | - |
| POST | `/api/audit/report` | 副系统上报本地事件（如 KB 检索） | 敏感读 |
| GET | `/api/embedding-config` | 下发 embedding 配置 + 指纹 | - |

> 服务端强制：`/api/kb/ingest` 写入时强制 `owner_id=会话身份`、公司 `scope` 仅管理员可写（防污染，见 P4-3）。设备 Token 失效或用户密钥被吊销 → 返回 401。

### 4.2 Tauri 命令（前端调用，模式感知）
- 工厂数据：`factory_personnel_list/upsert/delete`、`factory_inventory_list/upsert/delete`
  - 主模式 → 本地 `factory.db`；副模式 → reqwest 代理到主系统 API。
- 密钥（仅主）：`factory_key_issue(person_id)`、`factory_key_reset(person_id)`、`factory_key_list()`
- 服务（仅主）：`factory_server_start/stop/status`
- 连接（仅副）：`factory_test_master()`、`factory_connect(master_url, key)`
- 知识库：`kb_search(scope?)`（改造为本地合并检索）、`kb_sync_company()`、`kb_sync_personal()`
- 审计（仅主）：`factory_audit_list(filter)`

---

## 5. 分阶段任务单（带勾选 + 怎么做）

### P0 — 工厂系统页骨架 ✅ 已完成
- [x] 侧边栏「工厂系统」入口（`src/App.vue`：PageKey + 导航项 + main 块）
- [x] `FactorySystemView.vue`（内部 tabbar：人员管理 / 库存管理）
- [x] `factory/FactoryPersonnel.vue`、`factory/FactoryInventory.vue`（前端内存版）

---

### P1 — 工厂数据落库（持久化 + 管理员身份）
> 目标：人员/库存从前端内存改为主系统 SQLite 持久化，单机可用。

- [ ] **P1-1** 建 `factory.db` 与表（personnel/inventory/**inventory_txn**/api_keys/**device_keys**/audit_log/**kb_metadata**/kb_sync_state）
  - 怎么做：仿 `src-tauri/src/db.rs::init_db`，新增 `init_factory_db(data_dir)`；**直接上 r2d2 连接池**（`AppState` 加 `factory_pool: r2d2::Pool`，见 P2-1）+ 建库设 `PRAGMA journal_mode=WAL` + `busy_timeout`（一步到位，避免后面从 `Mutex<Connection>` 返工）。
- [ ] **P1-2** 新增命令模块 `commands/factory/`（`mod.rs` + `personnel.rs` + `inventory.rs`）
  - 怎么做：实现 `factory_personnel_list/upsert/delete`、`factory_inventory_list/upsert/delete`；在 `commands/mod.rs` 加 `pub mod factory;`，在 `lib.rs` 的 `invoke_handler!` 注册。
  - **库存变更走流水**：新增 `factory_inventory_adjust(item_id, delta, reason)` —— 同一事务里插 `inventory_txn` + `inventory.quantity += delta`；不提供「直接覆盖 quantity」的接口（盘点也以一条调整流水体现）。前端库存页改为「入库/出库」按钮触发增量。
- [ ] **P1-3** 初始化「管理员」人员身份
  - 怎么做：`init_factory_db` 时若无 `is_admin=1` 的人员，插入一条默认管理员（用于主系统本机操作留痕）。
- [ ] **P1-4** 前端改为调用后端命令
  - 怎么做：`FactoryPersonnel.vue`/`FactoryInventory.vue` 的内存数组改为 `invoke('factory_*')`；建议抽 `composables/useFactory.ts` 收敛调用。
- [ ] **P1-5** 验证：`cargo check` + `vue-tsc` 通过；增删改重启后数据仍在。

---

### P2 — 主系统服务（HTTP API + 密钥 + 审计）
> 目标：主系统能对外提供 API，可对人员签发/重置密钥，本机写操作留痕。

- [ ] **P2-1** 加依赖：`Cargo.toml` 增加 `axum`、`tower`/`tower-http`、**`r2d2` + `r2d2_sqlite`**。
  - **并发底座（重要）**：`factory.db` 用 **r2d2 连接池**（不再用单一 `Mutex<Connection>`）；建库时设 **`PRAGMA journal_mode=WAL`**（读写并发、读不被写阻塞）+ **`busy_timeout`**（防 `database is locked`）。UI 的 invoke 与 axum 各自从池取连接，互不阻塞。
- [ ] **P2-2** 内嵌 axum 服务 `commands/factory/server.rs`
  - 怎么做：`factory_server_start/stop/status` 命令；主模式下在 tokio 任务启动 axum，共享 **连接池**（P2-1）；端口取 `config.factory.server_port`；Bearer API Key 中间件。
  - **CORS**：挂 `tower-http::CorsLayer` 允许任意来源（安全由 Bearer Key 保证），为将来 Web/内嵌 webview 客户端预留。
  - **防火墙**：文档给「Windows 防火墙放行指引」；可选「一键放行」按钮执行 `netsh advfirewall firewall add rule`（需管理员/触发 UAC，best-effort，不强保证）。
- [ ] **P2-3** 密钥模块 `commands/factory/keys.rs`
  - 怎么做：`factory_key_issue`（生成 `fk_<短码>_<随机>`、存哈希、返回明文一次）、`factory_key_reset`（旧把 status=revoked + 新签一把）、`factory_key_list`。
  - **离职自动吊销**：人员 `status` 改为「离职」时，自动把其 active 用户密钥置 revoked（副系统无法再登入）；个人库**保留**（不删）；离职操作记审计。
  - **设备密钥**：`factory_device_issue/list/revoke`（设备准入密钥，含 `mode=personal|shared`），与用户密钥分开管理。
- [ ] **P2-3b** 两层鉴权中间件：`POST /device/auth`（验设备密钥→设备 Token）→ `POST /auth/login`（验用户密钥→会话 Token）；后续 `/api/*` 需带「设备 Token + 会话 Token」，缺任一或被吊销返回 401。
- [ ] **P2-4** 审计模块 `commands/factory/audit.rs`
  - 怎么做：`write_audit(person_id, api_key_id, action, ...)` 辅助函数；P1 的写命令 + API 写端点统一调用；主机写操作用管理员 `person_id`、`source=master`。
  - **异步写入（不卡主路径）**：审计走内存 channel + 单独后台任务批量落库（fire-and-forget），请求路径不等待审计 IO。append-only、查询分页。
- [ ] **P2-5** API 端点：`/health` `/auth/login` `/api/personnel` `/api/inventory`（接 P2-2 中间件 + P2-4 审计）。
- [ ] **P2-6** 设置页「工厂系统」分区（主模式部分）
  - 怎么做：`types/settings.ts` 加 `FactoryConfig`；`useAppConfig.ts` 加默认值；`SettingsView.vue` 的 `TABS` 加 `factory`；新建 `settings/SettingsFactory.vue`（模式单选、端口、生成密钥入口、服务启停/状态）。
- [ ] **P2-7** 验证：本机用 curl/浏览器访问 `/health` 与 `/api/personnel`（带 Key）正常。
- [ ] **P2-8** 主系统数据备份：`factory_backup_export()`（一键导出 `factory.db` + 知识库目录为压缩包）+ 定时本地备份（保留最近 N 份）；设置页提供「立即备份 / 备份目录 / 频率」。

---

### P3 — 副系统接入（密钥认证 + 命令代理 + 留痕）
> 目标：另一台设备设为副系统，填地址+密钥即可读写主系统数据，全程留痕。

- [ ] **P3-1** 设置页副模式部分（`SettingsFactory.vue`）：主系统地址、**设备准入密钥**、**设备模式（个人/公共）**、连接测试。
  - 怎么做：`factory_test_master()` 调远端 `/health` + `/device/auth`；复用 Hermes 健康检查 UI 风格。
- [ ] **P3-1b** 用户登入流程：
  - **个人设备**（`remember_login=true`）：首次输入用户密钥登入后记住会话，后续无感。
  - **公共设备**（`shared`）：进入操作前/切换操作人时**必须登入**（输工号+用户密钥/刷卡）；会话可设空闲超时自动登出。审计记**实际登入的人**。
- [ ] **P3-2** 命令层「模式感知」改造
  - 怎么做：`factory_personnel_*`/`factory_inventory_*` 内部判断 `config.factory.mode`：master→本地；slave→`reqwest` 调 `master_url` 对应 API（带**设备 Token + 会话 Token**）。
  - **离线写报错**：副模式下事务写（人员/库存）若连不上主系统，**直接返回错误**（提示「未连接主系统，无法写入」），不脱机写、不排队。
- [ ] **P3-3** 主系统认证：`/auth/login` 与每个 `/api/*` 校验设备 Token + 会话 Token，定位 `person_id`，更新 `last_used_at`，写审计（登入/写，`source=slave`，记 `source_ip`）。被吊销→401。
- [ ] **P3-4** 验证：公共设备上张三/李四分别登入操作 → 审计分别记到本人；离线写报错。
- [ ] **P3-5** 验证重置：重置某人密钥后旧密钥认证失败；该人此前的审计仍指向其本人。

---

### P4 — 知识库分层（公司库 / 个人库 作用域）
> 目标：知识库区分「公司」与「个人(按人)」两个作用域。

- [ ] **P4-1** **集中摄取改造**：`kb_manager.py` 仅在主系统跑解析/切片/向量化；写 `kb_metadata`(SQLite truth) + `kb_chunks`(LanceDB，按 `doc_id` 归组) + 存原件到 `uploads/`。
  - **多身份隔离**：`scope`/`owner_id` 走 **CLI 参数 `--scope --owner-id`**（per-spawn env 本就独立，CLI 更显式零竞争）。
  - **性能**：P4 先 per-request spawn 跑通正确性；高频优化放 P5-7（常驻 KB worker）。
- [ ] **P4-2** 摄取入口 `POST /api/kb/ingest`（上传原文件 + scope）：副系统不本地向量化，**上传原件触发主系统摄取**；现有「企业知识库」页归为 `company`（仅主系统）。
- [ ] **P4-3** 知识库管理页区分作用域：主系统可管理公司库；副系统管理拥有人的个人库。
  - 约束：**副系统对公司库只读** —— 公司库的上传/编辑/删除入口在副模式下隐藏/禁用；主系统 API 对公司库只提供下行（`GET /api/kb/changes?scope=company`），`POST /api/kb/ingest` 的 `scope=company` 仅管理员可写。
  - **【强制隔离·防污染·必须实现】** 主系统在 `POST /api/kb/ingest` 落库时，对非管理员**强制** `scope=personal`、**强制** `owner_id = 本次登入的 person_id`，**完全忽略客户端传来的 `scope`/`owner_id`**（按认证身份重写）。即使副系统被改/伪造请求，也无法把内容塞进 `scope=company` 或他人 `owner_id`。
    - 目的：保证副系统上传的个人知识**绝不污染公司知识库**，也不串到别人的个人库。
    - 检索侧同样隔离：公司检索只查 `scope=company`；个人检索只查 `owner_id=当前person_id`。
    - 个人内容「升级」进公司库只能由**管理员在主系统**人工操作（复制为 `scope=company`），副系统无此能力。
- [ ] **P4-4** `/api/embedding-config`：主系统下发 embedding 配置；副系统连接后采用并置 `embedding_synced=true`。
- [ ] **P4-5** 验证：公司库与个人库互不串；不同人的个人库隔离。

---

### P5 — 知识库同步（公司下行 + 个人双向 + 本地合并检索）
> 目标：副系统打开同步公司库、登入同步个人库，检索全程本地。

- [ ] **P5-1** 主系统增量接口（元数据驱动）：`GET /api/kb/changes?scope=&since=` 返回 `kb_metadata` 增量（变更/删除清单）；`GET /api/kb/doc/:id/chunks` 拉切片；`GET /api/kb/file/:id` 拉原件。
- [ ] **P5-2** 副系统 `kb_sync_company()`：打开时按元数据 diff → 拉变更文档的切片+原件写本地只读镜像；删除项按 `doc_id` 清本地。
- [ ] **P5-3** 副系统个人库同步：增/删由副系统**调主系统**（`/api/kb/ingest`、`DELETE /api/kb/doc/:id`）→ 主系统处理 → 各设备按元数据 diff **拉取**同步（下行为主，无 peer 合并；`updated_at` 仅兜底）。
- [ ] **P5-4** `kb_search` 改造：副系统对本地（个人+公司）合并检索并排序；主系统检索公司(+管理员个人)。
  - **去重**：合并后对近似/基本相同的片段折叠为一条；近似重复时**保留公司库那份**（或合并保留公司来源标记）。
  - **来源标注**：每条结果带 `source=company|personal`；拼进 RAG 提示词时标 `[公司库]`/`[个人库]`。
  - **公司库优先**：系统提示加入「检索资料中若有冲突，**以公司库为准**，个人库作补充/个性化」；排序可给公司库小幅加权（但仍以相关度为主）。
- [ ] **P5-5** 同步以**向量直传**实现（免重嵌入），依赖 P4-4 的 embedding 配置一致。
  - **版本校验/防乱码**：KB 维护 `embedding_fingerprint`（model+base_url+dim 的哈希）。`/health` 与 `/api/embedding-config` 下发该指纹；副系统每次同步先比对，**不一致 → 主系统下发「清空重拉」**，副系统清本地镜像 + 全量重拉新向量。
  - **模型变更可恢复**：主系统保留 `kb_chunks.text` 与 `uploads/` 原件，改 embedding 时**全量重嵌入** → bump 指纹（否则旧向量与新模型不在同一空间，检索全乱）。
- [ ] **P5-6** 验证：副系统离线也能检索已同步内容；个人库经主系统、换设备能拉回（含原件可下载）；**改 embedding 后副系统自动清空重拉、检索正常**。
- [ ] **P5-7** （性能）常驻 KB worker：Rust 与一个长驻 Python 进程走 stdin/stdout JSON-RPC，lancedb/模型保持热、写入天然串行；替代高频场景下的 per-request spawn。
- [ ] **P5-8** **吊销/离线安全**：副系统启动联网时若 `/auth/login` 返回 401（密钥被吊销）→ **立即清空本地公司库镜像并锁应用**；维护**离线租约**（`offline_lease_days` 内可离线用，超期未成功联网鉴权则锁本地库直到重新鉴权）；`force_online_search=true` 时检索强制走主系统 API（不落本地、全程审计）。

---

### P6 — 审计页 + 同步状态 + 断网降级
- [ ] **P6-1** `factory_audit_list(filter)` + 审计日志查询页（按人员/动作/时间/来源筛选）。
- [ ] **P6-2** KB 检索审计：副系统本地检索后异步 `POST /api/audit/report`（不阻塞检索）。**尽力而为**，客户端可绕过（见 §8）；`force_online_search` 模式下检索走主系统、审计可靠。
- [ ] **P6-3** 同步状态 UI：工厂系统页/知识库页显示「上次同步时间、进行中、失败重试」。
- [ ] **P6-4** 断网降级：副系统连不上主系统时，事务操作给出明确提示；知识库走本地镜像继续可用。
- [ ] **P6-5** 工厂系统页顶部模式徽标（主🟢 / 副🔵 + 连接状态）。

---

## 6. 技术默认与约定
- 鉴权：两层（设备准入密钥 + 用户登入密钥）；公共设备每次/切换操作人需登入。
- 摄取：**集中在主系统**（解析/切片/向量化/存原件）；副系统上传原件触发，不本地向量化。
- 同步：以 `kb_metadata`(SQLite) 为真相源、**整文档粒度**；公司库下行只读、个人库经主系统各设备拉取；`updated_at` 仅兜底。
- 同步触发：公司库=副系统打开时；个人库=登入 + 本地发起变更后 + 定时兜底。
- 密钥：明文**仅生成时显示一次**；库内只存哈希；格式 `fk_<人员短码>_<随机强密文>`。
- KB 向量：**直传向量**，embedding 仅在主系统算；改 embedding → bump `embedding_fingerprint`，副系统不一致即清空重拉。
- 并发：主系统 `factory.db` 用 **r2d2 连接池 + WAL + busy_timeout**；审计异步写。
- Windows 兼容：所有新增子进程沿用 `utils::tokio_command/std_command`（带 CREATE_NO_WINDOW）；路径用 `PathBuf`/`pathlib`。

## 8. 已接受的风险与边界（架构评审认知对齐）
> 这些是「本地优先」架构的固有妥协，**已知并接受**；若不可接受，对应缓解方案见括注。
- **本地副本不可信边界（#5 审计伪造）**：公司库下发到副系统后**本地检索**，审计依赖客户端上报，可被破解/阻断绕过。即：下发到副系统的公司知识，**已脱离主系统的绝对审计**。
  - 缓解（不可接受时）：开启 `force_online_search` → 检索全走主系统 API（牺牲离线/速度换全程审计）。
- **离线持有风险（#2）**：吊销密钥后，若设备立即断网，本地已同步的镜像在**离线租约 TTL 内**仍可读。
  - 缓解：缩短 `offline_lease_days`；联网即清空；（更强）本地镜像加密、解密密钥联网时获取——但仍挡不住直接拷贝 LanceDB 文件离线分析，属同一不可信边界。
- 结论：副系统的本地库适合「内部、半可信」场景；强保密内容要么只放公司库 + `force_online_search`，要么不下发副系统。

## 7. 如何继续 / 恢复工作
1. 读「§0 进度总览」找到下一步（第一个未勾选项）。
2. 到「§5」对应任务，照「怎么做」执行；涉及的数据模型/接口见 §3、§4。
3. 完成后：把该项 `- [ ]` 改为 `- [x]`，并更新 §0 的「当前阶段 / 下一步」。
4. 每阶段结束跑：`cd src-tauri && cargo check`、`npx vue-tsc --noEmit`、相关 Python `python -c "import ast,..."` 语法校验。
5. 决策若变更，先改 §2 再动代码。
