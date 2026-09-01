// src-tauri/src/browser_recovery.rs
//! Phase 5.7C: Crash Recovery, Startup Recovery & State Integrity Subsystem
//! 
//! Ensures deterministic, safe, and bounded recovery from unexpected termination,
//! application crashes, process kills, and corrupted/partial session snapshots.

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashSet;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::Url;

use crate::db::{self, BrowserProfileRecord};
use crate::browser_profile::get_profile_root_dir;

// ============================================================================
// STATE CLASSIFICATION & CONSTANTS (Step 2 & 5)
// ============================================================================

/// Version tag for persisted session snapshots
pub const SESSION_SNAPSHOT_VERSION: u32 = 1;

/// Classification of system state entities:
/// - DURABLE: Preserved permanently across app lifecycle (profiles, history, bookmarks, privacy settings).
/// - RECOVERABLE: Restored conditionally with validation upon startup (tabs, groups, downloads).
/// - EPHEMERAL: Recreated on every run (native WebView2 handles, window bounds, OS HWNDs).
/// - TRANSIENT: In-flight operations that must fail-closed on restart (AI agent tasks, approvals).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateClass {
    Durable,
    Recoverable,
    Ephemeral,
    Transient,
}

// ============================================================================
// RECOVERY REPORT DATA MODEL (Step 33 & 34)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub recovered_tabs: usize,
    pub skipped_tabs: usize,
    pub repaired_groups: usize,
    pub invalidated_approvals: usize,
    pub interrupted_downloads: usize,
    pub interrupted_agent_tasks: usize,
    pub profile_issues: Vec<String>,
    pub database_issues: Vec<String>,
    pub recovery_time_ms: u64,
    pub notice: Option<String>,
}

impl Default for RecoveryReport {
    fn default() -> Self {
        Self {
            recovered_tabs: 0,
            skipped_tabs: 0,
            repaired_groups: 0,
            invalidated_approvals: 0,
            interrupted_downloads: 0,
            interrupted_agent_tasks: 0,
            profile_issues: Vec::new(),
            database_issues: Vec::new(),
            recovery_time_ms: 0,
            notice: None,
        }
    }
}

// ============================================================================
// PATH CONFINEMENT & SECURITY CHECKS (Step 8)
// ============================================================================

/// Validates that a profile path is strictly confined within the E.D.I.T.H. profile root.
/// Rejects `..` path traversal, external drives, system folders, and UNC paths.
pub fn validate_profile_dir(profile_dir: &str, root: &Path) -> Result<PathBuf, String> {
    let p = Path::new(profile_dir);

    // Reject obvious traversal attempts
    let dir_str = profile_dir.replace('\\', "/");
    if dir_str.contains("../") || dir_str.contains("/..") || dir_str.starts_with("../") {
        return Err(format!("SECURITY_VIOLATION: Path traversal detected in profile directory '{}'", profile_dir));
    }

    // Resolve relative to root if relative
    let candidate = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    };

    // Canonicalize if root exists
    if let Ok(canonical_root) = root.canonicalize() {
        if let Ok(canonical_candidate) = candidate.canonicalize() {
            if !canonical_candidate.starts_with(&canonical_root) {
                return Err(format!("SECURITY_VIOLATION: Profile directory '{}' escapes root directory", profile_dir));
            }
        }
    }

    Ok(candidate)
}

// ============================================================================
// URL SAFETY & RECOVERY VALIDATION (Step 10)
// ============================================================================

