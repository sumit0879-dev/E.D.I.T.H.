//! adapter.rs — Provider adapter boundary for E.D.I.T.H. Realtime S2S.
//!
//! Isolates vendor-specific protocols (Gemini Multimodal Live, OpenAI Realtime) from the core voice runtime.
//! Enforces:
//! - Normalized `RealtimeProviderEvent` vocabulary.
//! - Dyn-compatible asynchronous adapter interface.
//! - `MockRealtimeSessionAdapter` for deterministic verification.

use super::frame::AudioFrame;
use super::transport::{AudioFrameTransport, TransportEvent};
use crate::tools::ToolExecutionResult;
use crate::voice::errors::VoiceError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Normalized events emitted by a realtime voice provider session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RealtimeProviderEvent {
    /// Provider connected and session initialized.
    Connected,
    /// Partial or finalized speech transcription delta.
    TranscriptDelta { text: String, is_final: bool },
    /// Synthesized speech audio chunk for immediate hardware playback.
    AudioDelta { frame: AudioFrame },
    /// Model-requested tool invocation requiring execution through Universal Tool Runtime.
    ToolCall {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    /// Interruption signal from provider (e.g. server-side VAD).
    Interrupted { reason: String },
    /// Conversational exchange boundary signaled by provider.
    TurnComplete,
    /// Provider or network error.
    Error { message: String },
}

/// Interface for an active realtime duplex voice provider session.
pub trait RealtimeSessionAdapter: Send + Sync {
    /// Transmits an outbound captured audio frame to the provider.
    fn send_audio<'a>(
        &'a self,
        frame: AudioFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;

    /// Awaits the next normalized event from the provider. Returns `None` on clean termination.
    fn next_event<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<RealtimeProviderEvent>, VoiceError>> + Send + 'a>>;

    /// Transmits the result of a tool execution back to the provider model.
    fn send_tool_result<'a>(
        &'a self,
        call_id: String,
        result: ToolExecutionResult,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;

    /// Signals an immediate user interruption to the provider.
    fn interrupt(&self) -> Result<(), VoiceError>;

    /// Closes the provider session cleanly.
    fn close(&self) -> Result<(), VoiceError>;
}

/// Deterministic mock realtime session adapter backed by an `AudioFrameTransport`.
pub struct MockRealtimeSessionAdapter {
    transport: Arc<dyn AudioFrameTransport>,
    interrupted_flag: AtomicBool,
    closed_flag: AtomicBool,
}

impl MockRealtimeSessionAdapter {
    pub fn new(transport: Arc<dyn AudioFrameTransport>) -> Self {
        Self {
            transport,
            interrupted_flag: AtomicBool::new(false),
            closed_flag: AtomicBool::new(false),
        }
    }

    pub fn was_interrupted(&self) -> bool {
        self.interrupted_flag.load(Ordering::SeqCst)
    }

    pub fn was_closed(&self) -> bool {
        self.closed_flag.load(Ordering::SeqCst)
    }
}

impl RealtimeSessionAdapter for MockRealtimeSessionAdapter {
    fn send_audio<'a>(
        &'a self,
        frame: AudioFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if self.closed_flag.load(Ordering::SeqCst) {
                return Err(VoiceError::Internal(
                    "Session adapter is closed".to_string(),
                ));
            }
            self.transport.send_frame(frame).await
        })
    }

    fn next_event<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<RealtimeProviderEvent>, VoiceError>> + Send + 'a>>
    {
        Box::pin(async move {
            if self.closed_flag.load(Ordering::SeqCst) {
                return Ok(None);
            }

            match self.transport.recv_event().await? {
                Some(TransportEvent::Connected) => Ok(Some(RealtimeProviderEvent::Connected)),
                Some(TransportEvent::Audio(frame)) => {
                    Ok(Some(RealtimeProviderEvent::AudioDelta { frame }))
                }
                Some(TransportEvent::TranscriptDelta { text, is_final }) => {
                    Ok(Some(RealtimeProviderEvent::TranscriptDelta {
                        text,
                        is_final,
                    }))
                }
                Some(TransportEvent::ToolCall {
                    call_id,
                    tool_name,
                    arguments,
                }) => Ok(Some(RealtimeProviderEvent::ToolCall {
                    call_id,
                    tool_name,
                    arguments,
                })),
                Some(TransportEvent::Interrupted { reason }) => {
                    Ok(Some(RealtimeProviderEvent::Interrupted { reason }))
                }
                Some(TransportEvent::TurnComplete) => Ok(Some(RealtimeProviderEvent::TurnComplete)),
                Some(TransportEvent::Error(message)) => {
                    Ok(Some(RealtimeProviderEvent::Error { message }))
                }
                None => Ok(None),
            }
        })
    }

    fn send_tool_result<'a>(
        &'a self,
        _call_id: String,
        _result: ToolExecutionResult,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            // In mock mode, sending tool result succeeds immediately
            Ok(())
        })
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        self.interrupted_flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn close(&self) -> Result<(), VoiceError> {
        self.closed_flag.store(true, Ordering::SeqCst);
        Ok(())
    }
}
