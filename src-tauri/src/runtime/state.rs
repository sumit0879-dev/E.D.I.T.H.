//! state.rs — EdithRuntimeState: Centralized runtime observation and coordination read-model.
//!
//! Aggregates live state from authoritative subsystems without duplicating domain state.

use super::autonomy::AutonomyState;
use super::projections::*;
use crate::ai::registry::ProviderSummary;
use crate::ai::ProviderRegistry;
use crate::browser::BrowserState;
use crate::browser_control::{BrowserControlState, GLOBAL_CONTROL_MGR};
use crate::computer_control::{ComputerControlState, GLOBAL_COMPUTER_CONTROL_MGR};
use crate::conversation::ConversationCore;
use crate::events::TaskId;
use crate::policy::PolicyEngine;
use crate::task::types::TaskSnapshot;
use crate::task::TaskRuntime;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Authoritative coordination and observation layer for E.D.I.T.H.
#[derive(Clone)]
pub struct EdithRuntimeState {
    conversation_core: Arc<ConversationCore>,
    task_runtime: Arc<TaskRuntime>,
    tool_registry: Arc<ToolRegistry>,
    tool_router: Arc<ToolRouter>,
    provider_registry: Arc<RwLock<ProviderRegistry>>,
    policy_engine: Arc<PolicyEngine>,
    app_handle: Option<tauri::AppHandle>,
    db_conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    start_time: Instant,
}

impl EdithRuntimeState {
    /// Creates a new EdithRuntimeState bound to active application subsystems.
    pub fn new(
        conversation_core: Arc<ConversationCore>,
        task_runtime: Arc<TaskRuntime>,
        tool_registry: Arc<ToolRegistry>,
        tool_router: Arc<ToolRouter>,
        provider_registry: Arc<RwLock<ProviderRegistry>>,
        policy_engine: Arc<PolicyEngine>,
        app_handle: Option<tauri::AppHandle>,
        db_conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    ) -> Self {
        Self {
            conversation_core,
            task_runtime,
            tool_registry,
            tool_router,
            provider_registry,
            policy_engine,
            app_handle,
            db_conn,
            start_time: Instant::now(),
        }
    }

    /// Creates an in-memory mock EdithRuntimeState for tests without database or Tauri handles.
    pub fn mock(
        conversation_core: Arc<ConversationCore>,
        task_runtime: Arc<TaskRuntime>,
        tool_registry: Arc<ToolRegistry>,
        tool_router: Arc<ToolRouter>,
        provider_registry: Arc<RwLock<ProviderRegistry>>,
        policy_engine: Arc<PolicyEngine>,
    ) -> Self {
        Self::new(
            conversation_core,
            task_runtime,
            tool_registry,
            tool_router,
            provider_registry,
            policy_engine,
            None,
            None,
        )
    }

    /// Accessors for underlying subsystems (coordination boundary)
    pub fn conversation_core(&self) -> &Arc<ConversationCore> {
        &self.conversation_core
    }

