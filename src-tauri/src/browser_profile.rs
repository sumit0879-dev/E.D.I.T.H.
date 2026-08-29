use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::db::{self, DbState, BrowserProfileRecord};
use crate::browser::BrowserState;

// ============================================================================
// PHASE 5.6C BROWSER PROFILE DATA MODEL (Step 2)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrowserProfileType {
    Default,
    User,
    Work,
    Research,
    AgentTemporary,
}

impl BrowserProfileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserProfileType::Default => "DEFAULT",
            BrowserProfileType::User => "USER",
            BrowserProfileType::Work => "WORK",
            BrowserProfileType::Research => "RESEARCH",
            BrowserProfileType::AgentTemporary => "AGENT_TEMPORARY",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DEFAULT" => BrowserProfileType::Default,
            "WORK" => BrowserProfileType::Work,
            "RESEARCH" => BrowserProfileType::Research,
            "AGENT_TEMPORARY" | "TEMPORARY" => BrowserProfileType::AgentTemporary,
            _ => BrowserProfileType::User,
        }
    }
}

pub struct BrowserProfileManager {
    pub active_profile_id: Mutex<String>,
}

impl Default for BrowserProfileManager {
    fn default() -> Self {
        Self {
            active_profile_id: Mutex::new("profile_default".to_string()),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_PROFILE_MGR: Arc<BrowserProfileManager> = Arc::new(BrowserProfileManager::default());
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

// ============================================================================
// USER-DATA DIRECTORY & PATH CONFINEMENT (Step 4, 26, 27)
// ============================================================================

pub fn get_profile_root_dir() -> PathBuf {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let path = PathBuf::from(user_profile).join(".gemini").join("antigravity-ide").join("edith_browser_profiles");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".gemini").join("antigravity-ide").join("edith_browser_profiles");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    let fallback = PathBuf::from("browser_profiles");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

pub fn sanitize_profile_id(id: &str) -> String {
    let cleaned: String = id.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "profile_custom".to_string()
    } else {
        cleaned
    }
}

pub fn get_profile_data_dir(profile_id: &str, is_temporary: bool) -> PathBuf {
    let sanitized = sanitize_profile_id(profile_id);
    let root = get_profile_root_dir();
    let path = if is_temporary {
        root.join("temporary").join(&sanitized)
    } else {
        root.join("profiles").join(&sanitized)
    };
    let _ = std::fs::create_dir_all(&path);
    path
}

// ============================================================================
// PROFILE MANAGEMENT OPERATIONS (Step 7, 8, 9, 11, 27)
// ============================================================================

impl BrowserProfileManager {
    pub fn get_active_profile_id(&self) -> String {
        self.active_profile_id.lock().unwrap().clone()
    }

    pub fn set_active_profile_id(&self, id: &str) {
        *self.active_profile_id.lock().unwrap() = id.to_string();
    }

    pub fn create_profile(
        &self,
        app: &AppHandle,
        name: &str,
        profile_type_str: &str,
        custom_id: Option<&str>,
    ) -> Result<BrowserProfileRecord, String> {
        let p_type = BrowserProfileType::from_str(profile_type_str);
        let id = custom_id
            .map(|s| sanitize_profile_id(s))
            .unwrap_or_else(|| {
                let suffix = uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>();
                format!("profile_{}", suffix)
            });

        let is_temp = p_type == BrowserProfileType::AgentTemporary;
        let data_dir = get_profile_data_dir(&id, is_temp);
        let now = current_timestamp_ms();

        let record = BrowserProfileRecord {
            id: id.clone(),
            name: name.trim().to_string(),
            profile_type: p_type.as_str().to_string(),
            user_data_dir: data_dir.to_string_lossy().to_string(),
            created_at: now,
            updated_at: now,
            is_default: false,
            is_active: false,
        };

        if let Some(db_state) = app.try_state::<DbState>() {
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            db::upsert_browser_profile(&conn, &record).map_err(|e| e.to_string())?;
        }

        Ok(record)
    }

    pub fn switch_profile(&self, app: &AppHandle, profile_id: &str) -> Result<BrowserProfileRecord, String> {
        let db_state = app.try_state::<DbState>()
            .ok_or_else(|| "Database state not initialized.".to_string())?;
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

        let profile = db::get_browser_profile(&conn, profile_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("PROFILE_NOT_FOUND: Profile '{}' does not exist.", profile_id))?;

        db::set_active_browser_profile(&conn, profile_id).map_err(|e| e.to_string())?;
        self.set_active_profile_id(profile_id);

        Ok(profile)
    }

    pub fn delete_profile(&self, app: &AppHandle, profile_id: &str, browser_state: &BrowserState) -> Result<bool, String> {
        if profile_id == "profile_default" {
            return Err("CANNOT_DELETE_DEFAULT: The default browser profile cannot be deleted.".to_string());
        }

        // Step 8 & 27: Verify no active tabs are using this profile
        let tabs = browser_state.tabs.lock().unwrap();
        let active_count = tabs.iter().filter(|t| t.profile_id == profile_id).count();
        if active_count > 0 {
            return Err(format!("PROFILE_IN_USE: Cannot delete profile '{}' because {} active tab(s) are currently attached to it.", profile_id, active_count));
        }
        drop(tabs);

        let db_state = app.try_state::<DbState>()
            .ok_or_else(|| "Database state not initialized.".to_string())?;
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

        // Retrieve record to get user_data_dir
        if let Some(record) = db::get_browser_profile(&conn, profile_id).map_err(|e| e.to_string())? {
            let path = PathBuf::from(&record.user_data_dir);
            let root = get_profile_root_dir();
            
            // Path security check (Step 26 & 27): ensure path is inside root
            if path.starts_with(&root) && path != root {
                let _ = std::fs::remove_dir_all(&path);
            }
        }

        db::delete_browser_profile_record(&conn, profile_id).map_err(|e| e.to_string())?;

        // If active profile was deleted, revert to default
        if self.get_active_profile_id() == profile_id {
            let _ = db::set_active_browser_profile(&conn, "profile_default");
            self.set_active_profile_id("profile_default");
        }

        Ok(true)
    }

    /// Step 11: Creates an isolated, disposable profile for autonomous AI research tasks
    pub fn create_agent_temporary_profile(&self, app: &AppHandle, task_id: &str) -> Result<BrowserProfileRecord, String> {
        let custom_id = format!("agent_{}", sanitize_profile_id(task_id));
        self.create_profile(app, &format!("AI Task {}", task_id), "AGENT_TEMPORARY", Some(&custom_id))
    }

    /// Step 11: Cleans up and deletes an agent temporary profile
    pub fn cleanup_agent_temporary_profile(&self, app: &AppHandle, profile_id: &str, browser_state: &BrowserState) -> Result<bool, String> {
        if !profile_id.starts_with("agent_") && !profile_id.contains("temporary") {
            return Err("INVALID_OPERATION: cleanup_agent_temporary_profile can only be used on temporary profiles.".to_string());
        }
        self.delete_profile(app, profile_id, browser_state)
    }
}

// ============================================================================
// TAURI COMMANDS FOR PROFILES
// ============================================================================

#[tauri::command]
pub fn browser_profiles_list(db_state: State<'_, DbState>) -> Result<Vec<BrowserProfileRecord>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::list_browser_profiles(&conn).map_err(|e| format!("DB_ERROR: Failed to list profiles: {}", e))
}

#[tauri::command]
pub fn browser_profile_get(profile_id: String, db_state: State<'_, DbState>) -> Result<Option<BrowserProfileRecord>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::get_browser_profile(&conn, &profile_id).map_err(|e| format!("DB_ERROR: Failed to get profile: {}", e))
}

#[tauri::command]
pub fn browser_profile_create(
    app: AppHandle,
    name: String,
    profile_type: Option<String>,
) -> Result<BrowserProfileRecord, String> {
    let p_type = profile_type.unwrap_or_else(|| "USER".to_string());
    GLOBAL_PROFILE_MGR.create_profile(&app, &name, &p_type, None)
}

#[tauri::command]
pub fn browser_profile_switch(app: AppHandle, profile_id: String) -> Result<BrowserProfileRecord, String> {
    GLOBAL_PROFILE_MGR.switch_profile(&app, &profile_id)
}

#[tauri::command]
pub fn browser_profile_rename(
    profile_id: String,
    new_name: String,
    db_state: State<'_, DbState>,
) -> Result<BrowserProfileRecord, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let mut profile = db::get_browser_profile(&conn, &profile_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("PROFILE_NOT_FOUND: Profile '{}' does not exist.", profile_id))?;

    profile.name = new_name.trim().to_string();
    profile.updated_at = current_timestamp_ms();

    db::upsert_browser_profile(&conn, &profile).map_err(|e| e.to_string())?;
    Ok(profile)
}

#[tauri::command]
pub fn browser_profile_delete(
    app: AppHandle,
    profile_id: String,
    state: State<'_, BrowserState>,
) -> Result<bool, String> {
    GLOBAL_PROFILE_MGR.delete_profile(&app, &profile_id, &state)
}

#[tauri::command]
pub fn browser_profile_create_temporary(app: AppHandle, task_id: String) -> Result<BrowserProfileRecord, String> {
    GLOBAL_PROFILE_MGR.create_agent_temporary_profile(&app, &task_id)
}

#[tauri::command]
pub fn browser_profile_cleanup_temporary(
    app: AppHandle,
    profile_id: String,
    state: State<'_, BrowserState>,
) -> Result<bool, String> {
    GLOBAL_PROFILE_MGR.cleanup_agent_temporary_profile(&app, &profile_id, &state)
}
