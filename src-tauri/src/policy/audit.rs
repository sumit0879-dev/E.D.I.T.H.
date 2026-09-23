use super::types::{ActionRequest, ActionTarget, PolicyOutcome, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// An immutable, privacy-sanitized security audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Unique identifier for this audit entry
    pub event_id: String,
    /// Epoch timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Correlated session ID
    pub session_id: Option<String>,
    /// Correlated turn ID
    pub turn_id: Option<String>,
    /// Correlated task ID
    pub task_id: Option<String>,
    /// Target action domain
    pub action_domain: String,
    /// Target action operation
    pub action_operation: String,
    /// Target evaluated
    pub target: ActionTarget,
    /// Sanitized arguments (sensitive tokens and secrets redacted)
    pub sanitized_arguments: serde_json::Value,
    /// Risk level evaluated
    pub risk_level: RiskLevel,
    /// Policy outcome decided
    pub outcome: PolicyOutcome,
    /// Evaluated decision reason or justification
    pub reason: String,
    /// Associated approval request ID, if any
    pub approval_id: Option<String>,
    /// Policy configuration version at decision time
    pub policy_version: u32,
}

impl AuditRecord {
    pub fn new(
        session_id: Option<String>,
        turn_id: Option<String>,
        task_id: Option<String>,
        req: &ActionRequest,
        risk_level: RiskLevel,
        outcome: PolicyOutcome,
        reason: impl Into<String>,
        approval_id: Option<String>,
        policy_version: u32,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp_ms: now_ms(),
            session_id,
            turn_id,
            task_id,
            action_domain: req.domain.clone(),
            action_operation: req.operation.clone(),
            target: req.target.clone(),
            sanitized_arguments: sanitize_value(&req.arguments),
            risk_level,
            outcome,
            reason: reason.into(),
            approval_id,
            policy_version,
        }
    }
}

/// Sensitive field names that must be redacted in audit trails to prevent credential leakage.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "token",
    "secret",
    "api_key",
    "apikey",
    "auth",
    "authorization",
    "cookie",
    "bearer",
    "private_key",
    "credential",
    "access_token",
    "refresh_token",
    "id_token",
];

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|k| lower.contains(k))
}

/// Recursively traverses a JSON value and redacts sensitive keys.
pub fn sanitize_value(val: &serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Object(map) => {
            let mut sanitized_map = serde_json::Map::new();
            for (k, v) in map {
                if is_sensitive_key(k) {
                    sanitized_map.insert(
                        k.clone(),
                        serde_json::Value::String("[REDACTED]".to_string()),
                    );
                } else {
                    sanitized_map.insert(k.clone(), sanitize_value(v));
                }
            }
            serde_json::Value::Object(sanitized_map)
        }
        serde_json::Value::Array(arr) => {
            let sanitized_arr: Vec<serde_json::Value> = arr.iter().map(sanitize_value).collect();
            serde_json::Value::Array(sanitized_arr)
        }
        _ => val.clone(),
    }
}

/// Thread-safe in-memory security audit trail with fixed-capacity ring buffer.
#[derive(Debug, Clone)]
pub struct SecurityAuditTrail {
    capacity: usize,
    entries: Arc<RwLock<VecDeque<AuditRecord>>>,
}

impl Default for SecurityAuditTrail {
    fn default() -> Self {
        Self::new(500)
    }
}

impl SecurityAuditTrail {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
        }
    }

    /// Records a new audit entry, evicting the oldest record if capacity is reached.
    pub async fn record(&self, entry: AuditRecord) {
        let mut guard = self.entries.write().await;
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(entry);
    }

    /// Retrieves up to `limit` recent audit records in descending chronological order.
    pub async fn get_recent(&self, limit: usize) -> Vec<AuditRecord> {
        let guard = self.entries.read().await;
        guard.iter().rev().take(limit).cloned().collect()
    }

    /// Returns the current number of stored audit records.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Clears all audit records.
    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }
}
