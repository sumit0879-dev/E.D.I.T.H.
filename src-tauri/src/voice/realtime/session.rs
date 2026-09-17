//! session.rs — Realtime duplex voice session representation for E.D.I.T.H.
//!
//! Enforces:
//! - Authoritative turn management via `ConversationCore` (no local `TurnId::new()` authority).
//! - Ephemeral duplex connection tracking via `VoiceSessionId`.
//! - Monotonic sequence numbering and atomic generation tracking for instant interruption.
//! - Cooperative cancellation across the entire session graph.

use crate::events::{ConversationId, TurnId, VoiceSessionId};
use crate::task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Operational lifecycle states of a realtime duplex voice session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeSessionState {
    Connecting,
    Connected,
    Listening,
    Processing,
    Speaking,
    Interrupted,
    Reconnecting,
    Closed,
    Failed(String),
}

/// Active duplex realtime voice session spanning multiple potential conversational exchanges.
pub struct RealtimeVoiceSession {
    /// Unique identifier for this realtime voice connection.
    pub id: VoiceSessionId,
    /// Authoritative conversation thread in ConversationCore.
    pub conversation_id: ConversationId,
    /// Authoritative TurnId for the currently active exchange (governed strictly by ConversationCore).
    pub active_turn_id: Arc<RwLock<Option<TurnId>>>,
    /// High-level session operational state.
    pub state: RealtimeSessionState,
    /// Atomic generation counter for assistant speech. Incremented on every speech start or barge-in.
    pub active_generation_id: Arc<AtomicU64>,
    /// Root cancellation token for the session.
    pub cancellation_token: CancellationToken,
    /// Realtime provider name (e.g. "gemini-live", "mock-realtime").
    pub provider_id: String,
    /// Transport type (e.g. "mock", "websocket").
    pub transport_type: String,
    /// Timestamp when session was initiated.
    pub started_at: Instant,
    /// Monotonic sequence generator for inbound microphone frames.
    pub input_sequence: AtomicU64,
    /// Monotonic sequence generator for outbound assistant frames.
    pub output_sequence: AtomicU64,
}

impl RealtimeVoiceSession {
    pub fn new(
        conversation_id: ConversationId,
        provider_id: impl Into<String>,
        transport_type: impl Into<String>,
    ) -> Self {
        Self {
            id: VoiceSessionId::new(),
            conversation_id,
            active_turn_id: Arc::new(RwLock::new(None)),
            state: RealtimeSessionState::Connecting,
            active_generation_id: Arc::new(AtomicU64::new(1)),
            cancellation_token: CancellationToken::new(),
            provider_id: provider_id.into(),
            transport_type: transport_type.into(),
            started_at: Instant::now(),
            input_sequence: AtomicU64::new(1),
            output_sequence: AtomicU64::new(1),
        }
    }

    /// Increments and returns the next monotonic input frame sequence number.
    pub fn next_input_sequence(&self) -> u64 {
        self.input_sequence.fetch_add(1, Ordering::SeqCst)
    }

    /// Increments and returns the next monotonic output frame sequence number.
    pub fn next_output_sequence(&self) -> u64 {
        self.output_sequence.fetch_add(1, Ordering::SeqCst)
    }

    /// Reads the current active assistant response generation ID.
    pub fn current_generation(&self) -> u64 {
        self.active_generation_id.load(Ordering::SeqCst)
    }

    /// Atomically increments the generation counter, immediately invalidating any older frames.
    pub fn increment_generation(&self) -> u64 {
        self.active_generation_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Computes elapsed session uptime in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}
