use crate::ai::capabilities::{Capability, CapabilitySet};
use crate::ai::errors::{normalize_http_error, ProviderError};
use crate::ai::model::{Modality, ModelMetadata};
use crate::ai::provider::{
    GenerateRequest, GenerateResponse, ModelDiscoveryCapability, Provider, StreamChunk,
    StreamingTextCapability, TextGenerationCapability, ToolCall, ToolChoice,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub const GEMINI_PROVIDER_ID: &str = "gemini";
pub const GEMINI_PROVIDER_NAME: &str = "Google Gemini API";
pub const GEMINI_OPENAI_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";
pub const GEMINI_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Adapter for Google Gemini API via OpenAI-compatible endpoint.
#[derive(Debug, Clone)]
pub struct GeminiAdapter {
    client: Client,
    endpoint: String,
    models_endpoint: String,
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: GEMINI_OPENAI_URL.to_string(),
            models_endpoint: GEMINI_MODELS_URL.to_string(),
        }
    }

    fn default_models() -> Vec<ModelMetadata> {
        let text_stream_vision = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::Vision,
        ]);
        let text_and_stream =
            CapabilitySet::from_slice(&[Capability::TextGeneration, Capability::Streaming]);

        vec![
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-2.5-flash",
                "Gemini 2.5 Flash",
                text_stream_vision.clone(),
            )
            .with_modalities(vec![Modality::Text, Modality::Image])
            .with_context_window(1_000_000),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-2.5-flash-lite",
                "Gemini 2.5 Flash Lite",
                text_and_stream.clone(),
            )
            .with_context_window(1_000_000),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
                text_stream_vision.clone(),
            )
            .with_modalities(vec![Modality::Text, Modality::Image])
            .with_context_window(2_000_000),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-2.0-flash",
                "Gemini 2.0 Flash",
                text_stream_vision.clone(),
            )
            .with_modalities(vec![Modality::Text, Modality::Image])
            .with_context_window(1_000_000),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-2.0-flash-lite",
                "Gemini 2.0 Flash Lite",
                text_and_stream.clone(),
            ),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-1.5-flash",
                "Gemini 1.5 Flash",
                text_stream_vision.clone(),
            )
            .with_modalities(vec![Modality::Text, Modality::Image]),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-1.5-pro",
                "Gemini 1.5 Pro",
                text_stream_vision.clone(),
            )
            .with_modalities(vec![Modality::Text, Modality::Image])
            .with_context_window(2_000_000),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-3.7-flash",
                "Gemini 3.7 Flash",
                text_stream_vision.clone(),
            ),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-3.6-flash",
                "Gemini 3.6 Flash",
                text_stream_vision.clone(),
            ),
            ModelMetadata::new(
                GEMINI_PROVIDER_ID,
                "gemini-3.1-pro",
                "Gemini 3.1 Pro",
                text_stream_vision,
            ),
        ]
    }
}

impl Provider for GeminiAdapter {
    fn id(&self) -> &str {
        GEMINI_PROVIDER_ID
    }

    fn name(&self) -> &str {
        GEMINI_PROVIDER_NAME
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::Vision,
            Capability::ToolCalling,
        ])
    }

    fn models(&self) -> Vec<ModelMetadata> {
        Self::default_models()
    }

    fn default_model(&self) -> Option<String> {
        Some("gemini-2.5-flash".to_string())
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

fn to_gemini_name(name: &str) -> String {
    name.replace('.', "__")
}

fn from_gemini_name(name: &str) -> String {
    name.replace("__", ".")
}

impl TextGenerationCapability for GeminiAdapter {
    fn generate<'a>(
        &'a self,
        req: &'a GenerateRequest,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let api_key = creds.as_deref().unwrap_or("").trim();
            if api_key.is_empty() {
                return Err(ProviderError::AuthFailure {
                    message:
                        "Gemini API key is missing. Please configure your API key in Settings."
                            .to_string(),
                });
            }

            let mut body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": false
            });

            if let Some(ref tools) = req.tools {
                if !tools.is_empty() {
                    let tools_json: Vec<Value> = tools
                        .iter()
                        .map(|t| {
                            json!({
                                "type": "function",
                                "function": {
                                    "name": to_gemini_name(&t.name),
                                    "description": t.description,
                                    "parameters": t.parameters_schema,
                                }
                            })
                        })
                        .collect();
                    body["tools"] = json!(tools_json);

                    if let Some(ref choice) = req.tool_choice {
                        match choice {
                            ToolChoice::Auto => body["tool_choice"] = json!("auto"),
                            ToolChoice::None => body["tool_choice"] = json!("none"),
                            ToolChoice::Required => body["tool_choice"] = json!("required"),
                            ToolChoice::Specific(name) => {
                                body["tool_choice"] = json!({
                                    "type": "function",
                                    "function": { "name": to_gemini_name(name) }
                                })
                            }
                        }
                    }
                }
            }

            let res = self
                .client
                .post(&self.endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::NetworkFailure {
                    message: format!("Failed to reach Gemini API: {}", e),
                })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res
                .json()
                .await
                .map_err(|e| ProviderError::MalformedResponse {
                    message: format!("Failed to parse Gemini response: {}", e),
                })?;

            let text = val["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let finish_reason = val["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string());

            let tool_calls = val["choices"][0]["message"]["tool_calls"]
                .as_array()
                .and_then(|arr| {
                    let list: Vec<ToolCall> = arr
                        .iter()
                        .filter_map(|tc| {
                            let id = tc["id"].as_str()?.to_string();
                            let raw_name = tc["function"]["name"].as_str()?;
                            let name = from_gemini_name(raw_name);
                            let args = if let Some(s) = tc["function"]["arguments"].as_str() {
                                s.to_string()
                            } else {
                                tc["function"]["arguments"].to_string()
                            };
                            Some(ToolCall::new(id, name, args))
                        })
                        .collect();
                    if list.is_empty() {
                        None
                    } else {
                        Some(list)
                    }
                });

            Ok(GenerateResponse {
                text,
                model: req.model.clone(),
                finish_reason,
                tool_calls,
            })
        })
    }
}

