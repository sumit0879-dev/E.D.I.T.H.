//! voice/mod.rs — Fallback Voice Pipeline for E.D.I.T.H.
//!
//! Subsystem architecture:
//! - `audio`: Canonical AudioBuffer, conversions, mono normalization.
//! - `capture`: AudioCaptureDriver, single authoritative capture owner.
//! - `errors`: Normalized VoiceError domain errors.
//! - `stt`: WebSpeechSTTBridge (Mode A) and STTAdapter (Mode B).
//! - `tts`: TTSAdapter, EdgeTtsAdapter, MockTtsAdapter, LocalTtsAdapter.
//! - `output`: AudioOutputDriver, single authoritative hardware playback sink.
//! - `session`: VoiceSession lifecycle, ConversationCore turn convergence, barge-in, VoiceController.

pub mod audio;
pub mod capture;
pub mod devices;
pub mod dsp;
pub mod errors;
pub mod output;
pub mod realtime;
pub mod session;
pub mod stt;
pub mod telemetry;
pub mod tts;

#[cfg(test)]
pub mod tests;

pub use audio::{
    AudioBuffer, CANONICAL_CHANNELS, CANONICAL_STT_SAMPLE_RATE, CANONICAL_TTS_SAMPLE_RATE,
    MAX_CAPTURE_DURATION_MS,
};
pub use capture::{
    AudioCaptureDriver, BrowserCaptureBridge, CaptureOwner, CaptureState, MockAudioCaptureDriver,
    NativeCpalCaptureDriver,
};
pub use devices::{compute_opaque_device_id, AudioDeviceInfo, AudioDeviceManager, AudioDevicesSummary};
pub use dsp::{
    AudioNormalizer, AudioPreprocessor, EchoCanceller, EnergyVad, SoftwareDuckingEchoCanceller,
    VadConfig, VoiceActivityDetector,
};
pub use errors::VoiceError;
pub use output::{AudioOutputDriver, MockAudioOutputDriver, RodioAudioOutputDriver};
pub use realtime::{
    AudioFrame, AudioFrameTransport, FrameDirection, MockAudioFrameTransport,
    MockRealtimeSessionAdapter, RealtimeEngineConfig, RealtimeProviderEvent,
    RealtimeSessionAdapter, RealtimeSessionState, RealtimeVoiceEngine, RealtimeVoiceSession,
    TransportEvent, TransportState,
};
pub use session::{VoiceController, VoiceSession, VoiceSessionState, VoiceStatusSummary};
pub use stt::{CloudSTTAdapter, MockSTTAdapter, STTAdapter, STTOptions, Transcript, WebSpeechSTTBridge};
pub use telemetry::{VoiceTelemetryCollector, VoiceTelemetryReport};
pub use tts::{EdgeTtsAdapter, LocalTtsAdapter, MockTtsAdapter, TTSAdapter, TTSOptions, VoiceDescriptor};
