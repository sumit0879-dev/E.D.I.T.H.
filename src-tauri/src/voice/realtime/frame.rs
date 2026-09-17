//! frame.rs — Streaming AudioFrame contract and conversion utilities for E.D.I.T.H. Realtime S2S.
//!
//! Enforces:
//! - Strict separation from batch `AudioBuffer`.
//! - Monotonic sequence numbering for packet ordering.
//! - Directional distinction (`Input` mic frames vs `Output` assistant frames).
//! - Response `generation_id` association for instantaneous barge-in invalidation.

use super::super::audio::AudioBuffer;
use crate::events::VoiceSessionId;
use serde::{Deserialize, Serialize};

/// Direction of audio frame streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameDirection {
    /// Microphone capture inbound to realtime provider.
    Input,
    /// Synthesized speech outbound from realtime provider to speaker sink.
    Output,
}

/// A streaming audio frame chunk containing normalized linear PCM float samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFrame {
    /// Identifier of the active realtime voice session.
    pub session_id: VoiceSessionId,
    /// Strictly monotonic sequence number per session direction.
    pub sequence: u64,
    /// Elapsed millisecond offset relative to session start.
    pub timestamp_ms: u64,
    /// Sample rate in Hertz (e.g. 16,000 for input, 24,000 for output).
    pub sample_rate: u32,
    /// Channel count (typically 1 for Mono).
    pub channels: u16,
    /// Normalized audio samples [-1.0, 1.0].
    pub samples: Vec<f32>,
    /// Frame transmission direction.
    pub direction: FrameDirection,
    /// Assistant response generation counter (crucial for instantaneous barge-in discarding).
    pub generation_id: u64,
}

impl AudioFrame {
    /// Creates a new `AudioFrame` with clamped normalized samples.
    pub fn new(
        session_id: VoiceSessionId,
        sequence: u64,
        timestamp_ms: u64,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
        direction: FrameDirection,
        generation_id: u64,
    ) -> Self {
        let clamped: Vec<f32> = samples.into_iter().map(|s| s.clamp(-1.0, 1.0)).collect();
        Self {
            session_id,
            sequence,
            timestamp_ms,
            sample_rate,
            channels: channels.max(1),
            samples: clamped,
            direction,
            generation_id,
        }
    }

    /// Computes the duration of this streaming frame in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 || self.samples.is_empty() {
            return 0;
        }
        let total_frames = self.samples.len() as u64 / self.channels as u64;
        (total_frames * 1000) / self.sample_rate as u64
    }

    /// Returns whether this frame contains zero samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Computes Root-Mean-Square (RMS) signal energy for amplitude / VAD analysis.
    pub fn rms_energy(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = self.samples.iter().map(|&s| s * s).sum();
        (sum_sq / self.samples.len() as f32).sqrt()
    }

    /// Encodes normalized floating-point samples to 16-bit signed integer PCM bytes (little-endian).
    pub fn to_i16_pcm(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.samples.len() * 2);
        for &sample in &self.samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let s_i16 = if clamped >= 0.0 {
                (clamped * i16::MAX as f32) as i16
            } else {
                (-clamped * i16::MIN as f32) as i16
            };
            bytes.extend_from_slice(&s_i16.to_le_bytes());
        }
        bytes
    }

    /// Decodes 16-bit signed integer PCM bytes (little-endian) into an normalized `AudioFrame`.
    pub fn from_i16_pcm(
        bytes: &[u8],
        session_id: VoiceSessionId,
        sequence: u64,
        timestamp_ms: u64,
        sample_rate: u32,
        channels: u16,
        direction: FrameDirection,
        generation_id: u64,
    ) -> Self {
        let sample_count = bytes.len() / 2;
        let mut samples = Vec::with_capacity(sample_count);

        for chunk in bytes.chunks_exact(2) {
            let val = i16::from_le_bytes([chunk[0], chunk[1]]);
            let normalized = if val >= 0 {
                val as f32 / i16::MAX as f32
            } else {
                -(val as f32 / i16::MIN as f32)
            };
            samples.push(normalized.clamp(-1.0, 1.0));
        }

        Self {
            session_id,
            sequence,
            timestamp_ms,
            sample_rate,
            channels: channels.max(1),
            samples,
            direction,
            generation_id,
        }
    }

    /// Converts this streaming frame into a canonical batch `AudioBuffer`.
    pub fn to_audio_buffer(&self) -> AudioBuffer {
        AudioBuffer::new(self.sample_rate, self.channels, self.samples.clone())
    }

    /// Converts a batch `AudioBuffer` into an `AudioFrame`.
    pub fn from_audio_buffer(
        buffer: &AudioBuffer,
        session_id: VoiceSessionId,
        sequence: u64,
        timestamp_ms: u64,
        direction: FrameDirection,
        generation_id: u64,
    ) -> Self {
        Self::new(
            session_id,
            sequence,
            timestamp_ms,
            buffer.sample_rate,
            buffer.channels,
            buffer.samples.clone(),
            direction,
            generation_id,
        )
    }
}