impl StreamingTextCapability for GeminiAdapter {
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
                    message:
                        "Gemini API key is missing. Please configure your API key in Settings."
                            .to_string(),
                });
            }

            let mut body = json!({
                "model": req.model,
                "messages": req.messages,
                "temperature": req.temperature,
                "stream": true
            });

            if let Some(ref tools) = req.tools {
                if !tools.is_empty() {
                    let tools_json: Vec<Value> = tools
                        .iter()
                        .map(|t| {
                            json!({
                                "type": "function",
                                "function": {
                                    "name": to_gemini_name(&t.name),
                                    "description": t.description,
                                    "parameters": t.parameters_schema,
                                }
                            })
                        })
                        .collect();
                    body["tools"] = json!(tools_json);

                    if let Some(ref choice) = req.tool_choice {
                        match choice {
                            ToolChoice::Auto => body["tool_choice"] = json!("auto"),
                            ToolChoice::None => body["tool_choice"] = json!("none"),
                            ToolChoice::Required => body["tool_choice"] = json!("required"),
                            ToolChoice::Specific(name) => {
                                body["tool_choice"] = json!({
                                    "type": "function",
                                    "function": { "name": to_gemini_name(name) }
                                })
                            }
                        }
                    }
                }
            }

            let mut res = self
                .client
                .post(&self.endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::NetworkFailure {
                    message: format!("Failed to reach Gemini API: {}", e),
                })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let mut full_text = String::new();
            let mut finish_reason = None;
            let mut partial_tool_calls: std::collections::HashMap<usize, ToolCall> =
                std::collections::HashMap::new();

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
                                tool_calls: None,
                            });
                            break;
                        }

                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str()
                            {
                                full_text.push_str(content);
                                on_chunk(StreamChunk {
                                    text: content.to_string(),
                                    is_done: false,
                                    tool_calls: None,
                                });
                            }
                            if let Some(delta_tool_calls) =
                                parsed["choices"][0]["delta"]["tool_calls"].as_array()
                            {
                                for tc in delta_tool_calls {
                                    let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                                    let entry = partial_tool_calls.entry(idx).or_insert_with(|| {
                                        ToolCall {
                                            id: String::new(),
                                            name: String::new(),
                                            arguments: String::new(),
                                        }
                                    });
                                    if let Some(id) = tc["id"].as_str() {
                                        entry.id.push_str(id);
                                    }
                                    if let Some(name) = tc["function"]["name"].as_str() {
                                        entry.name.push_str(&from_gemini_name(name));
                                    }
                                    if let Some(args) = tc["function"]["arguments"].as_str() {
                                        entry.arguments.push_str(args);
                                    }
                                }
                            }
                            if let Some(reason) = parsed["choices"][0]["finish_reason"].as_str() {
                                finish_reason = Some(reason.to_string());
                            }
                        }
                    }
                }
            }

            let final_tool_calls = if partial_tool_calls.is_empty() {
                None
            } else {
                let mut entries: Vec<_> = partial_tool_calls.into_iter().collect();
                entries.sort_by_key(|(k, _)| *k);
                Some(entries.into_iter().map(|(_, tc)| tc).collect())
            };

            Ok(GenerateResponse {
                text: full_text,
                model: req.model.clone(),
                finish_reason,
                tool_calls: final_tool_calls,
            })
        })
    }
}

impl ModelDiscoveryCapability for GeminiAdapter {
    fn discover_models<'a>(
        &'a self,
        creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelMetadata>, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            let api_key = creds.as_deref().unwrap_or("").trim();
            let url = if !api_key.is_empty() {
                format!("{}?key={}", self.models_endpoint, api_key)
            } else {
                self.models_endpoint.clone()
            };

            let res =
                self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| ProviderError::NetworkFailure {
                        message: format!("Failed to query Gemini models: {}", e),
                    })?;

            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(normalize_http_error(status, &text));
            }

            let val: Value = res
                .json()
                .await
                .map_err(|e| ProviderError::MalformedResponse {
                    message: format!("Failed to parse Gemini models JSON: {}", e),
                })?;

            let mut discovered = Vec::new();
            let text_stream_vision = CapabilitySet::from_slice(&[
                Capability::TextGeneration,
                Capability::Streaming,
                Capability::Vision,
            ]);

            if let Some(data) = val.get("models").and_then(|d| d.as_array()) {
                for item in data {
                    if let Some(name) = item
                        .get("name")
                        .or_else(|| item.get("id"))
                        .and_then(|n| n.as_str())
                    {
                        let clean_id = name.strip_prefix("models/").unwrap_or(name);
                        let is_gen_model = item
                            .get("supportedGenerationMethods")
                            .and_then(|m| m.as_array())
                            .map(|methods| {
                                methods
                                    .iter()
                                    .any(|m| m.as_str() == Some("generateContent"))
                            })
                            .unwrap_or(true);

                        if is_gen_model {
                            let display_name = item
                                .get("displayName")
                                .and_then(|d| d.as_str())
                                .unwrap_or(clean_id);

                            discovered.push(
                                ModelMetadata::new(
                                    GEMINI_PROVIDER_ID,
                                    clean_id,
                                    display_name,
                                    text_stream_vision.clone(),
                                )
                                .with_modalities(vec![Modality::Text, Modality::Image]),
                            );
                        }
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
