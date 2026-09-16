use super::decision::PolicyDecision;
use super::types::{ActionRequest, RiskLevel};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lifecycle status of a human confirmation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Awaiting operator review in UI
    Pending,
    /// Explicitly approved by authorized operator
    Approved,
    /// Rejected by operator
    Denied,
    /// TTL elapsed without decision
    Expired,
    /// Cancelled before resolution (e.g. session/turn aborted)
    Cancelled,
    /// Consumed by the authorized execution (strictly single-use)
    Consumed,
}

impl ApprovalStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ApprovalStatus::Denied
                | ApprovalStatus::Expired
                | ApprovalStatus::Cancelled
                | ApprovalStatus::Consumed
        )
    }
}

/// Computes a canonical SHA-256 digest over the action request for replay and tamper prevention.
pub fn compute_action_hash(req: &ActionRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(req.domain.as_bytes());
    hasher.update(b"|");
    hasher.update(req.operation.as_bytes());
    hasher.update(b"|");

    // Canonicalize target
    let target_json = serde_json::to_string(&req.target).unwrap_or_default();
    hasher.update(target_json.as_bytes());
    hasher.update(b"|");

    // Canonicalize arguments
    let args_json = serde_json::to_string(&req.arguments).unwrap_or_default();
    hasher.update(args_json.as_bytes());

    format!("{:x}", hasher.finalize())
}

/// Authoritative Human-In-The-Loop approval entity stored host-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique identifier for this approval instance
    pub approval_id: String,
    /// Full action request being authorized
    pub action_request: ActionRequest,
    /// SHA-256 cryptographic digest binding this approval to exact arguments and target
    pub request_hash: String,
    /// Risk level evaluated for this action
    pub risk_level: RiskLevel,
    /// Explanation of why human confirmation is required
    pub reason: String,
    /// Session context binding
    pub session_id: Option<String>,
    /// Turn context binding
    pub turn_id: Option<String>,
    /// Task context binding
    pub task_id: Option<String>,
    /// Policy configuration version at request time
    pub policy_version: u32,
    /// Creation epoch timestamp in milliseconds
    pub created_at_ms: u64,
    /// Expiration epoch timestamp in milliseconds
    pub expires_at_ms: u64,
    /// Current approval lifecycle status
    pub status: ApprovalStatus,
}

impl ApprovalRequest {
    /// Validates whether this approval authorizes the specific incoming action request.
    /// Strictly protects against replay, argument modification, and expiration.
    pub fn validate_authorization(
        &self,
        incoming_req: &ActionRequest,
        current_policy_version: u32,
    ) -> Result<(), String> {
        let now = now_ms();

        // 1. Expiration check
        if now > self.expires_at_ms || self.status == ApprovalStatus::Expired {
            return Err("Security Authorization Error: Approval request has expired.".to_string());
        }

        // 2. Status check
        match self.status {
            ApprovalStatus::Pending => {
                return Err("Security Authorization Error: Action is still pending operator confirmation.".to_string());
            }
            ApprovalStatus::Denied => {
                return Err("Security Authorization Error: Action was denied by operator.".to_string());
            }
            ApprovalStatus::Cancelled => {
                return Err("Security Authorization Error: Approval request was cancelled.".to_string());
            }
            ApprovalStatus::Consumed => {
                return Err("Security Authorization Error: Approval has already been consumed (replay violation).".to_string());
            }
            ApprovalStatus::Approved => {
                // Valid approved state, proceed to tamper verification
            }
            ApprovalStatus::Expired => {
                return Err("Security Authorization Error: Approval request has expired.".to_string());
            }
        }

        // 3. Policy version invariance
        if self.policy_version != current_policy_version {
            return Err(format!(
                "Security Authorization Error: Policy version mismatch (request v{}, current v{}). Re-evaluation required.",
                self.policy_version, current_policy_version
            ));
        }

        // 4. Exact cryptographic hash match (replay and argument tampering protection)
        let incoming_hash = compute_action_hash(incoming_req);
        if self.request_hash != incoming_hash {
            return Err(
                "Security Tamper Error: Action parameters do not match approved signature. Replay or modification rejected."
                    .to_string(),
            );
        }

        Ok(())
    }
}

/// Action decision input from operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatorDecision {
    Approve,
    Deny,
    Cancel,
}

/// In-memory host-side store managing approval lifecycles.
#[derive(Clone, Default)]
pub struct ApprovalStore {
    approvals: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            approvals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates and registers a new pending approval.
    pub async fn create_request(
        &self,
        req: ActionRequest,
        decision: &PolicyDecision,
        session_id: Option<String>,
        turn_id: Option<String>,
        task_id: Option<String>,
        ttl_secs: u64,
    ) -> ApprovalRequest {
        let now = now_ms();
        let expires_at_ms = now + (ttl_secs * 1000);
        let request_hash = compute_action_hash(&req);
        let approval_id = uuid::Uuid::new_v4().to_string();

        let approval = ApprovalRequest {
            approval_id: approval_id.clone(),
            action_request: req,
            request_hash,
            risk_level: decision.risk_level,
            reason: decision.reason.clone(),
            session_id,
            turn_id,
            task_id,
            policy_version: decision.policy_version,
            created_at_ms: now,
            expires_at_ms,
            status: ApprovalStatus::Pending,
        };

        let mut lock = self.approvals.write().await;
        lock.insert(approval_id, approval.clone());
        approval
    }

