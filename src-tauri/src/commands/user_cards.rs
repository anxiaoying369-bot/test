use std::fs;
use std::path::PathBuf;
use serde_json::Value;
use crate::models::{UserCard, UserCardsStoreFile};
use crate::utils::{get_data_dir, chrono_now};
use crate::commands::scraper::fetch_douyin_user_info;

fn store_path() -> PathBuf {
    get_data_dir().join("user_cards.json")
}

pub fn load_user_cards() -> UserCardsStoreFile {
    let path = store_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UserCardsStoreFile::default()
    }
}

fn save_user_cards(store: &UserCardsStoreFile) -> Result<(), String> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

/// 数值字段兼容 number / 字符串两种返回。
fn as_i64(v: &Value, key: &str) -> i64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn as_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// 把查询脚本返回的 user 对象映射为 UserCard。
/// 兼容快脚本(ip_location/unique_id) 与浏览器脚本(location) 的字段命名差异。
fn value_to_card(user: &Value) -> UserCard {
    let ip_location = {
        let a = as_str(user, "ip_location");
        if a.is_empty() { as_str(user, "location") } else { a }
    };
    UserCard {
        sec_uid: as_str(user, "sec_uid"),
        uid: as_str(user, "uid"),
        unique_id: as_str(user, "unique_id"),
        nickname: as_str(user, "nickname"),
        avatar_url: as_str(user, "avatar_url"),
        signature: as_str(user, "signature"),
        follower_count: as_i64(user, "follower_count"),
        following_count: as_i64(user, "following_count"),
        total_favorited: as_i64(user, "total_favorited"),
        aweme_count: as_i64(user, "aweme_count"),
        ip_location,
        updated_at: chrono_now(),
    }
}

fn upsert_card(card: UserCard) -> Result<(), String> {
    if card.sec_uid.is_empty() {
        return Err("查询结果缺少 sec_uid，无法入库".to_string());
    }
    let mut store = load_user_cards();
    if let Some(existing) = store.cards.iter_mut().find(|c| c.sec_uid == card.sec_uid) {
        *existing = card;
    } else {
        store.cards.push(card);
    }
    save_user_cards(&store)
}

#[tauri::command]
pub async fn list_user_cards() -> Result<Vec<UserCard>, String> {
    let mut store = load_user_cards();
    // 最近更新的排前面
    store.cards.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(store.cards)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_user_card(secUid: String) -> Result<(), String> {
    let mut store = load_user_cards();
    store.cards.retain(|c| c.sec_uid != secUid);
    save_user_cards(&store)
}

/// 查询用户并入库；返回入库后的卡片。user_id 支持 sec_uid / 主页链接（短链/数字走兜底脚本）。
#[tauri::command]
pub async fn query_and_save_user(account_name: String, user_id: String) -> Result<UserCard, String> {
    let result = fetch_douyin_user_info(account_name, user_id).await?;
    let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("error");
    if status != "ok" && status != "partial" {
        let err = result.get("error").and_then(|e| e.as_str()).unwrap_or("查询失败");
        return Err(err.to_string());
    }
    let user = result.get("user").ok_or_else(|| "查询结果缺少 user 字段".to_string())?;
    let card = value_to_card(user);
    upsert_card(card.clone())?;
    Ok(card)
}

/// 按已知 sec_uid 重新查询并刷新卡片。
#[tauri::command]
#[allow(non_snake_case)]
pub async fn refresh_user_card(account_name: String, secUid: String) -> Result<UserCard, String> {
    query_and_save_user(account_name, secUid).await
}
