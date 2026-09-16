use crate::events::envelope::EventCorrelation;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Authoritative domains under which tools are namespaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolDomain {
    Browser,
    Computer,
    Application,
    Filesystem,
    System,
    Memory,
    Edith,
}

impl ToolDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolDomain::Browser => "browser",
            ToolDomain::Computer => "computer",
            ToolDomain::Application => "application",
            ToolDomain::Filesystem => "filesystem",
            ToolDomain::System => "system",
            ToolDomain::Memory => "memory",
            ToolDomain::Edith => "edith",
        }
    }
}

impl fmt::Display for ToolDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ToolDomain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "browser" => Ok(ToolDomain::Browser),
            "computer" => Ok(ToolDomain::Computer),
            "application" | "app" => Ok(ToolDomain::Application),
            "filesystem" | "fs" => Ok(ToolDomain::Filesystem),
            "system" => Ok(ToolDomain::System),
            "memory" => Ok(ToolDomain::Memory),
            "edith" => Ok(ToolDomain::Edith),
            other => Err(format!("Unknown tool domain: '{}'", other)),
        }
    }
}

/// Strongly-typed, host-authoritative tool execution identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolExecutionId(String);

impl ToolExecutionId {
    pub fn new() -> Self {
        Self(format!("tool-exec-{}", Uuid::new_v4()))
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ToolExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ToolExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle status of a tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Requested,
    ApprovalRequired,
    Approved,
    Started,
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

/// Normalized, machine-readable Tool Definition contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Canonical, namespaced tool identifier (e.g. "browser.navigate", "system.execute")
    pub name: String,
    /// Authoritative domain
    pub domain: ToolDomain,
    /// Machine-readable description for models and documentation
    pub description: String,
    /// JSON Schema validating parameters
    pub parameters_schema: serde_json::Value,
    /// Optional JSON Schema describing output data
    pub return_schema: Option<serde_json::Value>,
    /// Indicates whether the tool is passive/read-only with no side effects
    pub is_read_only: bool,
    /// Default execution timeout in milliseconds
    pub default_timeout_ms: u64,
    /// Whether execution may be long-running and bridge to TaskRuntime
    pub is_long_running: bool,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        domain: ToolDomain,
        description: impl Into<String>,
        parameters_schema: serde_json::Value,
        is_read_only: bool,
        default_timeout_ms: u64,
    ) -> Self {
        Self {
            name: name.into(),
            domain,
            description: description.into(),
            parameters_schema,
            return_schema: None,
            is_read_only,
            default_timeout_ms,
            is_long_running: false,
        }
    }

    pub fn with_long_running(mut self, long_running: bool) -> Self {
        self.is_long_running = long_running;
        self
    }

    pub fn with_return_schema(mut self, schema: serde_json::Value) -> Self {
        self.return_schema = Some(schema);
        self
    }
}

/// Invocation request for executing a tool through the Universal Tool Runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Host-authoritative execution identifier
    pub execution_id: ToolExecutionId,
    /// Target namespaced tool name
    pub tool_name: String,
    /// Evaluated JSON arguments
    pub arguments: serde_json::Value,
    /// Phase 2 correlation metadata
    pub correlation: EventCorrelation,
    /// Pre-authorized approval ID from human operator (if resuming after CONFIRMATION_REQUIRED)
    pub active_approval_id: Option<String>,
    /// Optional caller-specified timeout override (ms)
    pub timeout_ms: Option<u64>,
}

impl ToolRequest {
    pub fn new(
        tool_name: impl Into<String>,
        arguments: serde_json::Value,
        correlation: EventCorrelation,
    ) -> Self {
        Self {
            execution_id: ToolExecutionId::new(),
            tool_name: tool_name.into(),
            arguments,
            correlation,
            active_approval_id: None,
            timeout_ms: None,
        }
    }

    pub fn simple(tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self::new(tool_name, arguments, EventCorrelation::default())
    }

    pub fn with_execution_id(mut self, id: ToolExecutionId) -> Self {
        self.execution_id = id;
        self
    }

