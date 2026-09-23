use crate::ai::ProviderError;
use serde::{Deserialize, Serialize};

/// Normalized high-level errors originating from Conversation Core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error_type", content = "message", rename_all = "snake_case")]
pub enum ConversationError {
    Authentication(String),
    ModelUnavailable(String),
    RateLimit(String),
    NetworkFailure(String),
    Timeout(String),
    GenerationFailure(String),
    Cancellation(String),
    InvalidTurnState { current: String, target: String },
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for ConversationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ConversationError::ModelUnavailable(msg) => write!(f, "Model unavailable: {}", msg),
            ConversationError::RateLimit(msg) => write!(f, "Rate limit exceeded: {}", msg),
            ConversationError::NetworkFailure(msg) => write!(f, "Network failure: {}", msg),
            ConversationError::Timeout(msg) => write!(f, "Request timed out: {}", msg),
            ConversationError::GenerationFailure(msg) => write!(f, "Generation failed: {}", msg),
            ConversationError::Cancellation(reason) => write!(f, "Turn cancelled: {}", reason),
            ConversationError::InvalidTurnState { current, target } => {
                write!(
                    f,
                    "Invalid turn state transition from '{}' to '{}'",
                    current, target
                )
            }
            ConversationError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ConversationError::Internal(msg) => write!(f, "Internal conversation error: {}", msg),
        }
    }
}

impl std::error::Error for ConversationError {}

impl From<ProviderError> for ConversationError {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::AuthFailure { message } => ConversationError::Authentication(message),
            ProviderError::InvalidRequest { message } => ConversationError::Internal(message),
            ProviderError::ModelUnavailable { model, reason } => {
                ConversationError::ModelUnavailable(format!("{}: {}", model, reason))
            }
            ProviderError::CapabilityUnsupported { capability } => {
                ConversationError::ModelUnavailable(format!(
                    "Capability not supported: {}",
                    capability
                ))
            }
            ProviderError::RateLimited { message, .. } => ConversationError::RateLimit(message),
            ProviderError::NetworkFailure { message } => ConversationError::NetworkFailure(message),
            ProviderError::Timeout { message } => ConversationError::Timeout(message),
            ProviderError::ServerError { message, .. } => {
                ConversationError::GenerationFailure(message)
            }
            ProviderError::MalformedResponse { message } => ConversationError::Internal(message),
            ProviderError::Unknown { message } => ConversationError::Internal(message),
        }
    }
}
