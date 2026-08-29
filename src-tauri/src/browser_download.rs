use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{self, DbState, BrowserDownloadRecord};

// ============================================================================
// PHASE 5.6B DOWNLOAD DATA MODEL & STATUS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

impl DownloadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DownloadStatus::Queued => "QUEUED",
            DownloadStatus::Downloading => "DOWNLOADING",
            DownloadStatus::Paused => "PAUSED",
            DownloadStatus::Completed => "COMPLETED",
            DownloadStatus::Failed => "FAILED",
            DownloadStatus::Cancelled => "CANCELLED",
            DownloadStatus::Blocked => "BLOCKED",
        }
    }
}

pub struct ActiveDownloadHandle {
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct BrowserDownloadManager {
    pub active_downloads: Mutex<HashMap<String, ActiveDownloadHandle>>,
    pub http_client: Client,
}

impl Default for BrowserDownloadManager {
    fn default() -> Self {
        Self {
            active_downloads: Mutex::new(HashMap::new()),
            http_client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_default(),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_DOWNLOAD_MGR: Arc<BrowserDownloadManager> = Arc::new(BrowserDownloadManager::default());
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

// ============================================================================
// DESTINATION & FILENAME SANITIZATION POLICIES (Steps 4, 5, 20)
// ============================================================================

/// Returns a secure, controlled downloads directory owned by E.D.I.T.H.
pub fn get_safe_downloads_dir() -> PathBuf {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let path = PathBuf::from(user_profile).join("Downloads").join("EDITH_Downloads");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }

    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("Downloads").join("EDITH_Downloads");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }

    let fallback = PathBuf::from("downloads");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Sanitizes a remote/suggested filename against path traversal, control characters, and Windows reserved names.
pub fn sanitize_filename(suggested: &str) -> String {
    let mut cleaned = suggested.trim().replace('\\', "/");
    
    // Extract base filename only (strip any path components)
    if let Some(pos) = cleaned.rfind('/') {
        cleaned = cleaned[(pos + 1)..].to_string();
    }

    // Strip ../ or ..
    cleaned = cleaned.replace("..", "").replace('/', "");

    // Replace invalid Windows characters: < > : " / \ | ? *
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    cleaned = cleaned.chars().map(|c| if invalid_chars.contains(&c) || c.is_control() { '_' } else { c }).collect();
    cleaned = cleaned.trim_matches(['.', ' ']).to_string();

    // Check against Windows reserved device names
    let upper = cleaned.to_uppercase();
    let base_stem = upper.split('.').next().unwrap_or("");
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
    ];

    if reserved.contains(&base_stem) {
        cleaned = format!("download_{}", cleaned);
    }

    if cleaned.is_empty() {
        "downloaded_file.bin".to_string()
    } else {
        cleaned
    }
}

/// Computes a unique collision-free destination file path.
pub fn resolve_collision_path(base_dir: &Path, filename: &str) -> (PathBuf, String) {
    let target = base_dir.join(filename);
    if !target.exists() {
        return (target, filename.to_string());
    }

    let path_obj = Path::new(filename);
    let stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path_obj.extension().and_then(|s| s.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();

    for index in 1..1000 {
        let candidate_name = format!("{} ({}){}", stem, index, ext);
        let candidate_path = base_dir.join(&candidate_name);
        if !candidate_path.exists() {
            return (candidate_path, candidate_name);
        }
    }

    let fallback_name = format!("{}_{}{}", stem, current_timestamp_ms(), ext);
    let fallback_path = base_dir.join(&fallback_name);
    (fallback_path, fallback_name)
}

// ============================================================================
// DOWNLOAD EXECUTION ENGINE (Steps 6, 7, 11, 12, 13, 20)
// ============================================================================

impl BrowserDownloadManager {
    /// Initiates an asynchronous, progress-streamed download
    pub fn start_download(
        self: &Arc<Self>,
        app: AppHandle,
        url: String,
        tab_id: Option<String>,
        suggested_name: Option<String>,
    ) -> Result<BrowserDownloadRecord, String> {
        let url_trimmed = url.trim().to_string();
        if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
            return Err("INVALID_URL: Only http:// and https:// URLs can be downloaded.".to_string());
        }

        let raw_name = suggested_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                url_trimmed
                    .split('?')
                    .next()
                    .and_then(|s| s.split('/').last())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("download.bin")
                    .to_string()
            });

        let sanitized = sanitize_filename(&raw_name);
        let safe_dir = get_safe_downloads_dir();
        let (final_path, final_filename) = resolve_collision_path(&safe_dir, &sanitized);
        let temp_path = safe_dir.join(format!("{}.edith-download", final_filename));

        let download_id = uuid::Uuid::new_v4().to_string();
        let now = current_timestamp_ms();

