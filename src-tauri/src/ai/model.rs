use super::capabilities::CapabilitySet;
use serde::{Deserialize, Serialize};

/// Supported modalities for an AI model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Audio,
    Image,
    Video,
}

/// Availability lifecycle status of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelAvailability {
    #[default]
    Available,
    Preview,
    Deprecated,
    Unknown,
}

/// Typed metadata describing a model within a provider's catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub capabilities: CapabilitySet,
    pub context_window: Option<u32>,
    pub modalities: Vec<Modality>,
    pub availability: ModelAvailability,
    pub is_custom: bool,
}

impl ModelMetadata {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            display_name: display_name.into(),
            capabilities,
            context_window: None,
            modalities: vec![Modality::Text],
            availability: ModelAvailability::Available,
            is_custom: false,
        }
    }

    pub fn with_context_window(mut self, context_window: u32) -> Self {
        self.context_window = Some(context_window);
        self
    }

    pub fn with_modalities(mut self, modalities: Vec<Modality>) -> Self {
        self.modalities = modalities;
        self
    }

    pub fn with_custom(mut self, is_custom: bool) -> Self {
        self.is_custom = is_custom;
        self
    }
}
