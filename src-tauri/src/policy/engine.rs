use super::adapters::{BrowserAdapter, CommandAdapter, ComputerAdapter, EdithAdapter};
use super::approval::{ApprovalRequest, ApprovalStore, OperatorDecision};
use super::audit::{AuditRecord, SecurityAuditTrail};
use super::context::{PolicyContext, SecurityMode};
use super::decision::PolicyDecision;
use super::types::{ActionRequest, PolicyConstraints, PolicyOutcome, RiskLevel};
use crate::events::emitter::EventEmitter;
use crate::events::envelope::EventCorrelation;
use crate::events::payload::{EdithPayload, SecurityPolicyPayload};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The central host-enforced Security Policy and Permission Engine.
/// Guarantees that AI models are never the final decision authority for action execution.
#[derive(Clone)]
pub struct PolicyEngine {
    constraints: Arc<RwLock<PolicyConstraints>>,
    approvals: Arc<ApprovalStore>,
    audit_trail: Arc<SecurityAuditTrail>,
    emitter: Option<EventEmitter>,
    policy_version: Arc<AtomicU32>,
    security_mode: Arc<RwLock<SecurityMode>>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PolicyEngine {
    /// Creates a new PolicyEngine with optional correlated event emitter.
    pub fn new(emitter: Option<EventEmitter>) -> Self {
        Self::with_constraints(emitter, PolicyConstraints::default())
    }

    /// Creates a new PolicyEngine with specific initial policy constraints.
    pub fn with_constraints(emitter: Option<EventEmitter>, constraints: PolicyConstraints) -> Self {
        Self {
            constraints: Arc::new(RwLock::new(constraints)),
            approvals: Arc::new(ApprovalStore::default()),
            audit_trail: Arc::new(SecurityAuditTrail::default()),
            emitter,
            policy_version: Arc::new(AtomicU32::new(1)),
            security_mode: Arc::new(RwLock::new(SecurityMode::Standard)),
        }
    }

    /// Returns the active security mode.
    pub async fn get_security_mode(&self) -> SecurityMode {
        *self.security_mode.read().await
    }

    /// Updates the active security mode and increments the policy version.
    pub async fn set_security_mode(&self, mode: SecurityMode) {
        let mut guard = self.security_mode.write().await;
        *guard = mode;
        self.policy_version.fetch_add(1, Ordering::SeqCst);
    }

    /// Returns the current policy configuration version.
    pub fn get_policy_version(&self) -> u32 {
        self.policy_version.load(Ordering::SeqCst)
    }

    /// Returns a copy of the active policy constraints.
    pub async fn get_constraints(&self) -> PolicyConstraints {
        self.constraints.read().await.clone()
    }

    /// Updates policy constraints and atomically increments the policy version.
    /// This immediately invalidates previously issued pending approvals under older policies.
    pub async fn update_constraints(&self, new_constraints: PolicyConstraints) {
        let mut guard = self.constraints.write().await;
        *guard = new_constraints;
        self.policy_version.fetch_add(1, Ordering::SeqCst);
    }

    /// Returns access to the internal approval store.
    pub fn approvals(&self) -> &Arc<ApprovalStore> {
        &self.approvals
    }

    /// Returns access to the internal audit trail.
    pub fn audit_trail(&self) -> &Arc<SecurityAuditTrail> {
        &self.audit_trail
    }

    /// Evaluates an incoming action request in host context against active policies.
    /// Strictly host-enforced: produces a deterministic decision (Allow, ConfirmationRequired, Restricted, Blocked).
    pub async fn evaluate(&self, req: &ActionRequest, ctx: &PolicyContext) -> PolicyDecision {
        let current_policy_version = self.get_policy_version();
        let constraints = self.get_constraints().await;

        // 1. Path of Pre-Authorized Execution: Active Approval ID Verification
        if let Some(ref approval_id) = ctx.active_approval_id {
            if let Some(approval) = self.approvals.get_approval(approval_id).await {
                match approval.validate_authorization(req, current_policy_version) {
                    Ok(()) => {
                        // Strictly consume approval to prevent replay attacks
                        if let Err(consume_err) = self.approvals.consume_approval(approval_id).await
                        {
                            return PolicyDecision::blocked(
                                RiskLevel::High,
                                "APPROVAL_CONSUMPTION_FAILURE",
                                format!("Approval consumption failure: {}", consume_err),
                                current_policy_version,
                            );
                        }

                        // Record authorized audit entry
                        let record = AuditRecord::new(
                            ctx.session_id.clone(),
                            ctx.turn_id.clone(),
                            ctx.task_id.clone(),
                            req,
                            approval.risk_level,
                            PolicyOutcome::Allow,
                            "Action executed under validated one-time human authorization.",
                            Some(approval_id.clone()),
                            current_policy_version,
                        );
                        self.audit_trail.record(record).await;

                        return PolicyDecision::allow(
                            approval.risk_level,
                            "PRE_AUTHORIZED",
                            "Authorized via confirmed operator approval.",
                            current_policy_version,
                        );
                    }
                    Err(err) => {
                        // Record rejected authorization audit entry
                        let record = AuditRecord::new(
                            ctx.session_id.clone(),
                            ctx.turn_id.clone(),
                            ctx.task_id.clone(),
                            req,
                            RiskLevel::High,
                            PolicyOutcome::Blocked,
                            format!("Replay or invalid approval token: {}", err),
                            Some(approval_id.clone()),
                            current_policy_version,
                        );
                        self.audit_trail.record(record).await;

                        return PolicyDecision::blocked(
                            RiskLevel::High,
                            "INVALID_AUTHORIZATION",
                            format!("Invalid authorization token: {}", err),
                            current_policy_version,
                        );
                    }
                }
            } else {
                return PolicyDecision::blocked(
                    RiskLevel::High,
                    "APPROVAL_NOT_FOUND",
                    format!("Approval ID '{}' not found or expired.", approval_id),
                    current_policy_version,
                );
            }
        }

        // 2. Domain-Specific Policy Assessment
        let domain_lower = req.domain.trim().to_lowercase();
        let (risk_level, outcome, policy_code, reason) = if domain_lower == "system"
            || domain_lower == "command"
            || domain_lower == "terminal"
        {
            let (r, o, msg) = CommandAdapter::evaluate(req, ctx, &constraints);
            (r, o, "POLICY_ALLOW".to_string(), msg)
        } else if domain_lower == "browser" {
            let (r, o, msg) = BrowserAdapter::evaluate(req, ctx, &constraints);
            (r, o, "POLICY_ALLOW".to_string(), msg)
        } else if domain_lower == "computer" {
            let (r, o, msg) = ComputerAdapter::evaluate(req, ctx, &constraints);
            (r, o, "POLICY_ALLOW".to_string(), msg)
        } else if domain_lower == "edith" {
            let decision = EdithAdapter::evaluate(req, ctx, current_policy_version);
            (
                decision.risk_level,
                decision.outcome,
                decision.policy_code,
                decision.reason,
            )
        } else {
            // General external actions evaluation
            if !constraints.allow_external_services {
                (
                    RiskLevel::High,
                    PolicyOutcome::Blocked,
                    "EXTERNAL_SERVICES_DISABLED".to_string(),
                    "External services are disabled in policy configuration.".to_string(),
                )
            } else {
                (
                    RiskLevel::Medium,
                    PolicyOutcome::Allow,
                    "POLICY_ALLOW".to_string(),
                    "Standard operation evaluated under general policy constraints.".to_string(),
                )
            }
        };

        // 3. Construct Final Decision & Process Approval Requirement
        let (final_decision, approval_id) = match outcome {
            PolicyOutcome::Allow => (
                PolicyDecision::allow(
                    risk_level,
                    if policy_code.is_empty() || policy_code == "POLICY_BLOCKED" {
                        "POLICY_ALLOW"
                    } else {
                        &policy_code
                    },
                    reason.clone(),
                    current_policy_version,
                ),
                None,
            ),
            PolicyOutcome::ConfirmationRequired => {
                let approval = self
                    .approvals
                    .create_request_direct(
                        req,
                        risk_level,
                        reason.clone(),
                        ctx.session_id.clone(),
                        ctx.turn_id.clone(),
                        ctx.task_id.clone(),
                        current_policy_version,
                        300, // 300-second TTL
                    )
                    .await;

                let app_id = approval.approval_id.clone();

                // Emit ApprovalRequested correlated event
                if let Some(ref emitter) = self.emitter {
                    let corr = EventCorrelation {
                        conversation_id: ctx
                            .conversation_id
                            .clone()
                            .or_else(|| ctx.session_id.clone()),
                        turn_id: ctx.turn_id.clone(),
                        task_id: ctx.task_id.clone(),
                        stream_id: None,
                        tool_execution_id: None,
                        voice_session_id: None,
                    };
                    let payload =
                        EdithPayload::SecurityPolicy(SecurityPolicyPayload::ApprovalRequested {
                            approval_id: app_id.clone(),
                            action_domain: req.domain.clone(),
                            action_operation: req.operation.clone(),
                            risk_level: format!("{:?}", risk_level),
                            reason: reason.clone(),
                            expires_at_ms: approval.expires_at_ms,
                        });
                    let _ = emitter.emit_payload(corr, payload);
                }

                (
                    PolicyDecision::confirmation_required(
                        risk_level,
                        "CONFIRMATION_REQUIRED",
                        reason.clone(),
                        current_policy_version,
                    )
                    .with_approval_id(app_id.clone()),
                    Some(app_id),
                )
            }
            PolicyOutcome::Restricted => (
                PolicyDecision::restricted(
                    risk_level,
                    "RESTRICTED",
                    reason.clone(),
                    constraints.clone(),
                    current_policy_version,
                ),
                None,
            ),
            PolicyOutcome::Blocked => (
                PolicyDecision::blocked(
                    risk_level,
                    if policy_code.is_empty() {
                        "POLICY_BLOCKED"
                    } else {
                        &policy_code
                    },
                    reason.clone(),
                    current_policy_version,
                ),
                None,
            ),
        };

        // 4. Record Audit Trail
        let audit_entry = AuditRecord::new(
            ctx.session_id.clone(),
            ctx.turn_id.clone(),
            ctx.task_id.clone(),
            req,
            risk_level,
            outcome,
            reason.clone(),
            approval_id.clone(),
            current_policy_version,
        );
        self.audit_trail.record(audit_entry).await;

        // 5. Emit PolicyEvaluated Correlated Event
        if let Some(ref emitter) = self.emitter {
            let corr = EventCorrelation {
                conversation_id: ctx
                    .conversation_id
                    .clone()
                    .or_else(|| ctx.session_id.clone()),
                turn_id: ctx.turn_id.clone(),
                task_id: ctx.task_id.clone(),
                stream_id: None,
                tool_execution_id: None,
                voice_session_id: None,
            };
            let payload = EdithPayload::SecurityPolicy(SecurityPolicyPayload::PolicyEvaluated {
                action_domain: req.domain.clone(),
                action_operation: req.operation.clone(),
                risk_level: format!("{:?}", risk_level),
                outcome: format!("{:?}", outcome),
                reason,
                approval_id,
            });
            let _ = emitter.emit_payload(corr, payload);
        }

        final_decision
    }

    /// Lists all pending human approval requests awaiting operator confirmation.
    pub async fn list_pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.approvals.list_pending().await
    }

