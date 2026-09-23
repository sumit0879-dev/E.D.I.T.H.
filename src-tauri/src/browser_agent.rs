use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::browser::{browser_get_multi_state, BrowserState};
use crate::browser_tools::execute_browser_tool;
use crate::db::DbState;
use crate::events::{EventCorrelation, TaskId};
use crate::llm::ChatMessage;
use crate::task::runtime::TaskRuntime;
use crate::task::types::{TaskOwner, TaskType};
use crate::tools::types::{ToolRequest, ToolStatus};
use crate::tools::ToolRouter;

/// Translates legacy browser tool names to canonical Universal Tool Runtime names ("browser.*")
pub fn to_universal_tool_name(name: &str) -> String {
    match name {
        "browser_open_url" => "browser.navigate".to_string(),
        "browser_observe" => "browser.observe".to_string(),
        "browser_screenshot" => "browser.screenshot".to_string(),
        "browser_get_tabs" => "browser.get_tabs".to_string(),
        "browser_get_active_tab" => "browser.get_active_tab".to_string(),
        "browser_switch_tab" => "browser.switch_tab".to_string(),
        "browser_close_tab" => "browser.close_tab".to_string(),
        "browser_back" => "browser.back".to_string(),
        "browser_forward" => "browser.forward".to_string(),
        "browser_reload" => "browser.reload".to_string(),
        "browser_click" => "browser.click".to_string(),
        "browser_type" => "browser.type".to_string(),
        "browser_scroll" => "browser.scroll".to_string(),
        "browser_press_key" => "browser.press_key".to_string(),
        "browser_focus" => "browser.focus".to_string(),
        "browser_wait" => "browser.wait".to_string(),
        "browser_select_option" => "browser.select_option".to_string(),
        "browser_new_tab" => "browser.new_tab".to_string(),
        "browser_history_recent" => "browser.history_recent".to_string(),
        "browser_history_search" => "browser.history_search".to_string(),
        "browser_bookmarks_list" => "browser.bookmarks_list".to_string(),
        "browser_bookmarks_search" => "browser.bookmarks_search".to_string(),
        "browser_downloads_recent" => "browser.downloads_recent".to_string(),
        "browser_download_get" => "browser.download_get".to_string(),
        other => {
            if other.starts_with("browser.") {
                other.to_string()
            } else if let Some(stripped) = other.strip_prefix("browser_") {
                format!("browser.{}", stripped)
            } else {
                format!("browser.{}", other)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserTaskStatus {
    Planning,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTaskState {
    pub task_id: String,
    pub goal: String,
    pub status: BrowserTaskStatus,
    pub current_tab_id: String,
    pub step_count: u32,
    pub max_steps: u32,
    pub started_at: u64,
    pub timeout_ms: u64,
    pub last_observation: Option<String>,
    pub last_action: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTaskResult {
    pub task_id: String,
    pub status: BrowserTaskStatus,
    pub goal: String,
    pub summary: String,
    pub steps_taken: u32,
    pub duration_ms: u64,
    pub final_tab_id: String,
    pub error: Option<String>,
}

pub struct BrowserAgentManager {
    pub active_task: Mutex<Option<BrowserTaskState>>,
    pub cancellation_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Default for BrowserAgentManager {
    fn default() -> Self {
        Self {
            active_task: Mutex::new(None),
            cancellation_flags: Mutex::new(HashMap::new()),
        }
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

use crate::ai::CredentialStore;

// -----------------------------------------------------------------------------
// Step 2: Robust, Bracket-Aware Tool Parser
// -----------------------------------------------------------------------------

fn extract_single_browser_tool_call(
    text: &str,
) -> Result<Option<(String, serde_json::Value)>, String> {
    let prefix = "[BROWSER_TOOL:";
    let start_idx = match text.find(prefix) {
        Some(idx) => idx + prefix.len(),
        None => return Ok(None),
    };

    let remaining = &text[start_idx..];
    let trimmed = remaining.trim_start();
    if !trimmed.starts_with('{') {
        return Err(
            "TOOL_SYNTAX_ERROR: Tool arguments must start with a JSON object '{'.".to_string(),
        );
    }

    // Bracket-aware JSON slice extractor
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut is_escaped = false;
    let mut end_idx = None;

    for (i, c) in trimmed.char_indices() {
        if is_escaped {
            is_escaped = false;
            continue;
        }
        if c == '\\' && in_string {
            is_escaped = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if c == '{' {
                brace_depth += 1;
            } else if c == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    end_idx = Some(i + 1);
                    break;
                }
            }
        }
    }

    let json_end = match end_idx {
        Some(idx) => idx,
        None => {
            return Err("TOOL_SYNTAX_ERROR: Unclosed JSON object in BROWSER_TOOL call.".to_string())
        }
    };

    let json_str = &trimmed[..json_end];
    let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        format!(
            "TOOL_SYNTAX_ERROR: Malformed JSON in BROWSER_TOOL call: {}",
            e
        )
    })?;

    let tool_name = parsed
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "TOOL_VALIDATION_ERROR: Missing 'name' field in BROWSER_TOOL call.".to_string()
        })?
        .to_string();

    let empty_args = json!({});
    let args = parsed.get("args").unwrap_or(&empty_args).clone();

    if !args.is_object() {
        return Err("TOOL_VALIDATION_ERROR: Tool 'args' field must be a JSON object.".to_string());
    }

    Ok(Some((tool_name, args)))
}

// -----------------------------------------------------------------------------
// Step 3: Central Pre-Execution Tool Validation & Bounds Check
// -----------------------------------------------------------------------------

fn validate_browser_tool_call(
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<(), (String, String)> {
    let valid_tools = crate::browser_tools::get_browser_tool_definitions();
    let def = valid_tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| {
            (
                "UNKNOWN_TOOL".to_string(),
                format!(
                    "Tool '{}' is not recognized in the Browser Tool Layer.",
                    tool_name
                ),
            )
        })?;

    // Check risk classification
    if def.risk_level == "BLOCKED_FOR_AI" {
        return Err((
            "SECURITY_BLOCK".to_string(),
            format!(
                "Tool '{}' is blocked from autonomous execution by security policy.",
                tool_name
            ),
        ));
    }

    // Check required fields
    if let Some(req_arr) = def.parameters.get("required").and_then(|v| v.as_array()) {
        for r in req_arr {
            if let Some(field) = r.as_str() {
                if args.get(field).is_none() {
                    return Err((
                        "MISSING_ARGUMENT".to_string(),
                        format!(
                            "Missing required argument '{}' for tool '{}'.",
                            field, tool_name
                        ),
                    ));
                }
            }
        }
    }

    // Specific bounded parameter validation
    if tool_name == "browser_type" {
        if let Some(txt) = args.get("text").and_then(|v| v.as_str()) {
            if txt.len() > 5000 {
                return Err((
                    "INPUT_TOO_LARGE".to_string(),
                    "Type text exceeds bounded maximum length (5000 characters).".to_string(),
                ));
            }
        }
    }

    if tool_name == "browser_scroll" {
        if let Some(dir) = args.get("direction").and_then(|v| v.as_str()) {
            let allowed_dirs = ["up", "down", "left", "right", "top", "bottom"];
            if !allowed_dirs.contains(&dir) {
                return Err(("INVALID_ENUM".to_string(), format!("Invalid scroll direction '{}'. Allowed: up, down, left, right, top, bottom.", dir)));
            }
        }
    }

    if tool_name == "browser_press_key" {
        if let Some(k) = args.get("key").and_then(|v| v.as_str()) {
            let allowed_keys = [
                "Enter",
                "Escape",
                "Tab",
                "Backspace",
                "Delete",
                "ArrowUp",
                "ArrowDown",
                "ArrowLeft",
                "ArrowRight",
                "Home",
                "End",
                "PageUp",
                "PageDown",
                "Space",
            ];
            if !allowed_keys.contains(&k) {
                return Err((
                    "INVALID_ENUM".to_string(),
                    format!("Invalid key '{}'. Must be one of: {:?}", k, allowed_keys),
                ));
            }
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Step 5 & 6: False Success Protection & Evidence Engine
// -----------------------------------------------------------------------------

#[derive(Default)]
struct TaskEvidence {
    observed_urls: Vec<String>,
    observed_titles: Vec<String>,
    successful_actions_count: u32,
    executed_tools: Vec<String>,
}

fn verify_completion_evidence(goal: &str, evidence: &TaskEvidence) -> Result<(), String> {
    if evidence.successful_actions_count == 0 && evidence.observed_urls.is_empty() {
        return Err("COMPLETION_CLAIM_REJECTED: No browser actions or observations were performed. You must observe the browser or execute required actions before claiming task completion.".to_string());
    }

    let goal_lower = goal.to_lowercase();
    for domain in [
        "example.com",
        "wikipedia.org",
        "google.com",
        "github.com",
        "iana.org",
    ] {
        if goal_lower.contains(domain) {
            let visited = evidence
                .observed_urls
                .iter()
                .any(|u| u.to_lowercase().contains(domain));
            if !visited {
                return Err(format!("COMPLETION_CLAIM_REJECTED: The goal requested interaction with '{}', but no observation or navigation to that domain was recorded in task evidence.", domain));
            }
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Main Autonomous Loop Engine with Hardened Reliability
// -----------------------------------------------------------------------------

pub async fn run_autonomous_browser_loop(
    app: AppHandle,
    goal: String,
    max_steps_opt: Option<u32>,
    timeout_ms_opt: Option<u64>,
    browser_state: State<'_, BrowserState>,
    db_state: State<'_, DbState>,
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<BrowserTaskResult, String> {
    // Step 12: Single Active Task Enforcement
    {
        let active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
        if let Some(task) = active.as_ref() {
            if task.status == BrowserTaskStatus::Planning
                || task.status == BrowserTaskStatus::Running
            {
                return Err("TASK_ALREADY_RUNNING: An autonomous browser task is already running. Please cancel or wait for it to complete.".to_string());
            }
        }
    }

    let task_id = format!("task_{}", current_timestamp_ms());
    let max_steps = max_steps_opt.unwrap_or(20).clamp(1, 20);
    let timeout_ms = timeout_ms_opt.unwrap_or(120_000).clamp(5_000, 120_000);
    let start_instant = Instant::now();

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut flags = agent_mgr
            .cancellation_flags
            .lock()
            .map_err(|e| e.to_string())?;
        flags.insert(task_id.clone(), cancel_flag.clone());
    }

    let tool_router = app.try_state::<ToolRouter>();
    let task_runtime = app.try_state::<TaskRuntime>();
    let task_id_obj = TaskId::from_string(&task_id);
    let correlation = EventCorrelation {
        task_id: Some(task_id.clone()),
        ..Default::default()
    };

    if let Some(ref rt) = task_runtime {
        let _ = rt
            .create_task_with_id(
                task_id_obj.clone(),
                TaskType::BrowserAgent,
                goal.clone(),
                correlation.clone(),
                TaskOwner::System,
            )
            .await;
        let _ = rt.start_task(&task_id_obj).await;
    }

    // Determine initial active tab
    let initial_tab_id = match browser_get_multi_state(app.clone(), browser_state.clone()).await {
        Ok(multi) => multi.active_tab_id.unwrap_or_else(|| "tab_a".to_string()),
        Err(_) => "tab_a".to_string(),
    };

    let mut current_task = BrowserTaskState {
        task_id: task_id.clone(),
        goal: goal.clone(),
        status: BrowserTaskStatus::Planning,
        current_tab_id: initial_tab_id.clone(),
        step_count: 0,
        max_steps,
        started_at: current_timestamp_ms(),
        timeout_ms,
        last_observation: None,
        last_action: None,
        last_error: None,
    };

    {
        let mut active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
        *active = Some(current_task.clone());
    }

    let _ = app.emit(
        "browser-agent-status",
        json!({
            "task_id": task_id,
            "status": "Planning",
            "step": 0,
            "max_steps": max_steps,
            "message": format!("Planning autonomous browser task: {}", goal)
        }),
    );

    let settings = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        crate::db::get_all_settings(&conn).unwrap_or_default()
    };

    let ai_mode = settings
        .get("aiMode")
        .cloned()
        .unwrap_or_else(|| "api".to_string());
    let provider_id = if ai_mode == "local" {
        "local".to_string()
    } else {
        settings
            .get("selectedProvider")
            .cloned()
            .unwrap_or_else(|| "groq".to_string())
    };
    let model = settings
        .get("selectedModel")
        .cloned()
        .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());

    let mut registry = crate::ai::ProviderRegistry::standard_builtins();
    if let Some(custom_providers_raw) = settings.get("customProviders") {
        registry.load_custom_providers(custom_providers_raw);
    }
    let cred_store = crate::ai::SettingsCredentialStore::new(settings.clone());
    let creds = cred_store.get_credential(&provider_id).ok().flatten();

    if ai_mode != "local" && creds.is_none() && provider_id != "local" {
        let err_msg = "API Key is missing for selected provider.".to_string();
        current_task.status = BrowserTaskStatus::Failed;
        current_task.last_error = Some(err_msg.clone());
        let _ = app.emit(
            "browser-agent-status",
            json!({
                "task_id": task_id,
                "status": "Failed",
                "error": err_msg
            }),
        );

        // Cleanup cancellation flag
        if let Ok(mut flags) = agent_mgr.cancellation_flags.lock() {
            flags.remove(&task_id);
        }

        return Ok(BrowserTaskResult {
            task_id,
            status: BrowserTaskStatus::Failed,
            goal,
            summary: "Failed before start: missing API key.".to_string(),
            steps_taken: 0,
            duration_ms: start_instant.elapsed().as_millis() as u64,
            final_tab_id: initial_tab_id,
            error: Some(err_msg),
        });
    }

    let adapter = match registry.resolve_provider(&provider_id) {
        Ok(a) => a,
        Err(e) => {
            let err_msg = e.to_string();
            current_task.status = BrowserTaskStatus::Failed;
            current_task.last_error = Some(err_msg.clone());
            let _ = app.emit(
                "browser-agent-status",
                json!({
                    "task_id": task_id,
                    "status": "Failed",
                    "error": err_msg
                }),
            );
            if let Ok(mut flags) = agent_mgr.cancellation_flags.lock() {
                flags.remove(&task_id);
            }
            return Ok(BrowserTaskResult {
                task_id,
                status: BrowserTaskStatus::Failed,
                goal,
                summary: "Failed before start: unknown provider.".to_string(),
                steps_taken: 0,
                duration_ms: start_instant.elapsed().as_millis() as u64,
                final_tab_id: initial_tab_id,
                error: Some(err_msg),
            });
        }
    };

    let system_prompt = format!(
        "You are E.D.I.T.H.'s Autonomous Browser Agent.
Your objective is to accomplish the user's goal step-by-step using deterministic browser tools.

CURRENT GOAL: \"{}\"
ACTIVE TAB: \"{}\"

RULES & DIRECTIVES:
1. Observe before acting: If you don't know the current URL or available interactive elements, run `browser_observe` first.
2. Element IDs come strictly from `browser_observe` snapshots (e.g. `id_search` or `el_button_...`). Never guess arbitrary element IDs.
3. If an element ID is stale or not found, re-observe the tab and pick an updated element ID.
4. Strictly NO password fields. Never attempt to type into password or credential inputs.
5. Verify results: Check the result of each action before declaring completion.
6. When the goal is completed, output `[TASK_COMPLETE: <detailed summary with evidence>]`.
7. If the task cannot be completed, output `[TASK_FAILED: <reason>]`.

TOOL FORMAT:
To execute a tool, output:
[BROWSER_TOOL: {{\"name\": \"<tool_name>\", \"args\": {{ ... }}}}]

AVAILABLE TOOLS:
- browser_get_tabs: {{}}
- browser_get_active_tab: {{}}
- browser_observe: {{\"tab_id\": \"<tab_id>\", \"scope\": \"full_page\"}}
- browser_screenshot: {{\"tab_id\": \"<tab_id>\"}}
- browser_open_url: {{\"tab_id\": \"<tab_id>\", \"url\": \"https://example.com\"}}
- browser_switch_tab: {{\"tab_id\": \"<tab_id>\"}}
- browser_close_tab: {{\"tab_id\": \"<tab_id>\"}}
- browser_back: {{\"tab_id\": \"<tab_id>\"}}
- browser_forward: {{\"tab_id\": \"<tab_id>\"}}
- browser_reload: {{\"tab_id\": \"<tab_id>\"}}
- browser_click: {{\"tab_id\": \"<tab_id>\", \"element_id\": \"<eid>\"}}
- browser_type: {{\"tab_id\": \"<tab_id>\", \"element_id\": \"<eid>\", \"text\": \"<text>\"}}
- browser_scroll: {{\"tab_id\": \"<tab_id>\", \"direction\": \"down\"}}
- browser_press_key: {{\"tab_id\": \"<tab_id>\", \"key\": \"Enter\"}}
- browser_focus: {{\"tab_id\": \"<tab_id>\", \"element_id\": \"<eid>\"}}
- browser_wait: {{\"tab_id\": \"<tab_id>\", \"condition\": \"timeout\", \"timeout_ms\": 2000}}
- browser_history_recent: {{\"limit\": 20}}
- browser_history_search: {{\"query\": \"<search_term>\"}}
- browser_bookmarks_list: {{}}
- browser_bookmarks_search: {{\"query\": \"<search_term>\"}}
- browser_bookmark_add: {{\"title\": \"<title>\", \"url\": \"https://...\"}}
- browser_bookmark_open: {{\"tab_id\": \"<tab_id>\", \"url\": \"https://...\"}}
- browser_downloads_recent: {{\"limit\": 20}}
- browser_download_get: {{\"download_id\": \"<id>\"}}
- browser_download_cancel: {{\"download_id\": \"<id>\"}}
- browser_download_start: {{\"tab_id\": \"<tab_id>\", \"url\": \"https://...\", \"suggested_filename\": \"file.pdf\"}}
- browser_profiles_list: {{}}
- browser_profile_get: {{\"profile_id\": \"<id>\"}}
- browser_profile_create: {{\"name\": \"<name>\", \"profile_type\": \"AGENT_TEMPORARY\"}}
- browser_profile_switch: {{\"profile_id\": \"<id>\"}}

Output ONLY ONE tool call per turn. Wait for the tool result before taking the next action.",
        goal, initial_tab_id
    );

    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Start executing the goal: \"{}\"", goal),
        },
    ];

    let mut step_count = 0;
    let mut last_action_sig: Option<String> = None;
    let mut consecutive_repeat_count = 0;
    let mut final_summary = String::new();
    let mut final_error: Option<String> = None;
    let mut final_status = BrowserTaskStatus::Running;
    let mut current_tab = initial_tab_id;
    let mut evidence = TaskEvidence::default();

    current_task.status = BrowserTaskStatus::Running;

    while step_count < max_steps {
        // 1. Check cooperative cancellation (both atomic flag and TaskRuntime cancellation token)
        let is_task_cancelled = cancel_flag.load(Ordering::Relaxed) || {
            if let Some(ref rt) = task_runtime {
                rt.get_cancellation_token(&task_id_obj)
                    .await
                    .map(|t| t.is_cancelled())
                    .unwrap_or(false)
            } else {
                false
            }
        };
        if is_task_cancelled {
            final_status = BrowserTaskStatus::Cancelled;
            final_summary = "Task was cancelled by operator.".to_string();
            let _ = app.emit(
                "browser-agent-status",
                json!({
                    "task_id": task_id,
                    "status": "Cancelled",
                    "step": step_count,
                    "message": "Task was cancelled."
                }),
            );
            break;
        }

        // 2. Check wall-clock timeout
        if start_instant.elapsed().as_millis() as u64 > timeout_ms {
            final_status = BrowserTaskStatus::TimedOut;
            final_summary = format!("Task timed out after {} ms.", timeout_ms);
            final_error = Some("EXECUTION_TIMEOUT".to_string());
            let _ = app.emit(
                "browser-agent-status",
                json!({
                    "task_id": task_id,
                    "status": "TimedOut",
                    "step": step_count,
                    "error": "Execution timeout exceeded."
                }),
            );
            break;
        }

        step_count += 1;
        current_task.step_count = step_count;

        let status_msg = format!("Step {}/{}: Agent reasoning...", step_count, max_steps);
        let _ = app.emit(
            "browser-agent-status",
            json!({
                "task_id": task_id,
                "status": "Running",
                "step": step_count,
                "max_steps": max_steps,
                "message": status_msg.clone()
            }),
        );

        if let Some(ref rt) = task_runtime {
            let _ = rt
                .update_progress(&task_id_obj, step_count, max_steps, status_msg)
                .await;
        }

        // Step 15: Bounded Context Truncation
        if messages.len() > 14 {
            let sys = messages[0].clone();
            let goal_msg = messages[1].clone();
            let tail = messages[messages.len() - 10..].to_vec();
            messages = vec![sys, goal_msg];
            messages.extend(tail);
        }

        let ai_messages: Vec<crate::ai::ChatMessage> = messages
            .iter()
            .map(|m| crate::ai::ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                ..Default::default()
            })
            .collect();

        let gen_req = crate::ai::GenerateRequest {
            model: if ai_mode == "local" {
                "local-model".to_string()
            } else {
                model.clone()
            },
            messages: ai_messages,
            temperature: 0.2,
            max_tokens: None,
            stream: false,
            ..Default::default()
        };

        let ai_reply: Result<String, String> = if let Some(gen) = adapter.as_text_generation() {
            gen.generate(&gen_req, &creds)
                .await
                .map(|r| r.text)
                .map_err(|e| e.to_string())
        } else if let Some(streamer) = adapter.as_streaming_text() {
            streamer
                .stream(&gen_req, &creds, Box::new(|_| {}))
                .await
                .map(|r| r.text)
                .map_err(|e| e.to_string())
        } else {
            Err(format!(
                "Provider '{}' does not support text generation",
                provider_id
            ))
        };

        let ai_text = match ai_reply {
            Ok(txt) => txt,
            Err(e) => {
                final_status = BrowserTaskStatus::Failed;
                final_error = Some(format!("LLM_ERROR: {}", e));
                final_summary = format!("Task aborted due to LLM provider error: {}", e);
                break;
            }
        };

        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: ai_text.clone(),
        });

        // 4. Check for task completion claim with False Success Protection (Step 5)
        if ai_text.contains("[TASK_COMPLETE:") {
            if let Some(start) = ai_text.find("[TASK_COMPLETE:") {
                if let Some(end) = ai_text[start..].find("]") {
                    let claimed_summary = ai_text[start + 15..start + end].trim().to_string();

                    // Validate completion claim against task evidence
                    match verify_completion_evidence(&goal, &evidence) {
                        Ok(()) => {
                            final_summary = claimed_summary;
                            final_status = BrowserTaskStatus::Completed;
                            let _ = app.emit(
                                "browser-agent-status",
                                json!({
                                    "task_id": task_id,
                                    "status": "Completed",
                                    "step": step_count,
                                    "summary": final_summary
                                }),
                            );
                            break;
                        }
                        Err(rejection_msg) => {
                            messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: format!("{}\nPlease take the required observation or action to satisfy the goal.", rejection_msg)
                            });
                            continue;
                        }
                    }
                }
            }
        }

        if ai_text.contains("[TASK_FAILED:") {
            if let Some(start) = ai_text.find("[TASK_FAILED:") {
                if let Some(end) = ai_text[start..].find("]") {
                    let reason = ai_text[start + 13..start + end].trim().to_string();
                    final_summary = format!("Task reported failure: {}", reason);
                    final_error = Some(reason);
                    final_status = BrowserTaskStatus::Failed;
                    let _ = app.emit(
                        "browser-agent-status",
                        json!({
                            "task_id": task_id,
                            "status": "Failed",
                            "step": step_count,
                            "error": final_summary
                        }),
                    );
                    break;
                }
            }
        }

        // 5. Parse tool call with robust bracket-aware extractor (Step 2)
        match extract_single_browser_tool_call(&ai_text) {
            Ok(Some((tool_name, args))) => {
                // Step 3: Central Pre-Execution Tool Validation
                if let Err((err_code, err_msg)) = validate_browser_tool_call(&tool_name, &args) {
                    messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("TOOL_VALIDATION_ERROR ({}): {}", err_code, err_msg),
                    });
                    continue;
                }

                // Track target tab
                if let Some(t_id) = args.get("tab_id").and_then(|v| v.as_str()) {
                    current_tab = t_id.to_string();
                    current_task.current_tab_id = current_tab.clone();
                }

                // Step 10: Repetition Protection
                let action_sig = format!("{}:{}:{}", tool_name, current_tab, args.to_string());
                if Some(&action_sig) == last_action_sig.as_ref() {
                    consecutive_repeat_count += 1;
                    if consecutive_repeat_count >= 2 {
                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: "REPETITION_TERMINATION: You have repeated the exact same tool call consecutively without state changes. Task terminating safely.".to_string()
                        });
                        final_status = BrowserTaskStatus::Failed;
                        final_error = Some("REPETITION_DETECTED_TERMINATION".to_string());
                        final_summary =
                            "Task terminated due to repetitive failure loops.".to_string();
                        break;
                    }
                } else {
                    consecutive_repeat_count = 0;
                    last_action_sig = Some(action_sig);
                }

                let _ = app.emit(
                    "browser-agent-status",
                    json!({
                        "task_id": task_id,
                        "status": "Running",
                        "step": step_count,
                        "message": format!("Executing `{}` on tab `{}`...", tool_name, current_tab)
                    }),
                );

                let universal_tool = to_universal_tool_name(&tool_name);
                let tool_correlation = EventCorrelation {
                    task_id: Some(task_id.clone()),
                    ..Default::default()
                };

                let tool_exec_outcome: Result<
                    (
                        bool,
                        Option<serde_json::Value>,
                        Option<String>,
                        Option<String>,
                        u64,
                    ),
                    String,
                > = if let Some(ref router) = tool_router {
                    let req =
                        ToolRequest::new(universal_tool.clone(), args.clone(), tool_correlation);
                    let res = router.execute(req).await;
                    match res.status {
                        ToolStatus::Completed => Ok((true, res.data, None, None, res.duration_ms)),
                        ToolStatus::ApprovalRequired => {
                            let policy_code = "REQUIRE_APPROVAL".to_string();
                            let msg = "REQUIRE_APPROVAL: Action requires operator confirmation. Execution paused.".to_string();
                            current_task.status = BrowserTaskStatus::Waiting;
                            let _ = app.emit(
                                "browser-agent-status",
                                json!({
                                    "task_id": task_id,
                                    "status": "Waiting",
                                    "step": step_count,
                                    "message": msg.clone()
                                }),
                            );
                            Ok((false, None, Some(msg), Some(policy_code), res.duration_ms))
                        }
                        ToolStatus::Blocked => {
                            let err_msg = res.error.map(|e| e.to_string()).unwrap_or_else(|| {
                                "POLICY_BLOCKED: Action blocked by Policy Engine.".to_string()
                            });
                            Ok((
                                false,
                                None,
                                Some(err_msg.clone()),
                                Some("POLICY_BLOCKED".to_string()),
                                res.duration_ms,
                            ))
                        }
                        ToolStatus::Cancelled => {
                            final_status = BrowserTaskStatus::Cancelled;
                            final_summary = "Tool execution was cancelled.".to_string();
                            break;
                        }
                        ToolStatus::Failed => {
                            let err_msg = res
                                .error
                                .map(|e| e.to_string())
                                .unwrap_or_else(|| "Tool execution failed.".to_string());
                            Ok((
                                false,
                                None,
                                Some(err_msg),
                                Some("TOOL_EXECUTION_FAILED".to_string()),
                                res.duration_ms,
                            ))
                        }
                        ToolStatus::Requested | ToolStatus::Approved | ToolStatus::Started => Ok((
                            false,
                            None,
                            Some("Execution in intermediate status.".to_string()),
                            Some("TOOL_IN_PROGRESS".to_string()),
                            res.duration_ms,
                        )),
                    }
                } else {
                    execute_browser_tool(app.clone(), &tool_name, &args, browser_state.clone())
                        .await
                        .map(|r| (r.success, r.data, r.error, r.error_code, r.duration_ms))
                };

                match tool_exec_outcome {
                    Ok((success, data, error, error_code, duration_ms)) => {
                        evidence.executed_tools.push(tool_name.clone());
                        if success {
                            evidence.successful_actions_count += 1;
                        }

                        // Collect evidence on observation or navigation
                        if tool_name == "browser_observe"
                            || tool_name == "browser_open_url"
                            || universal_tool == "browser.observe"
                            || universal_tool == "browser.navigate"
                        {
                            if let Some(d) = data.as_ref() {
                                if let Some(u) = d.get("url").and_then(|v| v.as_str()) {
                                    evidence.observed_urls.push(u.to_string());
                                }
                                if let Some(t) = d.get("title").and_then(|v| v.as_str()) {
                                    evidence.observed_titles.push(t.to_string());
                                }
                            }
                        }

                        let res_json = json!({
                            "success": success,
                            "tool_name": tool_name,
                            "tab_id": current_tab,
                            "data": data,
                            "error": error,
                            "error_code": error_code,
                            "duration_ms": duration_ms,
                        });

                        let _ = app.emit(
                            "browser-agent-step",
                            json!({
                                "task_id": task_id,
                                "step": step_count,
                                "tool": tool_name,
                                "tab_id": current_tab,
                                "success": success,
                                "duration_ms": duration_ms
                            }),
                        );

                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Browser Tool Result:\n{}",
                                serde_json::to_string_pretty(&res_json).unwrap_or_default()
                            ),
                        });
                    }
                    Err(e) => {
                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!("Browser Tool Execution Failed: {}", e),
                        });
                    }
                }
                continue;
            }
            Ok(None) => {
                // No tool call in response
            }
            Err(parse_err) => {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("{}\nPlease format tool calls as [BROWSER_TOOL: {{\"name\": \"...\", \"args\": {{...}}}}]", parse_err)
                });
                continue;
            }
        }

        // If no tool was called and no completion signal was given, prompt agent to take an action
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: "Please select the next browser tool to execute, or output [TASK_COMPLETE: <summary>] if the goal is finished.".to_string()
        });
    }

    if step_count >= max_steps && final_status == BrowserTaskStatus::Running {
        final_status = BrowserTaskStatus::TimedOut;
        final_summary = format!(
            "Task terminated: maximum step limit ({}) reached.",
            max_steps
        );
        final_error = Some("MAX_STEPS_REACHED".to_string());
    }

    current_task.status = final_status;
    current_task.step_count = step_count;
    {
        let mut active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
        *active = Some(current_task);
    }

    // Synchronize TaskRuntime status
    if let Some(ref rt) = task_runtime {
        match final_status {
            BrowserTaskStatus::Completed => {
                let _ = rt
                    .complete_task(
                        &task_id_obj,
                        if final_summary.is_empty() {
                            "Autonomous task finished.".to_string()
                        } else {
                            final_summary.clone()
                        },
                    )
                    .await;
            }
            BrowserTaskStatus::Cancelled => {
                let _ = rt
                    .cancel_task(
                        &task_id_obj,
                        Some("Cancelled by user or system.".to_string()),
                    )
                    .await;
            }
            _ => {
                let err = final_error
                    .clone()
                    .unwrap_or_else(|| "Task ended with failure or timeout.".to_string());
                let _ = rt.fail_task(&task_id_obj, err).await;
            }
        }
    }

    // Step 11: Cleanup cancellation flag on exit
    if let Ok(mut flags) = agent_mgr.cancellation_flags.lock() {
        flags.remove(&task_id);
    }

    Ok(BrowserTaskResult {
        task_id,
        status: final_status,
        goal,
        summary: if final_summary.is_empty() {
            "Autonomous task finished.".to_string()
        } else {
            final_summary
        },
        steps_taken: step_count,
        duration_ms: start_instant.elapsed().as_millis() as u64,
        final_tab_id: current_tab,
        error: final_error,
    })
}

// -----------------------------------------------------------------------------
// Tauri Commands for Autonomous Browser Agent
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_agent_run_task(
    app: AppHandle,
    goal: String,
    max_steps: Option<u32>,
    timeout_ms: Option<u64>,
    browser_state: State<'_, BrowserState>,
    db_state: State<'_, DbState>,
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<BrowserTaskResult, String> {
    run_autonomous_browser_loop(
        app,
        goal,
        max_steps,
        timeout_ms,
        browser_state,
        db_state,
        agent_mgr,
    )
    .await
}

#[tauri::command]
pub async fn browser_agent_cancel_task(
    app: AppHandle,
    task_id: String,
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<bool, String> {
    let mut cancelled = false;
    {
        let flags = agent_mgr
            .cancellation_flags
            .lock()
            .map_err(|e| e.to_string())?;
        if let Some(flag) = flags.get(&task_id) {
            flag.store(true, Ordering::Relaxed);
            cancelled = true;
        }
    }
    if let Some(rt) = app.try_state::<TaskRuntime>() {
        let _ = rt
            .cancel_task(
                &TaskId::from_string(&task_id),
                Some("Operator cancelled task via browser_agent_cancel_task".to_string()),
            )
            .await;
        cancelled = true;
    }
    Ok(cancelled)
}

#[tauri::command]
pub async fn browser_agent_get_current_task(
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<Option<BrowserTaskState>, String> {
    let active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
    Ok(active.clone())
}
