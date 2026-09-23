//! errors.rs — Normalized error types for the E.D.I.T.H. Fallback Voice subsystem.
//!
//! Enforces strict privacy boundaries: never leaks API keys, tokens, or private secrets in error messages.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum VoiceError {
    MicrophoneUnavailable(String),
    PermissionDenied(String),
    EmptyTranscript,
    STTUnavailable(String),
    STTTimeout,
    ProviderUnavailable(String),
    TTSUnavailable(String),
    AudioDeviceUnavailable(String),
    PlaybackFailure(String),
    Cancelled(String),
    CaptureConflict(String),
    Internal(String),
}

impl fmt::Display for VoiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MicrophoneUnavailable(msg) => write!(f, "Microphone unavailable: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Microphone permission denied: {}", msg),
            Self::EmptyTranscript => write!(f, "No speech detected in audio / empty transcript"),
            Self::STTUnavailable(msg) => write!(f, "STT service unavailable: {}", msg),
            Self::STTTimeout => write!(f, "STT transcription timed out"),
            Self::ProviderUnavailable(msg) => write!(f, "Voice provider unavailable: {}", msg),
            Self::TTSUnavailable(msg) => write!(f, "TTS synthesis failed: {}", msg),
            Self::AudioDeviceUnavailable(msg) => {
                write!(f, "Audio output device unavailable: {}", msg)
            }
            Self::PlaybackFailure(msg) => write!(f, "Audio playback error: {}", msg),
            Self::Cancelled(msg) => write!(f, "Voice operation cancelled: {}", msg),
            Self::CaptureConflict(msg) => {
                write!(f, "Microphone capture ownership conflict: {}", msg)
            }
            Self::Internal(msg) => write!(f, "Internal voice error: {}", msg),
        }
    }
}

impl std::error::Error for VoiceError {}

impl From<String> for VoiceError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

impl From<&str> for VoiceError {
    fn from(s: &str) -> Self {
        Self::Internal(s.to_string())
    }
}
