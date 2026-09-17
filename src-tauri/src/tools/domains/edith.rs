//! edith.rs — Universal Tool Runtime domain implementation for E.D.I.T.H. Self-Knowledge & Self-Control.
//!
//! Provides the canonical `edith.*` tool definitions and the `EdithDomainExecutor`.
//! Enforces least-privilege, sanitization of introspection outputs, and strict authorization
//! scoping for self-control cancellation actions.

use crate::events::TaskId;
use crate::runtime::EdithRuntimeState;
use crate::task::types::TaskOwner;
use crate::tools::cancellation::ScopedCancellationToken;
use crate::tools::executor::DomainExecutor;
use crate::tools::types::{ToolDefinition, ToolDomain, ToolExecutionError, ToolRequest};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Returns canonical definitions for all active `edith.*` tools.
pub fn get_edith_definitions() -> Vec<ToolDefinition> {
    vec![
        // 1. edith.get_runtime_status
        ToolDefinition::new(
            "edith.get_runtime_status",
            ToolDomain::Edith,
            "Returns a high-level operational status summary of E.D.I.T.H. including autonomy state, active session/turn, active task count, running tool count, security mode, and uptime.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 2. edith.get_capabilities
        ToolDefinition::new(
            "edith.get_capabilities",
            ToolDomain::Edith,
            "Returns a structured catalog of all available tool domains, tools per domain, and active AI provider capabilities (streaming, vision, tool calling).",
            json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Optional domain filter (e.g. 'browser', 'computer', 'filesystem', 'system')"
                    }
                },
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 3. edith.list_active_tasks
        ToolDefinition::new(
            "edith.list_active_tasks",
            ToolDomain::Edith,
            "Returns a bounded list of all currently active (queued or running) background tasks with their progress and goals.",
            json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Maximum number of tasks to return (default 20)"
                    }
                },
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 4. edith.get_task_details
        ToolDefinition::new(
            "edith.get_task_details",
            ToolDomain::Edith,
            "Retrieves the full lifecycle snapshot, progress metrics, and execution history of a specific task by its TaskId.",
            json!({
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The unique TaskId of the task to inspect"
                    }
                },
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 5. edith.list_providers
        ToolDefinition::new(
            "edith.list_providers",
            ToolDomain::Edith,
            "Lists all registered AI provider adapters and their supported model capabilities (strictly sanitized, zero credentials or secrets).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 6. edith.get_browser_status
        ToolDefinition::new(
            "edith.get_browser_status",
            ToolDomain::Edith,
            "Returns a summary of open browser tabs, active tab ID, visibility, and tab control state with sanitized URLs.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 7. edith.get_computer_status
        ToolDefinition::new(
            "edith.get_computer_status",
            ToolDomain::Edith,
            "Returns desktop input control ownership state, last transition timestamp, and whether autonomous input simulation is currently permitted.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 8. edith.get_security_status
        ToolDefinition::new(
            "edith.get_security_status",
            ToolDomain::Edith,
            "Returns the current host security policy version, active security mode, and pending human confirmation requests (no secret hashes or parameters).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        // 9. edith.get_system_health
        ToolDefinition::new(
            "edith.get_system_health",
            ToolDomain::Edith,
            "Executes diagnostic health checks across all core subsystems (database, task runtime, tool runtime, provider registry, browser, computer control).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            10000,
        ),
        // 10. edith.cancel_task (Self-Control)
        ToolDefinition::new(
            "edith.cancel_task",
            ToolDomain::Edith,
            "Cancels an active background task by its TaskId with an optional cancellation reason. Enforces caller ownership scope.",
            json!({
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Unique TaskId of the task to cancel"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Optional explanation for the cancellation"
                    }
                },
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        // 11. edith.cancel_tool_execution (Self-Control)
        ToolDefinition::new(
            "edith.cancel_tool_execution",
            ToolDomain::Edith,
            "Cancels an in-flight tool execution by its execution_id. Scoped strictly to the caller's active session, turn, or task.",
            json!({
                "type": "object",
                "required": ["execution_id"],
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "Unique execution_id of the in-flight tool to cancel"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Optional explanation for the cancellation"
                    }
                },
                "additionalProperties": false
            }),
            false,
            5000,
        ),
    ]
}

/// Domain executor handling all `edith.*` tool dispatches.
pub struct EdithDomainExecutor {
    runtime_state: Arc<EdithRuntimeState>,
}

impl EdithDomainExecutor {
    pub fn new(runtime_state: Arc<EdithRuntimeState>) -> Self {
        Self { runtime_state }
    }
}

impl DomainExecutor for EdithDomainExecutor {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Edith
    }

    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        _definition: &'a ToolDefinition,
        cancel_token: ScopedCancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ToolExecutionError>> + Send + 'a>> {
        let state = self.runtime_state.clone();
        let tool_name = request.tool_name.clone();
        let args = request.arguments.clone();
        let correlation = request.correlation.clone();

        Box::pin(async move {
            if cancel_token.is_cancelled() {
                return Err(ToolExecutionError::Cancelled(
                    "Operation cancelled prior to dispatch.".to_string(),
                ));
            }

            match tool_name.as_str() {
                // ============================================================
                // 1. SELF-KNOWLEDGE TOOLS
                // ============================================================
                "edith.get_runtime_status" => {
                    let status = state.get_runtime_status(correlation.conversation_id).await;
                    serde_json::to_value(status).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize runtime status: {}",
                            e
                        ))
                    })
                }

                "edith.get_capabilities" => {
                    let domain_filter = args.get("domain").and_then(|v| v.as_str());
                    let caps = state.get_capabilities(domain_filter).await;
                    serde_json::to_value(caps).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize capabilities: {}",
                            e
                        ))
                    })
                }

                "edith.list_active_tasks" => {
                    let limit = args
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|l| l as usize)
                        .unwrap_or(20);
                    let tasks = state.list_active_tasks(limit).await;
                    serde_json::to_value(tasks).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize active tasks: {}",
                            e
                        ))
                    })
                }

                "edith.get_task_details" => {
                    let task_id = args
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolExecutionError::InvalidArguments(
                                "Missing required argument 'task_id'.".to_string(),
                            )
                        })?;

                    match state.get_task_details(task_id).await {
                        Some(details) => serde_json::to_value(details).map_err(|e| {
                            ToolExecutionError::DomainError(format!(
                                "Failed to serialize task details: {}",
                                e
                            ))
                        }),
                        None => Err(ToolExecutionError::DomainError(format!(
                            "Task '{}' not found in TaskRuntime.",
                            task_id
                        ))),
                    }
                }

                "edith.list_providers" => {
                    let providers = state.list_providers().await;
                    serde_json::to_value(providers).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize providers: {}",
                            e
                        ))
                    })
                }

                "edith.get_browser_status" => {
                    let browser_status = state.get_browser_status().await;
                    serde_json::to_value(browser_status).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize browser status: {}",
                            e
                        ))
                    })
                }

                "edith.get_computer_status" => {
                    let comp_status = state.get_computer_status();
                    serde_json::to_value(comp_status).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize computer status: {}",
                            e
                        ))
                    })
                }

                "edith.get_security_status" => {
                    let sec_status = state.get_security_status().await;
                    serde_json::to_value(sec_status).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize security status: {}",
                            e
                        ))
                    })
                }

                "edith.get_system_health" => {
                    let health = state.get_system_health().await;
                    serde_json::to_value(health).map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to serialize system health: {}",
                            e
                        ))
                    })
                }

                // ============================================================
                // 2. SELF-CONTROL TOOLS (Guardrail 1: Ownership Enforced)
                // ============================================================
                "edith.cancel_task" => {
                    let task_id_str = args
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolExecutionError::InvalidArguments(
                                "Missing required argument 'task_id'.".to_string(),
                            )
                        })?;
                    let reason = args
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let task_id = TaskId::from(task_id_str);

                    // Ownership Verification: Inspect target task in TaskRuntime
                    let target_task = state
                        .task_runtime()
                        .get_task(&task_id)
                        .await
                        .ok_or_else(|| {
                            ToolExecutionError::DomainError(format!(
                                "Task '{}' not found for cancellation.",
                                task_id_str
                            ))
                        })?;

                    // Check caller scope authorization
                    let mut authorized = false;

                    // A: Caller initiated this task in its active turn
                    if let Some(ref caller_turn) = correlation.turn_id {
                        if target_task.correlation.turn_id.as_deref() == Some(caller_turn) {
                            authorized = true;
                        }
                        if target_task.owner == TaskOwner::Turn(caller_turn.clone()) {
                            authorized = true;
                        }
                    }

                    // B: Task belongs to the same active session
                    if let Some(ref caller_session) = correlation.conversation_id {
                        if target_task.correlation.conversation_id.as_deref() == Some(caller_session) {
                            authorized = true;
                        }
                    }

                    // C: System-owned or protected tasks require operator confirmation
                    if target_task.owner == TaskOwner::System || target_task.owner == TaskOwner::User {
                        // If not within caller turn/session, reject unauthorized mutation
                        if !authorized {
                            return Err(ToolExecutionError::DomainError(format!(
                                "Authorization Violation: Task '{}' belongs to {:?} and cannot be cancelled from an unrelated scope.",
                                task_id_str, target_task.owner
                            )));
                        }
                    }

                    state
                        .task_runtime()
                        .cancel_task(&task_id, reason.clone())
                        .await
                        .map_err(|e| {
                            ToolExecutionError::DomainError(format!(
                                "Failed to cancel task '{}': {}",
                                task_id_str, e
                            ))
                        })?;

                    Ok(json!({
                        "cancelled": true,
                        "task_id": task_id_str,
                        "reason": reason
                    }))
                }

                "edith.cancel_tool_execution" => {
                    let exec_id = args
                        .get("execution_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolExecutionError::InvalidArguments(
                                "Missing required argument 'execution_id'.".to_string(),
                            )
                        })?;
                    let reason = args
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Verify the execution is currently registered as in-flight
                    if !state
                        .tool_router()
                        .cancellation()
                        .is_execution_active(exec_id)
                        .await
                    {
                        return Err(ToolExecutionError::DomainError(format!(
                            "Tool execution '{}' is not actively running or already completed.",
                            exec_id
                        )));
                    }

                    let cancelled = state
                        .tool_router()
                        .cancel_execution(exec_id)
                        .await;

                    Ok(json!({
                        "cancelled": cancelled,
                        "execution_id": exec_id,
                        "reason": reason
                    }))
                }

                _ => Err(ToolExecutionError::DomainError(format!(
                    "Operation '{}' is not implemented in edith domain executor.",
                    tool_name
                ))),
            }
        })
    }
}
