use crate::events::EventCorrelation;
use serde::{Deserialize, Serialize};

/// Identifies the category or kind of asynchronous task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Background,
    BrowserAgent,
    DevAgent,
    Maintenance,
    Custom(String),
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Background => write!(f, "background"),
            TaskType::BrowserAgent => write!(f, "browser_agent"),
            TaskType::DevAgent => write!(f, "dev_agent"),
            TaskType::Maintenance => write!(f, "maintenance"),
            TaskType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Identifies who initiated or owns the task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "owner_type", content = "owner_id", rename_all = "snake_case")]
pub enum TaskOwner {
    User,
    Turn(String),
    System,
    External(String),
}

/// Dynamic step progress information for an active task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    pub step: u32,
    pub max_steps: u32,
    pub status_text: String,
}

impl Default for TaskProgress {
    fn default() -> Self {
        Self {
            step: 0,
            max_steps: 0,
            status_text: String::new(),
        }
    }
}

/// Immutable snapshot of a task for IPC inspection and telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub task_id: String,
    pub task_type: TaskType,
    pub owner: TaskOwner,
    pub goal: String,
    pub status: super::state::TaskStatus,
    pub progress: TaskProgress,
    pub correlation: EventCorrelation,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub error: Option<String>,
    pub result_summary: Option<String>,
}

/// Normalized errors originating from the Task Runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskError {
    NotFound(String),
    InvalidStateTransition {
        current: String,
        attempted: String,
    },
    AlreadyExists(String),
    ExecutionFailed(String),
    Cancelled(String),
    Internal(String),
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::NotFound(id) => write!(f, "Task not found: {}", id),
            TaskError::InvalidStateTransition { current, attempted } => {
                write!(f, "Invalid task transition from '{}' to '{}'", current, attempted)
            }
            TaskError::AlreadyExists(id) => write!(f, "Task already exists: {}", id),
            TaskError::ExecutionFailed(err) => write!(f, "Task execution failed: {}", err),
            TaskError::Cancelled(reason) => write!(f, "Task cancelled: {}", reason),
            TaskError::Internal(err) => write!(f, "Task runtime internal error: {}", err),
        }
    }
}

impl std::error::Error for TaskError {}
