use super::cancellation::CancellationRegistry;
use super::executor::DomainExecutorRegistry;
use super::registry::ToolRegistry;
use super::types::{
    ToolDefinition, ToolDomain, ToolExecutionError, ToolExecutionResult, ToolRequest,
};
use super::validator::ArgumentValidator;
use crate::events::emitter::EventEmitter;
use crate::events::envelope::EventCorrelation;
use crate::events::payload::{EdithPayload, ToolPayload};
use crate::policy::context::{ActionSource, PolicyContext, SecurityMode};
use crate::policy::engine::PolicyEngine;
use crate::policy::types::{ActionRequest, ActionTarget, PolicyOutcome};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Central router coordinating tool validation, policy authorization, cancellation, and execution dispatch.
#[derive(Clone)]
pub struct ToolRouter {
    registry: Arc<ToolRegistry>,
    executors: Arc<DomainExecutorRegistry>,
    policy_engine: Arc<PolicyEngine>,
    emitter: Option<EventEmitter>,
    cancellation: Arc<CancellationRegistry>,
}

impl ToolRouter {
    pub fn new(
        registry: Arc<ToolRegistry>,
        executors: Arc<DomainExecutorRegistry>,
        policy_engine: Arc<PolicyEngine>,
        emitter: Option<EventEmitter>,
        cancellation: Arc<CancellationRegistry>,
    ) -> Self {
        Self {
            registry,
            executors,
            policy_engine,
            emitter,
            cancellation,
        }
    }

    pub fn with_defaults(
        registry: Arc<ToolRegistry>,
        executors: Arc<DomainExecutorRegistry>,
        policy_engine: Arc<PolicyEngine>,
        emitter: Option<EventEmitter>,
    ) -> Self {
        Self::new(
            registry,
            executors,
            policy_engine,
            emitter,
            Arc::new(CancellationRegistry::new()),
        )
    }

    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    pub fn executors(&self) -> &Arc<DomainExecutorRegistry> {
        &self.executors
    }

    pub fn policy_engine(&self) -> &Arc<PolicyEngine> {
        &self.policy_engine
    }

    pub fn cancellation(&self) -> &Arc<CancellationRegistry> {
        &self.cancellation
    }

    /// Cancels a running tool execution by ID.
    pub async fn cancel_execution(&self, execution_id: &str) -> bool {
        self.cancellation.cancel_execution(execution_id).await
    }

    /// Emits a correlated tool payload if event emitter is configured.
    fn emit_tool_event(&self, correlation: EventCorrelation, payload: ToolPayload) {
        if let Some(ref emitter) = self.emitter {
            let mut corr = correlation;
            // Set tool_execution_id on correlation if not set
            if corr.tool_execution_id.is_none() {
                match &payload {
                    ToolPayload::Proposed { execution_id, .. }
                    | ToolPayload::Started { execution_id, .. }
                    | ToolPayload::Completed { execution_id, .. }
                    | ToolPayload::Failed { execution_id, .. }
                    | ToolPayload::ApprovalRequired { execution_id, .. }
                    | ToolPayload::Approved { execution_id, .. }
                    | ToolPayload::Denied { execution_id, .. }
                    | ToolPayload::Progress { execution_id, .. }
                    | ToolPayload::Cancelled { execution_id, .. } => {
                        corr.tool_execution_id = Some(execution_id.clone());
                    }
                }
            }
            let _ = emitter.emit_payload(corr, EdithPayload::Tool(payload));
        }
    }

    /// Resolves target resource from tool definition and JSON arguments for policy evaluation.
    fn resolve_action_target(def: &ToolDefinition, args: &serde_json::Value) -> ActionTarget {
        match def.domain {
            ToolDomain::Browser => {
                if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                    ActionTarget::Url(url.to_string())
                } else if args.get("element_id").is_some() || args.get("selector").is_some() {
                    ActionTarget::BrowserElement {
                        selector: args.get("selector").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        element_id: args.get("element_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        text: args.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    }
                } else {
                    ActionTarget::None
                }
            }
            ToolDomain::Filesystem => {
                if let Some(path_str) = args.get("path").and_then(|v| v.as_str()) {
                    ActionTarget::Path(PathBuf::from(path_str))
                } else {
                    ActionTarget::None
                }
            }
            ToolDomain::System | ToolDomain::Application => {
                if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                    ActionTarget::Command {
                        program: cmd.to_string(),
                        args: Vec::new(),
                        working_dir: None,
                    }
                } else if let Some(prog) = args.get("program").and_then(|v| v.as_str()) {
                    let parsed_args = args
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    ActionTarget::Command {
                        program: prog.to_string(),
                        args: parsed_args,
                        working_dir: None,
                    }
                } else {
                    ActionTarget::None
                }
            }
            _ => ActionTarget::None,
        }
    }

