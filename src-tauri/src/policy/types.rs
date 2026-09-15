use crate::events::EventCorrelation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Final deterministic security decision outcome rendered by the Policy Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyOutcome {
    /// Action is completely safe and authorized under current policy.
    Allow,
    /// Action involves operational or security risk requiring explicit human confirmation (HITL).
    ConfirmationRequired,
    /// Action is constrained or permitted only within restricted parameters (e.g. read-only mode).
    Restricted,
    /// Action is strictly prohibited by security policy.
    Blocked,
}

impl PolicyOutcome {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyOutcome::Allow)
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self, PolicyOutcome::ConfirmationRequired)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, PolicyOutcome::Blocked)
    }
}

/// Intrinsic and contextual risk level classification.
/// Evaluated independently from the final PolicyOutcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "SAFE",
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }
}

/// Target entity upon which an action is to be performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ActionTarget {
    /// Command line executable and arguments
    Command {
        program: String,
        args: Vec<String>,
        working_dir: Option<PathBuf>,
    },
    /// Local filesystem path
    Path(PathBuf),
    /// Network URL or URI
    Url(String),
    /// Browser DOM element target
    BrowserElement {
        selector: Option<String>,
        element_id: Option<String>,
        text: Option<String>,
    },
    /// System resource or application identifier
    SystemTarget(String),
    /// Action has no external target
    None,
}

/// Tool-agnostic action proposal submitted to the Policy Engine for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Unique identifier for this action instance
    pub action_id: String,
    /// Functional domain (e.g. "command", "filesystem", "browser", "system", "edith")
    pub domain: String,
    /// Specific operation verb (e.g. "execute", "read", "write", "navigate", "click", "configure")
    pub operation: String,
    /// Target resource being operated upon
    pub target: ActionTarget,
    /// Arbitrary JSON arguments passed to the action
    pub arguments: serde_json::Value,
    /// Phase 2 correlation metadata
    pub correlation: EventCorrelation,
}

impl ActionRequest {
    pub fn new(
        domain: impl Into<String>,
        operation: impl Into<String>,
        target: ActionTarget,
        arguments: serde_json::Value,
        correlation: EventCorrelation,
    ) -> Self {
        Self {
            action_id: uuid::Uuid::new_v4().to_string(),
            domain: domain.into(),
            operation: operation.into(),
            target,
            arguments,
            correlation,
        }
    }

    pub fn new_command(
        domain: impl Into<String>,
        operation: impl Into<String>,
        program: impl Into<String>,
        args: Vec<String>,
        arguments: serde_json::Value,
    ) -> Self {
        let prog = program.into();
        Self::new(
            domain,
            operation,
            ActionTarget::Command {
                program: prog,
                args,
                working_dir: None,
            },
            arguments,
            EventCorrelation::default(),
        )
    }

    pub fn new_url(
        domain: impl Into<String>,
        operation: impl Into<String>,
        url: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        let u = url.into();
        Self::new(
            domain,
            operation,
            ActionTarget::Url(u),
            arguments,
            EventCorrelation::default(),
        )
    }
}

/// Extensible policy constraints applied to an execution context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConstraints {
    /// Approved filesystem roots (outside these paths, filesystem access is blocked)
    pub allowed_paths: Vec<PathBuf>,
    /// Explicitly blocked filesystem roots
    pub blocked_paths: Vec<PathBuf>,
    /// Whitelisted command line programs
    pub allowed_programs: Vec<String>,
    /// Explicitly blocked executables (e.g. "cmd.exe", "powershell.exe")
    pub blocked_programs: Vec<String>,
    /// Whitelisted web domains
    pub allowed_domains: Vec<String>,
    /// Blocked web domains or IP ranges
    pub blocked_domains: Vec<String>,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: Option<u64>,
    /// Maximum output buffer size in bytes
    pub max_output_bytes: Option<usize>,
    /// Restrict operations to read-only semantics
    pub read_only: bool,
    /// Allow external command execution
    pub allow_command_execution: bool,
    /// Allow web navigation and network socket access
    pub allow_network_access: bool,
    /// Allow external cloud services
    pub allow_external_services: bool,
}

impl Default for PolicyConstraints {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            blocked_paths: Vec::new(),
            allowed_programs: Vec::new(),
            blocked_programs: Vec::new(),
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            max_execution_time_ms: None,
            max_output_bytes: None,
            read_only: false,
            allow_command_execution: true,
            allow_network_access: true,
            allow_external_services: true,
        }
    }
}

