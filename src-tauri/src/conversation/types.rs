use serde::{Deserialize, Serialize};

/// Identifies an interactive user session.
pub type SessionId = String;

/// Message role taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// Authoritative conversation message model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Option<i32>,
    pub role: MessageRole,
    pub text: String,
    pub time: String,
    pub session_id: SessionId,
}

/// Request DTO submitted from frontend or IPC to initiate a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSubmissionRequest {
    pub session_id: SessionId,
    pub message: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub temperature: Option<f64>,
    /// Optional legacy correlation hint for backwards compatibility with older clients.
    /// NON-AUTHORITATIVE: The backend ALWAYS generates the authoritative TurnId.
    /// This hint is never stored as TurnId, never emitted as TurnId, and never used for turn lookup.
    pub client_turn_id: Option<String>,
}

/// Authoritative response returned immediately when a turn is registered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSubmissionResult {
    pub turn_id: String,
    pub session_id: SessionId,
    pub stream_id: String,
    pub user_message_text: String,
}

/// Model and provider selection parameters for a conversational turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
    pub temperature: f64,
}

impl Default for ModelSelection {
    fn default() -> Self {
        Self {
            provider_id: "groq".to_string(),
            model_id: "llama-3.3-70b-versatile".to_string(),
            temperature: 0.7,
        }
    }
}

/// Snapshot of a turn for frontend inspection or query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSnapshot {
    pub turn_id: String,
    pub session_id: SessionId,
    pub stream_id: String,
    pub status: super::turn::TurnStatus,
    pub model_selection: ModelSelection,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub error: Option<String>,
    pub final_response: Option<String>,
}
