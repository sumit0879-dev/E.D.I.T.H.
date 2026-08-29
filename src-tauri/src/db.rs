use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: Option<i32>,
    pub role: String,
    pub text: String,
    pub content: Option<String>, // Alias for text (for API compatibility)
    pub time: Option<String>,
    pub created_at: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
    pub id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomApp {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub keywords: String,
}

// Phase 5.6A: Browser History & Bookmarks Models
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserHistoryEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub visited_at: u64,
    pub tab_id: Option<String>,
    pub visit_count: u32,
    pub last_visited_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserBookmarkFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserBookmark {
    pub id: String,
    pub title: String,
    pub url: String,
    pub folder_id: Option<String>,
    pub favicon: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

// Phase 5.6B: Browser Download Model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserDownloadRecord {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub suggested_filename: String,
    pub destination: String,
    pub total_bytes: Option<u64>,
    pub received_bytes: u64,
    pub progress: f64,
    pub status: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
    pub tab_id: Option<String>,
}

// Phase 5.6C: Browser Profile Model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserProfileRecord {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub user_data_dir: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_default: bool,
    pub is_active: bool,
}

// Phase 5.6D: Browser Tab Session Model & Phase 5.6F-C Tab Group Association
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserTabRecord {
    pub id: String,
    pub url: String,
    pub title: String,
    pub profile_id: String,
    pub is_pinned: bool,
    pub is_active: bool,
    pub position: i64,
    pub group_id: Option<String>,
}

// Phase 5.6F-C: Tab Groups Model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserTabGroupRecord {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub color: String, // "blue", "purple", "green", "yellow", "orange", "red", "gray"
    pub is_collapsed: bool,
    pub position: i64,
    pub created_at: u64,
    pub updated_at: u64,
}

