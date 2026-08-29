#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Manager;
pub mod security;
mod agent;
mod chat;
pub mod db;
mod llm;
mod plugins;
mod providers;
mod tts;
mod memory;
pub mod embedding;
pub mod screen;
pub mod windows;
pub mod browser;
pub mod browser_tools;
pub mod browser_agent;
pub mod browser_risk;
pub mod browser_orchestrator;
pub mod browser_control;
pub mod browser_storage;
pub mod weather;

use db::DbState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
fn agent_resolve_proposal(
    proposal_id: String,
    session_id: Option<String>,
    action: String,
) -> Result<security::CommandPolicyResult, String> {
    let sess = session_id.unwrap_or_default();
    security::ProposalEngine::resolve_proposal(&proposal_id, &sess, &action)
}


#[tauri::command]
fn get_base_dir() -> Result<String, String> {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if dir.ends_with("src-tauri") {
        dir.pop();
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
fn get_all_settings(state: State<'_, DbState>) -> Result<HashMap<String, String>, String> {
    let conn = state.conn.lock().unwrap();
    db::get_all_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_setting(key: String, value: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::save_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn sync_settings(
    settings: HashMap<String, String>,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    for (k, v) in settings {
        db::save_setting(&conn, &k, &v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_all_sessions(state: State<'_, DbState>) -> Result<Vec<db::Session>, String> {
    let conn = state.conn.lock().unwrap();
    db::get_all_sessions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_session(
    session_id: String,
    title: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::create_session(&conn, &session_id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_session(session_id: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::delete_session(&conn, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_session(
    session_id: String,
    new_title: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::rename_session(&conn, &session_id, &new_title).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_session_messages(
    session_id: String,
    state: State<'_, DbState>,
) -> Result<Vec<db::Message>, String> {
    let conn = state.conn.lock().unwrap();
    db::get_session_messages(&conn, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_session_message(
    session_id: String,
    role: String,
    text: String,
    time: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::save_session_message(&conn, &session_id, &role, &text, &time).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_personal_notes(state: State<'_, DbState>) -> Result<Vec<db::Note>, String> {
    let conn = state.conn.lock().unwrap();
    db::get_personal_notes(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_personal_note(content: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::save_personal_note(&conn, &content).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_personal_note(note_id: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::delete_personal_note(&conn, &note_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_custom_apps(state: State<'_, DbState>) -> Result<Vec<db::CustomApp>, String> {
    let conn = state.conn.lock().unwrap();
    db::get_custom_apps(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_custom_app(
    name: String,
    path: String,
    keywords: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::add_custom_app(&conn, &name, &path, &keywords).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_custom_app(app_id: i32, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::delete_custom_app(&conn, app_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn sync_apps_registry() -> Result<(), String> {
    // No-op: custom apps are already stored in SQLite via add_custom_app/delete_custom_app
    // This exists for frontend compatibility
    Ok(())
}

#[tauri::command]
fn launch_app(path: String, state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    security::AppLauncherPolicy::validate_and_launch(&path, Some(&conn))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;
            std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
            let db_path = app_data_dir.join("edith_memory.db");
            let conn = db::init_db_at(&db_path).map_err(|e| e.to_string())?;
            app.manage(DbState {
                conn: std::sync::Mutex::new(conn),
            });
            app.manage(agent::AgentState {
                path: std::sync::Mutex::new(String::new()),
            });
            app.manage(browser::BrowserState::default());
            app.manage(browser_agent::BrowserAgentManager::default());
            Ok(())
        })
        .on_window_event(|_window, event| match event {
              tauri::WindowEvent::CloseRequested { .. } => {
                  #[cfg(target_os = "windows")]
                  let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "llama-server.exe"]).creation_flags(0x08000000).output();
              }
              _ => {}
          })
          .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            browser::browser_create_tab,
            browser::browser_switch_tab,
            browser::browser_close_tab,
            browser::browser_reopen_last_closed_tab,
            browser::browser_navigate_tab,
            browser::browser_go_back_tab,
            browser::browser_go_forward_tab,
            browser::browser_reload_tab,
            browser::browser_get_multi_state,
            browser::browser_set_bounds_all,
            browser::browser_hide_all,
            browser::browser_show_active,
            browser::browser_observe_tab,
            browser::browser_screenshot_tab,
            browser::browser_click_element,
            browser::browser_type_element,
            browser::browser_scroll,
            browser::browser_press_key,
            browser::browser_focus_element,
            browser::browser_wait,
            browser::browser_select_option,
            browser_tools::browser_get_tool_definitions_cmd,
            browser_tools::browser_execute_tool_cmd,
            browser_risk::browser_assess_action_risk,
            browser_risk::browser_get_risk_audit_log,
            browser_risk::browser_resolve_action_approval,
            browser_agent::browser_agent_run_task,
            browser_agent::browser_agent_cancel_task,
            browser_agent::browser_agent_get_current_task,
            browser_orchestrator::browser_orchestrator_run_task,
            browser_orchestrator::browser_orchestrator_cancel_task,
            browser_orchestrator::browser_orchestrator_get_current_task,
            browser_control::browser_request_ai_control,
            browser_control::browser_takeover_tab,
            browser_control::browser_release_ai_control,
            browser_control::browser_pause_ai_control,
            browser_control::browser_resume_ai_control,
            browser_control::browser_get_tab_control_info,
            browser_control::browser_get_all_tab_controls,
            browser_storage::browser_history_add,
            browser_storage::browser_history_get_recent,
            browser_storage::browser_history_search,
            browser_storage::browser_history_delete,
            browser_storage::browser_history_clear,
            browser_storage::browser_bookmark_add,
            browser_storage::browser_bookmark_update,
            browser_storage::browser_bookmark_delete,
            browser_storage::browser_bookmarks_list,
            browser_storage::browser_bookmarks_search,
            browser_storage::browser_bookmark_is_bookmarked,
            browser_storage::browser_bookmark_create_folder,
            browser_storage::browser_bookmark_delete_folder,
            browser::browser_get_tab_url,
            browser::browser_get_tab_title,
            browser::browser_get_tab_visible_text,
            browser::browser_create,
            browser::browser_destroy,
            browser::browser_show,
            browser::browser_hide,
            browser::browser_navigate,
            browser::browser_go_back,
            browser::browser_go_forward,
            browser::browser_reload,
            browser::browser_set_bounds,
            browser::browser_get_url,
            browser::browser_get_title,
            browser::browser_get_visible_text,
            agent::agent_status,
            agent::agent_chat,
            agent::agent_set_path,
            agent::agent_reset,
            agent_resolve_proposal,
            get_base_dir,
            get_all_settings,
            save_setting,
            sync_settings,
            get_all_sessions,
            create_session,
            rename_session,
            delete_session,
            get_session_messages,
            save_session_message,
            get_personal_notes,
            save_personal_note,
            delete_personal_note,
            get_custom_apps,
            add_custom_app,
            delete_custom_app,
            sync_apps_registry,
            launch_app,
            plugins::plugin_system_terminal,
            plugins::plugin_system_control,
            plugins::plugin_app_launcher,
            plugins::plugin_web_search,
            plugins::plugin_media_player,
            plugins::plugin_whatsapp,
            plugins::plugin_gmail,
            plugins::take_screenshot,
            weather::get_weather,
            screen::take_screenshot_cmd,
            windows::arrange_windows_cmd,
            plugins::get_plugins,
            plugins::toggle_plugin,
            memory::save_to_memory_cmd,
            memory::search_memory_cmd,
            memory::delete_memory_cmd,
            memory::get_memories_cmd,
            plugins::get_builtin_apps,
            llm::api_chat_cloud,
            llm::load_local_llm,
            llm::local_chat,
            llm::stop_local_llm,
            chat::chat_command,
            providers::get_providers,
            providers::fetch_custom_models,
            tts::tts_speak,
            tts::local_tts_speak,
            tts::get_kokoro_models,
            tts::tts_stop,
            tts::tts_set_voice
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {});
}




