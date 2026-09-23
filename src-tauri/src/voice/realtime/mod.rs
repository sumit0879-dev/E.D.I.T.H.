//! realtime/mod.rs — Realtime Duplex Speech-to-Speech Subsystem for E.D.I.T.H. (Phase 10).
//!
//! Subsystem architecture:
//! - `frame`: Canonical streaming `AudioFrame`, directional metadata, monotonic sequencing.
//! - `transport`: Transport abstraction (`AudioFrameTransport`), explicit states, and deterministic mock.
//! - `adapter`: Provider adapter boundary (`RealtimeSessionAdapter`, `RealtimeProviderEvent`).
//! - `session`: `RealtimeVoiceSession` lifecycle, turn management in `ConversationCore`.
//! - `engine`: `RealtimeVoiceEngine` coordinating transport, UTR tool calling, barge-in, and fallback.

pub mod adapter;
pub mod engine;
pub mod frame;
pub mod recovery;
pub mod session;
pub mod transport;

#[cfg(test)]
pub mod production_tests;
#[cfg(test)]
pub mod tests;

pub use adapter::{MockRealtimeSessionAdapter, RealtimeProviderEvent, RealtimeSessionAdapter};
pub use engine::{RealtimeEngineConfig, RealtimeVoiceEngine};
pub use frame::{AudioFrame, FrameDirection};
pub use recovery::{RecoveryConfig, RecoveryManager, RecoveryState};
pub use session::{RealtimeSessionState, RealtimeVoiceSession};
pub use transport::{AudioFrameTransport, MockAudioFrameTransport, TransportEvent, TransportState};