    /// Primary entry point: routes and executes a tool request through policy authorization.
    pub async fn execute(&self, request: ToolRequest) -> ToolExecutionResult {
        let start = Instant::now();
        let execution_id = request.execution_id.clone();
        let tool_name = request.tool_name.clone();

        // 1. Resolve ToolDefinition from Registry
        let tool_def = match self.registry.get(&tool_name) {
            Some(d) => d,
            None => {
                let duration = start.elapsed().as_millis() as u64;
                let err_msg = format!("Tool '{}' not registered in Universal Tool Registry.", tool_name);
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Failed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        error: err_msg.clone(),
                    },
                );
                return ToolExecutionResult::failed(
                    execution_id,
                    tool_name,
                    err_msg,
                    "TOOL_NOT_FOUND".to_string(),
                    duration,
                );
            }
        };

        // 2. Validate arguments against JSON Schema
        if let Err(val_err) = ArgumentValidator::validate(&request.arguments, &tool_def.parameters_schema) {
            let duration = start.elapsed().as_millis() as u64;
            self.emit_tool_event(
                request.correlation.clone(),
                ToolPayload::Failed {
                    execution_id: execution_id.to_string(),
                    tool_name: tool_name.clone(),
                    error: val_err.to_string(),
                },
            );
            return ToolExecutionResult::failed(
                execution_id,
                tool_name,
                val_err.to_string(),
                "INVALID_ARGUMENTS".to_string(),
                duration,
            );
        }

        // 3. Register and bind scoped cancellation
        let cancel_token = self
            .cancellation
            .register_execution(
                execution_id.as_str(),
                request.correlation.turn_id.as_deref(),
                request.correlation.task_id.as_deref(),
                request.correlation.conversation_id.as_deref(),
            )
            .await;

        if cancel_token.is_cancelled() {
            let duration = start.elapsed().as_millis() as u64;
            self.cancellation.cleanup_execution(execution_id.as_str()).await;
            self.emit_tool_event(
                request.correlation.clone(),
                ToolPayload::Cancelled {
                    execution_id: execution_id.to_string(),
                    tool_name: tool_name.clone(),
                    reason: Some("Cancelled by parent scope prior to authorization.".to_string()),
                },
            );
            return ToolExecutionResult::cancelled(
                execution_id,
                tool_name,
                Some("Cancelled by parent scope prior to authorization.".to_string()),
                duration,
            );
        }

        // 4. Formulate ActionRequest & PolicyContext for Host-Enforced PolicyEngine
        let target = Self::resolve_action_target(&tool_def, &request.arguments);
        let domain_str = tool_def.domain.as_str();
        let operation = if let Some(suffix) = tool_def.name.strip_prefix(&format!("{}.", domain_str)) {
            suffix.to_string()
        } else {
            tool_def.name.clone()
        };
        let action_req = ActionRequest::new(
            domain_str,
            operation,
            target,
            request.arguments.clone(),
            request.correlation.clone(),
        );

        let policy_ctx = PolicyContext {
            session_id: request.correlation.conversation_id.clone(),
            conversation_id: request.correlation.conversation_id.clone(),
            turn_id: request.correlation.turn_id.clone(),
            task_id: request.correlation.task_id.clone(),
            source: ActionSource::AiAutonomous,
            security_mode: SecurityMode::Standard,
            workspace_roots: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            active_approval_id: request.active_approval_id.clone(),
        };

        // 5. Host Security Boundary Evaluation
        let decision = self.policy_engine.evaluate(&action_req, &policy_ctx).await;

        match decision.outcome {
            PolicyOutcome::Blocked => {
                let duration = start.elapsed().as_millis() as u64;
                self.cancellation.cleanup_execution(execution_id.as_str()).await;
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Failed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        error: format!("POLICY_BLOCKED: {}", decision.reason),
                    },
                );
                return ToolExecutionResult::blocked(
                    execution_id,
                    tool_name,
                    decision.reason,
                    decision.policy_code,
                    duration,
                );
            }
            PolicyOutcome::ConfirmationRequired => {
                let duration = start.elapsed().as_millis() as u64;
                self.cancellation.cleanup_execution(execution_id.as_str()).await;
                let approval_id = decision.approval_id.unwrap_or_default();
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::ApprovalRequired {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        approval_id: approval_id.clone(),
                        reason: decision.reason.clone(),
                    },
                );
                return ToolExecutionResult::approval_required(
                    execution_id,
                    tool_name,
                    approval_id,
                    decision.reason,
                    duration,
                );
            }
            PolicyOutcome::Restricted => {
                // Enforce read-only constraint if specified
                if let Some(ref c) = decision.constraints {
                    if c.read_only && !tool_def.is_read_only {
                        let duration = start.elapsed().as_millis() as u64;
                        self.cancellation.cleanup_execution(execution_id.as_str()).await;
                        self.emit_tool_event(
                            request.correlation.clone(),
                            ToolPayload::Failed {
                                execution_id: execution_id.to_string(),
                                tool_name: tool_name.clone(),
                                error: "Restricted read-only policy constraint prevents mutating action.".to_string(),
                            },
                        );
                        return ToolExecutionResult::blocked(
                            execution_id,
                            tool_name,
                            "Restricted read-only policy constraint prevents mutating action.".to_string(),
                            "RESTRICTED_VIOLATION".to_string(),
                            duration,
                        );
                    }
                }
            }
            PolicyOutcome::Allow => {
                // Fully authorized (either inherently or pre-approved)
            }
        }

        // 6. Emit Started Event
        self.emit_tool_event(
            request.correlation.clone(),
            ToolPayload::Started {
                execution_id: execution_id.to_string(),
                tool_name: tool_name.clone(),
            },
        );

        // 7. Resolve Domain Executor
        let executor = match self.executors.get(&tool_def.domain) {
            Some(e) => e,
            None => {
                let duration = start.elapsed().as_millis() as u64;
                self.cancellation.cleanup_execution(execution_id.as_str()).await;
                let err_msg = format!("No executor registered for domain '{}'.", tool_def.domain);
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Failed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        error: err_msg.clone(),
                    },
                );
                return ToolExecutionResult::failed(
                    execution_id,
                    tool_name,
                    err_msg,
                    "EXECUTOR_UNAVAILABLE".to_string(),
                    duration,
                );
            }
        };

        // 8. Bounded Execution with Timeout & Cancellation Race
        let timeout_ms = request.timeout_ms.unwrap_or(tool_def.default_timeout_ms);
        let exec_fut = executor.execute(&request, &tool_def, cancel_token.clone());
        let timeout_dur = std::time::Duration::from_millis(timeout_ms);

        let exec_result = tokio::select! {
            _ = cancel_token.cancelled() => {
                Err(ToolExecutionError::Cancelled("Execution cancelled during run.".to_string()))
            }
            _ = tokio::time::sleep(timeout_dur) => {
                Err(ToolExecutionError::Timeout(format!("Execution timed out after {}ms.", timeout_ms)))
            }
            res = exec_fut => res,
        };

        // 9. Cleanup Cancellation Token
        self.cancellation.cleanup_execution(execution_id.as_str()).await;
        let duration = start.elapsed().as_millis() as u64;

        // 10. Process Result & Emit Terminal Events
        match exec_result {
            Ok(output) => {
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Completed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        success: true,
                        duration_ms: duration,
                        result_summary: None,
                    },
                );
                ToolExecutionResult::success(execution_id, tool_name, output, duration)
            }
            Err(ToolExecutionError::Cancelled(reason)) => {
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Cancelled {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        reason: Some(reason.clone()),
                    },
                );
                ToolExecutionResult::cancelled(execution_id, tool_name, Some(reason), duration)
            }
            Err(ToolExecutionError::Timeout(msg)) => {
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Failed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        error: msg,
                    },
                );
                ToolExecutionResult::timeout(execution_id, tool_name, duration)
            }
            Err(err) => {
                self.emit_tool_event(
                    request.correlation.clone(),
                    ToolPayload::Failed {
                        execution_id: execution_id.to_string(),
                        tool_name: tool_name.clone(),
                        error: err.to_string(),
                    },
                );
                ToolExecutionResult::failed(
                    execution_id,
                    tool_name,
                    err.to_string(),
                    err.error_code().to_string(),
                    duration,
                )
            }
        }
    }
}
