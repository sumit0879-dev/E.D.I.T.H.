//! stt.rs — Speech-to-Text (STT) capabilities and adapters for E.D.I.T.H.
//!
//! Explicitly supports two distinct input modes:
//! - Mode A (Web Speech Bridge): Ingests already-transcribed text from WebView2 SpeechRecognition without fabricating dummy AudioBuffers.
//! - Mode B (Raw-Audio STT): Transcribes raw AudioBuffers via STTAdapter implementations (Cloud Whisper, Local, Mock).

use super::audio::AudioBuffer;
use super::errors::VoiceError;
use crate::task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Configuration options for speech recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTOptions {
    /// BCP-47 language tag (e.g. "en-US", "hi-IN").
    pub language: String,
    /// Optional specific model ID (e.g. "whisper-large-v3", "whisper-1").
    pub model: Option<String>,
    /// Optional temperature parameter for model decoding.
    pub temperature: Option<f32>,
}

impl Default for STTOptions {
    fn default() -> Self {
        Self {
            language: "en-US".to_string(),
            model: None,
            temperature: Some(0.0),
        }
    }
}

/// A normalized speech transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    /// Recognized spoken text.
    pub text: String,
    /// Confidence score between 0.0 and 1.0, if provided by the engine.
    pub confidence: Option<f32>,
    /// Recognized language tag.
    pub language: String,
    /// Whether this transcript is final or intermediate.
    pub is_final: bool,
}

/// Mode A: Bridge for client-side Web Speech recognition events.
/// Directly normalizes already-transcribed text without creating artificial audio buffers.
pub struct WebSpeechSTTBridge;

impl WebSpeechSTTBridge {
    /// Normalizes raw client transcript input into a validated `Transcript` contract.
    pub fn normalize_transcript(
        raw_text: String,
        confidence: Option<f32>,
        language: Option<String>,
        is_final: bool,
    ) -> Result<Transcript, VoiceError> {
        let trimmed = raw_text.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::EmptyTranscript);
        }

        let lang = language
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "en-US".to_string());

        Ok(Transcript {
            text: trimmed.to_string(),
            confidence: confidence.map(|c| c.clamp(0.0, 1.0)),
            language: lang,
            is_final,
        })
    }
}

/// Mode B: Trait for engines that transcribe raw audio frames.
/// Follows Phase 1 dyn-compatible boxed future design pattern.
pub trait STTAdapter: Send + Sync {
    /// Identifying name of this STT adapter (e.g. "groq-whisper", "mock-stt").
    fn name(&self) -> &str;

    /// Transcribes an `AudioBuffer` into a normalized `Transcript`.
    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioBuffer,
        options: &'a STTOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Transcript, VoiceError>> + Send + 'a>>;
}

/// Deterministic mock STT adapter for testing.
pub struct MockSTTAdapter {
    name: String,
    mock_transcript: std::sync::RwLock<Option<String>>,
    simulated_delay_ms: std::sync::RwLock<u64>,
}

impl MockSTTAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mock_transcript: std::sync::RwLock::new(None),
            simulated_delay_ms: std::sync::RwLock::new(0),
        }
    }

    pub fn set_transcript(&self, text: impl Into<String>) {
        *self.mock_transcript.write().unwrap() = Some(text.into());
    }

    pub fn set_delay_ms(&self, ms: u64) {
        *self.simulated_delay_ms.write().unwrap() = ms;
    }
}

impl Default for MockSTTAdapter {
    fn default() -> Self {
        Self::new("mock-stt")
    }
}

impl STTAdapter for MockSTTAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioBuffer,
        options: &'a STTOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Transcript, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled(
                    "STT cancelled before transcription".to_string(),
                ));
            }

            if audio.is_empty() {
                return Err(VoiceError::EmptyTranscript);
            }

            let delay = *self.simulated_delay_ms.read().unwrap();
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                if cancellation.is_cancelled() {
                    return Err(VoiceError::Cancelled(
                        "STT cancelled during transcription".to_string(),
                    ));
                }
            }

            let text = self
                .mock_transcript
                .read()
                .unwrap()
                .clone()
                .unwrap_or_else(|| "Mock transcription test query".to_string());

            Ok(Transcript {
                text,
                confidence: Some(0.98),
                language: options.language.clone(),
                is_final: true,
            })
        })
    }
}