        let initial_record = BrowserDownloadRecord {
            id: download_id.clone(),
            url: url_trimmed.clone(),
            filename: final_filename.clone(),
            suggested_filename: raw_name,
            destination: final_path.to_string_lossy().to_string(),
            total_bytes: None,
            received_bytes: 0,
            progress: 0.0,
            status: DownloadStatus::Downloading.as_str().to_string(),
            started_at: now,
            completed_at: None,
            error: None,
            tab_id: tab_id.clone(),
        };

        // Persist initial record in SQLite
        if let Some(db_state) = app.try_state::<DbState>() {
            if let Ok(conn) = db_state.conn.lock() {
                let _ = db::upsert_browser_download(&conn, &initial_record);
            }
        }

        // Register cancellation token
        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active_downloads.lock().unwrap();
            active.insert(download_id.clone(), ActiveDownloadHandle {
                cancel_flag: cancel_flag.clone(),
            });
        }

        // Emit started event
        let _ = app.emit("browser-download-progress", json!(&initial_record));

        let mgr = self.clone();
        let app_clone = app.clone();
        let id_clone = download_id.clone();
        let url_clone = url_trimmed.clone();
        let tab_id_clone = tab_id.clone();
        let initial_record_clone = initial_record.clone();

        // Spawn async streaming worker
        tokio::spawn(async move {
            let res = mgr.execute_download_stream(
                app_clone.clone(),
                id_clone.clone(),
                url_clone,
                tab_id_clone,
                initial_record_clone.filename.clone(),
                temp_path.clone(),
                final_path.clone(),
                cancel_flag,
            ).await;

            // Remove from active list
            {
                let mut active = mgr.active_downloads.lock().unwrap();
                active.remove(&id_clone);
            }

            if let Err(e) = res {
                let _ = std::fs::remove_file(&temp_path);
                let fail_now = current_timestamp_ms();
                let failed_record = BrowserDownloadRecord {
                    id: id_clone.clone(),
                    url: initial_record_clone.url.clone(),
                    filename: initial_record_clone.filename.clone(),
                    suggested_filename: initial_record_clone.suggested_filename.clone(),
                    destination: initial_record_clone.destination.clone(),
                    total_bytes: initial_record_clone.total_bytes,
                    received_bytes: 0,
                    progress: 0.0,
                    status: if e.contains("CANCELLED") { DownloadStatus::Cancelled.as_str().to_string() } else { DownloadStatus::Failed.as_str().to_string() },
                    started_at: initial_record_clone.started_at,
                    completed_at: Some(fail_now),
                    error: Some(e.clone()),
                    tab_id: initial_record_clone.tab_id.clone(),
                };

                if let Some(db_state) = app_clone.try_state::<DbState>() {
                    if let Ok(conn) = db_state.conn.lock() {
                        let _ = db::upsert_browser_download(&conn, &failed_record);
                    }
                }
                let _ = app_clone.emit("browser-download-progress", json!(&failed_record));
            }
        });