// Phase 5.6E: Privacy & Content Blocking Models
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserPrivacySettingsRecord {
    pub profile_id: String,
    pub enabled: bool,
    pub block_ads: bool,
    pub block_trackers: bool,
    pub send_dnt: bool,
    pub send_gpc: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserPrivacyAllowlistRecord {
    pub id: String,
    pub domain: String,
    pub profile_id: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserPrivacyRuleRecord {
    pub id: String,
    pub pattern: String,
    pub rule_type: String, // 'DOMAIN', 'WILDCARD', 'REGEX', 'KEYWORD'
    pub action: String,    // 'BLOCK', 'ALLOW'
    pub category: String,  // 'AD', 'TRACKER', 'MALWARE', 'CUSTOM'
    pub profile_id: String,
    pub enabled: bool,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserPrivacySourceRecord {
    pub id: String,
    pub name: String,
    pub url: String,
    pub version: String,
    pub rule_count: i64,
    pub updated_at: u64,
}

pub struct DbState {
    pub conn: Mutex<Connection>,
}

pub fn init_db_at(db_path: &PathBuf) -> Result<Connection> {

    // Create the parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).unwrap_or(());
        }
    }

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE IF NOT EXISTS personal_notes (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT, timestamp TEXT);
        CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, title TEXT, timestamp TEXT);
        CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT, role TEXT, text TEXT, time TEXT);

        CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            category TEXT NOT NULL,
            source TEXT NOT NULL,
            confidence REAL NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS knowledge_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            tags TEXT,
            summary TEXT
        );

        -- Phase 5.6A: Browser History & Bookmarks Tables
        CREATE TABLE IF NOT EXISTS browser_history (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            visit_count INTEGER NOT NULL DEFAULT 1,
            last_visited_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_browser_history_visited_at ON browser_history(last_visited_at DESC);
        CREATE INDEX IF NOT EXISTS idx_browser_history_url ON browser_history(url);

        CREATE TABLE IF NOT EXISTS browser_bookmark_folders (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS browser_bookmarks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            folder_id TEXT,
            favicon TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_browser_bookmarks_url ON browser_bookmarks(url);
        CREATE INDEX IF NOT EXISTS idx_browser_bookmarks_folder ON browser_bookmarks(folder_id);

        -- Phase 5.6B: Browser Downloads Persistent Storage
        CREATE TABLE IF NOT EXISTS browser_downloads (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            filename TEXT NOT NULL,
            suggested_filename TEXT NOT NULL,
            destination TEXT NOT NULL,
            total_bytes INTEGER,
            received_bytes INTEGER NOT NULL,
            progress REAL NOT NULL,
            status TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            completed_at INTEGER,
            error TEXT,
            tab_id TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_browser_downloads_started_at ON browser_downloads(started_at DESC);

        -- Step 19 Restart Recovery: Mark in-flight downloads as Failed on application startup
        UPDATE browser_downloads 
        SET status = 'FAILED', error = 'Interrupted by application restart' 
        WHERE status IN ('DOWNLOADING', 'QUEUED');

        -- Phase 5.6C: Browser Profiles Persistent Storage
        CREATE TABLE IF NOT EXISTS browser_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            profile_type TEXT NOT NULL,
            user_data_dir TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 0
        );

        -- Ensure default profile exists (Step 25)
        INSERT OR IGNORE INTO browser_profiles (id, name, profile_type, user_data_dir, created_at, updated_at, is_default, is_active)
        VALUES ('profile_default', 'Default Profile', 'DEFAULT', 'profiles/profile_default', 1700000000000, 1700000000000, 1, 1);

        -- Phase 5.6D: Tab State Persistence for Application Restart (Step 17 & 18)
        CREATE TABLE IF NOT EXISTS browser_tabs (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            profile_id TEXT NOT NULL,
            is_pinned INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 0,
            position INTEGER NOT NULL DEFAULT 0
        );

        -- Phase 5.6E: Content Blocking & Web Request Privacy Policy Engine
        CREATE TABLE IF NOT EXISTS browser_privacy_settings (
            profile_id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 1,
            block_ads INTEGER NOT NULL DEFAULT 1,
            block_trackers INTEGER NOT NULL DEFAULT 1,
            send_dnt INTEGER NOT NULL DEFAULT 1,
            send_gpc INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        INSERT OR IGNORE INTO browser_privacy_settings (profile_id, enabled, block_ads, block_trackers, send_dnt, send_gpc, created_at, updated_at)
        VALUES ('global', 1, 1, 1, 1, 1, 1700000000000, 1700000000000);

        CREATE TABLE IF NOT EXISTS browser_privacy_allowlist (
            id TEXT PRIMARY KEY,
            domain TEXT NOT NULL,
            profile_id TEXT NOT NULL DEFAULT 'global',
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_privacy_allowlist_domain ON browser_privacy_allowlist(domain);

        CREATE TABLE IF NOT EXISTS browser_privacy_rules (
            id TEXT PRIMARY KEY,
            pattern TEXT NOT NULL,
            rule_type TEXT NOT NULL,
            action TEXT NOT NULL,
            category TEXT NOT NULL,
            profile_id TEXT NOT NULL DEFAULT 'global',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_privacy_rules_profile ON browser_privacy_rules(profile_id);

        CREATE TABLE IF NOT EXISTS browser_privacy_sources (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            version TEXT NOT NULL,
            rule_count INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        -- Phase 5.6F-C: Tab Groups Storage
        CREATE TABLE IF NOT EXISTS browser_tab_groups (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            name TEXT NOT NULL,
            color TEXT NOT NULL,
            is_collapsed INTEGER NOT NULL DEFAULT 0,
            position INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tab_groups_profile ON browser_tab_groups(profile_id);
        "
    )?;

    // Non-destructive migrations for profile-scoping history & bookmarks (Step 24 & 25)
    let _ = conn.execute("ALTER TABLE browser_history ADD COLUMN profile_id TEXT DEFAULT 'profile_default';", []);
    let _ = conn.execute("ALTER TABLE browser_bookmarks ADD COLUMN profile_id TEXT DEFAULT 'profile_default';", []);
    // Phase 5.6F-C: Non-destructive migration for tab group association
    let _ = conn.execute("ALTER TABLE browser_tabs ADD COLUMN group_id TEXT;", []);

    Ok(conn)
}

// SEC-05 Hardening: Windows DPAPI Secret Protection with Custom Provider Credential Isolation
fn is_secret_key(key: &str) -> bool {
    let k = key.to_lowercase();
    k.contains("apikey") || k.contains("api_key") || k.ends_with("token") || k.ends_with("secret") || k.starts_with("custom_provider_key_")
}

// Settings
pub fn get_all_settings(conn: &Connection) -> Result<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

    let mut map = std::collections::HashMap::new();
    let mut raw_map = std::collections::HashMap::new();

    for row in rows {
        if let Ok((k, v)) = row {
            raw_map.insert(k.clone(), v.clone());
            let processed_v = if is_secret_key(&k) {
                crate::security::CredentialVault::unprotect(&v).unwrap_or(v)
            } else {
                v
            };
            map.insert(k, processed_v);
        }
    }

    // Reconstruct custom providers with securely decoupled credentials
    if let Some(cp_json) = map.get("customProviders") {
        if let Ok(mut list) = serde_json::from_str::<Vec<serde_json::Value>>(cp_json) {
            let mut modified_for_migration = false;

            for item in &mut list {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    let cred_key = format!("custom_provider_key_{}", id);
                    if let Some(enc_val) = raw_map.get(&cred_key) {
                        let plain_key = crate::security::CredentialVault::unprotect(enc_val).unwrap_or_default();
                        item["apiKey"] = serde_json::Value::String(plain_key);
                    } else if let Some(existing_key) = item.get("apiKey").and_then(|v| v.as_str()) {
                        // Migration: Encrypt legacy embedded plaintext key into separate vault key
                        if !existing_key.is_empty() && existing_key != "[PROTECTED_BY_DPAPI]" {
                            if let Ok(enc) = crate::security::CredentialVault::protect(existing_key) {
                                let _ = conn.execute(
                                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                                    rusqlite::params![cred_key, enc],
                                );
                                modified_for_migration = true;
                            }
                        }
                    }
                }
            }

            if modified_for_migration {
                // Save sanitized JSON back to disk without plaintext keys
                let mut sanitized_list = list.clone();
                for item in &mut sanitized_list {
                    if item.get("apiKey").is_some() {
                        item["apiKey"] = serde_json::Value::String("[PROTECTED_BY_DPAPI]".to_string());
                    }
                }
                if let Ok(sanitized_str) = serde_json::to_string(&sanitized_list) {
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO settings (key, value) VALUES ('customProviders', ?1)",
                        rusqlite::params![sanitized_str],
                    );
                }
            }

            if let Ok(reconstructed_str) = serde_json::to_string(&list) {
                map.insert("customProviders".to_string(), reconstructed_str);
            }
        }
    }

    Ok(map)
}

pub fn save_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    if key == "customProviders" {
        if let Ok(mut list) = serde_json::from_str::<Vec<serde_json::Value>>(value) {
            // For each provider, isolate apiKey, protect via DPAPI in separate key, and sanitize JSON
            for item in &mut list {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    let cred_key = format!("custom_provider_key_{}", id);
                    if let Some(api_key) = item.get("apiKey").and_then(|v| v.as_str()) {
                        if !api_key.is_empty() && api_key != "[PROTECTED_BY_DPAPI]" {
                            let protected = crate::security::CredentialVault::protect(api_key)
                                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))?;
                            conn.execute(
                                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                                rusqlite::params![cred_key, protected],
                            )?;
                        }
                    }
                    item["apiKey"] = serde_json::Value::String("[PROTECTED_BY_DPAPI]".to_string());
                }
            }
            let sanitized_json = serde_json::to_string(&list).unwrap_or_else(|_| value.to_string());
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, sanitized_json],
            )?;
            return Ok(());
        }
    }

    let stored_val = if is_secret_key(key) && !value.is_empty() {
        crate::security::CredentialVault::protect(value)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))?
    } else {
        value.to_string()
    };
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, stored_val],
    )?;
    Ok(())
}

