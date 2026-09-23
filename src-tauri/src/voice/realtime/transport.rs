//! transport.rs — Transport abstraction for E.D.I.T.H. Realtime Duplex S2S.
//!
//! Provides a transport-neutral boundary:
//! - Decouples audio frames and realtime events from physical wire protocols (WebSocket, WebRTC, IPC).
//! - Enforces explicit connection lifecycle states.
//! - Includes `MockAudioFrameTransport` for deterministic, offline testing of latencies, backpressure, and disconnects.

use super::frame::AudioFrame;
use crate::voice::errors::VoiceError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, Mutex, RwLock};

/// Operational state of the realtime audio frame transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Disconnected,
    Connecting,
    Connected,
    Closing,
    Closed,
    Failed,
}

/// Normalized event received over the audio frame transport from the remote provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransportEvent {
    /// Remote provider acknowledged connection.
    Connected,
    /// Inbound audio chunk for speaker playback.
    Audio(AudioFrame),
    /// Partial or final speech transcription delta.
    TranscriptDelta { text: String, is_final: bool },
    /// Inbound tool/function invocation requested by the model.
    ToolCall {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    /// Remote provider signaling speech interruption (e.g. server VAD detected user speech).
    Interrupted { reason: String },
    /// Conversational turn complete boundary.
    TurnComplete,
    /// Transport or protocol-level error.
    Error(String),
}

/// Transport-neutral interface for bi-directional realtime audio frame streaming.
pub trait AudioFrameTransport: Send + Sync {
    /// Sends an outbound microphone audio frame to the remote provider.
    fn send_frame<'a>(
        &'a self,
        frame: AudioFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;

    /// Receives the next transport event from the provider. Returns `None` if the transport is closed.
    fn recv_event<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<TransportEvent>, VoiceError>> + Send + 'a>>;

    /// Closes the transport connection cleanly.
    fn close<'a>(
        &'a self,
        reason: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;

    /// Current operational state of the transport.
    fn state(&self) -> TransportState;
}

/// In-memory mock audio frame transport for testing.
pub struct MockAudioFrameTransport {
    state: RwLock<TransportState>,
    is_connected: AtomicBool,
    simulate_disconnect: AtomicBool,
    outbound_tx: mpsc::Sender<AudioFrame>,
    outbound_rx: Mutex<mpsc::Receiver<AudioFrame>>,
    inbound_tx: mpsc::Sender<TransportEvent>,
    inbound_rx: Mutex<mpsc::Receiver<TransportEvent>>,
}

impl MockAudioFrameTransport {
    pub fn new(capacity: usize) -> Self {
        let (out_tx, out_rx) = mpsc::channel(capacity.max(1));
        let (in_tx, in_rx) = mpsc::channel(capacity.max(1));
        Self {
            state: RwLock::new(TransportState::Connected),
            is_connected: AtomicBool::new(true),
            simulate_disconnect: AtomicBool::new(false),
            outbound_tx: out_tx,
            outbound_rx: Mutex::new(out_rx),
            inbound_tx: in_tx,
            inbound_rx: Mutex::new(in_rx),
        }
    }

    /// Pushes an event into the transport to be returned by `recv_event`.
    pub async fn inject_inbound_event(&self, event: TransportEvent) -> Result<(), VoiceError> {
        self.inbound_tx
            .send(event)
            .await
            .map_err(|e| VoiceError::Internal(format!("Failed to inject inbound event: {}", e)))
    }

    /// Pops the next frame sent via `send_frame`.
    pub async fn pop_outbound_frame(&self) -> Option<AudioFrame> {
        let mut rx = self.outbound_rx.lock().await;
        rx.recv().await
    }

    /// Sets whether the mock transport should simulate an immediate network disconnect.
    pub fn set_simulate_disconnect(&self, disconnect: bool) {
        self.simulate_disconnect.store(disconnect, Ordering::SeqCst);
    }
}

impl Default for MockAudioFrameTransport {
    fn default() -> Self {
        Self::new(32)
    }
}

impl AudioFrameTransport for MockAudioFrameTransport {
    fn send_frame<'a>(
        &'a self,
        frame: AudioFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if self.simulate_disconnect.load(Ordering::SeqCst) {
                *self.state.write().await = TransportState::Failed;
                return Err(VoiceError::ProviderUnavailable(
                    "Simulated network transport disconnect".to_string(),
                ));
            }

            let st = *self.state.read().await;
            if st != TransportState::Connected {
                return Err(VoiceError::ProviderUnavailable(format!(
                    "Transport is not connected (state: {:?})",
                    st
                )));
            }

            self.outbound_tx
                .send(frame)
                .await
                .map_err(|e| VoiceError::Internal(format!("Outbound frame channel closed: {}", e)))
        })
    }

    fn recv_event<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<TransportEvent>, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if self.simulate_disconnect.load(Ordering::SeqCst) {
                *self.state.write().await = TransportState::Failed;
                return Err(VoiceError::ProviderUnavailable(
                    "Simulated network transport disconnect".to_string(),
                ));
            }

            let mut rx = self.inbound_rx.lock().await;
            Ok(rx.recv().await)
        })
    }

    fn close<'a>(
        &'a self,
        _reason: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            self.is_connected.store(false, Ordering::SeqCst);
            *self.state.write().await = TransportState::Closed;
            Ok(())
        })
    }

    fn state(&self) -> TransportState {
        // Fast atomic path if disconnected
        if !self.is_connected.load(Ordering::SeqCst) {
            TransportState::Closed
        } else if self.simulate_disconnect.load(Ordering::SeqCst) {
            TransportState::Failed
        } else {
            TransportState::Connected
        }
    }
}
