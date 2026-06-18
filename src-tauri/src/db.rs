use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::fs;

pub fn init_db(data_dir: PathBuf) -> Result<Connection> {
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    let db_path = data_dir.join("video_studio.db");
    let conn = Connection::open(db_path)?;

    // 创建视频项目表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS video_projects (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            config TEXT, -- JSON
            status TEXT DEFAULT 'draft',
            is_locked INTEGER DEFAULT 0,
            locked_at TEXT,
            final_video_path TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 创建素材表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS video_materials (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            type TEXT NOT NULL, -- video, image, audio, script
            local_path TEXT,
            remote_url TEXT,
            meta TEXT, -- JSON
            source TEXT DEFAULT 'uploaded', -- uploaded / ai-generated / reference
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(project_id) REFERENCES video_projects(id)
        )",
        [],
    )?;

    // 创建任务表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS video_tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            type TEXT NOT NULL, -- generation, editing, export
            status TEXT NOT NULL, -- pending, processing, completed, error, cancelled
            progress INTEGER DEFAULT 0,
            result_path TEXT,
            error_msg TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 手机设备表：记录通过 WebSocket 连接过的手机及其备注
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mobile_devices (
            device_id TEXT PRIMARY KEY,
            model TEXT,
            remark TEXT,
            last_seen INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 发布任务表：一键发布 / 定时排期 / 多账号矩阵分发。
    // 矩阵 = 同一视频对多个账号各插一条任务；排期 = scheduled_at 为未来时间戳。
    conn.execute(
        "CREATE TABLE IF NOT EXISTS publish_tasks (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL DEFAULT 'douyin',
            account_name TEXT NOT NULL,
            video_path TEXT NOT NULL,
            title TEXT NOT NULL,
            tags TEXT,                -- JSON 数组
            description TEXT,
            cover_path TEXT,
            location TEXT,
            scheduled_at INTEGER,     -- Unix 秒；NULL/0 = 立即发布
            status TEXT NOT NULL DEFAULT 'pending', -- pending/publishing/success/failed/cancelled
            progress INTEGER DEFAULT 0,
            stage TEXT,
            error_msg TEXT,
            result_url TEXT,
            created_at INTEGER DEFAULT (strftime('%s','now')),
            updated_at INTEGER DEFAULT (strftime('%s','now'))
        )",
        [],
    )?;

    // ─── Migration：旧库幂等加列（SQLite ALTER ADD COLUMN 重复时返回错误，吞掉即可） ───
    let migrations: &[&str] = &[
        "ALTER TABLE video_projects ADD COLUMN is_locked INTEGER DEFAULT 0",
        "ALTER TABLE video_projects ADD COLUMN locked_at TEXT",
        "ALTER TABLE video_projects ADD COLUMN final_video_path TEXT",
        "ALTER TABLE video_materials ADD COLUMN source TEXT DEFAULT 'uploaded'",
    ];
    for sql in migrations {
        // 重复加列会报 duplicate column；其它错误也吞，启动不应该被这个挡住
        let _ = conn.execute(sql, []);
    }

    Ok(conn)
}

// ─── 手机设备（备注）───

pub struct MobileDeviceRecord {
    pub device_id: String,
    pub model: String,
    pub remark: Option<String>,
    pub last_seen: i64,
}

pub fn mobile_upsert_device(conn: &Connection, device_id: &str, model: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO mobile_devices (device_id, model, last_seen) VALUES (?1, ?2, strftime('%s','now'))
         ON CONFLICT(device_id) DO UPDATE SET model = ?2, last_seen = strftime('%s','now')",
        [device_id, model],
    )?;
    Ok(())
}

pub fn mobile_touch_device(conn: &Connection, device_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE mobile_devices SET last_seen = strftime('%s','now') WHERE device_id = ?1",
        [device_id],
    )?;
    Ok(())
}

pub fn mobile_set_remark(conn: &Connection, device_id: &str, remark: &str) -> Result<()> {
    conn.execute(
        "UPDATE mobile_devices SET remark = ?2 WHERE device_id = ?1",
        [device_id, remark],
    )?;
    Ok(())
}

pub fn mobile_delete_device(conn: &Connection, device_id: &str) -> Result<()> {
    conn.execute("DELETE FROM mobile_devices WHERE device_id = ?1", [device_id])?;
    Ok(())
}

pub fn mobile_list_devices(conn: &Connection) -> Result<Vec<MobileDeviceRecord>> {
    let mut stmt = conn.prepare(
        "SELECT device_id, COALESCE(model, ''), remark, COALESCE(last_seen, 0) FROM mobile_devices",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(MobileDeviceRecord {
            device_id: row.get(0)?,
            model: row.get(1)?,
            remark: row.get(2)?,
            last_seen: row.get(3)?,
        })
    })?;
    rows.collect()
}