// Sessions
pub fn create_session(conn: &Connection, id: &str, title: &str) -> Result<()> {
    let timestamp = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO sessions (id, title, timestamp) VALUES (?1, ?2, ?3)",
        params![id, title, timestamp],
    )?;
    Ok(())
}

pub fn get_all_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare("SELECT id, title FROM sessions ORDER BY timestamp DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Session {
            id: row.get(0)?,
            title: row.get(1)?,
        })
    })?;

    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

pub fn rename_session(conn: &Connection, id: &str, new_title: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET title = ?1 WHERE id = ?2",
        params![new_title, id],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id=?1", params![id])?;
    conn.execute("DELETE FROM messages WHERE session_id=?1", params![id])?;
    Ok(())
}

// Messages
pub fn save_session_message(
    conn: &Connection,
    session_id: &str,
    role: &str,
    text: &str,
    time: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO messages (session_id, role, text, time) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, role, text, time],
    )?;
    Ok(())
}

pub fn get_session_messages(conn: &Connection, session_id: &str) -> Result<Vec<Message>> {
    let mut stmt =
        conn.prepare("SELECT id, role, text, time FROM messages WHERE session_id=?1 ORDER BY id ASC")?;
    let rows = stmt.query_map(params![session_id], |row| {
        let text: String = row.get(2)?;
        Ok(Message {
            id: row.get::<_, i32>(0).ok(),
            role: row.get(1)?,
            text: text.clone(),
            content: Some(text),
            time: row.get::<_, String>(3).ok(),
            created_at: row.get::<_, String>(3).ok(),
            session_id: Some(session_id.to_string()),
        })
    })?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row?);
    }
    Ok(messages)
}

// Personal Notes
pub fn get_personal_notes(conn: &Connection) -> Result<Vec<Note>> {
    let mut stmt = conn.prepare("SELECT id, content FROM personal_notes ORDER BY id DESC")?;
    let rows = stmt.query_map([], |row| {
        let id: i32 = row.get(0)?;
        Ok(Note {
            id: id.to_string(),
            content: row.get(1)?,
        })
    })?;

    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}

pub fn save_personal_note(conn: &Connection, content: &str) -> Result<()> {
    let timestamp = chrono::Local::now().to_rfc3339();
    // SEC-08 Hardening: Non-destructive deterministic upsert semantics
    conn.execute(
        "INSERT INTO personal_notes (id, content, timestamp) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET content = excluded.content, timestamp = excluded.timestamp",
        params![content, timestamp],
    )?;
    Ok(())
}

pub fn delete_personal_note(conn: &Connection, note_id: &str) -> Result<()> {
    conn.execute("DELETE FROM personal_notes WHERE id=?1", params![note_id])?;
    Ok(())
}

// Custom Apps
pub fn get_custom_apps(conn: &Connection) -> Result<Vec<CustomApp>> {
    let mut stmt =
        conn.prepare("SELECT id, name, path, keywords FROM custom_apps ORDER BY name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(CustomApp {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            keywords: row.get(3)?,
        })
    })?;

    let mut apps = Vec::new();
    for row in rows {
        apps.push(row?);
    }
    Ok(apps)
}

pub fn add_custom_app(conn: &Connection, name: &str, path: &str, keywords: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO custom_apps (name, path, keywords) VALUES (?1, ?2, ?3)",
        params![
            name.to_lowercase().trim(),
            path.trim(),
            keywords.to_lowercase().trim()
        ],
    )?;
    Ok(())
}