    /// Directly creates and registers a pending approval with explicit fields.
    pub async fn create_request_direct(
        &self,
        req: &ActionRequest,
        risk_level: RiskLevel,
        reason: String,
        session_id: Option<String>,
        turn_id: Option<String>,
        task_id: Option<String>,
        policy_version: u32,
        ttl_secs: u64,
    ) -> ApprovalRequest {
        let now = now_ms();
        let expires_at_ms = now + (ttl_secs * 1000);
        let request_hash = compute_action_hash(req);
        let approval_id = uuid::Uuid::new_v4().to_string();

        let approval = ApprovalRequest {
            approval_id: approval_id.clone(),
            action_request: req.clone(),
            request_hash,
            risk_level,
            reason,
            session_id,
            turn_id,
            task_id,
            policy_version,
            created_at_ms: now,
            expires_at_ms,
            status: ApprovalStatus::Pending,
        };

        let mut lock = self.approvals.write().await;
        lock.insert(approval_id, approval.clone());
        approval
    }

    /// Resolves an approval request with the operator's decision.
    pub async fn resolve(
        &self,
        approval_id: &str,
        decision: OperatorDecision,
        session_id: Option<&str>,
    ) -> Result<ApprovalRequest, String> {
        let now = now_ms();
        let mut lock = self.approvals.write().await;
        let approval = lock
            .get_mut(approval_id)
            .ok_or_else(|| format!("Approval request '{}' not found or purged.", approval_id))?;

        // Verify session binding if provided
        if let Some(expected_sess) = session_id {
            if let Some(ref bound_sess) = approval.session_id {
                if !expected_sess.trim().is_empty() && bound_sess != expected_sess {
                    return Err("Security Error: Session mismatch for approval resolution.".to_string());
                }
            }
        }

        // Verify status is currently pending
        if approval.status != ApprovalStatus::Pending {
            return Err(format!(
                "Security Error: Approval is already {:?} and cannot be updated.",
                approval.status
            ));
        }

        // Check if expired
        if now > approval.expires_at_ms {
            approval.status = ApprovalStatus::Expired;
            return Err("Security Error: Approval request has expired.".to_string());
        }

        match decision {
            OperatorDecision::Approve => approval.status = ApprovalStatus::Approved,
            OperatorDecision::Deny => approval.status = ApprovalStatus::Denied,
            OperatorDecision::Cancel => approval.status = ApprovalStatus::Cancelled,
        }

        Ok(approval.clone())
    }

    /// Helper for PolicyEngine: resolves approval with policy version invariance check.
    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        decision: OperatorDecision,
        current_policy_version: u32,
    ) -> Result<ApprovalRequest, String> {
        let mut lock = self.approvals.write().await;
        let approval = lock
            .get_mut(approval_id)
            .ok_or_else(|| format!("Approval request '{}' not found or purged.", approval_id))?;

        if approval.policy_version != current_policy_version {
            return Err(format!(
                "Security Error: Approval was created under policy v{}, but current policy is v{}. Approval is invalidated.",
                approval.policy_version, current_policy_version
            ));
        }

        let now = now_ms();
        if approval.status != ApprovalStatus::Pending {
            return Err(format!(
                "Security Error: Approval is already {:?} and cannot be updated.",
                approval.status
            ));
        }

        if now > approval.expires_at_ms {
            approval.status = ApprovalStatus::Expired;
            return Err("Security Error: Approval request has expired.".to_string());
        }

        match decision {
            OperatorDecision::Approve => approval.status = ApprovalStatus::Approved,
            OperatorDecision::Deny => approval.status = ApprovalStatus::Denied,
            OperatorDecision::Cancel => approval.status = ApprovalStatus::Cancelled,
        }

        Ok(approval.clone())
    }

    /// Retrieves an approval request by ID.
    pub async fn get(&self, approval_id: &str) -> Option<ApprovalRequest> {
        let lock = self.approvals.read().await;
        let req = lock.get(approval_id)?.clone();
        if now_ms() > req.expires_at_ms && req.status == ApprovalStatus::Pending {
            let mut expired = req;
            expired.status = ApprovalStatus::Expired;
            Some(expired)
        } else {
            Some(req)
        }
    }

    /// Alias for PolicyEngine
    pub async fn get_approval(&self, approval_id: &str) -> Option<ApprovalRequest> {
        self.get(approval_id).await
    }

    /// Consumes an approved request, transitioning it to `Consumed` (strictly single-use).
    pub async fn consume(&self, approval_id: &str) -> Result<(), String> {
        let mut lock = self.approvals.write().await;
        let approval = lock
            .get_mut(approval_id)
            .ok_or_else(|| format!("Approval request '{}' not found.", approval_id))?;

        if approval.status != ApprovalStatus::Approved {
            return Err(format!(
                "Security Error: Cannot consume approval in status {:?}.",
                approval.status
            ));
        }

        approval.status = ApprovalStatus::Consumed;
        Ok(())
    }

    /// Alias for PolicyEngine
    pub async fn consume_approval(&self, approval_id: &str) -> Result<(), String> {
        self.consume(approval_id).await
    }

    /// Lists all pending approvals.
    pub async fn list_pending(&self) -> Vec<ApprovalRequest> {
        let now = now_ms();
        let lock = self.approvals.read().await;
        lock.values()
            .filter(|a| a.status == ApprovalStatus::Pending && now <= a.expires_at_ms)
            .cloned()
            .collect()
    }
}
