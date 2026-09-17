//! telemetry.rs — Structured, privacy-safe voice performance metrics and telemetry.
//!
//! Enforces:
//! - Strict privacy: Opaque hashed device IDs only. Zero raw device names in telemetry payloads.
//! - Zero audio leakage: Never writes PCM samples, decibels, or speech transcripts to telemetry.
//! - Correlated identifiers: Tags every report with voice_session_id and conversation_id.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// Structured, privacy-safe voice telemetry report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceTelemetryReport {
    pub session_id: String,
    pub conversation_id: String,
    pub mode: String,
    pub provider: String,
    pub uptime_seconds: u64,
    pub total_input_frames: u64,
    pub total_output_frames: u64,
    pub stale_frames_dropped: u64,
    pub barge_in_count: u32,
    pub reconnect_count: u32,
    pub fallback_count: u32,
    pub avg_round_trip_ms: Option<u64>,
    /// Opaque hashed identifier — NEVER human-readable hardware name
    pub opaque_input_device_id: String,
    /// Opaque hashed identifier — NEVER human-readable hardware name
    pub opaque_output_device_id: String,
    pub error_count: u32,
}

/// Thread-safe voice telemetry collector.
pub struct VoiceTelemetryCollector {
    session_id: String,
    conversation_id: String,
    mode: String,
    provider: String,
    started_at: Instant,
    total_input_frames: AtomicU64,
    total_output_frames: AtomicU64,
    stale_frames_dropped: AtomicU64,
    barge_in_count: AtomicU32,
    reconnect_count: AtomicU32,
    fallback_count: AtomicU32,
    error_count: AtomicU32,
    opaque_input_device_id: RwLock<String>,
    opaque_output_device_id: RwLock<String>,
}

impl VoiceTelemetryCollector {
    pub fn new(
        session_id: impl Into<String>,
        conversation_id: impl Into<String>,
        mode: impl Into<String>,
        provider: impl Into<String>,
        opaque_input_device_id: impl Into<String>,
        opaque_output_device_id: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            conversation_id: conversation_id.into(),
            mode: mode.into(),
            provider: provider.into(),
            started_at: Instant::now(),
            total_input_frames: AtomicU64::new(0),
            total_output_frames: AtomicU64::new(0),
            stale_frames_dropped: AtomicU64::new(0),
            barge_in_count: AtomicU32::new(0),
            reconnect_count: AtomicU32::new(0),
            fallback_count: AtomicU32::new(0),
            error_count: AtomicU32::new(0),
            opaque_input_device_id: RwLock::new(opaque_input_device_id.into()),
            opaque_output_device_id: RwLock::new(opaque_output_device_id.into()),
        }
    }

    pub fn record_input_frame(&self) {
        self.total_input_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_output_frame(&self) {
        self.total_output_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_stale_frame_dropped(&self) {
        self.stale_frames_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_barge_in(&self) {
        self.barge_in_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_reconnect(&self) {
        self.reconnect_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_fallback(&self) {
        self.fallback_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_devices(&self, opaque_in: impl Into<String>, opaque_out: impl Into<String>) {
        *self.opaque_input_device_id.write().unwrap() = opaque_in.into();
        *self.opaque_output_device_id.write().unwrap() = opaque_out.into();
    }

    /// Exports an immutable, privacy-safe telemetry snapshot.
    pub fn snapshot(&self) -> VoiceTelemetryReport {
        VoiceTelemetryReport {
            session_id: self.session_id.clone(),
            conversation_id: self.conversation_id.clone(),
            mode: self.mode.clone(),
            provider: self.provider.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            total_input_frames: self.total_input_frames.load(Ordering::Relaxed),
            total_output_frames: self.total_output_frames.load(Ordering::Relaxed),
            stale_frames_dropped: self.stale_frames_dropped.load(Ordering::Relaxed),
            barge_in_count: self.barge_in_count.load(Ordering::Relaxed),
            reconnect_count: self.reconnect_count.load(Ordering::Relaxed),
            fallback_count: self.fallback_count.load(Ordering::Relaxed),
            avg_round_trip_ms: None,
            opaque_input_device_id: self.opaque_input_device_id.read().unwrap().clone(),
            opaque_output_device_id: self.opaque_output_device_id.read().unwrap().clone(),
            error_count: self.error_count.load(Ordering::Relaxed),
        }
    }
}
