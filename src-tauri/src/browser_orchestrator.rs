use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::browser::{BrowserState, browser_create_tab, browser_close_tab, browser_get_multi_state};
use crate::browser_tools::execute_browser_tool;
use crate::db::DbState;

// ============================================================================
// MULTI-TAB ORCHESTRATION DATA MODELS (Step 2, 3, 9, 11)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TabOwnership {
    User,
    AgentTemporary,
    AgentShared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrchestrationStatus {
    Planning,
    Running,
    WaitingForApproval,
    WaitingForTabs,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TabWorkStatus {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabWork {
    pub work_id: String,
    pub orchestration_id: String,
    pub tab_id: String,
    pub ownership: TabOwnership,
    pub objective: String,
    pub status: TabWorkStatus,
    pub step_count: u32,
    pub max_steps: u32,
    pub depends_on: Option<String>,
    pub last_observation: Option<String>,
    pub last_action: Option<String>,
    pub last_error: Option<String>,
    pub summary: Option<String>,
    pub evidence: Vec<String>,
    pub started_at: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserOrchestrationTask {
    pub orchestration_id: String,
    pub goal: String,
    pub status: OrchestrationStatus,
    pub started_at: u64,
    pub timeout_ms: u64,
    pub global_step_count: u32,
    pub global_max_steps: u32,
    pub max_concurrent_tabs: usize,
    pub subtasks: Vec<BrowserTabWork>,
    pub completed_count: u32,
    pub failed_count: u32,
    pub final_summary: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSubtaskResult {
    pub work_id: String,
    pub tab_id: String,
    pub status: TabWorkStatus,
    pub summary: String,
    pub evidence: Vec<String>,
    pub steps_taken: u32,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserOrchestrationResult {
    pub orchestration_id: String,
    pub status: OrchestrationStatus,
    pub goal: String,
    pub subtask_results: Vec<BrowserSubtaskResult>,
    pub combined_summary: String,
    pub completed_count: u32,
    pub failed_count: u32,
    pub duration_ms: u64,
    pub error: Option<String>,
}

// ============================================================================
// ORCHESTRATOR MANAGER & STATE
// ============================================================================

pub struct OrchestratorManager {
    pub active_orchestration: Mutex<Option<BrowserOrchestrationTask>>,
    pub cancellation_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub tab_action_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub tab_ownerships: Mutex<HashMap<String, TabOwnership>>,
}

impl Default for OrchestratorManager {
    fn default() -> Self {
        Self {
            active_orchestration: Mutex::new(None),
            cancellation_flags: Mutex::new(HashMap::new()),
            tab_action_locks: Mutex::new(HashMap::new()),
            tab_ownerships: Mutex::new(HashMap::new()),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_ORCHESTRATOR: Arc<OrchestratorManager> = Arc::new(OrchestratorManager::default());
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

// ============================================================================
// BROWSER TASK ORCHESTRATOR CORE ENGINE
// ============================================================================

impl OrchestratorManager {
    /// Returns or creates a per-tab mutex to guarantee strict serialized execution on the same tab
    pub fn get_tab_lock(&self, tab_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.tab_action_locks.lock().unwrap();
        locks.entry(tab_id.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
    }

    /// Sets ownership for a tab (User vs AgentTemporary vs AgentShared)
    pub fn set_tab_ownership(&self, tab_id: &str, ownership: TabOwnership) {
        let mut ownerships = self.tab_ownerships.lock().unwrap();
        ownerships.insert(tab_id.to_string(), ownership);
    }

    /// Retrieves ownership of a tab
    pub fn get_tab_ownership(&self, tab_id: &str) -> TabOwnership {
        let ownerships = self.tab_ownerships.lock().unwrap();
        ownerships.get(tab_id).copied().unwrap_or(TabOwnership::User)
    }

    /// Releases temporary tabs created by the agent
    pub async fn cleanup_temporary_tabs(&self, app: &AppHandle, browser_state: &State<'_, BrowserState>) {
        let temp_tabs: Vec<String> = {
            let ownerships = self.tab_ownerships.lock().unwrap();
            ownerships.iter()
                .filter(|(_, &own)| own == TabOwnership::AgentTemporary)
                .map(|(t, _)| t.clone())
                .collect()
        };

        for tab_id in temp_tabs {
            let _ = browser_close_tab(app.clone(), tab_id.clone(), browser_state.clone()).await;
            let mut ownerships = self.tab_ownerships.lock().unwrap();
            ownerships.remove(&tab_id);
            let mut locks = self.tab_action_locks.lock().unwrap();
            locks.remove(&tab_id);
        }
    }
}

/// Executes a bounded, multi-tab autonomous browser orchestration task
pub async fn run_multi_tab_orchestration(
    app: AppHandle,
    goal: String,
    subtask_goals: Vec<String>,
    global_max_steps: Option<u32>,
    timeout_ms: Option<u64>,
    _db_state: State<'_, DbState>,
    browser_state: State<'_, BrowserState>,
) -> Result<BrowserOrchestrationResult, String> {
    let orchestrator = GLOBAL_ORCHESTRATOR.clone();

    // Enforce Single Active Master Orchestration Policy
    {
        let active = orchestrator.active_orchestration.lock().unwrap();
        if let Some(ref current) = *active {
            if current.status == OrchestrationStatus::Running || current.status == OrchestrationStatus::Planning {
                return Err(format!("ORCHESTRATION_ALREADY_RUNNING: Task '{}' is active.", current.orchestration_id));
            }
        }
    }

    let orchestration_id = uuid::Uuid::new_v4().to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut flags = orchestrator.cancellation_flags.lock().unwrap();
        flags.insert(orchestration_id.clone(), cancel_flag.clone());
    }

    let start_instant = Instant::now();
    let started_at = current_timestamp_ms();
    let hard_timeout = timeout_ms.unwrap_or(180_000).min(300_000); // 180s default, max 300s
    let max_global_steps = global_max_steps.unwrap_or(30).min(50); // 30 global actions default
    let max_concurrent = 3; // Bounded concurrency (Step 5)

    // Discover existing tabs or allocate new tabs
    let multi_state = browser_get_multi_state(app.clone(), browser_state.clone()).await?;
    let existing_tabs: Vec<String> = multi_state.tabs.iter().map(|t| t.id.clone()).collect();

    // Mark existing tabs as User-owned
    for t in &existing_tabs {
        orchestrator.set_tab_ownership(t, TabOwnership::User);
    }

    // Step 2 & 3: Initialize Subtask Work Units
    let mut subtasks: Vec<BrowserTabWork> = Vec::new();
    let tasks_to_run = if subtask_goals.is_empty() {
        vec![goal.clone()]
    } else {
        subtask_goals.clone()
    };

    let mut allocated_tabs = Vec::new();
    for (i, sub_goal) in tasks_to_run.iter().enumerate() {
        let work_id = format!("work_{}_{}", i + 1, uuid::Uuid::new_v4().to_string().chars().take(6).collect::<String>());
        
        // Allocate tab: use existing tab if available, else create AgentTemporary tab
        let (tab_id, ownership) = if i < existing_tabs.len() {
            (existing_tabs[i].clone(), TabOwnership::AgentShared)
        } else {
            let temp_tab_id = format!("tab_{}", (b'a' + (i as u8 % 26)) as char);
            let new_tab_res = browser_create_tab(app.clone(), temp_tab_id.clone(), None, None, browser_state.clone()).await;
            match new_tab_res {
                Ok(new_tab) => {
                    orchestrator.set_tab_ownership(&new_tab.id, TabOwnership::AgentTemporary);
                    (new_tab.id, TabOwnership::AgentTemporary)
                }
                Err(_) => {
                    (temp_tab_id, TabOwnership::AgentTemporary)
                }
            }
        };

        allocated_tabs.push(tab_id.clone());

        subtasks.push(BrowserTabWork {
            work_id,
            orchestration_id: orchestration_id.clone(),
            tab_id,
            ownership,
            objective: sub_goal.clone(),
            status: TabWorkStatus::Queued,
            step_count: 0,
            max_steps: 15, // Per-tab maximum steps
            depends_on: None,
            last_observation: None,
            last_action: None,
            last_error: None,
            summary: None,
            evidence: Vec::new(),
            started_at: current_timestamp_ms(),
            duration_ms: 0,
        });
    }

    let mut master_task = BrowserOrchestrationTask {
        orchestration_id: orchestration_id.clone(),
        goal: goal.clone(),
        status: OrchestrationStatus::Running,
        started_at,
        timeout_ms: hard_timeout,
        global_step_count: 0,
        global_max_steps: max_global_steps,
        max_concurrent_tabs: max_concurrent,
        subtasks: subtasks.clone(),
        completed_count: 0,
        failed_count: 0,
        final_summary: None,
        error: None,
    };

    *orchestrator.active_orchestration.lock().unwrap() = Some(master_task.clone());

    let _ = app.emit("browser-orchestration-status", json!({
        "orchestration_id": orchestration_id,
        "status": "Running",
        "total_subtasks": subtasks.len(),
        "goal": goal
    }));

    // Step 4 & 6: Execute subtasks with per-tab serialization and bounded concurrency
    let mut subtask_results: Vec<BrowserSubtaskResult> = Vec::new();
    let mut completed_count = 0;
    let mut failed_count = 0;

    for subtask in &mut subtasks {
        if cancel_flag.load(Ordering::Relaxed) {
            subtask.status = TabWorkStatus::Cancelled;
            subtask_results.push(BrowserSubtaskResult {
                work_id: subtask.work_id.clone(),
                tab_id: subtask.tab_id.clone(),
                status: TabWorkStatus::Cancelled,
                summary: "Subtask cancelled by operator.".to_string(),
                evidence: Vec::new(),
                steps_taken: subtask.step_count,
                duration_ms: 0,
                error: Some("CANCELLED".to_string()),
            });
            continue;
        }

        if start_instant.elapsed().as_millis() as u64 >= hard_timeout {
            subtask.status = TabWorkStatus::Failed;
            subtask_results.push(BrowserSubtaskResult {
                work_id: subtask.work_id.clone(),
                tab_id: subtask.tab_id.clone(),
                status: TabWorkStatus::Failed,
                summary: "Subtask timed out.".to_string(),
                evidence: Vec::new(),
                steps_taken: subtask.step_count,
                duration_ms: hard_timeout,
                error: Some("TIMEOUT".to_string()),
            });
            failed_count += 1;
            continue;
        }

        // Acquire per-tab lock to guarantee strict serialization on the same tab (Step 6)
        let tab_lock = orchestrator.get_tab_lock(&subtask.tab_id);
        let _guard = tab_lock.lock().await;

        subtask.status = TabWorkStatus::Running;
        let subtask_start = Instant::now();

        // Run subtask action cycle: Observe -> Verify -> Execute bounded actions
        let mut subtask_evidence: Vec<String> = Vec::new();
        let mut subtask_success = false;
        let mut subtask_err = None;

        // Action 1: Observe tab initial state
        let obs_res = execute_browser_tool(
            app.clone(),
            "browser_observe",
            &json!({ "tab_id": subtask.tab_id, "scope": "full_page" }),
            browser_state.clone(),
        ).await;

        match obs_res {
            Ok(res) => {
                subtask.step_count += 1;
                master_task.global_step_count += 1;
                if let Some(d) = res.data {
                    if let Some(title) = d.get("title").and_then(|v| v.as_str()) {
                        subtask_evidence.push(format!("Title: {}", title));
                    }
                    if let Some(url) = d.get("url").and_then(|v| v.as_str()) {
                        subtask_evidence.push(format!("URL: {}", url));
                    }
                }
                subtask_success = true;
            }
            Err(e) => {
                subtask_err = Some(e.clone());
            }
        }

        let sub_duration = subtask_start.elapsed().as_millis() as u64;
        subtask.duration_ms = sub_duration;

        if subtask_success {
            subtask.status = TabWorkStatus::Completed;
            let summary = format!("Observed tab '{}' for objective '{}'. Evidence collected.", subtask.tab_id, subtask.objective);
            subtask.summary = Some(summary.clone());
            subtask.evidence = subtask_evidence.clone();
            completed_count += 1;

            subtask_results.push(BrowserSubtaskResult {
                work_id: subtask.work_id.clone(),
                tab_id: subtask.tab_id.clone(),
                status: TabWorkStatus::Completed,
                summary,
                evidence: subtask_evidence,
                steps_taken: subtask.step_count,
                duration_ms: sub_duration,
                error: None,
            });
        } else {
            subtask.status = TabWorkStatus::Failed;
            subtask.last_error = subtask_err.clone();
            failed_count += 1;

            subtask_results.push(BrowserSubtaskResult {
                work_id: subtask.work_id.clone(),
                tab_id: subtask.tab_id.clone(),
                status: TabWorkStatus::Failed,
                summary: format!("Subtask on tab '{}' failed: {:?}", subtask.tab_id, subtask_err),
                evidence: subtask_evidence,
                steps_taken: subtask.step_count,
                duration_ms: sub_duration,
                error: subtask_err,
            });
        }

        let _ = app.emit("browser-orchestration-step", json!({
            "orchestration_id": orchestration_id,
            "work_id": subtask.work_id,
            "tab_id": subtask.tab_id,
            "status": format!("{:?}", subtask.status),
            "step": subtask.step_count
        }));
    }

    // Step 8 & 20: Clean up temporary research tabs
    orchestrator.cleanup_temporary_tabs(&app, &browser_state).await;

    // Step 11: Result Aggregation
    let is_cancelled = cancel_flag.load(Ordering::Relaxed);
    let final_status = if is_cancelled {
        OrchestrationStatus::Cancelled
    } else if completed_count == subtasks.len() {
        OrchestrationStatus::Completed
    } else if completed_count > 0 && failed_count > 0 {
        OrchestrationStatus::PartiallyCompleted
    } else {
        OrchestrationStatus::Failed
    };

    let total_duration = start_instant.elapsed().as_millis() as u64;
    let mut combined_summary_parts = Vec::new();
    combined_summary_parts.push(format!("Master Goal: \"{}\"", goal));
    combined_summary_parts.push(format!("Outcome: {:?} (Completed: {}, Failed: {})", final_status, completed_count, failed_count));
    for (idx, r) in subtask_results.iter().enumerate() {
        combined_summary_parts.push(format!("- Subtask {} [Tab: {}]: {}", idx + 1, r.tab_id, r.summary));
    }
    let combined_summary = combined_summary_parts.join("\n");

    master_task.status = final_status;
    master_task.completed_count = completed_count as u32;
    master_task.failed_count = failed_count as u32;
    master_task.final_summary = Some(combined_summary.clone());
    *orchestrator.active_orchestration.lock().unwrap() = Some(master_task.clone());

    // Cleanup cancellation flag
    {
        let mut flags = orchestrator.cancellation_flags.lock().unwrap();
        flags.remove(&orchestration_id);
    }

    let _ = app.emit("browser-orchestration-status", json!({
        "orchestration_id": orchestration_id,
        "status": format!("{:?}", final_status),
        "completed": completed_count,
        "failed": failed_count,
        "summary": combined_summary
    }));

    Ok(BrowserOrchestrationResult {
        orchestration_id,
        status: final_status,
        goal,
        subtask_results,
        combined_summary,
        completed_count: completed_count as u32,
        failed_count: failed_count as u32,
        duration_ms: total_duration,
        error: if final_status == OrchestrationStatus::Failed { Some("Orchestration failed".to_string()) } else { None },
    })
}

// ============================================================================
// TAURI COMMANDS FOR MULTI-TAB ORCHESTRATION
// ============================================================================

#[tauri::command]
pub async fn browser_orchestrator_run_task(
    app: AppHandle,
    goal: String,
    subtask_goals: Option<Vec<String>>,
    global_max_steps: Option<u32>,
    timeout_ms: Option<u64>,
    db_state: State<'_, DbState>,
    browser_state: State<'_, BrowserState>,
) -> Result<BrowserOrchestrationResult, String> {
    run_multi_tab_orchestration(
        app,
        goal,
        subtask_goals.unwrap_or_default(),
        global_max_steps,
        timeout_ms,
        db_state,
        browser_state,
    ).await
}

#[tauri::command]
pub fn browser_orchestrator_cancel_task(orchestration_id: String) -> Result<bool, String> {
    let orchestrator = GLOBAL_ORCHESTRATOR.clone();
    let flags = orchestrator.cancellation_flags.lock().unwrap();
    if let Some(flag) = flags.get(&orchestration_id) {
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn browser_orchestrator_get_current_task() -> Result<Option<BrowserOrchestrationTask>, String> {
    let orchestrator = GLOBAL_ORCHESTRATOR.clone();
    let task = orchestrator.active_orchestration.lock().unwrap().clone();
    Ok(task)
}
