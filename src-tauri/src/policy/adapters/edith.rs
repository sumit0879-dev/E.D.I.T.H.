//! edith.rs — Host Security Policy Adapter for E.D.I.T.H. Self-Knowledge & Self-Control domain.
//!
//! Evaluates incoming action requests for the `edith.*` namespace.
//! Guarantees that:
//! 1. Introspection and self-knowledge queries are read-only and low-risk.
//! 2. Self-control actions (task/tool cancellation) require confirmation in Strict/Restricted modes.
//! 3. Any self-escalation, policy modification, or secret revelation attempts are strictly blocked.

use crate::policy::context::{PolicyContext, SecurityMode};
use crate::policy::decision::PolicyDecision;
use crate::policy::types::{ActionRequest, RiskLevel};

pub struct EdithAdapter;

impl EdithAdapter {
    /// Evaluates an `edith.*` action request against host security policies.
    pub fn evaluate(
        req: &ActionRequest,
        ctx: &PolicyContext,
        current_policy_version: u32,
    ) -> PolicyDecision {
        let op = req.operation.to_lowercase();

        match op.as_str() {
            // ================================================================
            // 1. SELF-KNOWLEDGE (Passive observation: Low Risk)
            // ================================================================
            "get_runtime_status"
            | "get_capabilities"
            | "list_active_tasks"
            | "get_task_details"
            | "list_providers"
            | "get_browser_status"
            | "get_computer_status"
            | "get_security_status"
            | "get_system_health" => {
                PolicyDecision::allow(
                    RiskLevel::Low,
                    "EDITH_READ_ALLOWED",
                    format!("Read-only inspection tool '{}' permitted.", req.operation),
                    current_policy_version,
                )
            }

            // ================================================================
            // 2. SELF-CONTROL (Task & tool cancellation: Medium Risk)
            // ================================================================
            "cancel_task" | "cancel_tool_execution" => {
                match ctx.security_mode {
                    SecurityMode::Strict | SecurityMode::Restricted => {
                        PolicyDecision::confirmation_required(
                            RiskLevel::Medium,
                            "CONFIRMATION_REQUIRED",
                            format!(
                                "Operator confirmation required to execute '{}' in {:?} mode.",
                                req.operation, ctx.security_mode
                            ),
                            current_policy_version,
                        )
                    }
                    _ => {
                        // In Standard/Developer/Autonomous modes, self-cancellation is allowed
                        // (ownership verification is checked during execution).
                        PolicyDecision::allow(
                            RiskLevel::Medium,
                            "EDITH_SELF_CONTROL_ALLOWED",
                            format!("Self-control operation '{}' permitted.", req.operation),
                            current_policy_version,
                        )
                    }
                }
            }

            // ================================================================
            // 3. DEFERRED ACTIONS (Guardrail 2: Fail Closed)
            // ================================================================
            "pause_task" | "resume_task" => PolicyDecision::blocked(
                RiskLevel::Medium,
                "TASK_PAUSE_NOT_SUPPORTED",
                "Task pause/resume is deferred until TaskRuntime introduces cooperative suspension."
                    .to_string(),
                current_policy_version,
            ),

            // ================================================================
            // 4. ANTI-ESCALATION (Privilege modification: Blocked Critical)
            // ================================================================
            "modify_policy"
            | "grant_permission"
            | "set_security_mode"
            | "reveal_secrets"
            | "bypass_approval"
            | "modify_executable" => PolicyDecision::blocked(
                RiskLevel::Critical,
                "UNAUTHORIZED_SELF_ESCALATION",
                format!(
                    "Self-escalation or policy modification via '{}' is strictly prohibited.",
                    req.operation
                ),
                current_policy_version,
            ),

            // ================================================================
            // 5. UNKNOWN EDITH OPERATION
            // ================================================================
            _ => PolicyDecision::blocked(
                RiskLevel::High,
                "UNKNOWN_EDITH_OPERATION",
                format!("Unknown operation '{}' in edith domain.", req.operation),
                current_policy_version,
            ),
        }
    }
}
