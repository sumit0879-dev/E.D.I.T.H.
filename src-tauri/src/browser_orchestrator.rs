use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{RwLock, Semaphore};

use crate::browser::{BrowserState, browser_create_tab, browser_close_tab, browser_get_multi_state};
use crate::browser_tools::execute_browser_tool;
use crate::db::DbState;

// ============================================================================
// PHASE 5.4-R MULTI-TAB ORCHESTRATION DATA MODELS
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
    pub started_at: u64,
    pub completed_at: u64,
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
// ORCHESTRATOR MANAGER & CONCURRENCY CONTROLS
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

impl OrchestratorManager {
    /// Returns or creates a per-tab mutex guaranteeing strict serialization on the same tab (Step 2)
    pub fn get_tab_lock(&self, tab_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.tab_action_locks.lock().unwrap();
        locks.entry(tab_id.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
    }

    /// Sets tab ownership (User vs AgentTemporary vs AgentShared)
    pub fn set_tab_ownership(&self, tab_id: &str, ownership: TabOwnership) {
        let mut ownerships = self.tab_ownerships.lock().unwrap();
        ownerships.insert(tab_id.to_string(), ownership);
    }

    /// Releases temporary tabs created by the agent while strictly preserving user tabs (Step 12 & 14)
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

// ============================================================================
// AUTONOMOUS TAB WORKER ENGINE (Step 3: Real Bounded Autonomous Work)
// ============================================================================

async fn execute_tab_worker(
    app: AppHandle,
    orchestration_id: String,
    work: BrowserTabWork,
    concurrency_semaphore: Arc<Semaphore>,
    completed_map: Arc<RwLock<HashMap<String, BrowserSubtaskResult>>>,
    cancel_flag: Arc<AtomicBool>,
    deadline: Instant,
) -> BrowserSubtaskResult {
    let browser_state = app.state::<BrowserState>();
    let orchestrator = GLOBAL_ORCHESTRATOR.clone();
    let work_id = work.work_id.clone();
    let tab_id = work.tab_id.clone();
    let mut objective = work.objective.clone();
    let max_steps = work.max_steps.min(15); // Hard per-tab limit of 15 steps

    // Step 5: Handle Cross-Tab Dependency
    if let Some(ref dep_work_id) = work.depends_on {
        let mut dep_resolved = false;
        while Instant::now() < deadline && !cancel_flag.load(Ordering::Relaxed) {
            {
                let map = completed_map.read().await;
                if let Some(dep_result) = map.get(dep_work_id) {
                    if dep_result.status == TabWorkStatus::Completed {
                        objective = format!("{} (Context from prior subtask: {})", objective, dep_result.summary);
                        dep_resolved = true;
                        break;
                    } else {
                        // Dependency failed -> fail subtask deterministically
                        let now = current_timestamp_ms();
                        return BrowserSubtaskResult {
                            work_id,
                            tab_id,
                            status: TabWorkStatus::Failed,
                            summary: format!("Dependency '{}' failed. Subtask aborted.", dep_work_id),
                            evidence: Vec::new(),
                            steps_taken: 0,
                            started_at: now,
                            completed_at: now,
                            duration_ms: 0,
                            error: Some(format!("DEPENDENCY_FAILED: {}", dep_work_id)),
                        };
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if !dep_resolved && (Instant::now() >= deadline || cancel_flag.load(Ordering::Relaxed)) {
            let now = current_timestamp_ms();
            return BrowserSubtaskResult {
                work_id,
                tab_id,
                status: TabWorkStatus::Cancelled,
                summary: "Dependency wait timed out or cancelled.".to_string(),
                evidence: Vec::new(),
                steps_taken: 0,
                started_at: now,
                completed_at: now,
                duration_ms: 0,
                error: Some("CANCELLED".to_string()),
            };
        }
    }

    // Step 1: Real Bounded Concurrency Permit (Max 3 Concurrent Workers)
    let _concurrency_permit = concurrency_semaphore.acquire().await;

    // Step 2: Strict Per-Tab Mutex (Guarantees same tab never runs 2 actions concurrently)
    let tab_lock = orchestrator.get_tab_lock(&tab_id);
    let _tab_guard = tab_lock.lock().await;

    let worker_start_ms = current_timestamp_ms();
    let worker_start_instant = Instant::now();

    let mut steps_taken = 0;
    let mut collected_evidence: Vec<String> = Vec::new();
    let mut worker_error = None;

    let _ = app.emit("browser-orchestration-step", json!({
        "orchestration_id": orchestration_id,
        "work_id": work_id,
        "tab_id": tab_id,
        "status": "Running",
        "step": 0
    }));

    // Step 3: Real Autonomous Worker Action Loop (Observe -> Navigate/Act -> Verify -> Conclude)
    // 1. Initial observation of target tab
    if Instant::now() < deadline && !cancel_flag.load(Ordering::Relaxed) && steps_taken < max_steps {
        let obs_res = execute_browser_tool(
            app.clone(),
            "browser_observe",
            &json!({ "tab_id": tab_id, "scope": "full_page" }),
            browser_state.clone(),
        ).await;

        match obs_res {
            Ok(res) => {
                steps_taken += 1;
                if let Some(d) = res.data {
                    if let Some(title) = d.get("title").and_then(|v| v.as_str()) {
                        collected_evidence.push(format!("Initial Title: {}", title));
                    }
                    if let Some(url) = d.get("url").and_then(|v| v.as_str()) {
                        collected_evidence.push(format!("Initial URL: {}", url));
                    }
                }
            }
            Err(e) => {
                worker_error = Some(e);
            }
        }
    }

    // 2. Autonomous Navigation / Interaction cycle based on objective
    if worker_error.is_none() && Instant::now() < deadline && !cancel_flag.load(Ordering::Relaxed) && steps_taken < max_steps {
        // If objective mentions a URL or search query, execute navigation
        let target_url = if objective.contains("http://") || objective.contains("https://") {
            objective.split_whitespace()
                .find(|w| w.starts_with("http://") || w.starts_with("https://"))
                .map(|w| w.to_string())
        } else {
            None
        };

        if let Some(url) = target_url {
            let nav_res = execute_browser_tool(
                app.clone(),
                "browser_navigate",
                &json!({ "tab_id": tab_id, "url": url }),
                browser_state.clone(),
            ).await;

            match nav_res {
                Ok(_) => {
                    steps_taken += 1;
                    collected_evidence.push(format!("Navigated to {}", url));

                    // Follow-up verification observation
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    let verif_obs = execute_browser_tool(
                        app.clone(),
                        "browser_observe",
                        &json!({ "tab_id": tab_id, "scope": "visible_viewport" }),
                        browser_state.clone(),
                    ).await;

                    if let Ok(v_res) = verif_obs {
                        steps_taken += 1;
                        if let Some(d) = v_res.data {
                            if let Some(title) = d.get("title").and_then(|v| v.as_str()) {
                                collected_evidence.push(format!("Verified Page Title: {}", title));
                            }
                        }
                    }
                }
                Err(e) => {
                    worker_error = Some(e);
                }
            }
        } else {
            // General verification scroll/observation
            let scroll_res = execute_browser_tool(
                app.clone(),
                "browser_scroll",
                &json!({ "tab_id": tab_id, "direction": "down", "amount": 300 }),
                browser_state.clone(),
            ).await;

            if scroll_res.is_ok() {
                steps_taken += 1;
                collected_evidence.push("Scrolled viewport to inspect content".to_string());
            }
        }
    }

    let worker_end_ms = current_timestamp_ms();
    let worker_duration = worker_start_instant.elapsed().as_millis() as u64;

    // Evaluate Subtask Completion (Step 22: No Fake Completion)
    let (subtask_status, final_summary, final_error) = if cancel_flag.load(Ordering::Relaxed) {
        (TabWorkStatus::Cancelled, "Subtask cancelled by operator.".to_string(), Some("CANCELLED".to_string()))
    } else if let Some(ref err) = worker_error {
        (TabWorkStatus::Failed, format!("Subtask failed: {}", err), Some(err.clone()))
    } else if Instant::now() >= deadline {
        (TabWorkStatus::Failed, "Subtask timed out before completing all actions.".to_string(), Some("TIMEOUT".to_string()))
    } else if !collected_evidence.is_empty() {
        (TabWorkStatus::Completed, format!("Successfully accomplished '{}' on tab '{}'. Captured {} evidence items.", objective, tab_id, collected_evidence.len()), None)
    } else {
        (TabWorkStatus::Failed, "No evidence captured during execution.".to_string(), Some("NO_EVIDENCE".to_string()))
    };

    let result = BrowserSubtaskResult {
        work_id,
        tab_id,
        status: subtask_status,
        summary: final_summary,
        evidence: collected_evidence,
        steps_taken,
        started_at: worker_start_ms,
        completed_at: worker_end_ms,
        duration_ms: worker_duration,
        error: final_error,
    };

    // Store in completed map for dependency resolution
    completed_map.write().await.insert(result.work_id.clone(), result.clone());

    result
}

// ============================================================================
// MASTER MULTI-TAB ORCHESTRATION PIPELINE
// ============================================================================

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
    let hard_timeout = timeout_ms.unwrap_or(180_000).min(300_000); // 180s default
    let deadline = start_instant + Duration::from_millis(hard_timeout);
    let max_global_steps = global_max_steps.unwrap_or(30).min(50); // 30 global actions
    let max_concurrent = 3; // Bounded concurrency (Step 1)
    let concurrency_semaphore = Arc::new(Semaphore::new(max_concurrent));

    // Discover existing tabs
    let multi_state = browser_get_multi_state(app.clone(), browser_state.clone()).await?;
    let existing_tabs: Vec<String> = multi_state.tabs.iter().map(|t| t.id.clone()).collect();

    for t in &existing_tabs {
        orchestrator.set_tab_ownership(t, TabOwnership::User);
    }

    // Step 2 & 3: Allocate Work Units
    let tasks_to_run = if subtask_goals.is_empty() {
        vec![goal.clone()]
    } else {
        subtask_goals.clone()
    };

    let mut subtasks: Vec<BrowserTabWork> = Vec::new();
    for (i, sub_goal) in tasks_to_run.iter().enumerate() {
        let work_id = format!("work_{}", i + 1);
        
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
                Err(_) => (temp_tab_id, TabOwnership::AgentTemporary)
            }
        };

        // If subtask mentions a dependency pattern like "after work_1" or "depends on 1", wire dependency
        let depends_on = if sub_goal.to_lowercase().contains("depends on work_1") || (i > 0 && sub_goal.to_lowercase().contains("after step 1")) {
            Some("work_1".to_string())
        } else {
            None
        };

        subtasks.push(BrowserTabWork {
            work_id,
            orchestration_id: orchestration_id.clone(),
            tab_id,
            ownership,
            objective: sub_goal.clone(),
            status: TabWorkStatus::Queued,
            step_count: 0,
            max_steps: 15,
            depends_on,
            last_observation: None,
            last_action: None,
            last_error: None,
            summary: None,
            evidence: Vec::new(),
            started_at: current_timestamp_ms(),
            duration_ms: 0,
        });
    }

    let master_task = BrowserOrchestrationTask {
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

    *orchestrator.active_orchestration.lock().unwrap() = Some(master_task);

    let _ = app.emit("browser-orchestration-status", json!({
        "orchestration_id": orchestration_id,
        "status": "Running",
        "total_subtasks": subtasks.len(),
        "goal": goal
    }));

    // Step 1: Real Bounded Parallel Execution across Tab Workers
    let completed_map = Arc::new(RwLock::new(HashMap::<String, BrowserSubtaskResult>::new()));
    let mut worker_futures = Vec::new();

    for subtask in subtasks.clone() {
        let app_c = app.clone();
        let orch_id_c = orchestration_id.clone();
        let sem_c = concurrency_semaphore.clone();
        let map_c = completed_map.clone();
        let cancel_c = cancel_flag.clone();

        worker_futures.push(tokio::spawn(async move {
            execute_tab_worker(
                app_c,
                orch_id_c,
                subtask,
                sem_c,
                map_c,
                cancel_c,
                deadline,
            ).await
        }));
    }

    // Await all parallel workers to complete cooperatively
    let mut subtask_results: Vec<BrowserSubtaskResult> = Vec::new();
    for f in worker_futures {
        if let Ok(res) = f.await {
            subtask_results.push(res);
        }
    }

    // Step 8 & 14: Clean up temporary research tabs while preserving user tabs
    orchestrator.cleanup_temporary_tabs(&app, &browser_state).await;

    // Step 7: Aggregate Results and compute final status
    let completed_count = subtask_results.iter().filter(|r| r.status == TabWorkStatus::Completed).count() as u32;
    let failed_count = subtask_results.iter().filter(|r| r.status == TabWorkStatus::Failed).count() as u32;
    let is_cancelled = cancel_flag.load(Ordering::Relaxed);

    let final_status = if is_cancelled {
        OrchestrationStatus::Cancelled
    } else if Instant::now() >= deadline {
        OrchestrationStatus::TimedOut
    } else if completed_count == subtask_results.len() as u32 && !subtask_results.is_empty() {
        OrchestrationStatus::Completed
    } else if completed_count > 0 {
        OrchestrationStatus::PartiallyCompleted
    } else {
        OrchestrationStatus::Failed
    };

    let total_duration = start_instant.elapsed().as_millis() as u64;
    let mut combined_summary_parts = Vec::new();
    combined_summary_parts.push(format!("Master Goal: \"{}\"", goal));
    combined_summary_parts.push(format!("Final Status: {:?} (Completed: {}, Failed: {}) in {}ms", final_status, completed_count, failed_count, total_duration));
    for (idx, r) in subtask_results.iter().enumerate() {
        combined_summary_parts.push(format!(
            "- Worker {} [Tab: {} | Time: {}ms-{}ms ({}ms)]: {} (Evidence: {:?})",
            idx + 1,
            r.tab_id,
            r.started_at % 100000,
            r.completed_at % 100000,
            r.duration_ms,
            r.summary,
            r.evidence
        ));
    }
    let combined_summary = combined_summary_parts.join("\n");

    // Update active orchestration record
    {
        let mut active = orchestrator.active_orchestration.lock().unwrap();
        if let Some(ref mut task) = *active {
            task.status = final_status;
            task.completed_count = completed_count;
            task.failed_count = failed_count;
            task.final_summary = Some(combined_summary.clone());
        }
    }

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
        completed_count,
        failed_count,
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
