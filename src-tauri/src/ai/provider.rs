use super::capabilities::CapabilitySet;
use super::errors::ProviderError;
use super::model::ModelMetadata;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// A single message in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Request payload for text generation or streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: None,
            stream: false,
        }
    }
}

/// Normalized response payload from text generation or streaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub text: String,
    pub model: String,
    pub finish_reason: Option<String>,
}

/// A streaming delta chunk produced during response generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamChunk {
    pub text: String,
    pub is_done: bool,
}

/// Capability interface for non-streaming text generation.
pub trait TextGenerationCapability: Send + Sync {
    fn generate<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>>;
}

/// Capability interface for real-time streaming text generation.
pub trait StreamingTextCapability: Send + Sync {
    fn stream<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>>;
}

/// Capability interface for dynamically fetching models from a provider endpoint.
pub trait ModelDiscoveryCapability: Send + Sync {
    fn discover_models<'a>(
        &'a self,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelMetadata>, ProviderError>> + Send + 'a>>;
}

/// The core Provider trait representing an AI service.
/// Rather than a monolithic trait containing all methods, this trait defines
/// provider identity, metadata, and accessor methods for specific capability interfaces.
pub trait Provider: std::fmt::Debug + Send + Sync {
    /// Unique identifier for the provider (e.g., "groq", "gemini", or custom ID).
    fn id(&self) -> &str;

    /// Human-friendly display name.
    fn name(&self) -> &str;

    /// The set of capabilities supported by this provider.
    fn capabilities(&self) -> CapabilitySet;

    /// The catalog of known models supported by this provider.
    fn models(&self) -> Vec<ModelMetadata>;

    /// The recommended default model for this provider, if available.
    fn default_model(&self) -> Option<String>;

    /// Capability downcasts / accessors. Returns None if the capability is unsupported.
    fn as_text_generation(&self) -> Option<&dyn TextGenerationCapability> {
        None
    }

    fn as_streaming_text(&self) -> Option<&dyn StreamingTextCapability> {
        None
    }

    fn as_model_discovery(&self) -> Option<&dyn ModelDiscoveryCapability> {
        None
    }
}
