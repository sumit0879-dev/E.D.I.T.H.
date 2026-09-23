use serde::{Deserialize, Serialize};

/// Explicit lifecycle states for an asynchronous task in E.D.I.T.H. Task Runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Created,
    Queued,
    Running,
    Completing,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    /// Returns true if this status represents a final terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    /// Evaluates whether a transition from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        if self == target {
            return true;
        }

        match self {
            TaskStatus::Created => matches!(
                target,
                TaskStatus::Queued | TaskStatus::Running | TaskStatus::Cancelled
            ),
            TaskStatus::Queued => matches!(target, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Running => matches!(
                target,
                TaskStatus::Completing
                    | TaskStatus::Completed
                    | TaskStatus::Failed
                    | TaskStatus::Cancelled
            ),
            TaskStatus::Completing => matches!(target, TaskStatus::Completed | TaskStatus::Failed),
            // Terminal states cannot transition to any other state
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => false,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Created => write!(f, "created"),
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completing => write!(f, "completing"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}
