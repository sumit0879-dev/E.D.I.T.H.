//! tts.rs — Text-to-Speech (TTS) capabilities and adapters for E.D.I.T.H.
//!
//! Enforces adapter-based synthesis isolated behind the `TTSAdapter` trait.
//! Integrates Azure EdgeTTS and Mock engines, outputting canonical `AudioBuffer` representations.

use super::audio::{AudioBuffer, CANONICAL_TTS_SAMPLE_RATE};
use super::errors::VoiceError;
use crate::task::CancellationToken;
use edge_tts_rust::{EdgeTtsClient, SpeakOptions};
use regex::Regex;
use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;

/// Configuration options for speech synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSOptions {
    /// Target voice identifier (e.g. "hi-IN-SwaraNeural", "en-US-JennyNeural").
    pub voice: String,
    /// Target language code.
    pub language: Option<String>,
    /// Speed multiplier (1.0 is default normal speed).
    pub rate: Option<f32>,
    /// Pitch modifier.
    pub pitch: Option<f32>,
    /// Volume scaling (0.0 to 1.0).
    pub volume: Option<f32>,
}

impl Default for TTSOptions {
    fn default() -> Self {
        Self {
            voice: "hi-IN-SwaraNeural".to_string(),
            language: Some("hi-IN".to_string()),
            rate: Some(1.0),
            pitch: Some(1.0),
            volume: Some(1.0),
        }
    }
}

/// Metadata descriptor for an available speech synthesis voice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceDescriptor {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: String,
}

/// Trait for speech synthesis adapters.
/// Follows Phase 1 dyn-compatible boxed future design pattern.
pub trait TTSAdapter: Send + Sync {
    /// Identifying name of this TTS adapter (e.g. "edge-tts", "mock-tts").
    fn name(&self) -> &str;

    /// Synthesizes text into a canonical `AudioBuffer`.
    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        options: &'a TTSOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AudioBuffer, VoiceError>> + Send + 'a>>;

    /// Lists supported voices for this adapter.
    fn list_voices<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceDescriptor>, VoiceError>> + Send + 'a>>;
}

/// Cloud TTS adapter using Microsoft Azure EdgeTTS.
pub struct EdgeTtsAdapter;

impl EdgeTtsAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EdgeTtsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TTSAdapter for EdgeTtsAdapter {
    fn name(&self) -> &str {
        "edge-tts"
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        options: &'a TTSOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AudioBuffer, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled(
                    "TTS cancelled prior to synthesis".to_string(),
                ));
            }

            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(AudioBuffer::empty(CANONICAL_TTS_SAMPLE_RATE));
            }

            // Sanitize markdown formatting and artifacts
            let re = Regex::new(r"[*`_~#]").map_err(|e| VoiceError::Internal(e.to_string()))?;
            let clean_text = re.replace_all(trimmed, "").to_string();

            let client = EdgeTtsClient::new().map_err(|e| {
                VoiceError::TTSUnavailable(format!("Failed to initialize EdgeTTS client: {}", e))
            })?;

            let voice = if options.voice.is_empty() {
                "hi-IN-SwaraNeural".to_string()
            } else {
                options.voice.clone()
            };

            let speak_opts = SpeakOptions {
                voice,
                ..Default::default()
            };

            // Synthesize via edge_tts_rust
            let res = client
                .synthesize(clean_text, speak_opts)
                .await
                .map_err(|e| {
                    VoiceError::TTSUnavailable(format!("EdgeTTS synthesis request failed: {}", e))
                })?;

            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled(
                    "TTS cancelled during/after synthesis".to_string(),
                ));
            }

            // Decode MP3 payload into canonical AudioBuffer via Rodio Decoder
            let cursor = Cursor::new(res.audio);
            let decoder = Decoder::new(cursor).map_err(|e| {
                VoiceError::PlaybackFailure(format!(
                    "Failed to decode synthesized MP3 stream: {}",
                    e
                ))
            })?;

            let sample_rate = decoder.sample_rate().get();
            let channels = decoder.channels().get();
            let samples: Vec<f32> = decoder.collect();

            Ok(AudioBuffer::new(sample_rate, channels, samples))
        })
    }

    fn list_voices<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceDescriptor>, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(vec![
                VoiceDescriptor {
                    id: "hi-IN-SwaraNeural".to_string(),
                    name: "Microsoft Swara Online (Natural) - Hindi (India)".to_string(),
                    language: "hi-IN".to_string(),
                    gender: "Female".to_string(),
                },
                VoiceDescriptor {
                    id: "hi-IN-MadhurNeural".to_string(),
                    name: "Microsoft Madhur Online (Natural) - Hindi (India)".to_string(),
                    language: "hi-IN".to_string(),
                    gender: "Male".to_string(),
                },
                VoiceDescriptor {
                    id: "en-US-JennyNeural".to_string(),
                    name: "Microsoft Jenny Online (Natural) - English (United States)".to_string(),
                    language: "en-US".to_string(),
                    gender: "Female".to_string(),
                },
                VoiceDescriptor {
                    id: "en-US-GuyNeural".to_string(),
                    name: "Microsoft Guy Online (Natural) - English (United States)".to_string(),
                    language: "en-US".to_string(),
                    gender: "Male".to_string(),
                },
            ])
        })
    }
}

