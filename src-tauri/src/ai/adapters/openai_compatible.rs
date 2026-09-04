use crate::ai::capabilities::{Capability, CapabilitySet};
use crate::ai::errors::{normalize_http_error, ProviderError};
use crate::ai::model::ModelMetadata;
use crate::ai::provider::{
    GenerateRequest, GenerateResponse, ModelDiscoveryCapability, Provider, StreamChunk,
    StreamingTextCapability, TextGenerationCapability,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Universal OpenAI-compatible adapter.
/// Supports custom user-defined providers, local Ollama, vLLM, LMStudio,
/// and third-party OpenAI-compatible cloud services (OpenRouter, DeepSeek, Together, etc.).
#[derive(Debug, Clone)]
pub struct OpenAICompatibleAdapter {
    id: String,
    name: String,
    client: Client,
    chat_endpoint: String,
    models_endpoint: String,
    models: Vec<ModelMetadata>,
    default_model: Option<String>,
}

impl OpenAICompatibleAdapter {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        models: Vec<ModelMetadata>,
        default_model: Option<String>,
    ) -> Self {
        let raw_url = base_url.into().trim().trim_end_matches('/').to_string();

        let chat_endpoint = if raw_url.ends_with("/chat/completions") {
            raw_url.clone()
        } else {
            format!("{}/chat/completions", raw_url)
        };

        let models_endpoint = if raw_url.ends_with("/chat/completions") {
            let base = raw_url.trim_end_matches("/chat/completions");
            format!("{}/models", base)
        } else if raw_url.ends_with("/models") {
            raw_url
        } else {
            format!("{}/models", raw_url)
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            id: id.into(),
            name: name.into(),
            client,
            chat_endpoint,
            models_endpoint,
            models,
            default_model,
        }
    }

    /// Helper to construct from custom provider JSON object.
    pub fn from_json_value(val: &Value) -> Option<Self> {
        let id = val.get("id").and_then(|i| i.as_str())?;
        let name = val.get("name").and_then(|n| n.as_str()).unwrap_or(id);
        let base_url = val.get("baseUrl").and_then(|u| u.as_str()).unwrap_or("");
        if base_url.is_empty() {
            return None;
        }

        let mut models = Vec::new();
        let default_caps = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
        ]);

        if let Some(arr) = val.get("models").and_then(|m| m.as_array()) {
            for m in arr {
                if let Some(m_id) = m.get("id").and_then(|i| i.as_str()) {
                    let label = m.get("label").and_then(|l| l.as_str()).unwrap_or(m_id);
                    models.push(
                        ModelMetadata::new(id, m_id, label, default_caps.clone())
                            .with_custom(true),
                    );
                }
            }
        }

        let default_model = models.first().map(|m| m.model_id.clone());

        Some(Self::new(id, name, base_url, models, default_model))
    }

    pub fn chat_endpoint(&self) -> &str {
        &self.chat_endpoint
    }
}

impl Provider for OpenAICompatibleAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
        ])
    }

    fn models(&self) -> Vec<ModelMetadata> {
        self.models.clone()
    }

    fn default_model(&self) -> Option<String> {
        self.default_model
            .clone()
            .or_else(|| self.models.first().map(|m| m.model_id.clone()))
    }

    fn as_text_generation(&self) -> Option<&dyn TextGenerationCapability> {
        Some(self)
    }

    fn as_streaming_text(&self) -> Option<&dyn StreamingTextCapability> {
        Some(self)
    }

    fn as_model_discovery(&self) -> Option<&dyn ModelDiscoveryCapability> {
        Some(self)
    }
}

impl TextGenerationCapability for OpenAICompatibleAdapter {
    fn generate<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": false
            });

            let mut req_builder = self.client.post(&self.chat_endpoint).json(&body);

            if let Some(key) = creds.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                req_builder = req_builder.bearer_auth(key);
            }

            let res = req_builder.send().await.map_err(|e| ProviderError::NetworkFailure {
                message: format!("Failed to reach provider '{}' at {}: {}", self.id, self.chat_endpoint, e),
            })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res.json().await.map_err(|e| ProviderError::MalformedResponse {
                message: format!("Failed to parse response from '{}': {}", self.id, e),
            })?;

            let text = val["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let finish_reason = val["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string());

            Ok(GenerateResponse {
                text,
                model: req.model.clone(),
                finish_reason,
            })
        })
    }
}