/// Cloud STT adapter for Whisper-compatible endpoints (Groq, OpenAI).
pub struct CloudSTTAdapter {
    provider_id: String,
    #[allow(dead_code)]
    endpoint_url: String,
    api_key: std::sync::RwLock<Option<String>>,
}

impl CloudSTTAdapter {
    pub fn new(provider_id: impl Into<String>, endpoint_url: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            endpoint_url: endpoint_url.into(),
            api_key: std::sync::RwLock::new(None),
        }
    }

    pub fn set_api_key(&self, key: Option<String>) {
        *self.api_key.write().unwrap() = key;
    }
}

impl STTAdapter for CloudSTTAdapter {
    fn name(&self) -> &str {
        &self.provider_id
    }

    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioBuffer,
        options: &'a STTOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Transcript, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled("Cloud STT cancelled".to_string()));
            }

            if audio.is_empty() {
                return Err(VoiceError::EmptyTranscript);
            }

            let key = match self.api_key.read().unwrap().clone() {
                Some(k) if !k.trim().is_empty() => k,
                _ => {
                    return Err(VoiceError::ProviderUnavailable(format!(
                        "API key not configured for STT provider '{}'",
                        self.provider_id
                    )));
                }
            };

            // Convert audio to canonical 16kHz mono WAV/PCM
            let mono = audio.to_mono();
            let canonical = if mono.sample_rate != crate::voice::audio::CANONICAL_STT_SAMPLE_RATE {
                mono.resample_linear(crate::voice::audio::CANONICAL_STT_SAMPLE_RATE)
            } else {
                mono
            };
            let wav_bytes = canonical.to_wav_bytes();

            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled(
                    "Cloud STT cancelled prior to network dispatch".to_string(),
                ));
            }

            let endpoint = if !self.endpoint_url.trim().is_empty() {
                self.endpoint_url.trim().to_string()
            } else if self.provider_id == "openai" {
                "https://api.openai.com/v1/audio/transcriptions".to_string()
            } else {
                "https://api.groq.com/openai/v1/audio/transcriptions".to_string()
            };

            let model_name = options.model.clone().unwrap_or_else(|| {
                if self.provider_id == "openai" {
                    "whisper-1".to_string()
                } else {
                    "whisper-large-v3".to_string()
                }
            });

            // Construct multipart/form-data payload with boundary
            let boundary = format!("----WebKitFormBoundary{}", uuid::Uuid::new_v4().simple());
            let mut body = Vec::new();

            // Part 1: file (audio.wav)
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n");
            body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
            body.extend_from_slice(&wav_bytes);
            body.extend_from_slice(b"\r\n");

            // Part 2: model
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
            body.extend_from_slice(model_name.as_bytes());
            body.extend_from_slice(b"\r\n");

            // Part 3: language (if provided and not empty)
            if !options.language.trim().is_empty() {
                let lang = options.language.split('-').next().unwrap_or(&options.language);
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(b"Content-Disposition: form-data; name=\"language\"\r\n\r\n");
                body.extend_from_slice(lang.as_bytes());
                body.extend_from_slice(b"\r\n");
            }

            // Part 4: response_format (json)
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"response_format\"\r\n\r\n");
            body.extend_from_slice(b"json\r\n");

            // End boundary
            body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

            let client = reqwest::Client::new();
            let res = client
                .post(&endpoint)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
                .body(body)
                .send()
                .await
                .map_err(|e| VoiceError::STTUnavailable(e.to_string()))?;

            if !res.status().is_success() {
                let status = res.status();
                let err_text = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(VoiceError::STTUnavailable(format!(
                    "STT provider returned HTTP {}: {}",
                    status, err_text
                )));
            }

            let resp_json: serde_json::Value = res
                .json()
                .await
                .map_err(|e| VoiceError::STTUnavailable(format!("Failed to parse STT response JSON: {}", e)))?;

            let text = resp_json
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if text.is_empty() {
                return Err(VoiceError::EmptyTranscript);
            }

            Ok(Transcript {
                text,
                confidence: Some(0.95),
                language: options.language.clone(),
                is_final: true,
            })
        })
    }
}