// ─── 发布任务 ───

#[derive(Clone, serde::Serialize)]
pub struct PublishTaskRecord {
    pub id: String,
    pub platform: String,
    pub account_name: String,
    pub video_path: String,
    pub title: String,
    pub tags: Vec<String>,
    pub description: String,
    pub cover_path: Option<String>,
    pub location: Option<String>,
    pub scheduled_at: Option<i64>,
    pub status: String,
    pub progress: i64,
    pub stage: Option<String>,
    pub error_msg: Option<String>,
    pub result_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_publish_task(row: &rusqlite::Row) -> Result<PublishTaskRecord> {
    let tags_json: Option<String> = row.get(5)?;
    let tags: Vec<String> = tags_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    Ok(PublishTaskRecord {
        id: row.get(0)?,
        platform: row.get(1)?,
        account_name: row.get(2)?,
        video_path: row.get(3)?,
        title: row.get(4)?,
        tags,
        description: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        cover_path: row.get(7)?,
        location: row.get(8)?,
        scheduled_at: row.get(9)?,
        status: row.get(10)?,
        progress: row.get::<_, Option<i64>>(11)?.unwrap_or(0),
        stage: row.get(12)?,
        error_msg: row.get(13)?,
        result_url: row.get(14)?,
        created_at: row.get::<_, Option<i64>>(15)?.unwrap_or(0),
        updated_at: row.get::<_, Option<i64>>(16)?.unwrap_or(0),
    })
}

const PUBLISH_COLS: &str = "id, platform, account_name, video_path, title, tags, description, \
    cover_path, location, scheduled_at, status, progress, stage, error_msg, result_url, \
    created_at, updated_at";

#[allow(clippy::too_many_arguments)]
pub fn publish_create_task(
    conn: &Connection,
    id: &str,
    platform: &str,
    account_name: &str,
    video_path: &str,
    title: &str,
    tags_json: &str,
    description: &str,
    cover_path: Option<&str>,
    location: Option<&str>,
    scheduled_at: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO publish_tasks
            (id, platform, account_name, video_path, title, tags, description, cover_path, location, scheduled_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        rusqlite::params![id, platform, account_name, video_path, title, tags_json, description, cover_path, location, scheduled_at],
    )?;
    Ok(())
}

pub fn publish_list_tasks(conn: &Connection) -> Result<Vec<PublishTaskRecord>> {
    let sql = format!("SELECT {PUBLISH_COLS} FROM publish_tasks ORDER BY created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_publish_task)?;
    rows.collect()
}

pub fn publish_get_task(conn: &Connection, id: &str) -> Result<Option<PublishTaskRecord>> {
    let sql = format!("SELECT {PUBLISH_COLS} FROM publish_tasks WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map([id], row_to_publish_task)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

/// 已到点、待执行的任务：pending 且（无排期 或 排期时间已过）。
pub fn publish_list_due(conn: &Connection, now: i64) -> Result<Vec<PublishTaskRecord>> {
    let sql = format!(
        "SELECT {PUBLISH_COLS} FROM publish_tasks
         WHERE status = 'pending' AND (scheduled_at IS NULL OR scheduled_at = 0 OR scheduled_at <= ?1)
         ORDER BY created_at ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([now], row_to_publish_task)?;
    rows.collect()
}

pub fn publish_update_progress(conn: &Connection, id: &str, progress: i64, stage: &str) -> Result<()> {
    conn.execute(
        "UPDATE publish_tasks SET progress = ?2, stage = ?3, updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![id, progress, stage],
    )?;
    Ok(())
}

pub fn publish_set_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE publish_tasks SET status = ?2, updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![id, status],
    )?;
    Ok(())
}

pub fn publish_set_success(conn: &Connection, id: &str, result_url: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE publish_tasks SET status = 'success', progress = 100, error_msg = NULL,
            result_url = ?2, updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![id, result_url],
    )?;
    Ok(())
}

pub fn publish_set_failed(conn: &Connection, id: &str, error: &str) -> Result<()> {
    conn.execute(
        "UPDATE publish_tasks SET status = 'failed', error_msg = ?2, updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![id, error],
    )?;
    Ok(())
}

pub fn publish_delete_task(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM publish_tasks WHERE id = ?1", [id])?;
    Ok(())
}

/// 启动时把上次异常退出残留的 publishing 任务退回 pending，让调度器重新拾取。
pub fn publish_reset_stuck(conn: &Connection) -> Result<usize> {
    let n = conn.execute(
        "UPDATE publish_tasks SET status = 'pending', progress = 0, stage = NULL,
            updated_at = strftime('%s','now') WHERE status = 'publishing'",
        [],
    )?;
    Ok(n)
}