        Ok(initial_record)
    }

    async fn execute_download_stream(
        &self,
        app: AppHandle,
        download_id: String,
        url: String,
        tab_id: Option<String>,
        filename: String,
        temp_path: PathBuf,
        final_path: PathBuf,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let mut resp = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| format!("NETWORK_ERROR: Failed to connect: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP_ERROR: Server returned status {}", resp.status()));
        }

        let total_bytes = resp.content_length();
        let mut file = File::create(&temp_path)
            .map_err(|e| format!("IO_ERROR: Failed to create temp file: {}", e))?;

        let mut received_bytes: u64 = 0;
        let mut last_emit = Instant::now();
        let start_time = current_timestamp_ms();

        while let Some(chunk) = resp.chunk().await.map_err(|e| format!("STREAM_ERROR: Transfer failed: {}", e))? {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("CANCELLED: Download was cancelled by operator.".to_string());
            }

            file.write_all(&chunk)
                .map_err(|e| format!("IO_ERROR: Failed to write to disk: {}", e))?;

            received_bytes += chunk.len() as u64;

            // Throttle progress events to once every 200ms
            if last_emit.elapsed() >= Duration::from_millis(200) {
                let progress = total_bytes.map(|total| (received_bytes as f64 / total as f64).clamp(0.0, 1.0)).unwrap_or(0.0);
                let update = BrowserDownloadRecord {
                    id: download_id.clone(),
                    url: url.clone(),
                    filename: filename.clone(),
                    suggested_filename: filename.clone(),
                    destination: final_path.to_string_lossy().to_string(),
                    total_bytes,
                    received_bytes,
                    progress,
                    status: DownloadStatus::Downloading.as_str().to_string(),
                    started_at: start_time,
                    completed_at: None,
                    error: None,
                    tab_id: tab_id.clone(),
                };

                let _ = app.emit("browser-download-progress", json!(&update));
                last_emit = Instant::now();
            }
        }

        file.flush().map_err(|e| format!("IO_ERROR: Flush failed: {}", e))?;
        drop(file);

        // Step 20: Atomic rename from .edith-download to target filename
        std::fs::rename(&temp_path, &final_path)
            .map_err(|e| format!("IO_ERROR: Failed to finalize download file: {}", e))?;

        let complete_time = current_timestamp_ms();
        let completed_record = BrowserDownloadRecord {
            id: download_id,
            url,
            filename,
            suggested_filename: final_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            destination: final_path.to_string_lossy().to_string(),
            total_bytes: Some(received_bytes),
            received_bytes,
            progress: 1.0,
            status: DownloadStatus::Completed.as_str().to_string(),
            started_at: start_time,
            completed_at: Some(complete_time),
            error: None,
            tab_id,
        };

        if let Some(db_state) = app.try_state::<DbState>() {
            if let Ok(conn) = db_state.conn.lock() {
                let _ = db::upsert_browser_download(&conn, &completed_record);
            }
        }

        let _ = app.emit("browser-download-progress", json!(&completed_record));
        Ok(())
    }

    /// Cancels an active download
    pub fn cancel_download(&self, download_id: &str) -> bool {
        let active = self.active_downloads.lock().unwrap();
        if let Some(handle) = active.get(download_id) {
            handle.cancel_flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

// ============================================================================
// TAURI COMMANDS FOR DOWNLOAD MANAGER
// ============================================================================

#[tauri::command]
pub fn browser_download_start(
    app: AppHandle,
    url: String,
    tab_id: Option<String>,
    suggested_filename: Option<String>,
) -> Result<BrowserDownloadRecord, String> {
    GLOBAL_DOWNLOAD_MGR.start_download(app, url, tab_id, suggested_filename)
}

#[tauri::command]
pub fn browser_download_cancel(download_id: String) -> Result<bool, String> {
    Ok(GLOBAL_DOWNLOAD_MGR.cancel_download(&download_id))
}

#[tauri::command]
pub fn browser_download_list(
    limit: Option<u32>,
    db_state: State<'_, DbState>,
) -> Result<Vec<BrowserDownloadRecord>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::list_browser_downloads(&conn, limit)
        .map_err(|e| format!("DB_ERROR: Failed to list downloads: {}", e))
}

#[tauri::command]
pub fn browser_download_get(
    download_id: String,
    db_state: State<'_, DbState>,
) -> Result<Option<BrowserDownloadRecord>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::get_browser_download(&conn, &download_id)
        .map_err(|e| format!("DB_ERROR: Failed to get download record: {}", e))
}

#[tauri::command]
pub fn browser_download_delete_record(
    download_id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::delete_browser_download_record(&conn, &download_id)
        .map_err(|e| format!("DB_ERROR: Failed to delete download record: {}", e))
}

#[tauri::command]
pub fn browser_download_clear_records(
    db_state: State<'_, DbState>,
) -> Result<usize, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::clear_all_browser_download_records(&conn)
        .map_err(|e| format!("DB_ERROR: Failed to clear download records: {}", e))
}

#[tauri::command]
pub fn browser_download_show_in_folder(
    download_id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    if let Some(record) = db::get_browser_download(&conn, &download_id).map_err(|e| e.to_string())? {
        let path = PathBuf::from(&record.destination);
        if let Some(parent) = path.parent() {
            let _ = open::that(parent);
            return Ok(true);
        }
    }
    Err("DOWNLOAD_NOT_FOUND: Record or file not found.".to_string())
}

#[tauri::command]
pub fn browser_download_open_file(
    download_id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    if let Some(record) = db::get_browser_download(&conn, &download_id).map_err(|e| e.to_string())? {
        let path = PathBuf::from(&record.destination);
        
        // Security Check (Step 9 & 15): Block automatic execution of executable binaries/scripts
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let dangerous_exts = ["exe", "msi", "bat", "cmd", "ps1", "scr", "dll", "vbs", "sh", "reg"];
        if dangerous_exts.contains(&ext.as_str()) {
            // For security, reveal in folder instead of direct execution
            if let Some(parent) = path.parent() {
                let _ = open::that(parent);
                return Ok(true);
            }
        }

        if path.exists() {
            let _ = open::that(&path);
            return Ok(true);
        } else {
            return Err("FILE_NOT_FOUND: Downloaded file does not exist on disk.".to_string());
        }
    }
    Err("DOWNLOAD_NOT_FOUND: Record not found.".to_string())
}
