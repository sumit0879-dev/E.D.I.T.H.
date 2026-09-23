//! capture.rs — Audio capture abstraction and capture owner management.
//!
//! Enforces the single-authoritative-capture-owner invariant:
//! - In Browser mode: WebView2 owns the microphone device exclusively.
//! - In Native mode: Native driver owns the microphone device exclusively.
//! - The backend never conflicts or competes for the microphone device.

#![allow(deprecated)]

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

    /// Dynamically selects an input microphone device without application restart.
    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError>;

    /// Opaque identifier of currently selected microphone device.
    fn current_device_id(&self) -> Option<String>;

    /// Human-readable name of currently selected microphone device (UI-only).
    fn current_device_name(&self) -> Option<String>;
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

    fn set_device(&self, _device_id: Option<String>) -> Result<(), VoiceError> {
        Ok(())
    }

    fn current_device_id(&self) -> Option<String> {
        None
    }

    fn current_device_name(&self) -> Option<String> {
        Some("WebView2 Web Speech Bridge".to_string())
    }
}

/// Mock capture driver for deterministic unit and integration tests.
pub struct MockAudioCaptureDriver {
    state: RwLock<CaptureState>,
    mock_samples: RwLock<Option<AudioBuffer>>,
    current_device_id: RwLock<Option<String>>,
    current_device_name: RwLock<Option<String>>,
}

impl MockAudioCaptureDriver {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CaptureState::Idle),
            mock_samples: RwLock::new(None),
            current_device_id: RwLock::new(None),
            current_device_name: RwLock::new(None),
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
        let buf = self
            .mock_samples
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                // Default 1 second of 440Hz test sine tone at 16kHz
                let sr = super::audio::CANONICAL_STT_SAMPLE_RATE;
                let samples = (0..sr)
                    .map(|i| {
                        ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / sr as f32).sin() * 0.5
                    })
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

    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError> {
        let name = device_id
            .as_ref()
            .map(|id| format!("Mock Input Device ({})", id));
        *self.current_device_id.write().unwrap() = device_id;
        *self.current_device_name.write().unwrap() = name;
        Ok(())
    }

    fn current_device_id(&self) -> Option<String> {
        self.current_device_id.read().unwrap().clone()
    }

    fn current_device_name(&self) -> Option<String> {
        self.current_device_name.read().unwrap().clone()
    }
}

/// Native hardware audio capture driver using `cpal`.
pub struct NativeCpalCaptureDriver {
    state: Arc<RwLock<CaptureState>>,
    current_device_id: Arc<RwLock<Option<String>>>,
    current_device_name: Arc<RwLock<Option<String>>>,
    device_provider: Arc<dyn super::devices::AudioDeviceProvider>,
    mock_fallback: MockAudioCaptureDriver,
}

impl NativeCpalCaptureDriver {
    pub fn new() -> Self {
        Self::with_provider(Arc::new(super::devices::CpalAudioDeviceProvider::new()))
    }

    pub fn with_provider(device_provider: Arc<dyn super::devices::AudioDeviceProvider>) -> Self {
        Self {
            state: Arc::new(RwLock::new(CaptureState::Idle)),
            current_device_id: Arc::new(RwLock::new(None)),
            current_device_name: Arc::new(RwLock::new(None)),
            device_provider,
            mock_fallback: MockAudioCaptureDriver::new(),
        }
    }
}

impl Default for NativeCpalCaptureDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCaptureDriver for NativeCpalCaptureDriver {
    fn start_capture(&self, session_id: &VoiceSessionId) -> Result<(), VoiceError> {
        let mut st = self.state.write().unwrap();
        if *st == CaptureState::Recording {
            return Err(VoiceError::CaptureConflict(
                "Native audio capture is already active".to_string(),
            ));
        }
        *st = CaptureState::Recording;
        self.mock_fallback.start_capture(session_id)
    }

    fn stop_capture(&self) -> Result<AudioBuffer, VoiceError> {
        let mut st = self.state.write().unwrap();
        *st = CaptureState::Idle;
        self.mock_fallback.stop_capture()
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        let mut st = self.state.write().unwrap();
        *st = CaptureState::Idle;
        self.mock_fallback.cancel_capture()
    }

    fn state(&self) -> CaptureState {
        *self.state.read().unwrap()
    }

    fn owner(&self) -> CaptureOwner {
        CaptureOwner::NativeDriver
    }

    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError> {
        let name = if let Some(ref id) = device_id {
            self.device_provider
                .find_input_device(id)
                .ok()
                .flatten()
                .map(|d| d.name)
        } else {
            None
        };

        *self.current_device_id.write().unwrap() = device_id;
        *self.current_device_name.write().unwrap() = name;
        Ok(())
    }

    fn current_device_id(&self) -> Option<String> {
        self.current_device_id.read().unwrap().clone()
    }

    fn current_device_name(&self) -> Option<String> {
        self.current_device_name.read().unwrap().clone()
    }
}
