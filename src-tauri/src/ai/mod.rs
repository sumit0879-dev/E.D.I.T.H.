pub mod adapters;
pub mod capabilities;
pub mod credentials;
pub mod errors;
pub mod model;
pub mod provider;
pub mod registry;

pub use capabilities::{Capability, CapabilitySet};
pub use credentials::{CredentialStore, SettingsCredentialStore};
pub use errors::{normalize_http_error, sanitize_error_message, ProviderError};
pub use model::{Modality, ModelAvailability, ModelMetadata};
pub use provider::{
    ChatMessage, GenerateRequest, GenerateResponse, ModelDiscoveryCapability, Provider,
    StreamChunk, StreamingTextCapability, TextGenerationCapability,
};
pub use registry::{ProviderRegistry, ProviderSummary};
