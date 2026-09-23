//! dsp/mod.rs — Digital Signal Processing and acoustic conditioning boundary.
//!
//! Enforces:
//! - Strict isolation: DSP is decoupled from ConversationCore and PolicyEngine.
//! - Continuous streaming invariant: VAD drives speech detection and visualizer UI, but does
//!   NOT arbitrarily drop silence frames from full-duplex realtime streams.
//! - Echo cancellation boundary: Software ducking implemented for Phase 11; hardware AEC defined
//!   as future native integration boundary.

pub mod echo;
pub mod normalizer;
pub mod vad;

pub use echo::{EchoCanceller, SoftwareDuckingEchoCanceller};
pub use normalizer::{AudioNormalizer, AudioPreprocessor};
pub use vad::{EnergyVad, VadConfig, VoiceActivityDetector};