pub fn delete_custom_app(conn: &Connection, app_id: i32) -> Result<()> {
    conn.execute("DELETE FROM custom_apps WHERE id=?1", params![app_id])?;
    Ok(())
}

// Plugin States
pub fn get_plugin_states(conn: &Connection) -> Result<std::collections::HashMap<String, bool>> {
    let mut stmt = conn.prepare("SELECT id, enabled FROM plugin_states")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        if let Ok((id, enabled)) = row {
            map.insert(id, enabled != 0);
        }
    }
    Ok(map)
}

pub fn set_plugin_state(conn: &Connection, plugin_id: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO plugin_states (id, enabled) VALUES (?1, ?2)",
        params![plugin_id, enabled as i32],
    )?;
    Ok(())
}

// ============================================================================
// Phase 5.6A: Browser History Database Helpers
// ============================================================================

pub fn add_browser_history_entry(
    conn: &Connection,
    url: &str,
    title: &str,
    tab_id: Option<&str>,
) -> Result<BrowserHistoryEntry> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let url_trimmed = url.trim();
    let title_trimmed = if title.trim().is_empty() { url_trimmed } else { title.trim() };

    // Dedup Policy: Check if same URL was visited within the last 15 seconds (15000 ms)
    let mut check_stmt = conn.prepare(
        "SELECT id, url, title, visited_at, tab_id, visit_count, last_visited_at 
         FROM browser_history 
         WHERE url = ?1 
         ORDER BY last_visited_at DESC 
         LIMIT 1"
    )?;

    let mut rows = check_stmt.query(params![url_trimmed])?;
    if let Some(row) = rows.next()? {
        let last_visited: u64 = row.get(6)?;
        if now.saturating_sub(last_visited) < 15_000 {
            let id: String = row.get(0)?;
            let current_count: u32 = row.get(5)?;
            let new_count = current_count + 1;
            conn.execute(
                "UPDATE browser_history SET title = ?1, visit_count = ?2, last_visited_at = ?3 WHERE id = ?4",
                params![title_trimmed, new_count, now, id],
            )?;
            return Ok(BrowserHistoryEntry {
                id,
                url: url_trimmed.to_string(),
                title: title_trimmed.to_string(),
                visited_at: row.get(3)?,
                tab_id: tab_id.map(|s| s.to_string()),
                visit_count: new_count,
                last_visited_at: now,
            });
        }
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO browser_history (id, url, title, visited_at, tab_id, visit_count, last_visited_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?4)",
        params![new_id, url_trimmed, title_trimmed, now, tab_id],
    )?;

    Ok(BrowserHistoryEntry {
        id: new_id,
        url: url_trimmed.to_string(),
        title: title_trimmed.to_string(),
        visited_at: now,
        tab_id: tab_id.map(|s| s.to_string()),
        visit_count: 1,
        last_visited_at: now,
    })
}

pub fn get_recent_browser_history(conn: &Connection, limit: Option<u32>) -> Result<Vec<BrowserHistoryEntry>> {
    let lim = limit.unwrap_or(50).clamp(1, 200);
    let mut stmt = conn.prepare(
        "SELECT id, url, title, visited_at, tab_id, visit_count, last_visited_at 
         FROM browser_history 
         ORDER BY last_visited_at DESC 
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![lim], |row| {
        Ok(BrowserHistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            visited_at: row.get(3)?,
            tab_id: row.get(4)?,
            visit_count: row.get(5)?,
            last_visited_at: row.get(6)?,
        })
    })?;

    let mut entries = Vec::new();
    for r in rows {
        entries.push(r?);
    }
    Ok(entries)
}

pub fn search_browser_history(conn: &Connection, query: &str, limit: Option<u32>) -> Result<Vec<BrowserHistoryEntry>> {
    let lim = limit.unwrap_or(50).clamp(1, 200);
    let pattern = format!("%{}%", query.trim().to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT id, url, title, visited_at, tab_id, visit_count, last_visited_at 
         FROM browser_history 
         WHERE LOWER(url) LIKE ?1 OR LOWER(title) LIKE ?1 
         ORDER BY last_visited_at DESC 
         LIMIT ?2"
    )?;

    let rows = stmt.query_map(params![pattern, lim], |row| {
        Ok(BrowserHistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            visited_at: row.get(3)?,
            tab_id: row.get(4)?,
            visit_count: row.get(5)?,
            last_visited_at: row.get(6)?,
        })
    })?;

    let mut entries = Vec::new();
    for r in rows {
        entries.push(r?);
    }
    Ok(entries)
}