impl StreamingTextCapability for OpenAICompatibleAdapter {
    fn stream<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": true
            });

            let mut req_builder = self.client.post(&self.chat_endpoint).json(&body);

            if let Some(key) = creds.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                req_builder = req_builder.bearer_auth(key);
            }

            let mut res = req_builder.send().await.map_err(|e| ProviderError::NetworkFailure {
                message: format!("Failed to reach provider '{}' at {}: {}", self.id, self.chat_endpoint, e),
            })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let mut full_text = String::new();
            let mut finish_reason = None;

            while let Ok(Some(chunk)) = res.chunk().await {
                let chunk_str = String::from_utf8_lossy(&chunk);
                for line in chunk_str.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("data: ") {
                        let data = &trimmed[6..].trim();
                        if *data == "[DONE]" {
                            on_chunk(StreamChunk {
                                text: String::new(),
                                is_done: true,
                            });
                            break;
                        }

                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                                full_text.push_str(content);
                                on_chunk(StreamChunk {
                                    text: content.to_string(),
                                    is_done: false,
                                });
                            }
                            if let Some(reason) = parsed["choices"][0]["finish_reason"].as_str() {
                                finish_reason = Some(reason.to_string());
                            }
                        }
                    }
                }
            }

            Ok(GenerateResponse {
                text: full_text,
                model: req.model.clone(),
                finish_reason,
            })
        })
    }
}

impl ModelDiscoveryCapability for OpenAICompatibleAdapter {
    fn discover_models<'a>(
        &'a self,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelMetadata>, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let mut req_builder = self.client.get(&self.models_endpoint);

            if let Some(key) = creds.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                req_builder = req_builder.bearer_auth(key);
            }

            let res = req_builder.send().await.map_err(|e| ProviderError::NetworkFailure {
                message: format!("Failed to query models from '{}' at {}: {}", self.id, self.models_endpoint, e),
            })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res.json().await.map_err(|e| ProviderError::MalformedResponse {
                message: format!("Failed to parse models JSON from '{}': {}", self.id, e),
            })?;

            let mut discovered = Vec::new();
            let text_and_stream = CapabilitySet::from_slice(&[
                Capability::TextGeneration,
                Capability::Streaming,
            ]);

            if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                for item in data {
                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                        let label = item
                            .get("name")
                            .or_else(|| item.get("description"))
                            .and_then(|n| n.as_str())
                            .unwrap_or(id);

                        discovered.push(
                            ModelMetadata::new(&self.id, id, label, text_and_stream.clone())
                                .with_custom(true),
                        );
                    }
                }
            }

            if discovered.is_empty() {
                return Ok(self.models.clone());
            }

            discovered.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            Ok(discovered)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_normalization() {
        let a1 = OpenAICompatibleAdapter::new("ollama", "Ollama", "http://localhost:11434/v1", vec![], None);
        assert_eq!(a1.chat_endpoint, "http://localhost:11434/v1/chat/completions");
        assert_eq!(a1.models_endpoint, "http://localhost:11434/v1/models");

        let a2 = OpenAICompatibleAdapter::new("custom", "Custom", "http://localhost:8080/v1/chat/completions/", vec![], None);
        assert_eq!(a2.chat_endpoint, "http://localhost:8080/v1/chat/completions");
        assert_eq!(a2.models_endpoint, "http://localhost:8080/v1/models");
    }

    #[test]
    fn test_from_json_value() {
        let json = json!({
            "id": "my_local",
            "name": "Local vLLM",
            "baseUrl": "http://127.0.0.1:8000/v1",
            "models": [
                {"id": "meta-llama/Llama-3-8b", "label": "Llama 3 8B"}
            ]
        });

        let adapter = OpenAICompatibleAdapter::from_json_value(&json).unwrap();
        assert_eq!(adapter.id(), "my_local");
        assert_eq!(adapter.name(), "Local vLLM");
        assert_eq!(adapter.default_model(), Some("meta-llama/Llama-3-8b".to_string()));
        assert_eq!(adapter.models().len(), 1);
    }
}
