use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scoped correlation identifiers linking events to their originating context.
/// Only populated with identifiers relevant to the specific event category.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventCorrelation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_execution_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_session_id: Option<String>,
}

impl EventCorrelation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Correlation for token-streaming LLM responses.
    pub fn for_stream(
        conversation_id: impl Into<Option<String>>,
        turn_id: impl Into<Option<String>>,
        stream_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            turn_id: turn_id.into(),
            stream_id: stream_id.into(),
            ..Default::default()
        }
    }

    /// Correlation for autonomous tasks (browser, dev agent).
    pub fn for_task(
        task_id: impl Into<String>,
        conversation_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            task_id: Some(task_id.into()),
            conversation_id: conversation_id.into(),
            ..Default::default()
        }
    }

    /// Correlation for tool proposal and execution.
    pub fn for_tool(
        tool_execution_id: impl Into<String>,
        task_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            tool_execution_id: Some(tool_execution_id.into()),
            task_id: task_id.into(),
            ..Default::default()
        }
    }

    /// Correlation for voice sessions.
    pub fn for_voice(
        voice_session_id: impl Into<String>,
        conversation_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            voice_session_id: Some(voice_session_id.into()),
            conversation_id: conversation_id.into(),
            ..Default::default()
        }
    }
}

/// The universal envelope wrapping all asynchronous runtime events dispatched across Tauri IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdithEventEnvelope<T> {
    pub event_id: String,
    pub timestamp_ms: u64,
    pub correlation: EventCorrelation,
    pub payload: T,
}

impl<T> EdithEventEnvelope<T> {
    pub fn new(correlation: EventCorrelation, payload: T) -> Self {
        let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp_ms,
            correlation,
            payload,
        }
    }
}
