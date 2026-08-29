use tauri::{AppHandle, Emitter, State};
use serde_json::json;

use crate::db::{
    self, DbState, BrowserHistoryEntry, BrowserBookmark, BrowserBookmarkFolder
};

// ============================================================================
// PHASE 5.6A BROWSER HISTORY TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn browser_history_add(
    app: AppHandle,
    url: String,
    title: String,
    tab_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<BrowserHistoryEntry, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let entry = db::add_browser_history_entry(&conn, &url, &title, tab_id.as_deref())
        .map_err(|e| format!("DB_ERROR: Failed to add history entry: {}", e))?;

    let _ = app.emit("browser-history-updated", json!({
        "action": "added",
        "entry": &entry
    }));

    Ok(entry)
}

#[tauri::command]
pub fn browser_history_get_recent(
    limit: Option<u32>,
    db_state: State<'_, DbState>,
) -> Result<Vec<BrowserHistoryEntry>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::get_recent_browser_history(&conn, limit)
        .map_err(|e| format!("DB_ERROR: Failed to retrieve history: {}", e))
}

#[tauri::command]
pub fn browser_history_search(
    query: String,
    limit: Option<u32>,
    db_state: State<'_, DbState>,
) -> Result<Vec<BrowserHistoryEntry>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::search_browser_history(&conn, &query, limit)
        .map_err(|e| format!("DB_ERROR: Failed to search history: {}", e))
}

#[tauri::command]
pub fn browser_history_delete(
    app: AppHandle,
    id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let deleted = db::delete_browser_history_entry(&conn, &id)
        .map_err(|e| format!("DB_ERROR: Failed to delete history item: {}", e))?;

    if deleted {
        let _ = app.emit("browser-history-updated", json!({
            "action": "deleted",
            "id": id
        }));
    }

    Ok(deleted)
}

#[tauri::command]
pub fn browser_history_clear(
    app: AppHandle,
    db_state: State<'_, DbState>,
) -> Result<usize, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let count = db::clear_browser_history(&conn)
        .map_err(|e| format!("DB_ERROR: Failed to clear history: {}", e))?;

    let _ = app.emit("browser-history-updated", json!({
        "action": "cleared",
        "count": count
    }));

    Ok(count)
}

// ============================================================================
// PHASE 5.6A BROWSER BOOKMARKS TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn browser_bookmark_add(
    app: AppHandle,
    title: String,
    url: String,
    folder_id: Option<String>,
    favicon: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<BrowserBookmark, String> {
    // Step H: URL Scheme Policy Validation
    let url_trimmed = url.trim();
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
        return Err("INVALID_URL: Only standard http:// and https:// URLs can be bookmarked.".to_string());
    }

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let bookmark = db::add_browser_bookmark(&conn, &title, url_trimmed, folder_id.as_deref(), favicon.as_deref())
        .map_err(|e| format!("DB_ERROR: Failed to add bookmark: {}", e))?;

    let _ = app.emit("browser-bookmarks-updated", json!({
        "action": "added",
        "bookmark": &bookmark
    }));

    Ok(bookmark)
}

#[tauri::command]
pub fn browser_bookmark_update(
    app: AppHandle,
    id: String,
    title: String,
    url: String,
    folder_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let url_trimmed = url.trim();
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
        return Err("INVALID_URL: Only standard http:// and https:// URLs can be bookmarked.".to_string());
    }

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let updated = db::update_browser_bookmark(&conn, &id, &title, url_trimmed, folder_id.as_deref())
        .map_err(|e| format!("DB_ERROR: Failed to update bookmark: {}", e))?;

    if updated {
        let _ = app.emit("browser-bookmarks-updated", json!({
            "action": "updated",
            "id": id
        }));
    }

    Ok(updated)
}

#[tauri::command]
pub fn browser_bookmark_delete(
    app: AppHandle,
    id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let deleted = db::delete_browser_bookmark(&conn, &id)
        .map_err(|e| format!("DB_ERROR: Failed to delete bookmark: {}", e))?;

    if deleted {
        let _ = app.emit("browser-bookmarks-updated", json!({
            "action": "deleted",
            "id": id
        }));
    }

    Ok(deleted)
}

#[tauri::command]
pub fn browser_bookmarks_list(
    db_state: State<'_, DbState>,
) -> Result<Vec<BrowserBookmark>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::get_all_browser_bookmarks(&conn)
        .map_err(|e| format!("DB_ERROR: Failed to list bookmarks: {}", e))
}

#[tauri::command]
pub fn browser_bookmarks_search(
    query: String,
    db_state: State<'_, DbState>,
) -> Result<Vec<BrowserBookmark>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::search_browser_bookmarks(&conn, &query)
        .map_err(|e| format!("DB_ERROR: Failed to search bookmarks: {}", e))
}

#[tauri::command]
pub fn browser_bookmark_is_bookmarked(
    url: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::is_url_bookmarked(&conn, &url)
        .map_err(|e| format!("DB_ERROR: Failed to check bookmark: {}", e))
}

#[tauri::command]
pub fn browser_bookmark_create_folder(
    app: AppHandle,
    name: String,
    parent_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<BrowserBookmarkFolder, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let folder = db::create_bookmark_folder(&conn, &name, parent_id.as_deref())
        .map_err(|e| format!("DB_ERROR: Failed to create folder: {}", e))?;

    let _ = app.emit("browser-bookmarks-updated", json!({
        "action": "folder_created",
        "folder": &folder
    }));

    Ok(folder)
}

#[tauri::command]
pub fn browser_bookmark_delete_folder(
    app: AppHandle,
    folder_id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let deleted = db::delete_bookmark_folder(&conn, &folder_id)
        .map_err(|e| format!("DB_ERROR: Failed to delete folder: {}", e))?;

    if deleted {
        let _ = app.emit("browser-bookmarks-updated", json!({
            "action": "folder_deleted",
            "folder_id": folder_id
        }));
    }

    Ok(deleted)
}