pub fn delete_browser_history_entry(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM browser_history WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

pub fn clear_browser_history(conn: &Connection) -> Result<usize> {
    let count = conn.execute("DELETE FROM browser_history", [])?;
    Ok(count)
}

// ============================================================================
// Phase 5.6A: Browser Bookmarks Database Helpers
// ============================================================================

pub fn add_browser_bookmark(
    conn: &Connection,
    title: &str,
    url: &str,
    folder_id: Option<&str>,
    favicon: Option<&str>,
) -> Result<BrowserBookmark> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let url_trimmed = url.trim();
    let title_trimmed = if title.trim().is_empty() { url_trimmed } else { title.trim() };

    let mut check_stmt = conn.prepare("SELECT id, created_at FROM browser_bookmarks WHERE url = ?1 LIMIT 1")?;
    let mut rows = check_stmt.query(params![url_trimmed])?;
    if let Some(row) = rows.next()? {
        let existing_id: String = row.get(0)?;
        let created_at: u64 = row.get(1)?;
        conn.execute(
            "UPDATE browser_bookmarks SET title = ?1, folder_id = ?2, favicon = ?3, updated_at = ?4 WHERE id = ?5",
            params![title_trimmed, folder_id, favicon, now, existing_id],
        )?;
        return Ok(BrowserBookmark {
            id: existing_id,
            title: title_trimmed.to_string(),
            url: url_trimmed.to_string(),
            folder_id: folder_id.map(|s| s.to_string()),
            favicon: favicon.map(|s| s.to_string()),
            created_at,
            updated_at: now,
        });
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO browser_bookmarks (id, title, url, folder_id, favicon, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![new_id, title_trimmed, url_trimmed, folder_id, favicon, now],
    )?;

    Ok(BrowserBookmark {
        id: new_id,
        title: title_trimmed.to_string(),
        url: url_trimmed.to_string(),
        folder_id: folder_id.map(|s| s.to_string()),
        favicon: favicon.map(|s| s.to_string()),
        created_at: now,
        updated_at: now,
    })
}

pub fn update_browser_bookmark(
    conn: &Connection,
    id: &str,
    title: &str,
    url: &str,
    folder_id: Option<&str>,
) -> Result<bool> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let count = conn.execute(
        "UPDATE browser_bookmarks SET title = ?1, url = ?2, folder_id = ?3, updated_at = ?4 WHERE id = ?5",
        params![title.trim(), url.trim(), folder_id, now, id],
    )?;
    Ok(count > 0)
}

