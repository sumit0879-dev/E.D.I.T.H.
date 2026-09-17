//! audio.rs — Canonical audio format and conversion utilities for E.D.I.T.H. Fallback Voice.
//!
//! Enforces canonical internal representation:
//! - STT / VAD canonical: 16,000 Hz, 1 channel (mono), f32 normalized [-1.0, 1.0]
//! - TTS canonical: 24,000 Hz, 1 channel (mono), f32 normalized [-1.0, 1.0]
//! - Wire/PCM conversion: 16-bit signed integer (i16, little-endian)

use serde::{Deserialize, Serialize};

/// Canonical sample rate for speech recognition and VAD processing (Hz).
pub const CANONICAL_STT_SAMPLE_RATE: u32 = 16_000;

/// Canonical sample rate for high-fidelity speech synthesis output (Hz).
pub const CANONICAL_TTS_SAMPLE_RATE: u32 = 24_000;

/// Canonical channel count for monaural voice interactions.
pub const CANONICAL_CHANNELS: u16 = 1;

/// Maximum allowable audio capture duration in milliseconds (30 seconds safety bound).
pub const MAX_CAPTURE_DURATION_MS: u64 = 30_000;

/// A canonical, normalized audio buffer containing linear floating-point PCM samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBuffer {
    /// Sample rate in Hertz (e.g. 16000 or 24000).
    pub sample_rate: u32,
    /// Channel count (typically 1 for Mono).
    pub channels: u16,
    /// Normalized audio samples [-1.0, 1.0]. Interleaved if multi-channel.
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    /// Creates a new `AudioBuffer` with given parameters and clamped f32 samples.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Self {
        let clamped: Vec<f32> = samples
            .into_iter()
            .map(|s| s.clamp(-1.0, 1.0))
            .collect();
        Self {
            sample_rate,
            channels: channels.max(1),
            samples: clamped,
        }
    }

    /// Creates an empty audio buffer.
    pub fn empty(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            channels: CANONICAL_CHANNELS,
            samples: Vec::new(),
        }
    }

    /// Computes playback/recording duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 || self.samples.is_empty() {
            return 0;
        }
        let total_frames = self.samples.len() as u64 / self.channels as u64;
        (total_frames * 1000) / self.sample_rate as u64
    }

    /// Returns whether the audio buffer contains zero samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Total sample count.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Converts a multi-channel buffer to single-channel (Mono) by averaging channels.
    pub fn to_mono(&self) -> AudioBuffer {
        if self.channels <= 1 {
            return self.clone();
        }

        let ch = self.channels as usize;
        let frame_count = self.samples.len() / ch;
        let mut mono_samples = Vec::with_capacity(frame_count);

        for i in 0..frame_count {
            let mut sum = 0.0f32;
            for c in 0..ch {
                sum += self.samples[i * ch + c];
            }
            mono_samples.push((sum / ch as f32).clamp(-1.0, 1.0));
        }

        AudioBuffer {
            sample_rate: self.sample_rate,
            channels: CANONICAL_CHANNELS,
            samples: mono_samples,
        }
    }

    /// Encodes normalized f32 samples to 16-bit signed integer PCM bytes (little-endian).
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

    /// Decodes 16-bit signed integer PCM bytes (little-endian) into an normalized `AudioBuffer`.
    pub fn from_i16_pcm(bytes: &[u8], sample_rate: u32, channels: u16) -> Self {
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
            sample_rate,
            channels: channels.max(1),
            samples,
        }
    }

    /// Performs linear interpolation resampling to the target sample rate.
    pub fn resample_linear(&self, target_sample_rate: u32) -> AudioBuffer {
        if self.sample_rate == target_sample_rate || self.samples.is_empty() {
            let mut out = self.clone();
            out.sample_rate = target_sample_rate;
            return out;
        }

        let ratio = self.sample_rate as f64 / target_sample_rate as f64;
        let ch = self.channels as usize;
        let in_frames = self.samples.len() / ch;
        let out_frames = ((in_frames as f64) / ratio).round() as usize;

        let mut out_samples = Vec::with_capacity(out_frames * ch);

        for out_idx in 0..out_frames {
            let in_pos = out_idx as f64 * ratio;
            let idx0 = in_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(in_frames.saturating_sub(1));
            let frac = (in_pos - idx0 as f64) as f32;

            for c in 0..ch {
                let s0 = self.samples[idx0 * ch + c];
                let s1 = self.samples[idx1 * ch + c];
                let interpolated = s0 + (s1 - s0) * frac;
                out_samples.push(interpolated.clamp(-1.0, 1.0));
            }
        }

        AudioBuffer {
            sample_rate: target_sample_rate,
            channels: self.channels,
            samples: out_samples,
        }
    }
}