    /// Resolves an operator confirmation request (Approve / Deny) with human notes.
    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        decision: OperatorDecision,
    ) -> Result<ApprovalRequest, String> {
        let current_policy_version = self.get_policy_version();
        let status_str = match decision {
            OperatorDecision::Approve => "Approved",
            OperatorDecision::Deny => "Denied",
            OperatorDecision::Cancel => "Cancelled",
        };

        let resolved = self
            .approvals
            .resolve_approval(approval_id, decision, current_policy_version)
            .await?;

        // Emit ApprovalResolved event
        if let Some(ref emitter) = self.emitter {
            let corr = EventCorrelation {
                conversation_id: resolved.session_id.clone(),
                turn_id: resolved.turn_id.clone(),
                task_id: resolved.task_id.clone(),
                stream_id: None,
                tool_execution_id: None,
                voice_session_id: None,
            };
            let payload = EdithPayload::SecurityPolicy(SecurityPolicyPayload::ApprovalResolved {
                approval_id: approval_id.to_string(),
                status: status_str.to_string(),
                notes: None,
            });
            let _ = emitter.emit_payload(corr, payload);
        }

        Ok(resolved)
    }

    /// Retrieves recent security audit entries for operator observability.
    pub async fn get_audit_log(&self, limit: usize) -> Vec<AuditRecord> {
        self.audit_trail.get_recent(limit).await
    }
}
