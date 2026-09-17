//! capture.rs — Audio capture abstraction and capture owner management.
//!
//! Enforces the single-authoritative-capture-owner invariant:
//! - In Browser mode: WebView2 owns the microphone device exclusively.
//! - In Native mode: Native driver owns the microphone device exclusively.
//! - The backend never conflicts or competes for the microphone device.

use super::audio::AudioBuffer;
use super::errors::VoiceError;
use crate::events::VoiceSessionId;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// The authoritative owner of microphone audio capture for an active turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureOwner {
    BrowserWebSpeech,
    NativeDriver,
    None,
}

/// State of audio capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Idle,
    Starting,
    Recording,
    Stopping,
    Stopped,
}

/// Hardware/system abstraction for capturing microphone audio into an `AudioBuffer`.
pub trait AudioCaptureDriver: Send + Sync {
    /// Commences audio capture for a given voice session.
    fn start_capture(&self, session_id: &VoiceSessionId) -> Result<(), VoiceError>;

    /// Concludes audio capture and returns the accumulated `AudioBuffer`.
    fn stop_capture(&self) -> Result<AudioBuffer, VoiceError>;

    /// Aborts capture immediately and discards any buffered audio.
    fn cancel_capture(&self) -> Result<(), VoiceError>;

    /// Current operational state of the capture driver.
    fn state(&self) -> CaptureState;

    /// The designated ownership domain.
    fn owner(&self) -> CaptureOwner;
}

/// Bridge adapter for browser/WebView2 Web Speech capture mode.
/// Ensures the backend respects browser ownership of the microphone.
pub struct BrowserCaptureBridge {
    is_capturing: Arc<AtomicBool>,
}

impl BrowserCaptureBridge {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for BrowserCaptureBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCaptureDriver for BrowserCaptureBridge {
    fn start_capture(&self, _session_id: &VoiceSessionId) -> Result<(), VoiceError> {
        if self.is_capturing.swap(true, Ordering::SeqCst) {
            return Err(VoiceError::CaptureConflict(
                "Browser speech capture is already active".to_string(),
            ));
        }
        Ok(())
    }

    fn stop_capture(&self) -> Result<AudioBuffer, VoiceError> {
        self.is_capturing.store(false, Ordering::SeqCst);
        // Web Speech mode delivers text directly via WebSpeechSTTBridge; no dummy AudioBuffer is created.
        Ok(AudioBuffer::empty(super::audio::CANONICAL_STT_SAMPLE_RATE))
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        self.is_capturing.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn state(&self) -> CaptureState {
        if self.is_capturing.load(Ordering::SeqCst) {
            CaptureState::Recording
        } else {
            CaptureState::Idle
        }
    }

    fn owner(&self) -> CaptureOwner {
        CaptureOwner::BrowserWebSpeech
    }
}

/// Mock capture driver for deterministic unit and integration tests.
pub struct MockAudioCaptureDriver {
    state: RwLock<CaptureState>,
    mock_samples: RwLock<Option<AudioBuffer>>,
}

impl MockAudioCaptureDriver {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CaptureState::Idle),
            mock_samples: RwLock::new(None),
        }
    }

    pub fn set_mock_samples(&self, buffer: AudioBuffer) {
        *self.mock_samples.write().unwrap() = Some(buffer);
    }
}

impl Default for MockAudioCaptureDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCaptureDriver for MockAudioCaptureDriver {
    fn start_capture(&self, _session_id: &VoiceSessionId) -> Result<(), VoiceError> {
        let mut st = self.state.write().unwrap();
        if *st == CaptureState::Recording {
            return Err(VoiceError::CaptureConflict(
                "Mock capture is already recording".to_string(),
            ));
        }
        *st = CaptureState::Recording;
        Ok(())
    }

    fn stop_capture(&self) -> Result<AudioBuffer, VoiceError> {
        let mut st = self.state.write().unwrap();
        *st = CaptureState::Idle;
        let buf = self.mock_samples.read().unwrap().clone().unwrap_or_else(|| {
            // Default 1 second of 440Hz test sine tone at 16kHz
            let sr = super::audio::CANONICAL_STT_SAMPLE_RATE;
            let samples = (0..sr)
                .map(|i| ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / sr as f32).sin() * 0.5)
                .collect();
            AudioBuffer::new(sr, 1, samples)
        });
        Ok(buf)
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        *self.state.write().unwrap() = CaptureState::Idle;
        Ok(())
    }

    fn state(&self) -> CaptureState {
        *self.state.read().unwrap()
    }

    fn owner(&self) -> CaptureOwner {
        CaptureOwner::NativeDriver
    }
}
