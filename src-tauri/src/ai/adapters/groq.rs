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

pub const GROQ_PROVIDER_ID: &str = "groq";
pub const GROQ_PROVIDER_NAME: &str = "Groq Cloud";
pub const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Adapter for Groq Cloud API.
#[derive(Debug, Clone)]
pub struct GroqAdapter {
    client: Client,
    endpoint: String,
    models_endpoint: String,
}

impl Default for GroqAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GroqAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: format!("{}/chat/completions", GROQ_BASE_URL),
            models_endpoint: format!("{}/models", GROQ_BASE_URL),
        }
    }

    fn default_models() -> Vec<ModelMetadata> {
        let text_and_stream = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
        ]);
        let full_caps = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::ToolCalling,
        ]);

        vec![
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "llama-3.3-70b-versatile",
                "Meta LLaMA 3.3 70B Versatile",
                full_caps.clone(),
            ).with_context_window(128_000),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "llama-3.1-8b-instant",
                "Meta LLaMA 3.1 8B Instant",
                text_and_stream.clone(),
            ).with_context_window(128_000),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "openai/gpt-oss-120b",
                "OpenAI GPT-OSS 120B",
                text_and_stream.clone(),
            ),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "openai/gpt-oss-20b",
                "OpenAI GPT-OSS 20B",
                text_and_stream.clone(),
            ),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "deepseek-r1-distill-llama-70b",
                "DeepSeek R1 Distill LLaMA 70B",
                text_and_stream.clone(),
            ).with_context_window(128_000),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "qwen/qwen3.6-27b",
                "Qwen 3.6 27B",
                text_and_stream.clone(),
            ),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "qwen-qwq-32b",
                "Qwen QwQ 32B",
                text_and_stream.clone(),
            ),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "groq/compound",
                "Groq Compound System",
                full_caps.clone(),
            ),
            ModelMetadata::new(
                GROQ_PROVIDER_ID,
                "groq/compound-mini",
                "Groq Compound Mini",
                full_caps,
            ),
        ]
    }
}

impl Provider for GroqAdapter {
    fn id(&self) -> &str {
        GROQ_PROVIDER_ID
    }

    fn name(&self) -> &str {
        GROQ_PROVIDER_NAME
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::ToolCalling,
        ])
    }

    fn models(&self) -> Vec<ModelMetadata> {
        Self::default_models()
    }

    fn default_model(&self) -> Option<String> {
        Some("llama-3.3-70b-versatile".to_string())
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

impl TextGenerationCapability for GroqAdapter {
    fn generate<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let api_key = creds.as_deref().unwrap_or("").trim();
            if api_key.is_empty() {
                return Err(ProviderError::AuthFailure {
                    message: "Groq API key is missing. Please configure your API key in Settings.".to_string(),
                });
            }

            let body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": false
            });

            let res = self
                .client
                .post(&self.endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::NetworkFailure {
                    message: format!("Failed to reach Groq API: {}", e),
                })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res.json().await.map_err(|e| ProviderError::MalformedResponse {
                message: format!("Failed to parse Groq response: {}", e),
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

impl StreamingTextCapability for GroqAdapter {
    fn stream<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let api_key = creds.as_deref().unwrap_or("").trim();
            if api_key.is_empty() {
                return Err(ProviderError::AuthFailure {
                    message: "Groq API key is missing. Please configure your API key in Settings.".to_string(),
                });
            }

            let body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": true
            });

            let mut res = self
                .client
                .post(&self.endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::NetworkFailure {
                    message: format!("Failed to reach Groq API: {}", e),
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

impl ModelDiscoveryCapability for GroqAdapter {
    fn discover_models<'a>(
        &'a self,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelMetadata>, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let api_key = creds.as_deref().unwrap_or("").trim();
            if api_key.is_empty() {
                return Err(ProviderError::AuthFailure {
                    message: "Groq API key is required to discover models.".to_string(),
                });
            }

            let res = self
                .client
                .get(&self.models_endpoint)
                .bearer_auth(api_key)
                .send()
                .await
                .map_err(|e| ProviderError::NetworkFailure {
                    message: format!("Failed to query Groq models: {}", e),
                })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res.json().await.map_err(|e| ProviderError::MalformedResponse {
                message: format!("Failed to parse Groq models JSON: {}", e),
            })?;

            let mut discovered = Vec::new();
            let text_and_stream = CapabilitySet::from_slice(&[
                Capability::TextGeneration,
                Capability::Streaming,
            ]);

            if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                for item in data {
                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                        discovered.push(ModelMetadata::new(
                            GROQ_PROVIDER_ID,
                            id,
                            id,
                            text_and_stream.clone(),
                        ));
                    }
                }
            }

            if discovered.is_empty() {
                return Ok(Self::default_models());
            }

            discovered.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            Ok(discovered)
        })
    }
}
