use super::capabilities::CapabilitySet;
use super::errors::ProviderError;
use super::model::ModelMetadata;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Normalized tool call emitted by an LLM model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Host or provider identifier for this call (e.g. "call_123" or uuid)
    pub id: String,
    /// Canonical tool name (e.g. "browser.navigate", "computer.click")
    pub name: String,
    /// Arguments as serialized JSON string (e.g. "{\"url\":\"https://...\"}")
    pub arguments: String,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

/// Strategy for directing the model's tool selection behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific(String),
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(tool_name.into()),
        }
    }
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            role: String::new(),
            content: String::new(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
}

/// Formats a slice of ChatMessage items into the canonical OpenAI/Groq wire format.
///
/// In OpenAI/Groq REST specifications:
/// - An assistant message with tool calls MUST serialize each call as:
///   `{"id": "...", "type": "function", "function": {"name": "...", "arguments": "..."}}`
///   Missing `"type": "function"` causes Groq API to reject with `messages.X.tool_calls.0.type is missing`.
/// - A tool result message MUST serialize with `role: "tool"`, `tool_call_id: "..."`, and `content: "..."`.
pub fn format_messages_for_openai_wire(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| {
            let mut obj = serde_json::Map::new();
            obj.insert("role".to_string(), serde_json::json!(m.role));

            if let Some(ref tool_calls) = m.tool_calls {
                let tc_json: Vec<serde_json::Value> = tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments,
                            }
                        })
                    })
                    .collect();
                obj.insert("tool_calls".to_string(), serde_json::json!(tc_json));
                if !m.content.is_empty() {
                    obj.insert("content".to_string(), serde_json::json!(m.content));
                } else {
                    obj.insert("content".to_string(), serde_json::Value::Null);
                }
            } else if m.role == "tool" {
                obj.insert("content".to_string(), serde_json::json!(m.content));
                if let Some(ref tcid) = m.tool_call_id {
                    obj.insert("tool_call_id".to_string(), serde_json::json!(tcid));
                }
                if let Some(ref name) = m.name {
                    obj.insert("name".to_string(), serde_json::json!(name));
                }
            } else {
                obj.insert("content".to_string(), serde_json::json!(m.content));
                if let Some(ref name) = m.name {
                    obj.insert("name".to_string(), serde_json::json!(name));
                }
            }

            serde_json::Value::Object(obj)
        })
        .collect()
}

/// Request payload for text generation or streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::ToolDefinition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }
}

/// Normalized response payload from text generation or streaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub text: String,
    pub model: String,
    pub finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A streaming delta chunk produced during response generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamChunk {
    pub text: String,
    pub is_done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
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

/// Capability interface for real-time duplex audio sessions.
pub trait RealtimeAudioCapability: Send + Sync {
    /// Returns the provider's default realtime model name or endpoint identifier.
    fn default_realtime_model(&self) -> &str;
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

    fn as_realtime_audio(&self) -> Option<&dyn RealtimeAudioCapability> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::types::{ToolDefinition, ToolDomain};
    use serde_json::json;

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall::new("call_123", "browser.navigate", "{\"url\":\"https://google.com\"}");
        let serialized = serde_json::to_string(&call).unwrap();
        let deserialized: ToolCall = serde_json::from_str(&serialized).unwrap();
        assert_eq!(call, deserialized);
        assert_eq!(deserialized.id, "call_123");
        assert_eq!(deserialized.name, "browser.navigate");
    }

    #[test]
    fn test_chat_message_tool_result_format() {
        let msg = ChatMessage::tool_result("call_999", "computer.click", "{\"status\":\"Completed\"}");
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_999"));
        assert_eq!(msg.name.as_deref(), Some("computer.click"));
        assert_eq!(msg.content, "{\"status\":\"Completed\"}");
    }

    #[test]
    fn test_generate_request_with_tools_json() {
        let tool = ToolDefinition::new(
            "browser.navigate",
            ToolDomain::Browser,
            "Navigates to URL",
            json!({
                "type": "object",
                "required": ["url"],
                "properties": { "url": { "type": "string" } }
            }),
            false,
            5000,
        );

        let req = GenerateRequest {
            model: "llama-3.3-70b-versatile".to_string(),
            messages: vec![ChatMessage::user("Open Google")],
            temperature: 0.7,
            max_tokens: None,
            stream: true,
            tools: Some(vec![tool]),
            tool_choice: Some(ToolChoice::Auto),
        };

        let json_val = serde_json::to_value(&req).unwrap();
        assert!(json_val.get("tools").is_some());
        let tools_arr = json_val["tools"].as_array().unwrap();
        assert_eq!(tools_arr.len(), 1);
        assert_eq!(tools_arr[0]["name"], "browser.navigate");
        assert_eq!(json_val["tool_choice"], "auto");
    }

    #[test]
    fn test_format_messages_for_openai_wire_assistant_tool_calls() {
        let assistant_msg = ChatMessage::assistant_with_tools(
            "",
            vec![
                ToolCall::new("call_abc123", "computer.click", "{\"x\": 100, \"y\": 200}"),
                ToolCall::new("call_def456", "browser.navigate", "{\"url\": \"https://example.com\"}"),
            ],
        );
        let wire_json = format_messages_for_openai_wire(&[assistant_msg]);
        assert_eq!(wire_json.len(), 1);

        let msg_obj = &wire_json[0];
        assert_eq!(msg_obj["role"], "assistant");
        assert!(msg_obj["content"].is_null());

        let tool_calls = msg_obj["tool_calls"].as_array().expect("tool_calls must be an array");
        assert_eq!(tool_calls.len(), 2);

        // Verify P0-A requirement: type MUST be "function"
        assert_eq!(tool_calls[0]["id"], "call_abc123");
        assert_eq!(tool_calls[0]["type"], "function");
        assert_eq!(tool_calls[0]["function"]["name"], "computer.click");
        assert_eq!(tool_calls[0]["function"]["arguments"], "{\"x\": 100, \"y\": 200}");

        assert_eq!(tool_calls[1]["id"], "call_def456");
        assert_eq!(tool_calls[1]["type"], "function");
        assert_eq!(tool_calls[1]["function"]["name"], "browser.navigate");
    }

    #[test]
    fn test_format_messages_for_openai_wire_tool_result() {
        let result_msg = ChatMessage::tool_result(
            "call_abc123",
            "computer.click",
            "{\"status\": \"success\"}",
        );
        let wire_json = format_messages_for_openai_wire(&[result_msg]);
        assert_eq!(wire_json.len(), 1);

        let msg_obj = &wire_json[0];
        assert_eq!(msg_obj["role"], "tool");
        assert_eq!(msg_obj["tool_call_id"], "call_abc123");
        assert_eq!(msg_obj["content"], "{\"status\": \"success\"}");
    }
}
