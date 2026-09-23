//! projections.rs — Bounded, sanitized read-model DTOs for E.D.I.T.H. Self-Knowledge.
//!
//! Enforces strict data-contract boundaries: zero credential leakage, bounded lists,
//! and sanitized URLs / metadata.

use super::autonomy::AutonomyState;
use crate::ai::capabilities::CapabilitySet;
use crate::computer_control::ComputerControlState;
use crate::policy::context::SecurityMode;
use crate::policy::types::RiskLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scrubs sensitive query parameters and credential patterns from URLs.
pub fn scrub_url(raw_url: &str) -> String {
    if raw_url.starts_with("file://") {
        if let Some(pos) = raw_url.rfind('/') {
            let filename = &raw_url[pos + 1..];
            return format!("file:///[REDACTED_DIR]/{}", filename);
        }
        return "file:///[REDACTED]".to_string();
    }

    if let Some((base, query)) = raw_url.split_once('?') {
        let sensitive_keys = [
            "token",
            "key",
            "password",
            "pwd",
            "auth",
            "api_key",
            "apikey",
            "code",
            "secret",
            "sig",
            "signature",
            "access_token",
            "refresh_token",
        ];

        let mut cleaned_params = Vec::new();
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let k_lower = k.to_lowercase();
                if sensitive_keys.iter().any(|s| k_lower.contains(s)) {
                    cleaned_params.push(format!("{}=[REDACTED]", k));
                } else {
                    cleaned_params.push(format!("{}={}", k, v));
                }
            } else {
                cleaned_params.push(pair.to_string());
            }
        }
        format!("{}?{}", base, cleaned_params.join("&"))
    } else {
        scrub_sensitive_text(raw_url)
    }
}

/// Redacts API keys, passwords, and bearer tokens from free-form text strings.
pub fn scrub_sensitive_text(text: &str) -> String {
    let mut result = text.to_string();
    // Common API key prefixes
    let patterns = [
        "gsk_", "AIzaSy", "sk-proj-", "sk-ant-", "Bearer ", "bearer ",
    ];
    for pattern in patterns {
        if let Some(pos) = result.find(pattern) {
            let end = result[pos..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&')
                .map(|p| pos + p)
                .unwrap_or(result.len());
            let redacted_segment = format!("{}[REDACTED]", pattern);
            result.replace_range(pos..end, &redacted_segment);
        }
    }
    result
}

/// Compact high-level summary of live runtime status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatusSummary {
    pub session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub autonomy_state: AutonomyState,
    pub security_mode: SecurityMode,
    pub policy_version: u32,
    pub active_task_count: usize,
    pub active_execution_count: usize,
    pub pending_approval_count: usize,
    pub active_provider: String,
    pub active_model: String,
    pub uptime_seconds: u64,
    #[serde(default)]
    pub voice: Option<crate::voice::VoiceStatusSummary>,
}

/// Summary of tools available in a specific domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCapabilitySummary {
    pub domain: String,
    pub tool_count: usize,
    pub tools: Vec<ToolSummaryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummaryItem {
    pub name: String,
    pub description: String,
    pub requires_approval: bool,
}

/// Structured catalog of all available capabilities across domains and providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesSummary {
    pub total_tools: usize,
    pub domains: Vec<DomainCapabilitySummary>,
    pub active_provider: String,
    pub active_provider_capabilities: Option<CapabilitySet>,
}

/// Bounded summary item for a task in the TaskRuntime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummaryItem {
    pub task_id: String,
    pub task_type: String,
    pub owner: String,
    pub goal: String,
    pub status: String,
    pub step: u32,
    pub max_steps: u32,
    pub status_text: String,
    pub created_at_ms: u64,
}

/// Bounded overview of active and recent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksSummary {
    pub total_active: usize,
    pub total_tasks: usize,
    pub active_tasks: Vec<TaskSummaryItem>,
}

/// Bounded summary of in-flight tool executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionsSummary {
    pub active_execution_count: usize,
    pub active_execution_ids: Vec<String>,
}

/// Bounded summary item for an open browser tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSummary {
    pub tab_id: String,
    pub title: String,
    pub url: String,
    pub is_active: bool,
    pub control_state: String,
}

/// Bounded status of the browser domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStatusSummary {
    pub open_tabs_count: usize,
    pub active_tab_id: Option<String>,
    pub is_visible: bool,
    pub tabs: Vec<TabSummary>,
}

/// Status of desktop computer control ownership and preemption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerStatusSummary {
    pub control_state: ComputerControlState,
    pub is_ai_controlled: bool,
    pub last_transition_ms: u64,
    pub reason: Option<String>,
}

/// Bounded summary item for a pending approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalSummary {
    pub approval_id: String,
    pub domain: String,
    pub operation: String,
    pub risk_level: RiskLevel,
    pub reason: String,
    pub expires_at_ms: u64,
}

/// Bounded summary of the host security policy posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStatusSummary {
    pub policy_version: u32,
    pub security_mode: SecurityMode,
    pub pending_approval_count: usize,
    pub pending_approvals: Vec<PendingApprovalSummary>,
}

/// Health diagnostic status of an individual subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}

/// Comprehensive health summary of core E.D.I.T.H. subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthSummary {
    pub overall_healthy: bool,
    pub timestamp_ms: u64,
    pub subsystems: HashMap<String, SubsystemHealth>,
}
