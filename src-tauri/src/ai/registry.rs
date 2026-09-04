use super::adapters::{GeminiAdapter, GroqAdapter, OpenAICompatibleAdapter};
use super::capabilities::CapabilitySet;
use super::errors::ProviderError;
use super::model::ModelMetadata;
use super::provider::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A lightweight summary of an available provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub capabilities: CapabilitySet,
    pub model_count: usize,
    pub default_model: Option<String>,
}

/// Central authoritative Provider Registry.
/// Single source of truth for provider identity, capabilities, and adapter resolution.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::standard_builtins()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Initializes a registry pre-populated with standard built-in providers
    /// (Groq, Gemini) and well-known OpenAI-compatible cloud services and local endpoints.
    pub fn standard_builtins() -> Self {
        let mut registry = Self::new();

        // 1. Primary built-in adapters
        registry.register(Arc::new(GroqAdapter::new()));
        registry.register(Arc::new(GeminiAdapter::new()));

        // 2. Standard OpenAI-compatible services previously referenced across call sites
        let standard_endpoints = [
            ("openai", "OpenAI", "https://api.openai.com/v1"),
            ("together", "Together AI", "https://api.together.xyz/v1"),
            ("openrouter", "OpenRouter", "https://openrouter.ai/api/v1"),
            ("deepseek", "DeepSeek", "https://api.deepseek.com"),
            ("cerebras", "Cerebras", "https://api.cerebras.ai/v1"),
            ("sambanova", "SambaNova", "https://api.sambanova.ai/v1"),
            ("mistral", "Mistral AI", "https://api.mistral.ai/v1"),
            ("huggingface", "Hugging Face", "https://api-inference.huggingface.co/v1"),
            ("local", "Local LLM (Ollama / Llama-Server)", "http://127.0.0.1:11434/v1"),
        ];

        for (id, name, base_url) in standard_endpoints {
            registry.register(Arc::new(OpenAICompatibleAdapter::new(
                id,
                name,
                base_url,
                vec![],
                None,
            )));
        }

        registry
    }

    /// Registers or updates a provider adapter.
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    /// Look up a provider by ID without failing.
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(provider_id).cloned()
    }

    /// Resolves a provider by ID or returns a typed `ProviderError`.
    pub fn resolve_provider(&self, provider_id: &str) -> Result<Arc<dyn Provider>, ProviderError> {
        self.providers
            .get(provider_id)
            .cloned()
            .ok_or_else(|| ProviderError::InvalidRequest {
                message: format!("Unknown AI provider '{}'. Please select a valid provider in Settings.", provider_id),
            })
    }

    /// Lists summaries of all currently registered providers.
    pub fn list_providers(&self) -> Vec<ProviderSummary> {
        let mut list: Vec<ProviderSummary> = self
            .providers
            .values()
            .map(|p| ProviderSummary {
                id: p.id().to_string(),
                name: p.name().to_string(),
                capabilities: p.capabilities(),
                model_count: p.models().len(),
                default_model: p.default_model(),
            })
            .collect();

        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Lists models available for a specific provider.
    pub fn list_models(&self, provider_id: &str) -> Result<Vec<ModelMetadata>, ProviderError> {
        let provider = self.resolve_provider(provider_id)?;
        Ok(provider.models())
    }

    /// Calculates effective capabilities for a provider and optional model.
    /// Effective capabilities = Provider capabilities ∩ Model capabilities.
    pub fn query_effective_capabilities(
        &self,
        provider_id: &str,
        model_id: Option<&str>,
    ) -> Result<CapabilitySet, ProviderError> {
        let provider = self.resolve_provider(provider_id)?;
        let provider_caps = provider.capabilities();

        if let Some(m_id) = model_id {
            if let Some(model_meta) = provider.models().into_iter().find(|m| m.model_id == m_id) {
                return Ok(provider_caps.intersection(&model_meta.capabilities));
            }
        }

        Ok(provider_caps)
    }

    /// Parses custom provider definitions from settings JSON and dynamically registers adapters.
    pub fn load_custom_providers(&mut self, custom_providers_json: &str) {
        if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(custom_providers_json) {
            for cp in list {
                if let Some(adapter) = OpenAICompatibleAdapter::from_json_value(&cp) {
                    self.register(Arc::new(adapter));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_builtins() {
        let reg = ProviderRegistry::standard_builtins();
        assert!(reg.get("groq").is_some());
        assert!(reg.get("gemini").is_some());
        assert!(reg.get("local").is_some());
        assert!(reg.get("openai").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_resolve_provider_error() {
        let reg = ProviderRegistry::standard_builtins();
        let err = match reg.resolve_provider("invalid_provider") {
            Err(e) => e,
            Ok(_) => panic!("Expected error for invalid provider"),
        };
        match err {
            ProviderError::InvalidRequest { message } => {
                assert!(message.contains("Unknown AI provider 'invalid_provider'"));
            }
            _ => panic!("Expected InvalidRequest, got {:?}", err),
        }
    }

    #[test]
    fn test_custom_provider_loading() {
        let mut reg = ProviderRegistry::standard_builtins();
        let custom_json = r#"[
            {
                "id": "my_vllm",
                "name": "Local vLLM Server",
                "baseUrl": "http://localhost:8000/v1",
                "models": [
                    {"id": "mistral-7b-instruct", "label": "Mistral 7B Instruct"}
                ]
            }
        ]"#;

        reg.load_custom_providers(custom_json);
        let resolved = reg.resolve_provider("my_vllm").unwrap();
        assert_eq!(resolved.id(), "my_vllm");
        assert_eq!(resolved.name(), "Local vLLM Server");
        assert_eq!(resolved.models().len(), 1);
        assert_eq!(resolved.models()[0].model_id, "mistral-7b-instruct");
    }

    #[test]
    fn test_query_effective_capabilities() {
        let reg = ProviderRegistry::standard_builtins();
        // Groq has Text, Streaming, ToolCalling
        // llama-3.1-8b-instant only has Text, Streaming
        let caps = reg
            .query_effective_capabilities("groq", Some("llama-3.1-8b-instant"))
            .unwrap();

        assert!(caps.has(crate::ai::capabilities::Capability::TextGeneration));
        assert!(caps.has(crate::ai::capabilities::Capability::Streaming));
        assert!(!caps.has(crate::ai::capabilities::Capability::ToolCalling));
    }
}