    pub fn task_runtime(&self) -> &Arc<TaskRuntime> {
        &self.task_runtime
    }

    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }

    pub fn tool_router(&self) -> &Arc<ToolRouter> {
        &self.tool_router
    }

    pub fn provider_registry(&self) -> &Arc<RwLock<ProviderRegistry>> {
        &self.provider_registry
    }

    pub fn policy_engine(&self) -> &Arc<PolicyEngine> {
        &self.policy_engine
    }

    pub fn app_handle(&self) -> Option<&tauri::AppHandle> {
        self.app_handle.as_ref()
    }

    /// Derives the high-level operational autonomy state from live subsystem metrics.
    pub async fn get_autonomy_state(&self) -> AutonomyState {
        // 1. User Takeover Preemption: Check desktop input or browser controls
        let computer_info = GLOBAL_COMPUTER_CONTROL_MGR.get_control_info();
        if computer_info.control_state == ComputerControlState::AiPaused {
            return AutonomyState::UserTakeover;
        }

        let tab_controls = GLOBAL_CONTROL_MGR.get_all_tab_controls();
        if tab_controls
            .iter()
            .any(|c| c.control_state == BrowserControlState::AiPaused)
        {
            return AutonomyState::UserTakeover;
        }

        // 2. Pending Approvals: Blocked on operator confirmation
        let pending = self.policy_engine.approvals().list_pending().await;
        if !pending.is_empty() {
            return AutonomyState::WaitingForApproval;
        }

        // 3. Autonomous Background Tasks
        let active_tasks = self.task_runtime.list_active_tasks().await;
        if !active_tasks.is_empty() {
            return AutonomyState::RunningTask;
        }

        // 4. In-flight Tool Executions
        let active_execs = self
            .tool_router
            .cancellation()
            .active_execution_count()
            .await;
        if active_execs > 0 {
            return AutonomyState::ExecutingTool;
        }

        // 5. Active Conversational Streaming Turns
        if self.conversation_core.has_active_turns().await {
            return AutonomyState::Conversing;
        }

        // 6. Default: Ready and Idle
        AutonomyState::Idle
    }

    /// Reads active provider and model settings from the database (or defaults).
    fn get_active_model_settings(&self) -> (String, String) {
        if let Some(ref db_conn) = self.db_conn {
            if let Ok(conn) = db_conn.lock() {
                let settings = crate::db::get_all_settings(&conn).unwrap_or_default();
                let provider = settings
                    .get("selectedProvider")
                    .cloned()
                    .unwrap_or_else(|| "groq".to_string());
                let model = settings
                    .get("selectedModel")
                    .cloned()
                    .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());
                return (provider, model);
            }
        }
        ("groq".to_string(), "llama-3.3-70b-versatile".to_string())
    }

    /// Projects live runtime status into a bounded summary DTO.
    pub async fn get_runtime_status(&self, session_id: Option<String>) -> RuntimeStatusSummary {
        let autonomy_state = self.get_autonomy_state().await;
        let security_mode = self.policy_engine.get_security_mode().await;
        let policy_version = self.policy_engine.get_policy_version();
        let active_tasks = self.task_runtime.list_active_tasks().await;
        let active_execs = self
            .tool_router
            .cancellation()
            .active_execution_count()
            .await;
        let pending_approvals = self.policy_engine.approvals().list_pending().await;
        let (active_provider, active_model) = self.get_active_model_settings();
        let active_turn_ids = self.conversation_core.get_active_turn_ids().await;

        RuntimeStatusSummary {
            session_id,
            active_turn_id: active_turn_ids.first().cloned(),
            autonomy_state,
            security_mode,
            policy_version,
            active_task_count: active_tasks.len(),
            active_execution_count: active_execs,
            pending_approval_count: pending_approvals.len(),
            active_provider,
            active_model,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Projects all registered tools and active provider capabilities into a structured catalog.
    pub async fn get_capabilities(
        &self,
        domain_filter: Option<&str>,
    ) -> CapabilitiesSummary {
        let all_tools = self.tool_registry.list();
        let mut grouped: HashMap<String, Vec<ToolSummaryItem>> = HashMap::new();

        for tool in all_tools {
            let domain_str = tool.domain.as_str().to_string();
            if let Some(filter) = domain_filter {
                if !domain_str.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }

            grouped
                .entry(domain_str)
                .or_default()
                .push(ToolSummaryItem {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    requires_approval: !tool.is_read_only,
                });
        }

        let mut domains: Vec<DomainCapabilitySummary> = grouped
            .into_iter()
            .map(|(domain, tools)| DomainCapabilitySummary {
                domain,
                tool_count: tools.len(),
                tools,
            })
            .collect();
        domains.sort_by(|a, b| a.domain.cmp(&b.domain));

        let (active_provider, _) = self.get_active_model_settings();
        let provider_caps = {
            let reg = self.provider_registry.read().await;
            reg.get(&active_provider).map(|p| p.capabilities().clone())
        };

        let total_tools = domains.iter().map(|d| d.tool_count).sum();

        CapabilitiesSummary {
            total_tools,
            domains,
            active_provider,
            active_provider_capabilities: provider_caps,
        }
    }

    /// Projects active and recent tasks with sanitized strings.
    pub async fn list_active_tasks(&self, limit: usize) -> TasksSummary {
        let active = self.task_runtime.list_active_tasks().await;
        let all = self.task_runtime.list_all_tasks().await;

        let items: Vec<TaskSummaryItem> = active
            .into_iter()
            .take(limit)
            .map(|t| TaskSummaryItem {
                task_id: t.task_id,
                task_type: t.task_type.to_string(),
                owner: format!("{:?}", t.owner),
                goal: scrub_sensitive_text(&t.goal),
                status: t.status.to_string(),
                step: t.progress.step,
                max_steps: t.progress.max_steps,
                status_text: scrub_sensitive_text(&t.progress.status_text),
                created_at_ms: t.created_at_ms,
            })
            .collect();

        TasksSummary {
            total_active: items.len(),
            total_tasks: all.len(),
            active_tasks: items,
        }
    }

    /// Projects detailed snapshot of a single task with sanitized fields.
    pub async fn get_task_details(&self, task_id: &str) -> Option<TaskSnapshot> {
        let mut snapshot = self.task_runtime.get_task(&TaskId::from(task_id)).await?;
        snapshot.goal = scrub_sensitive_text(&snapshot.goal);
        snapshot.progress.status_text = scrub_sensitive_text(&snapshot.progress.status_text);
        if let Some(ref mut err) = snapshot.error {
            *err = scrub_sensitive_text(err);
        }
        if let Some(ref mut sum) = snapshot.result_summary {
            *sum = scrub_sensitive_text(sum);
        }
        Some(snapshot)
    }

    /// Projects registered AI providers (guaranteed sanitized, zero credentials).
    pub async fn list_providers(&self) -> Vec<ProviderSummary> {
        let reg = self.provider_registry.read().await;
        reg.list_providers()
    }

    /// Projects live browser domain state with sanitized URLs.
    pub async fn get_browser_status(&self) -> BrowserStatusSummary {
        if let Some(ref app) = self.app_handle {
            use tauri::Manager;
            if let Some(browser_state) = app.try_state::<BrowserState>() {
                let tabs_guard = browser_state.tabs.lock().unwrap();
                let active_id_guard = browser_state.active_tab_id.lock().unwrap();
                let is_visible = *browser_state.is_visible.lock().unwrap();

                let active_id = active_id_guard.clone();
                let tab_controls = GLOBAL_CONTROL_MGR.get_all_tab_controls();

                let mut tab_summaries = Vec::new();
                for tab in tabs_guard.iter() {
                    let is_active = active_id.as_deref() == Some(&tab.id);
                    let ctrl_state = tab_controls
                        .iter()
                        .find(|c| c.tab_id == tab.id)
                        .map(|c| format!("{:?}", c.control_state))
                        .unwrap_or_else(|| "USER_CONTROLLED".to_string());

                    tab_summaries.push(TabSummary {
                        tab_id: tab.id.clone(),
                        title: scrub_sensitive_text(&tab.title),
                        url: scrub_url(&tab.url),
                        is_active,
                        control_state: ctrl_state,
                    });
                }

                return BrowserStatusSummary {
                    open_tabs_count: tabs_guard.len(),
                    active_tab_id: active_id,
                    is_visible,
                    tabs: tab_summaries,
                };
            }
        }
        BrowserStatusSummary {
            open_tabs_count: 0,
            active_tab_id: None,
            is_visible: false,
            tabs: Vec::new(),
        }
    }

    /// Projects desktop computer control ownership and preemption state.
    pub fn get_computer_status(&self) -> ComputerStatusSummary {
        let info = GLOBAL_COMPUTER_CONTROL_MGR.get_control_info();
        let is_ai_controlled = info.control_state == ComputerControlState::AiControlled;
        ComputerStatusSummary {
            control_state: info.control_state,
            is_ai_controlled,
            last_transition_ms: info.last_transition,
            reason: info.reason.map(|r| scrub_sensitive_text(&r)),
        }
    }

    /// Projects host security policy state and pending approvals (no secrets or hashes).
    pub async fn get_security_status(&self) -> SecurityStatusSummary {
        let security_mode = self.policy_engine.get_security_mode().await;
        let policy_version = self.policy_engine.get_policy_version();
        let pending = self.policy_engine.approvals().list_pending().await;
        let pending_summaries: Vec<PendingApprovalSummary> = pending
            .into_iter()
            .map(|a| PendingApprovalSummary {
                approval_id: a.approval_id,
                domain: a.action_request.domain,
                operation: a.action_request.operation,
                risk_level: a.risk_level,
                reason: scrub_sensitive_text(&a.reason),
                expires_at_ms: a.expires_at_ms,
            })
            .collect();

        SecurityStatusSummary {
            policy_version,
            security_mode,
            pending_approval_count: pending_summaries.len(),
            pending_approvals: pending_summaries,
        }
    }

    /// Executes live diagnostic health checks across all integrated subsystems.
    pub async fn get_system_health(&self) -> SystemHealthSummary {
        let mut subsystems = HashMap::new();
        let mut overall_healthy = true;

        // 1. Database Health
        if let Some(ref db_conn) = self.db_conn {
            let start = Instant::now();
            match db_conn.lock() {
                Ok(conn) => match conn.execute("SELECT 1", []) {
                    Ok(_) => {
                        subsystems.insert(
                            "database".to_string(),
                            SubsystemHealth {
                                healthy: true,
                                latency_ms: Some(start.elapsed().as_millis() as u64),
                                message: "SQLite operational".to_string(),
                            },
                        );
                    }
                    Err(e) => {
                        overall_healthy = false;
                        subsystems.insert(
                            "database".to_string(),
                            SubsystemHealth {
                                healthy: false,
                                latency_ms: Some(start.elapsed().as_millis() as u64),
                                message: format!("Database query failed: {}", e),
                            },
                        );
                    }
                },
                Err(e) => {
                    overall_healthy = false;
                    subsystems.insert(
                        "database".to_string(),
                        SubsystemHealth {
                            healthy: false,
                            latency_ms: None,
                            message: format!("Database lock poisoned: {}", e),
                        },
                    );
                }
            }
        } else {
            subsystems.insert(
                "database".to_string(),
                SubsystemHealth {
                    healthy: true,
                    latency_ms: None,
                    message: "In-memory test environment (no DB)".to_string(),
                },
            );
        }

        // 2. Task Runtime Health
        let tasks = self.task_runtime.list_active_tasks().await;
        subsystems.insert(
            "task_runtime".to_string(),
            SubsystemHealth {
                healthy: true,
                latency_ms: None,
                message: format!("{} active task(s)", tasks.len()),
            },
        );

        // 3. Tool Runtime Health
        let tool_count = self.tool_registry.count();
        subsystems.insert(
            "tool_runtime".to_string(),
            SubsystemHealth {
                healthy: tool_count > 0,
                latency_ms: None,
                message: format!("{} registered tool(s)", tool_count),
            },
        );

        // 4. Provider Registry Health
        let provider_count = {
            let reg = self.provider_registry.read().await;
            reg.list_providers().len()
        };
        subsystems.insert(
            "providers".to_string(),
            SubsystemHealth {
                healthy: provider_count > 0,
                latency_ms: None,
                message: format!("{} registered AI provider(s)", provider_count),
            },
        );

        // 5. Policy Engine Health
        let policy_v = self.policy_engine.get_policy_version();
        subsystems.insert(
            "policy_engine".to_string(),
            SubsystemHealth {
                healthy: policy_v > 0,
                latency_ms: None,
                message: format!("Host Policy v{} active", policy_v),
            },
        );

        // 6. Computer Control Health
        let comp_info = GLOBAL_COMPUTER_CONTROL_MGR.get_control_info();
        subsystems.insert(
            "computer_control".to_string(),
            SubsystemHealth {
                healthy: true,
                latency_ms: None,
                message: format!("Desktop state: {:?}", comp_info.control_state),
            },
        );

        // 7. Browser Domain Health
        let browser_msg = if let Some(ref app) = self.app_handle {
            use tauri::Manager;
            if let Some(bs) = app.try_state::<BrowserState>() {
                let tab_count = bs.tabs.lock().map(|t| t.len()).unwrap_or(0);
                format!("Browser active ({} tab(s))", tab_count)
            } else {
                "Browser state unmanaged".to_string()
            }
        } else {
            "Browser headless / mock".to_string()
        };
        subsystems.insert(
            "browser".to_string(),
            SubsystemHealth {
                healthy: true,
                latency_ms: None,
                message: browser_msg,
            },
        );

        SystemHealthSummary {
            overall_healthy,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            subsystems,
        }
    }
}