    pub fn with_approval(mut self, approval_id: impl Into<String>) -> Self {
        self.active_approval_id = Some(approval_id.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Normalized result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: ToolExecutionId,
    pub tool_name: String,
    pub status: ToolStatus,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: u64,
    pub approval_id: Option<String>,
}

impl ToolExecutionResult {
    pub fn success(
        execution_id: ToolExecutionId,
        tool_name: String,
        data: serde_json::Value,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::Completed,
            data: Some(data),
            error: None,
            error_code: None,
            duration_ms,
            approval_id: None,
        }
    }

    pub fn blocked(
        execution_id: ToolExecutionId,
        tool_name: String,
        reason: String,
        policy_code: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::Blocked,
            data: None,
            error: Some(reason),
            error_code: Some(policy_code),
            duration_ms,
            approval_id: None,
        }
    }

    pub fn approval_required(
        execution_id: ToolExecutionId,
        tool_name: String,
        approval_id: String,
        reason: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::ApprovalRequired,
            data: Some(serde_json::json!({
                "approval_id": approval_id,
                "reason": reason,
            })),
            error: Some("Execution paused awaiting operator confirmation.".to_string()),
            error_code: Some("CONFIRMATION_REQUIRED".to_string()),
            duration_ms,
            approval_id: Some(approval_id),
        }
    }

    pub fn failed(
        execution_id: ToolExecutionId,
        tool_name: String,
        error: String,
        error_code: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::Failed,
            data: None,
            error: Some(error),
            error_code: Some(error_code),
            duration_ms,
            approval_id: None,
        }
    }

    pub fn cancelled(
        execution_id: ToolExecutionId,
        tool_name: String,
        reason: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::Cancelled,
            data: None,
            error: Some(reason.unwrap_or_else(|| "Tool execution was cancelled.".to_string())),
            error_code: Some("CANCELLED".to_string()),
            duration_ms,
            approval_id: None,
        }
    }

    pub fn timeout(
        execution_id: ToolExecutionId,
        tool_name: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_name,
            status: ToolStatus::Failed,
            data: None,
            error: Some(format!("Tool execution timed out after {}ms.", duration_ms)),
            error_code: Some("TIMEOUT".to_string()),
            duration_ms,
            approval_id: None,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == ToolStatus::Completed
    }
}

/// Normalized errors originating during tool resolution, validation, authorization, or execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionError {
    ToolNotFound(String),
    InvalidArguments(String),
    PolicyBlocked(String),
    ApprovalDenied(String),
    ApprovalExpired(String),
    Timeout(String),
    Cancelled(String),
    ExecutorUnavailable(String),
    DomainError(String),
    InternalError(String),
}

impl fmt::Display for ToolExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolExecutionError::ToolNotFound(msg) => write!(f, "Tool not found: {}", msg),
            ToolExecutionError::InvalidArguments(msg) => write!(f, "Invalid tool arguments: {}", msg),
            ToolExecutionError::PolicyBlocked(msg) => write!(f, "Policy blocked: {}", msg),
            ToolExecutionError::ApprovalDenied(msg) => write!(f, "Approval denied: {}", msg),
            ToolExecutionError::ApprovalExpired(msg) => write!(f, "Approval expired: {}", msg),
            ToolExecutionError::Timeout(msg) => write!(f, "Execution timed out: {}", msg),
            ToolExecutionError::Cancelled(msg) => write!(f, "Execution cancelled: {}", msg),
            ToolExecutionError::ExecutorUnavailable(msg) => write!(f, "Executor unavailable: {}", msg),
            ToolExecutionError::DomainError(msg) => write!(f, "Domain execution error: {}", msg),
            ToolExecutionError::InternalError(msg) => write!(f, "Internal tool error: {}", msg),
        }
    }
}

impl std::error::Error for ToolExecutionError {}

impl ToolExecutionError {
    pub fn error_code(&self) -> &'static str {
        match self {
            ToolExecutionError::ToolNotFound(_) => "TOOL_NOT_FOUND",
            ToolExecutionError::InvalidArguments(_) => "INVALID_ARGUMENTS",
            ToolExecutionError::PolicyBlocked(_) => "POLICY_BLOCKED",
            ToolExecutionError::ApprovalDenied(_) => "APPROVAL_DENIED",
            ToolExecutionError::ApprovalExpired(_) => "APPROVAL_EXPIRED",
            ToolExecutionError::Timeout(_) => "TIMEOUT",
            ToolExecutionError::Cancelled(_) => "CANCELLED",
            ToolExecutionError::ExecutorUnavailable(_) => "EXECUTOR_UNAVAILABLE",
            ToolExecutionError::DomainError(_) => "DOMAIN_ERROR",
            ToolExecutionError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}