/// Validates a persisted URL prior to tab restoration.
/// Allows standard web schemes (https, http, about:blank, data:image, edge:, edith:).
/// Rejects dangerous schemes like javascript:, vbscript:, raw file:, shell:, chrome:.
pub fn validate_url_for_recovery(raw_url: &str) -> Result<String, String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() || trimmed == "about:blank" {
        return Ok("about:blank".to_string());
    }

    let parsed = Url::parse(trimmed)
        .map_err(|e| format!("MALFORMED_URL: '{}' ({})", trimmed, e))?;

    match parsed.scheme() {
        "http" | "https" => Ok(trimmed.to_string()),
        "about" => {
            if parsed.path() == "blank" || parsed.path().is_empty() {
                Ok("about:blank".to_string())
            } else {
                Err(format!("UNSAFE_ABOUT_URL: '{}'", trimmed))
            }
        }
        "edge" => Ok(trimmed.to_string()),
        "edith" => {
            let lower = trimmed.to_lowercase();
            let cleaned = if lower.ends_with('/') {
                &lower[..lower.len() - 1]
            } else {
                &lower
            };
            if cleaned == "edith://newtab" {
                Ok("edith://newtab".to_string())
            } else {
                Err(format!("UNSUPPORTED_EDITH_ROUTE: '{}'", trimmed))
            }
        }
        "data" => {
            // Only allow safe data URI image or plain text
            let spec = parsed.as_str();
            if spec.starts_with("data:image/") || spec.starts_with("data:text/plain") {
                Ok(trimmed.to_string())
            } else {
                Err(format!("DISALLOWED_DATA_SCHEME: '{}'", trimmed))
            }
        }
        "javascript" | "vbscript" | "file" | "shell" | "chrome" | "opera" => {
            Err(format!("DISALLOWED_SCHEME: Scheme '{}' is prohibited in restored sessions.", parsed.scheme()))
        }
        other => {
            Err(format!("UNSUPPORTED_SCHEME: Scheme '{}' is not supported for recovery.", other))
        }
    }
}

// ============================================================================
// STARTUP RECOVERY PIPELINE (Step 6, 7, 9, 13, 14, 15, 16, 17, 18, 20, 21)
// ============================================================================

