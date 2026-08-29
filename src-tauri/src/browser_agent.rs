use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

use crate::browser::{BrowserState, browser_get_multi_state};
use crate::browser_tools::execute_browser_tool;
use crate::db::DbState;
use crate::llm::{api_chat_cloud, ChatMessage, ChatRequest};

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

fn get_provider_url(provider: &str) -> String {
    match provider {
        "groq" => "https://api.groq.com/openai/v1/chat/completions".to_string(),
        "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
        "together" => "https://api.together.xyz/v1/chat/completions".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        "deepseek" => "https://api.deepseek.com/chat/completions".to_string(),
        "cerebras" => "https://api.cerebras.ai/v1/chat/completions".to_string(),
        "sambanova" => "https://api.sambanova.ai/v1/chat/completions".to_string(),
        "mistral" => "https://api.mistral.ai/v1/chat/completions".to_string(),
        "huggingface" => "https://api-inference.huggingface.co/v1/chat/completions".to_string(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string(),
        _ => "https://api.groq.com/openai/v1/chat/completions".to_string(),
    }
}

pub async fn run_autonomous_browser_loop(
    app: AppHandle,
    goal: String,
    max_steps_opt: Option<u32>,
    timeout_ms_opt: Option<u64>,
    browser_state: State<'_, BrowserState>,
    db_state: State<'_, DbState>,
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<BrowserTaskResult, String> {
    let task_id = format!("task_{}", current_timestamp_ms());
    let max_steps = max_steps_opt.unwrap_or(20).clamp(1, 20);
    let timeout_ms = timeout_ms_opt.unwrap_or(120_000).clamp(5_000, 120_000);
    let start_instant = Instant::now();

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut flags = agent_mgr.cancellation_flags.lock().map_err(|e| e.to_string())?;
        flags.insert(task_id.clone(), cancel_flag.clone());
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

    let _ = app.emit("browser-agent-status", json!({
        "task_id": task_id,
        "status": "Planning",
        "step": 0,
        "max_steps": max_steps,
        "message": format!("Planning autonomous browser task: {}", goal)
    }));

    let settings = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        crate::db::get_all_settings(&conn).unwrap_or_default()
    };

    let ai_mode = settings.get("aiMode").cloned().unwrap_or_else(|| "api".to_string());
    let provider = settings.get("selectedProvider").cloned().unwrap_or_else(|| "groq".to_string());
    let model = settings.get("selectedModel").cloned().unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());
    let api_key_key = format!("api_key_{}", provider);
    let api_key = settings.get(&api_key_key)
        .or_else(|| settings.get(&format!("apiKey_{}", provider)))
        .or_else(|| settings.get("apiKey")).cloned().unwrap_or_default();

    if ai_mode != "local" && api_key.is_empty() {
        let err_msg = "API Key is missing for selected provider.".to_string();
        current_task.status = BrowserTaskStatus::Failed;
        current_task.last_error = Some(err_msg.clone());
        let _ = app.emit("browser-agent-status", json!({
            "task_id": task_id,
            "status": "Failed",
            "error": err_msg
        }));
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
6. When the goal is completed, output `[TASK_COMPLETE: <detailed summary of what was accomplished and found>]`.
7. If the task cannot be completed, output `[TASK_FAILED: <reason>]`.

TOOL FORMAT:
To execute a tool, output:
[BROWSER_TOOL: {{\"name\": \"<tool_name>\", \"args\": {{ ... }}}}]

AVAILABLE TOOLS:
- browser_get_tabs: {{}}
- browser_get_active_tab: {{}}
- browser_observe: {{\"tab_id\": \"<tab_id>\"}}
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

Output ONLY ONE tool call per turn. Wait for the tool result before taking the next action.",
        goal, initial_tab_id
    );

    let mut messages = vec![
        ChatMessage { role: "system".to_string(), content: system_prompt },
        ChatMessage { role: "user".to_string(), content: format!("Start executing the goal: \"{}\"", goal) }
    ];

    let mut step_count = 0;
    let mut last_action_sig: Option<String> = None;
    let mut consecutive_repeat_count = 0;
    let mut final_summary = String::new();
    let mut final_error: Option<String> = None;
    let mut final_status = BrowserTaskStatus::Running;
    let mut current_tab = initial_tab_id;

    current_task.status = BrowserTaskStatus::Running;

    while step_count < max_steps {
        // 1. Check cooperative cancellation
        if cancel_flag.load(Ordering::Relaxed) {
            final_status = BrowserTaskStatus::Cancelled;
            final_summary = "Task was cancelled by user.".to_string();
            let _ = app.emit("browser-agent-status", json!({
                "task_id": task_id,
                "status": "Cancelled",
                "step": step_count,
                "message": "Task was cancelled."
            }));
            break;
        }

        // 2. Check wall-clock timeout
        if start_instant.elapsed().as_millis() as u64 > timeout_ms {
            final_status = BrowserTaskStatus::TimedOut;
            final_summary = format!("Task timed out after {} ms.", timeout_ms);
            final_error = Some("EXECUTION_TIMEOUT".to_string());
            let _ = app.emit("browser-agent-status", json!({
                "task_id": task_id,
                "status": "TimedOut",
                "step": step_count,
                "error": "Execution timeout exceeded."
            }));
            break;
        }

        step_count += 1;
        current_task.step_count = step_count;

        let _ = app.emit("browser-agent-status", json!({
            "task_id": task_id,
            "status": "Running",
            "step": step_count,
            "max_steps": max_steps,
            "message": format!("Step {}/{}: Agent reasoning...", step_count, max_steps)
        }));

        // 3. Query LLM
        let req = ChatRequest {
            model: if ai_mode == "local" { "local-model".to_string() } else { model.clone() },
            messages: messages.clone(),
            temperature: 0.2, // low temperature for deterministic tool choice
            provider: if ai_mode == "local" { "local".to_string() } else { provider.clone() },
        };

        let ai_reply = if ai_mode == "local" {
            api_chat_cloud(
                app.clone(),
                "".to_string(),
                "http://127.0.0.1:11434/v1/chat/completions".to_string(),
                req,
                None
            ).await
        } else {
            api_chat_cloud(
                app.clone(), 
                api_key.clone(), 
                get_provider_url(&provider), 
                req, 
                None
            ).await
        };

        let ai_text = match ai_reply {
            Ok(txt) => txt,
            Err(e) => {
                final_status = BrowserTaskStatus::Failed;
                final_error = Some(format!("LLM provider error: {}", e));
                final_summary = format!("Task aborted due to LLM provider error: {}", e);
                break;
            }
        };

        messages.push(ChatMessage { role: "assistant".to_string(), content: ai_text.clone() });

        // 4. Check for task completion / failure signals
        if ai_text.contains("[TASK_COMPLETE:") {
            if let Some(start) = ai_text.find("[TASK_COMPLETE:") {
                if let Some(end) = ai_text[start..].find("]") {
                    final_summary = ai_text[start + 15 .. start + end].trim().to_string();
                    final_status = BrowserTaskStatus::Completed;
                    let _ = app.emit("browser-agent-status", json!({
                        "task_id": task_id,
                        "status": "Completed",
                        "step": step_count,
                        "summary": final_summary
                    }));
                    break;
                }
            }
        }

        if ai_text.contains("[TASK_FAILED:") {
            if let Some(start) = ai_text.find("[TASK_FAILED:") {
                if let Some(end) = ai_text[start..].find("]") {
                    let reason = ai_text[start + 13 .. start + end].trim().to_string();
                    final_summary = format!("Task reported failure: {}", reason);
                    final_error = Some(reason);
                    final_status = BrowserTaskStatus::Failed;
                    let _ = app.emit("browser-agent-status", json!({
                        "task_id": task_id,
                        "status": "Failed",
                        "step": step_count,
                        "error": final_summary
                    }));
                    break;
                }
            }
        }

        // 5. Check for tool invocation
        if ai_text.contains("[BROWSER_TOOL:") {
            if let Some(start) = ai_text.find("[BROWSER_TOOL:") {
                if let Some(end) = ai_text[start..].find("]") {
                    let raw_json = ai_text[start + 14 .. start + end].trim();
                    let parsed: serde_json::Value = match serde_json::from_str(raw_json) {
                        Ok(p) => p,
                        Err(e) => {
                            messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: format!("Error: Malformed BROWSER_TOOL JSON: {}. Please output valid JSON.", e)
                            });
                            continue;
                        }
                    };

                    let tool_name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                    let empty_args = json!({});
                    let args = parsed.get("args").unwrap_or(&empty_args);

                    // Track target tab
                    if let Some(t_id) = args.get("tab_id").and_then(|v| v.as_str()) {
                        current_tab = t_id.to_string();
                        current_task.current_tab_id = current_tab.clone();
                    }

                    // Check Repetition
                    let action_sig = format!("{}:{}:{}", tool_name, current_tab, args.to_string());
                    if Some(&action_sig) == last_action_sig.as_ref() {
                        consecutive_repeat_count += 1;
                        if consecutive_repeat_count >= 2 {
                            messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: "REPETITION_WARNING: You repeated the exact same tool call twice with the same result. You must either re-observe the tab or try a different action to avoid an infinite loop.".to_string()
                            });
                            last_action_sig = Some(action_sig);
                            continue;
                        }
                    } else {
                        consecutive_repeat_count = 0;
                        last_action_sig = Some(action_sig);
                    }

                    let _ = app.emit("browser-agent-status", json!({
                        "task_id": task_id,
                        "status": "Running",
                        "step": step_count,
                        "message": format!("Executing `{}` on tab `{}`...", tool_name, current_tab)
                    }));

                    // Execute tool safely through Browser Tool Layer
                    match execute_browser_tool(app.clone(), tool_name, args, browser_state.clone()).await {
                        Ok(res) => {
                            let res_json = json!({
                                "success": res.success,
                                "tool_name": res.tool_name,
                                "tab_id": res.tab_id,
                                "data": res.data,
                                "error": res.error,
                                "error_code": res.error_code,
                                "duration_ms": res.duration_ms,
                            });

                            let _ = app.emit("browser-agent-step", json!({
                                "task_id": task_id,
                                "step": step_count,
                                "tool": tool_name,
                                "tab_id": current_tab,
                                "success": res.success,
                                "duration_ms": res.duration_ms
                            }));

                            messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: format!("Browser Tool Result:\n{}", serde_json::to_string_pretty(&res_json).unwrap_or_default())
                            });
                        }
                        Err(e) => {
                            messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: format!("Browser Tool Execution Failed: {}", e)
                            });
                        }
                    }
                    continue;
                }
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
        final_summary = format!("Task terminated: maximum step limit ({}) reached.", max_steps);
        final_error = Some("MAX_STEPS_REACHED".to_string());
    }

    current_task.status = final_status;
    current_task.step_count = step_count;
    {
        let mut active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
        *active = Some(current_task);
    }

    Ok(BrowserTaskResult {
        task_id,
        status: final_status,
        goal,
        summary: if final_summary.is_empty() { "Autonomous task finished.".to_string() } else { final_summary },
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
    ).await
}

#[tauri::command]
pub async fn browser_agent_cancel_task(
    task_id: String,
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<bool, String> {
    let flags = agent_mgr.cancellation_flags.lock().map_err(|e| e.to_string())?;
    if let Some(flag) = flags.get(&task_id) {
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn browser_agent_get_current_task(
    agent_mgr: State<'_, BrowserAgentManager>,
) -> Result<Option<BrowserTaskState>, String> {
    let active = agent_mgr.active_task.lock().map_err(|e| e.to_string())?;
    Ok(active.clone())
}
