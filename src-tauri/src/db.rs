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

        CREATE TABLE IF NOT EXISTS custom_apps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE,
            path TEXT,
            keywords TEXT
        );
        CREATE TABLE IF NOT EXISTS plugin_states (
            id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 1
        );
        "
    )?;

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

