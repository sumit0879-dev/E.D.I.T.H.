use super::types::{PolicyConstraints, PolicyOutcome, RiskLevel};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Comprehensive, immutable security decision output returned by the Policy Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Deterministic enforcement outcome
    pub outcome: PolicyOutcome,
    /// Contextual risk classification
    pub risk_level: RiskLevel,
    /// Human-readable explanation of the decision
    pub reason: String,
    /// Machine-readable policy classification code
    pub policy_code: String,
    /// Specific rule identifier if applicable
    pub rule_id: Option<String>,
    /// Explicit flag whether Human-In-The-Loop approval must be obtained
    pub requires_user_approval: bool,
    /// Pending approval token ID if confirmation is required
    pub approval_id: Option<String>,
    /// Execution constraints applied to the decision (e.g. read-only, path bounds)
    pub constraints: Option<PolicyConstraints>,
    /// Version/epoch of the policy configuration under which this decision was rendered
    pub policy_version: u32,
    /// Timestamp of evaluation in epoch milliseconds
    pub evaluated_at_ms: u64,
}

impl PolicyDecision {
    pub fn allow(
        risk_level: RiskLevel,
        policy_code: impl Into<String>,
        reason: impl Into<String>,
        policy_version: u32,
    ) -> Self {
        Self {
            outcome: PolicyOutcome::Allow,
            risk_level,
            reason: reason.into(),
            policy_code: policy_code.into(),
            rule_id: None,
            requires_user_approval: false,
            approval_id: None,
            constraints: None,
            policy_version,
            evaluated_at_ms: now_ms(),
        }
    }

    pub fn confirmation_required(
        risk_level: RiskLevel,
        policy_code: impl Into<String>,
        reason: impl Into<String>,
        policy_version: u32,
    ) -> Self {
        Self {
            outcome: PolicyOutcome::ConfirmationRequired,
            risk_level,
            reason: reason.into(),
            policy_code: policy_code.into(),
            rule_id: None,
            requires_user_approval: true,
            approval_id: None,
            constraints: None,
            policy_version,
            evaluated_at_ms: now_ms(),
        }
    }

    pub fn restricted(
        risk_level: RiskLevel,
        policy_code: impl Into<String>,
        reason: impl Into<String>,
        constraints: PolicyConstraints,
        policy_version: u32,
    ) -> Self {
        Self {
            outcome: PolicyOutcome::Restricted,
            risk_level,
            reason: reason.into(),
            policy_code: policy_code.into(),
            rule_id: None,
            requires_user_approval: false,
            approval_id: None,
            constraints: Some(constraints),
            policy_version,
            evaluated_at_ms: now_ms(),
        }
    }

    pub fn blocked(
        risk_level: RiskLevel,
        policy_code: impl Into<String>,
        reason: impl Into<String>,
        policy_version: u32,
    ) -> Self {
        Self {
            outcome: PolicyOutcome::Blocked,
            risk_level,
            reason: reason.into(),
            policy_code: policy_code.into(),
            rule_id: None,
            requires_user_approval: false,
            approval_id: None,
            constraints: None,
            policy_version,
            evaluated_at_ms: now_ms(),
        }
    }

    pub fn with_approval_id(mut self, approval_id: impl Into<String>) -> Self {
        self.approval_id = Some(approval_id.into());
        self
    }

    pub fn with_rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }
}