pub fn delete_browser_bookmark(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM browser_bookmarks WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

pub fn get_all_browser_bookmarks(conn: &Connection) -> Result<Vec<BrowserBookmark>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, url, folder_id, favicon, created_at, updated_at 
         FROM browser_bookmarks 
         ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(BrowserBookmark {
            id: row.get(0)?,
            title: row.get(1)?,
            url: row.get(2)?,
            folder_id: row.get(3)?,
            favicon: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    let mut bookmarks = Vec::new();
    for r in rows {
        bookmarks.push(r?);
    }
    Ok(bookmarks)
}

pub fn search_browser_bookmarks(conn: &Connection, query: &str) -> Result<Vec<BrowserBookmark>> {
    let pattern = format!("%{}%", query.trim().to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT id, title, url, folder_id, favicon, created_at, updated_at 
         FROM browser_bookmarks 
         WHERE LOWER(title) LIKE ?1 OR LOWER(url) LIKE ?1 
         ORDER BY updated_at DESC"
    )?;

    let rows = stmt.query_map(params![pattern], |row| {
        Ok(BrowserBookmark {
            id: row.get(0)?,
            title: row.get(1)?,
            url: row.get(2)?,
            folder_id: row.get(3)?,
            favicon: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    let mut bookmarks = Vec::new();
    for r in rows {
        bookmarks.push(r?);
    }
    Ok(bookmarks)
}

pub fn is_url_bookmarked(conn: &Connection, url: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM browser_bookmarks WHERE url = ?1")?;
    let count: i64 = stmt.query_row(params![url.trim()], |row| row.get(0))?;
    Ok(count > 0)
}

pub fn create_bookmark_folder(conn: &Connection, name: &str, parent_id: Option<&str>) -> Result<BrowserBookmarkFolder> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO browser_bookmark_folders (id, name, parent_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name.trim(), parent_id, now],
    )?;
    Ok(BrowserBookmarkFolder {
        id,
        name: name.trim().to_string(),
        parent_id: parent_id.map(|s| s.to_string()),
        created_at: now,
    })
}

pub fn delete_bookmark_folder(conn: &Connection, folder_id: &str) -> Result<bool> {
    conn.execute("UPDATE browser_bookmarks SET folder_id = NULL WHERE folder_id = ?1", params![folder_id])?;
    let count = conn.execute("DELETE FROM browser_bookmark_folders WHERE id = ?1", params![folder_id])?;
    Ok(count > 0)
}

// ============================================================================
// Phase 5.6B: Browser Downloads Database Helpers
// ============================================================================

pub fn upsert_browser_download(conn: &Connection, record: &BrowserDownloadRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO browser_downloads (
            id, url, filename, suggested_filename, destination,
            total_bytes, received_bytes, progress, status,
            started_at, completed_at, error, tab_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(id) DO UPDATE SET
            received_bytes = excluded.received_bytes,
            total_bytes = excluded.total_bytes,
            progress = excluded.progress,
            status = excluded.status,
            completed_at = excluded.completed_at,
            error = excluded.error;",
        params![
            record.id,
            record.url,
            record.filename,
            record.suggested_filename,
            record.destination,
            record.total_bytes,
            record.received_bytes,
            record.progress,
            record.status,
            record.started_at,
            record.completed_at,
            record.error,
            record.tab_id
        ],
    )?;
    Ok(())
}

pub fn get_browser_download(conn: &Connection, id: &str) -> Result<Option<BrowserDownloadRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, filename, suggested_filename, destination,
                total_bytes, received_bytes, progress, status,
                started_at, completed_at, error, tab_id
         FROM browser_downloads WHERE id = ?1 LIMIT 1"
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(BrowserDownloadRecord {
            id: row.get(0)?,
            url: row.get(1)?,
            filename: row.get(2)?,
            suggested_filename: row.get(3)?,
            destination: row.get(4)?,
            total_bytes: row.get(5)?,
            received_bytes: row.get(6)?,
            progress: row.get(7)?,
            status: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            error: row.get(11)?,
            tab_id: row.get(12)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_browser_downloads(conn: &Connection, limit: Option<u32>) -> Result<Vec<BrowserDownloadRecord>> {
    let lim = limit.unwrap_or(50).clamp(1, 200);
    let mut stmt = conn.prepare(
        "SELECT id, url, filename, suggested_filename, destination,
                total_bytes, received_bytes, progress, status,
                started_at, completed_at, error, tab_id
         FROM browser_downloads
         ORDER BY started_at DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![lim], |row| {
        Ok(BrowserDownloadRecord {
            id: row.get(0)?,
            url: row.get(1)?,
            filename: row.get(2)?,
            suggested_filename: row.get(3)?,
            destination: row.get(4)?,
            total_bytes: row.get(5)?,
            received_bytes: row.get(6)?,
            progress: row.get(7)?,
            status: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            error: row.get(11)?,
            tab_id: row.get(12)?,
        })
    })?;

    let mut downloads = Vec::new();
    for r in rows {
        downloads.push(r?);
    }
    Ok(downloads)
}

pub fn delete_browser_download_record(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM browser_downloads WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

pub fn clear_all_browser_download_records(conn: &Connection) -> Result<usize> {
    let count = conn.execute("DELETE FROM browser_downloads", [])?;
    Ok(count)
}

// ============================================================================
// Phase 5.6C: Browser Profiles Database Helpers
// ============================================================================

pub fn upsert_browser_profile(conn: &Connection, profile: &BrowserProfileRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO browser_profiles (
            id, name, profile_type, user_data_dir, created_at, updated_at, is_default, is_active
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            profile_type = excluded.profile_type,
            user_data_dir = excluded.user_data_dir,
            updated_at = excluded.updated_at,
            is_active = excluded.is_active;",
        params![
            profile.id,
            profile.name,
            profile.profile_type,
            profile.user_data_dir,
            profile.created_at,
            profile.updated_at,
            if profile.is_default { 1 } else { 0 },
            if profile.is_active { 1 } else { 0 }
        ],
    )?;
    Ok(())
}

pub fn get_browser_profile(conn: &Connection, id: &str) -> Result<Option<BrowserProfileRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, profile_type, user_data_dir, created_at, updated_at, is_default, is_active
         FROM browser_profiles WHERE id = ?1 LIMIT 1"
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(BrowserProfileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            profile_type: row.get(2)?,
            user_data_dir: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            is_default: row.get::<_, i64>(6)? != 0,
            is_active: row.get::<_, i64>(7)? != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_browser_profiles(conn: &Connection) -> Result<Vec<BrowserProfileRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, profile_type, user_data_dir, created_at, updated_at, is_default, is_active
         FROM browser_profiles
         ORDER BY is_default DESC, created_at ASC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(BrowserProfileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            profile_type: row.get(2)?,
            user_data_dir: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            is_default: row.get::<_, i64>(6)? != 0,
            is_active: row.get::<_, i64>(7)? != 0,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn set_active_browser_profile(conn: &Connection, profile_id: &str) -> Result<bool> {
    conn.execute("UPDATE browser_profiles SET is_active = 0", [])?;
    let count = conn.execute("UPDATE browser_profiles SET is_active = 1 WHERE id = ?1", params![profile_id])?;
    Ok(count > 0)
}

pub fn delete_browser_profile_record(conn: &Connection, profile_id: &str) -> Result<bool> {
    if profile_id == "profile_default" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let count = conn.execute("DELETE FROM browser_profiles WHERE id = ?1 AND is_default = 0", params![profile_id])?;
    Ok(count > 0)
}

// ============================================================================
// Phase 5.6D: Tab State Session Persistence Helpers (Step 17 & 18)
// ============================================================================

pub fn save_browser_tabs(conn: &Connection, tabs: &[BrowserTabRecord]) -> Result<()> {
    conn.execute("DELETE FROM browser_tabs", [])?;
    for (i, tab) in tabs.iter().enumerate() {
        conn.execute(
            "INSERT INTO browser_tabs (id, url, title, profile_id, is_pinned, is_active, position, group_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                tab.id,
                tab.url,
                tab.title,
                tab.profile_id,
                if tab.is_pinned { 1 } else { 0 },
                if tab.is_active { 1 } else { 0 },
                i as i64,
                tab.group_id,
            ],
        )?;
    }
    Ok(())
}

pub fn load_browser_tabs(conn: &Connection) -> Result<Vec<BrowserTabRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, profile_id, is_pinned, is_active, position, group_id
         FROM browser_tabs
         ORDER BY position ASC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(BrowserTabRecord {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            profile_id: row.get(3)?,
            is_pinned: row.get::<_, i64>(4)? != 0,
            is_active: row.get::<_, i64>(5)? != 0,
            position: row.get(6)?,
            group_id: row.get(7)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn clear_browser_tabs(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM browser_tabs", [])?;
    Ok(())
}

// ============================================================================
// Phase 5.6F-C: Tab Groups Database Helpers
// ============================================================================

pub fn upsert_browser_tab_group(conn: &Connection, group: &BrowserTabGroupRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO browser_tab_groups (
            id, profile_id, name, color, is_collapsed, position, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            profile_id = excluded.profile_id,
            name = excluded.name,
            color = excluded.color,
            is_collapsed = excluded.is_collapsed,
            position = excluded.position,
            updated_at = excluded.updated_at;",
        params![
            group.id,
            group.profile_id,
            group.name,
            group.color,
            if group.is_collapsed { 1 } else { 0 },
            group.position,
            group.created_at,
            group.updated_at,
        ],
    )?;
    Ok(())
}

pub fn get_browser_tab_group(conn: &Connection, id: &str) -> Result<Option<BrowserTabGroupRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, color, is_collapsed, position, created_at, updated_at
         FROM browser_tab_groups
         WHERE id = ?1 LIMIT 1"
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(BrowserTabGroupRecord {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            name: row.get(2)?,
            color: row.get(3)?,
            is_collapsed: row.get::<_, i64>(4)? != 0,
            position: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_browser_tab_groups(conn: &Connection, profile_id: Option<&str>) -> Result<Vec<BrowserTabGroupRecord>> {
    let mut list = Vec::new();
    if let Some(pid) = profile_id {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, color, is_collapsed, position, created_at, updated_at
             FROM browser_tab_groups
             WHERE profile_id = ?1
             ORDER BY position ASC, created_at ASC"
        )?;
        let rows = stmt.query_map(params![pid], |row| {
            Ok(BrowserTabGroupRecord {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                is_collapsed: row.get::<_, i64>(4)? != 0,
                position: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, color, is_collapsed, position, created_at, updated_at
             FROM browser_tab_groups
             ORDER BY position ASC, created_at ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BrowserTabGroupRecord {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                is_collapsed: row.get::<_, i64>(4)? != 0,
                position: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    }
    Ok(list)
}

pub fn delete_browser_tab_group(conn: &Connection, id: &str) -> Result<bool> {
    // Ungroup all associated tabs without deleting them (Step 5)
    let _ = conn.execute("UPDATE browser_tabs SET group_id = NULL WHERE group_id = ?1", params![id]);
    let count = conn.execute("DELETE FROM browser_tab_groups WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

pub fn set_browser_tab_group_collapsed(conn: &Connection, id: &str, is_collapsed: bool) -> Result<bool> {
    let count = conn.execute(
        "UPDATE browser_tab_groups SET is_collapsed = ?1, updated_at = ?2 WHERE id = ?3",
        params![if is_collapsed { 1 } else { 0 }, chrono::Utc::now().timestamp_millis() as u64, id],
    )?;
    Ok(count > 0)
}

// ============================================================================
// Phase 5.6E: Privacy & Content Blocking Database Helpers
// ============================================================================

pub fn get_browser_privacy_settings(conn: &Connection, profile_id: &str) -> Result<BrowserPrivacySettingsRecord> {
    let mut stmt = conn.prepare(
        "SELECT profile_id, enabled, block_ads, block_trackers, send_dnt, send_gpc, created_at, updated_at
         FROM browser_privacy_settings
         WHERE profile_id = ?1
         LIMIT 1"
    )?;

    let mut rows = stmt.query_map(params![profile_id], |row| {
        Ok(BrowserPrivacySettingsRecord {
            profile_id: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            block_ads: row.get::<_, i64>(2)? != 0,
            block_trackers: row.get::<_, i64>(3)? != 0,
            send_dnt: row.get::<_, i64>(4)? != 0,
            send_gpc: row.get::<_, i64>(5)? != 0,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    if let Some(r) = rows.next() {
        r
    } else {
        // Fallback to global settings
        let mut global_stmt = conn.prepare(
            "SELECT profile_id, enabled, block_ads, block_trackers, send_dnt, send_gpc, created_at, updated_at
             FROM browser_privacy_settings
             WHERE profile_id = 'global'
             LIMIT 1"
        )?;
        let mut global_rows = global_stmt.query_map([], |row| {
            Ok(BrowserPrivacySettingsRecord {
                profile_id: profile_id.to_string(),
                enabled: row.get::<_, i64>(1)? != 0,
                block_ads: row.get::<_, i64>(2)? != 0,
                block_trackers: row.get::<_, i64>(3)? != 0,
                send_dnt: row.get::<_, i64>(4)? != 0,
                send_gpc: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        if let Some(gr) = global_rows.next() {
            gr
        } else {
            Ok(BrowserPrivacySettingsRecord {
                profile_id: profile_id.to_string(),
                enabled: true,
                block_ads: true,
                block_trackers: true,
                send_dnt: true,
                send_gpc: true,
                created_at: 1700000000000,
                updated_at: 1700000000000,
            })
        }
    }
}

pub fn upsert_browser_privacy_settings(conn: &Connection, settings: &BrowserPrivacySettingsRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO browser_privacy_settings (profile_id, enabled, block_ads, block_trackers, send_dnt, send_gpc, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(profile_id) DO UPDATE SET
            enabled = excluded.enabled,
            block_ads = excluded.block_ads,
            block_trackers = excluded.block_trackers,
            send_dnt = excluded.send_dnt,
            send_gpc = excluded.send_gpc,
            updated_at = excluded.updated_at",
        params![
            settings.profile_id,
            if settings.enabled { 1 } else { 0 },
            if settings.block_ads { 1 } else { 0 },
            if settings.block_trackers { 1 } else { 0 },
            if settings.send_dnt { 1 } else { 0 },
            if settings.send_gpc { 1 } else { 0 },
            settings.created_at,
            settings.updated_at
        ],
    )?;
    Ok(())
}

pub fn list_browser_privacy_allowlist(conn: &Connection, profile_id: Option<&str>) -> Result<Vec<BrowserPrivacyAllowlistRecord>> {
    let mut list = Vec::new();
    if let Some(pid) = profile_id {
        let mut stmt = conn.prepare(
            "SELECT id, domain, profile_id, created_at
             FROM browser_privacy_allowlist
             WHERE profile_id = ?1 OR profile_id = 'global'
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![pid], |row| {
            Ok(BrowserPrivacyAllowlistRecord {
                id: row.get(0)?,
                domain: row.get(1)?,
                profile_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, domain, profile_id, created_at
             FROM browser_privacy_allowlist
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BrowserPrivacyAllowlistRecord {
                id: row.get(0)?,
                domain: row.get(1)?,
                profile_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    }
    Ok(list)
}

pub fn add_browser_privacy_allowlist(conn: &Connection, domain: &str, profile_id: &str) -> Result<BrowserPrivacyAllowlistRecord> {
    let id = format!("al_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp_millis() as u64;
    conn.execute(
        "INSERT INTO browser_privacy_allowlist (id, domain, profile_id, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, domain, profile_id, now],
    )?;
    Ok(BrowserPrivacyAllowlistRecord {
        id,
        domain: domain.to_string(),
        profile_id: profile_id.to_string(),
        created_at: now,
    })
}

pub fn remove_browser_privacy_allowlist(conn: &Connection, domain: &str, profile_id: Option<&str>) -> Result<bool> {
    let count = if let Some(pid) = profile_id {
        conn.execute(
            "DELETE FROM browser_privacy_allowlist WHERE domain = ?1 AND (profile_id = ?2 OR profile_id = 'global')",
            params![domain, pid],
        )?
    } else {
        conn.execute(
            "DELETE FROM browser_privacy_allowlist WHERE domain = ?1",
            params![domain],
        )?
    };
    Ok(count > 0)
}

pub fn list_browser_privacy_rules(conn: &Connection, profile_id: Option<&str>) -> Result<Vec<BrowserPrivacyRuleRecord>> {
    let mut list = Vec::new();
    if let Some(pid) = profile_id {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, rule_type, action, category, profile_id, enabled, created_at
             FROM browser_privacy_rules
             WHERE profile_id = ?1 OR profile_id = 'global'
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![pid], |row| {
            Ok(BrowserPrivacyRuleRecord {
                id: row.get(0)?,
                pattern: row.get(1)?,
                rule_type: row.get(2)?,
                action: row.get(3)?,
                category: row.get(4)?,
                profile_id: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, rule_type, action, category, profile_id, enabled, created_at
             FROM browser_privacy_rules
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BrowserPrivacyRuleRecord {
                id: row.get(0)?,
                pattern: row.get(1)?,
                rule_type: row.get(2)?,
                action: row.get(3)?,
                category: row.get(4)?,
                profile_id: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })?;
        for r in rows {
            list.push(r?);
        }
    }
    Ok(list)
}

pub fn add_browser_privacy_rule(conn: &Connection, rule: &BrowserPrivacyRuleRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO browser_privacy_rules (id, pattern, rule_type, action, category, profile_id, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            rule.id,
            rule.pattern,
            rule.rule_type,
            rule.action,
            rule.category,
            rule.profile_id,
            if rule.enabled { 1 } else { 0 },
            rule.created_at
        ],
    )?;
    Ok(())
}

pub fn delete_browser_privacy_rule(conn: &Connection, rule_id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM browser_privacy_rules WHERE id = ?1", params![rule_id])?;
    Ok(count > 0)
}

pub fn toggle_browser_privacy_rule(conn: &Connection, rule_id: &str, enabled: bool) -> Result<bool> {
    let count = conn.execute(
        "UPDATE browser_privacy_rules SET enabled = ?1 WHERE id = ?2",
        params![if enabled { 1 } else { 0 }, rule_id],
    )?;
    Ok(count > 0)
}