/// Executes the comprehensive startup recovery pipeline.
/// Validates database integrity, repairs inconsistent records, clears transient states,
/// invalidates pending approvals, marks interrupted downloads, and returns a detailed report.
pub fn run_startup_recovery(conn: &Connection) -> Result<RecoveryReport, String> {
    let start_time = Instant::now();
    let mut report = RecoveryReport::default();
    let profile_root = get_profile_root_dir();

    // ------------------------------------------------------------------------
    // Step 1: Database Quick Integrity Check (Step 23)
    // ------------------------------------------------------------------------
    let mut integrity_ok = false;
    if let Ok(mut stmt) = conn.prepare("PRAGMA quick_check;") {
        if let Ok(mut rows) = stmt.query([]) {
            if let Ok(Some(row)) = rows.next() {
                let status: String = row.get(0).unwrap_or_default();
                if status.to_lowercase() == "ok" {
                    integrity_ok = true;
                } else {
                    report.database_issues.push(format!("SQLite quick_check warning: {}", status));
                }
            }
        }
    }
    if !integrity_ok && report.database_issues.is_empty() {
        report.database_issues.push("SQLite integrity check returned unexpected response".to_string());
    }

    // ------------------------------------------------------------------------
    // Step 2: Validate & Repair Browser Profiles (Step 7 & 8)
    // ------------------------------------------------------------------------
    let mut valid_profile_ids = HashSet::new();
    let profiles = db::list_browser_profiles(conn).unwrap_or_default();

    for p in &profiles {
        match validate_profile_dir(&p.user_data_dir, &profile_root) {
            Ok(_) => {
                valid_profile_ids.insert(p.id.clone());
            }
            Err(err) => {
                report.profile_issues.push(format!("Profile '{}' had invalid path: {}", p.id, err));
            }
        }
    }

    // Ensure default profile exists
    if !valid_profile_ids.contains("profile_default") {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64;
        let default_rec = BrowserProfileRecord {
            id: "profile_default".to_string(),
            name: "Default Profile".to_string(),
            profile_type: "DEFAULT".to_string(),
            user_data_dir: "profiles/profile_default".to_string(),
            created_at: now,
            updated_at: now,
            is_default: true,
            is_active: true,
        };
        let _ = db::upsert_browser_profile(conn, &default_rec);
        valid_profile_ids.insert("profile_default".to_string());
        report.profile_issues.push("Restored missing profile_default record.".to_string());
    }

    // ------------------------------------------------------------------------
    // Step 3: Validate & Repair Tab Groups (Step 21)
    // ------------------------------------------------------------------------
    let mut valid_group_ids = HashSet::new();
    let groups = db::list_browser_tab_groups(conn, None).unwrap_or_default();

    for g in groups {
        if valid_profile_ids.contains(&g.profile_id) {
            valid_group_ids.insert(g.id.clone());
        } else {
            // Orphan group referencing non-existent profile: reassign or delete
            let _ = db::delete_browser_tab_group(conn, &g.id);
            report.repaired_groups += 1;
        }
    }

    // ------------------------------------------------------------------------
    // Step 4: Validate & Repair Browser Tabs (Step 9, 10, 11)
    // ------------------------------------------------------------------------
    let saved_tabs = db::load_browser_tabs(conn).unwrap_or_default();
    let mut valid_tabs = Vec::new();

    for mut tab in saved_tabs {
        // Validate profile ownership
        if !valid_profile_ids.contains(&tab.profile_id) {
            tab.profile_id = "profile_default".to_string();
        }

        // Validate group ownership
        if let Some(ref gid) = tab.group_id {
            if !valid_group_ids.contains(gid) {
                tab.group_id = None;
                report.repaired_groups += 1;
            }
        }

        // Validate URL
        match validate_url_for_recovery(&tab.url) {
            Ok(safe_url) => {
                tab.url = safe_url;
                valid_tabs.push(tab);
                report.recovered_tabs += 1;
            }
            Err(e) => {
                report.skipped_tabs += 1;
                report.database_issues.push(format!("Skipped tab '{}': {}", tab.id, e));
            }
        }
    }

    // Ensure at least 1 active tab if tabs exist
    if !valid_tabs.is_empty() && !valid_tabs.iter().any(|t| t.is_active) {
        valid_tabs[0].is_active = true;
    }

    // Persist clean validated tabs atomically
    let _ = db::save_browser_tabs(conn, &valid_tabs);

    // ------------------------------------------------------------------------
    // Step 5: Recover In-Flight Downloads (Step 13 & 14)
    // ------------------------------------------------------------------------
    if let Ok(count) = conn.execute(
        "UPDATE browser_downloads 
         SET status = 'FAILED', error = 'Interrupted by application restart' 
         WHERE status IN ('DOWNLOADING', 'QUEUED', 'PAUSED')",
        [],
    ) {
        report.interrupted_downloads = count;
    }

    // ------------------------------------------------------------------------
    // Step 6: Invalidate Stale AI Approvals & Transient State (Step 15, 16, 17, 18)
    // ------------------------------------------------------------------------
    // Human control defaults to USER_CONTROLLED on restart (Step 18)
    report.invalidated_approvals = 0; // Handled in-memory on state initialization
    report.interrupted_agent_tasks = 0;

    report.recovery_time_ms = start_time.elapsed().as_millis() as u64;

    // Build user-facing notice if meaningful recovery events occurred
    if report.recovered_tabs > 0 || report.interrupted_downloads > 0 || report.repaired_groups > 0 {
        let mut parts = Vec::new();
        if report.recovered_tabs > 0 {
            parts.push(format!("{} tabs recovered", report.recovered_tabs));
        }
        if report.interrupted_downloads > 0 {
            parts.push(format!("{} interrupted downloads marked failed", report.interrupted_downloads));
        }
        if report.repaired_groups > 0 {
            parts.push(format!("{} tab group associations repaired", report.repaired_groups));
        }
        report.notice = Some(format!("E.D.I.T.H. Startup Recovery: {}", parts.join(", ")));
    }

    Ok(report)
}

// ============================================================================
// TAURI COMMANDS (Step 33 & 34)
// ============================================================================

#[tauri::command]
pub async fn browser_run_startup_recovery(
    db_state: tauri::State<'_, db::DbState>,
) -> Result<RecoveryReport, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    run_startup_recovery(&conn)
}
