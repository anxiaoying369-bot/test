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
