#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Manager;
use crate::ai::CredentialStore;
pub mod security;
pub mod ai;
pub mod events;
pub mod conversation;
pub mod task;
pub mod policy;
pub mod tools;
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
pub mod computer_control;
pub mod browser_storage;
pub mod browser_download;
pub mod browser_profile;
pub mod browser_privacy;
pub mod browser_recovery;
pub mod weather;
pub mod runtime;
pub mod voice;

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

#[tauri::command]
fn ai_list_providers(db_state: State<'_, DbState>) -> Result<Vec<ai::ProviderSummary>, String> {
    let mut registry = ai::ProviderRegistry::standard_builtins();
    if let Ok(conn) = db_state.conn.lock() {
        if let Ok(settings) = db::get_all_settings(&conn) {
            if let Some(custom_raw) = settings.get("customProviders") {
                registry.load_custom_providers(custom_raw);
            }
        }
    }
    Ok(registry.list_providers())
}

#[tauri::command]
fn ai_list_models(provider_id: String, db_state: State<'_, DbState>) -> Result<Vec<ai::ModelMetadata>, String> {
    let mut registry = ai::ProviderRegistry::standard_builtins();
    if let Ok(conn) = db_state.conn.lock() {
        if let Ok(settings) = db::get_all_settings(&conn) {
            if let Some(custom_raw) = settings.get("customProviders") {
                registry.load_custom_providers(custom_raw);
            }
        }
    }
    registry.list_models(&provider_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn ai_query_capabilities(
    provider_id: String,
    model_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<ai::CapabilitySet, String> {
    let mut registry = ai::ProviderRegistry::standard_builtins();
    if let Ok(conn) = db_state.conn.lock() {
        if let Ok(settings) = db::get_all_settings(&conn) {
            if let Some(custom_raw) = settings.get("customProviders") {
                registry.load_custom_providers(custom_raw);
            }
        }
    }
    registry
        .query_effective_capabilities(&provider_id, model_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn conversation_submit_turn(
    session_id: String,
    message: String,
    provider_id: Option<String>,
    model_id: Option<String>,
    temperature: Option<f64>,
    client_turn_id: Option<String>,
    core: State<'_, conversation::ConversationCore>,
) -> Result<conversation::TurnSubmissionResult, String> {
    let req = conversation::TurnSubmissionRequest {
        session_id,
        message,
        provider_id,
        model_id,
        temperature,
        client_turn_id,
    };
    core.submit_turn(req).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn conversation_execute_turn(
    turn_id: String,
    _stream_id: Option<String>,
    app_settings: Option<serde_json::Value>,
    core: State<'_, conversation::ConversationCore>,
) -> Result<String, String> {
    let tid = events::TurnId::from_string(turn_id);

    let creds = if let Some(ref settings) = app_settings {
        let cred_store = ai::SettingsCredentialStore::from_json_value(settings);
        let status = core.get_turn_status(&tid).await;
        let prov_id = status.map(|s| s.model_selection.provider_id).unwrap_or_else(|| "groq".to_string());
        cred_store.get_credential(&prov_id).ok().flatten()
    } else {
        None
    };

    core.execute_turn(&tid, creds).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn conversation_cancel_turn(
    turn_id: String,
    reason: Option<String>,
    core: State<'_, conversation::ConversationCore>,
) -> Result<(), String> {
    let tid = events::TurnId::from_string(turn_id);
    core.cancel_turn(&tid, reason).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn conversation_get_turn_status(
    turn_id: String,
    core: State<'_, conversation::ConversationCore>,
) -> Result<Option<conversation::TurnSnapshot>, String> {
    let tid = events::TurnId::from_string(turn_id);
    Ok(core.get_turn_status(&tid).await)
}

#[tauri::command]
async fn task_create(
    task_type: String,
    goal: String,
    session_id: Option<String>,
    turn_id: Option<String>,
    task_runtime: State<'_, task::TaskRuntime>,
) -> Result<String, String> {
    let t_type = match task_type.to_lowercase().as_str() {
        "background" => task::TaskType::Background,
        "browser_agent" => task::TaskType::BrowserAgent,
        "dev_agent" => task::TaskType::DevAgent,
        "maintenance" => task::TaskType::Maintenance,
        other => task::TaskType::Custom(other.to_string()),
    };
    let mut correlation = events::EventCorrelation::default();
    correlation.conversation_id = session_id;
    correlation.turn_id = turn_id;
    let id = task_runtime.create_task(t_type, goal, correlation, task::TaskOwner::User).await;
    Ok(id.to_string())
}

#[tauri::command]
async fn task_cancel(
    task_id: String,
    reason: Option<String>,
    task_runtime: State<'_, task::TaskRuntime>,
) -> Result<(), String> {
    let tid = events::TaskId::from_string(task_id);
    task_runtime.cancel_task(&tid, reason).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn task_get_status(
    task_id: String,
    task_runtime: State<'_, task::TaskRuntime>,
) -> Result<Option<task::TaskSnapshot>, String> {
    let tid = events::TaskId::from_string(task_id);
    Ok(task_runtime.get_task(&tid).await)
}

#[tauri::command]
async fn task_list_active(
    task_runtime: State<'_, task::TaskRuntime>,
) -> Result<Vec<task::TaskSnapshot>, String> {
    Ok(task_runtime.list_active_tasks().await)
}

#[tauri::command]
async fn policy_evaluate_action(
    request: policy::ActionRequest,
    context: policy::PolicyContext,
    policy_engine: State<'_, policy::PolicyEngine>,
) -> Result<policy::PolicyDecision, String> {
    Ok(policy_engine.evaluate(&request, &context).await)
}

#[tauri::command]
async fn policy_list_pending_approvals(
    policy_engine: State<'_, policy::PolicyEngine>,
) -> Result<Vec<policy::ApprovalRequest>, String> {
    Ok(policy_engine.list_pending_approvals().await)
}

#[tauri::command]
async fn policy_resolve_approval(
    approval_id: String,
    decision: policy::OperatorDecision,
    policy_engine: State<'_, policy::PolicyEngine>,
) -> Result<policy::ApprovalRequest, String> {
    policy_engine.resolve_approval(&approval_id, decision).await
}

#[tauri::command]
async fn policy_get_audit_log(
    limit: Option<usize>,
    policy_engine: State<'_, policy::PolicyEngine>,
) -> Result<Vec<policy::AuditRecord>, String> {
    Ok(policy_engine.get_audit_log(limit.unwrap_or(100)).await)
}

#[tauri::command]
fn tools_list_definitions(
    registry: State<'_, tools::ToolRegistry>,
) -> Result<Vec<tools::ToolDefinition>, String> {
    Ok(registry.list())
}

#[tauri::command]
fn tools_get_definition(
    name: String,
    registry: State<'_, tools::ToolRegistry>,
) -> Result<Option<tools::ToolDefinition>, String> {
    Ok(registry.get(&name).map(|d| (*d).clone()))
}

#[tauri::command]
async fn tools_execute(
    request: tools::ToolRequest,
    router: State<'_, tools::ToolRouter>,
) -> Result<tools::ToolExecutionResult, String> {
    Ok(router.execute(request).await)
}

#[tauri::command]
async fn tools_cancel_execution(
    execution_id: String,
    router: State<'_, tools::ToolRouter>,
) -> Result<bool, String> {
    Ok(router.cancel_execution(&execution_id).await)
}

#[tauri::command]
async fn runtime_get_status(
    session_id: Option<String>,
    runtime_state: State<'_, runtime::EdithRuntimeState>,
) -> Result<runtime::RuntimeStatusSummary, String> {
    Ok(runtime_state.get_runtime_status(session_id).await)
}

#[tauri::command]
async fn runtime_get_capabilities(
    domain: Option<String>,
    runtime_state: State<'_, runtime::EdithRuntimeState>,
) -> Result<runtime::CapabilitiesSummary, String> {
    Ok(runtime_state.get_capabilities(domain.as_deref()).await)
}


#[tauri::command]
async fn voice_session_start(
    conversation_id: String,
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<String, String> {
    let cid = events::ConversationId::from_string(conversation_id);
    let sid = controller
        .start_session(cid, voice::CaptureOwner::BrowserWebSpeech)
        .await
        .map_err(|e| e.to_string())?;
    Ok(sid.to_string())
}

#[tauri::command]
async fn voice_session_submit_transcript(
    session_id: String,
    transcript: String,
    confidence: Option<f32>,
    language: Option<String>,
    provider_id: Option<String>,
    model_id: Option<String>,
    app_settings: Option<serde_json::Value>,
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<String, String> {
    let sid = events::VoiceSessionId::from_string(session_id);
    let creds = if let Some(ref settings) = app_settings {
        let cred_store = ai::SettingsCredentialStore::from_json_value(settings);
        let prov = provider_id.clone().unwrap_or_else(|| "groq".to_string());
        cred_store.get_credential(&prov).ok().flatten()
    } else {
        None
    };
    controller
        .submit_web_speech_transcript(
            &sid,
            transcript,
            confidence,
            language,
            provider_id,
            model_id,
            creds,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_session_cancel(
    session_id: String,
    reason: Option<String>,
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<(), String> {
    let sid = events::VoiceSessionId::from_string(session_id);
    controller
        .cancel_session(&sid, reason)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_stop_playback(
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<(), String> {
    controller.stop_playback().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_get_status(
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<voice::VoiceStatusSummary, String> {
    Ok(controller.status_summary().await)
}

#[tauri::command]
async fn realtime_voice_start(
    conversation_id: String,
    provider_id: Option<String>,
    engine: State<'_, std::sync::Arc<voice::RealtimeVoiceEngine>>,
) -> Result<String, String> {
    let cid = events::ConversationId::from_string(conversation_id);
    let prov = provider_id.unwrap_or_else(|| "gemini-live".to_string());
    let transport = std::sync::Arc::new(voice::MockAudioFrameTransport::new(32));
    let adapter = std::sync::Arc::new(voice::MockRealtimeSessionAdapter::new(transport));
    let sid = engine
        .start_session(cid, prov, adapter)
        .await
        .map_err(|e| e.to_string())?;
    Ok(sid.to_string())
}

#[tauri::command]
async fn realtime_voice_stop(
    engine: State<'_, std::sync::Arc<voice::RealtimeVoiceEngine>>,
) -> Result<(), String> {
    engine.stop_session().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn realtime_voice_get_status(
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<voice::VoiceStatusSummary, String> {
    Ok(controller.status_summary().await)
}

#[tauri::command]
async fn voice_list_devices(
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<voice::AudioDevicesSummary, String> {
    controller.list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_set_input_device(
    device_id: Option<String>,
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<(), String> {
    controller.set_input_device(device_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_set_output_device(
    device_id: Option<String>,
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<(), String> {
    controller.set_output_device(device_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn voice_get_device_status(
    controller: State<'_, std::sync::Arc<voice::VoiceController>>,
) -> Result<voice::AudioDevicesSummary, String> {
    controller.list_devices().map_err(|e| e.to_string())
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
            let conn2 = db::init_db_at(&db_path).map_err(|e| e.to_string())?;
            app.manage(DbState {
                conn: std::sync::Mutex::new(conn),
            });
            app.manage(agent::AgentState {
                path: std::sync::Mutex::new(String::new()),
            });
            app.manage(browser::BrowserState::default());
            app.manage(browser_agent::BrowserAgentManager::default());

            let emitter = events::EventEmitter::new(app.handle().clone());
            let task_runtime = task::TaskRuntime::new(emitter.clone());
            let registry = ai::ProviderRegistry::standard_builtins();
            let conn2_arc = std::sync::Arc::new(std::sync::Mutex::new(conn2));
            let conversation_core = conversation::ConversationCore::new(
                registry,
                emitter.clone(),
                Some(conn2_arc.clone()),
                None,
            );
            let policy_engine = policy::PolicyEngine::new(Some(emitter.clone()));
            let task_runtime_arc = std::sync::Arc::new(task_runtime.clone());
            let conversation_core_arc = std::sync::Arc::new(conversation_core.clone());
            let policy_engine_arc = std::sync::Arc::new(policy_engine.clone());

            let tool_registry = tools::ToolRegistry::new();
            for def in tools::get_browser_definitions() {
                let _ = tool_registry.register(def);
            }
            for def in tools::get_computer_definitions() {
                let _ = tool_registry.register(def);
            }
            for def in tools::get_edith_definitions() {
                let _ = tool_registry.register(def);
            }
            let browser_executor = std::sync::Arc::new(tools::BrowserDomainExecutor::new(Some(app.handle().clone())));
            let computer_executor = std::sync::Arc::new(tools::ComputerDomainExecutor::new(Some(app.handle().clone())));
            let domain_executors = tools::DomainExecutorRegistry::new();
            domain_executors.register(browser_executor);
            domain_executors.register(computer_executor);
            let domain_executors_arc = std::sync::Arc::new(domain_executors);
            let tool_registry_arc = std::sync::Arc::new(tool_registry.clone());

            let tool_router = tools::ToolRouter::with_defaults(
                tool_registry_arc.clone(),
                domain_executors_arc.clone(),
                policy_engine_arc.clone(),
                Some(emitter.clone()),
            );
            let tool_router_arc = std::sync::Arc::new(tool_router.clone());

            let voice_stt = std::sync::Arc::new(voice::CloudSTTAdapter::new("cloud-stt", ""));
            let voice_tts = std::sync::Arc::new(voice::EdgeTtsAdapter::new());
            let voice_output = std::sync::Arc::new(voice::RodioAudioOutputDriver::new());
            let voice_capture = std::sync::Arc::new(voice::BrowserCaptureBridge::new());
            let realtime_engine = std::sync::Arc::new(voice::RealtimeVoiceEngine::new(
                conversation_core_arc.clone(),
                tool_router_arc.clone(),
                voice_output.clone(),
                voice_capture.clone(),
                Some(std::sync::Arc::new(emitter.clone())),
            ));
            let voice_controller = std::sync::Arc::new(
                voice::VoiceController::new(
                    conversation_core_arc.clone(),
                    voice_stt,
                    voice_tts,
                    voice_output,
                    voice_capture,
                    Some(std::sync::Arc::new(emitter.clone())),
                )
                .with_realtime_engine(realtime_engine.clone()),
            );

            let runtime_state = runtime::EdithRuntimeState::new(
                conversation_core_arc,
                task_runtime_arc,
                tool_registry_arc,
                tool_router_arc,
                conversation_core.registry(),
                policy_engine_arc,
                Some(app.handle().clone()),
                Some(conn2_arc),
            ).with_voice_controller(voice_controller.clone());
            let edith_executor = std::sync::Arc::new(tools::EdithDomainExecutor::new(std::sync::Arc::new(runtime_state.clone())));
            domain_executors_arc.register(edith_executor);

            app.manage(task_runtime);
            app.manage(conversation_core);
            app.manage(policy_engine);
            app.manage(tool_registry);
            app.manage(tool_router);
            app.manage(runtime_state);
            app.manage(realtime_engine);
            app.manage(voice_controller);

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
            browser::browser_duplicate_tab,
            browser::browser_toggle_pin_tab,
            browser::browser_close_other_tabs,
            browser::browser_close_tabs_to_right,
            browser::browser_save_session,
            browser::browser_restore_session,
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
            browser_download::browser_download_start,
            browser_download::browser_download_cancel,
            browser_download::browser_download_list,
            browser_download::browser_download_get,
            browser_download::browser_download_delete_record,
            browser_download::browser_download_clear_records,
            browser_download::browser_download_show_in_folder,
            browser_download::browser_download_open_file,
            browser_profile::browser_profiles_list,
            browser_profile::browser_profile_get,
            browser_profile::browser_profile_create,
            browser_profile::browser_profile_switch,
            browser_profile::browser_profile_rename,
            browser_profile::browser_profile_delete,
            browser_profile::browser_profile_create_temporary,
            browser_profile::browser_profile_cleanup_temporary,
            browser_privacy::browser_privacy_get_status,
            browser_privacy::browser_privacy_toggle_protection,
            browser_privacy::browser_privacy_allowlist_domain,
            browser_privacy::browser_privacy_remove_allowlist,
            browser_privacy::browser_privacy_add_block_rule,
            browser_privacy::browser_privacy_remove_block_rule,
            browser_privacy::browser_privacy_list_rules,
            browser_privacy::browser_privacy_get_tab_stats,
            browser_privacy::browser_privacy_reset_stats,
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
            browser::browser_find_in_page,
            browser::browser_clear_find,
            browser::browser_zoom_set,
            browser::browser_zoom_in,
            browser::browser_zoom_out,
            browser::browser_zoom_reset,
            browser::browser_print_tab,
            browser::browser_open_link_tab,
            browser::browser_save_page_html,
            browser::browser_reader_extract,
            browser::browser_reader_mode_enter,
            browser::browser_reader_mode_exit,
            browser::browser_reader_mode_get,
            browser::browser_tab_group_create,
            browser::browser_tab_group_rename,
            browser::browser_tab_group_delete,
            browser::browser_tab_group_list,
            browser::browser_tab_group_set_collapsed,
            browser::browser_tab_group_move_tab,
            browser::browser_tab_group_remove_tab,
            browser::browser_tab_group_reorder,
            browser::browser_tab_group_close_tabs,
            browser_recovery::browser_run_startup_recovery,
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
            tts::tts_set_voice,
            ai_list_providers,
            ai_list_models,
            ai_query_capabilities,
            conversation_submit_turn,
            conversation_execute_turn,
            conversation_cancel_turn,
            conversation_get_turn_status,
            task_create,
            task_cancel,
            task_get_status,
            task_list_active,
            policy_evaluate_action,
            policy_list_pending_approvals,
            policy_resolve_approval,
            policy_get_audit_log,
            tools_list_definitions,
            tools_get_definition,
            tools_execute,
            tools_cancel_execution,
            runtime_get_status,
            runtime_get_capabilities,
            voice_session_start,
            voice_session_submit_transcript,
            voice_session_cancel,
            voice_stop_playback,
            voice_get_status,
            realtime_voice_start,
            realtime_voice_stop,
            realtime_voice_get_status,
            voice_list_devices,
            voice_set_input_device,
            voice_set_output_device,
            voice_get_device_status
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {});
}




