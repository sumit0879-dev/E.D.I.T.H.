use super::types::{ModelSelection, SessionId, TurnSnapshot};
use crate::events::{StreamId, TurnId};
use crate::task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Explicit lifecycle states for a single conversational turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Created,
    InputAccepted,
    Processing,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

impl TurnStatus {
    /// Returns true if this state is final and cannot be transitioned from.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Cancelled
        )
    }

    /// Evaluates if transitioning from `self` to `target` is allowed by the lifecycle state machine.
    pub fn can_transition_to(&self, target: &TurnStatus) -> bool {
        if self == target {
            return true;
        }

        match self {
            TurnStatus::Created => matches!(
                target,
                TurnStatus::InputAccepted | TurnStatus::Cancelled
            ),
            TurnStatus::InputAccepted => matches!(
                target,
                TurnStatus::Processing | TurnStatus::Cancelled
            ),
            TurnStatus::Processing => matches!(
                target,
                TurnStatus::Streaming | TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Cancelled
            ),
            TurnStatus::Streaming => matches!(
                target,
                TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Cancelled
            ),
            // Terminal states cannot transition to anything
            TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Cancelled => false,
        }
    }
}

impl std::fmt::Display for TurnStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnStatus::Created => write!(f, "created"),
            TurnStatus::InputAccepted => write!(f, "input_accepted"),
            TurnStatus::Processing => write!(f, "processing"),
            TurnStatus::Streaming => write!(f, "streaming"),
            TurnStatus::Completed => write!(f, "completed"),
            TurnStatus::Failed => write!(f, "failed"),
            TurnStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// In-memory representation of an authoritative conversational turn.
pub struct Turn {
    pub turn_id: TurnId,
    pub session_id: SessionId,
    pub stream_id: StreamId,
    pub user_message: String,
    pub model_selection: ModelSelection,
    pub status: TurnStatus,
    pub cancellation_token: CancellationToken,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub final_response: Option<String>,
    pub error: Option<String>,
}

impl Turn {
    pub fn new(
        turn_id: TurnId,
        session_id: SessionId,
        stream_id: StreamId,
        user_message: impl Into<String>,
        model_selection: ModelSelection,
    ) -> Self {
        Self {
            turn_id,
            session_id,
            stream_id,
            user_message: user_message.into(),
            model_selection,
            status: TurnStatus::Created,
            cancellation_token: CancellationToken::new(),
            created_at_ms: now_ms(),
            completed_at_ms: None,
            final_response: None,
            error: None,
        }
    }

    pub fn to_snapshot(&self) -> TurnSnapshot {
        TurnSnapshot {
            turn_id: self.turn_id.to_string(),
            session_id: self.session_id.clone(),
            stream_id: self.stream_id.to_string(),
            status: self.status,
            model_selection: self.model_selection.clone(),
            created_at_ms: self.created_at_ms,
            completed_at_ms: self.completed_at_ms,
            error: self.error.clone(),
            final_response: self.final_response.clone(),
        }
    }
}