/// Deterministic mock TTS adapter for testing.
pub struct MockTtsAdapter {
    name: String,
    simulated_delay_ms: std::sync::RwLock<u64>,
}

impl MockTtsAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            simulated_delay_ms: std::sync::RwLock::new(0),
        }
    }

    pub fn set_delay_ms(&self, ms: u64) {
        *self.simulated_delay_ms.write().unwrap() = ms;
    }
}

impl Default for MockTtsAdapter {
    fn default() -> Self {
        Self::new("mock-tts")
    }
}

impl TTSAdapter for MockTtsAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        _options: &'a TTSOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AudioBuffer, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(VoiceError::Cancelled(
                    "Mock TTS cancelled prior to synthesis".to_string(),
                ));
            }

            let delay = *self.simulated_delay_ms.read().unwrap();
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                if cancellation.is_cancelled() {
                    return Err(VoiceError::Cancelled(
                        "Mock TTS cancelled during synthesis".to_string(),
                    ));
                }
            }

            if text.trim().is_empty() {
                return Ok(AudioBuffer::empty(CANONICAL_TTS_SAMPLE_RATE));
            }

            // Generate 500ms of 440Hz test sine tone at 24kHz
            let sr = CANONICAL_TTS_SAMPLE_RATE;
            let sample_count = (sr as f32 * 0.5) as usize;
            let samples = (0..sample_count)
                .map(|i| ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / sr as f32).sin() * 0.2)
                .collect();

            Ok(AudioBuffer::new(sr, 1, samples))
        })
    }

    fn list_voices<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceDescriptor>, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(vec![VoiceDescriptor {
                id: "mock-voice-1".to_string(),
                name: "Mock Voice Standard".to_string(),
                language: "en-US".to_string(),
                gender: "Neutral".to_string(),
            }])
        })
    }
}

/// Stub adapter for local Kokoro ONNX TTS engine (preserves boundary for future local models).
pub struct LocalTtsAdapter;

impl TTSAdapter for LocalTtsAdapter {
    fn name(&self) -> &str {
        "local-kokoro"
    }

    fn synthesize<'a>(
        &'a self,
        _text: &'a str,
        _options: &'a TTSOptions,
        _cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AudioBuffer, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            Err(VoiceError::TTSUnavailable(
                "Local Kokoro TTS engine is currently disabled to optimize app binary size."
                    .to_string(),
            ))
        })
    }

    fn list_voices<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceDescriptor>, VoiceError>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![]) })
    }
}
