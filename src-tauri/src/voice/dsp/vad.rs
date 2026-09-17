//! vad.rs — Voice Activity Detection (VAD) using acoustic energy and zero-crossing analysis.
//!
//! Enforces:
//! - Continuous stream invariant: VAD drives local speech detection, visualizer states, and
//!   barge-in thresholding. It does NOT drop silence frames from continuous realtime transport.

use serde::{Deserialize, Serialize};

/// Configuration parameters for Voice Activity Detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    /// Energy threshold below which audio is classified as silence [0.0, 1.0]
    pub energy_threshold: f32,
    /// Minimum zero-crossing rate for unvoiced speech / fricatives
    pub zcr_threshold: f32,
    /// Number of consecutive silent frames before transitioning to unvoiced
    pub hangover_frames: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            zcr_threshold: 0.05,
            hangover_frames: 5,
        }
    }
}

/// Abstract contract for Voice Activity Detectors.
pub trait VoiceActivityDetector: Send + Sync {
    /// Computes speech probability in [0.0, 1.0] for the given PCM buffer.
    fn detect_speech(&mut self, samples: &[f32], sample_rate: u32) -> f32;

    /// Evaluates whether the buffer contains human speech exceeding the threshold.
    fn is_speech(&mut self, samples: &[f32], sample_rate: u32) -> bool;

    /// Resets internal hangover and energy filters.
    fn reset(&mut self);
}

/// Lightweight energy and zero-crossing rate VAD.
pub struct EnergyVad {
    config: VadConfig,
    consecutive_silent_frames: u32,
    is_in_speech: bool,
}

impl EnergyVad {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            consecutive_silent_frames: 0,
            is_in_speech: false,
        }
    }

    /// Computes Root Mean Square (RMS) energy.
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    /// Computes Zero-Crossing Rate (ZCR).
    fn calculate_zcr(samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }
        let mut crossings = 0;
        for i in 1..samples.len() {
            if (samples[i] >= 0.0 && samples[i - 1] < 0.0)
                || (samples[i] < 0.0 && samples[i - 1] >= 0.0)
            {
                crossings += 1;
            }
        }
        crossings as f32 / (samples.len() - 1) as f32
    }
}

impl Default for EnergyVad {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

impl VoiceActivityDetector for EnergyVad {
    fn detect_speech(&mut self, samples: &[f32], _sample_rate: u32) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let rms = Self::calculate_rms(samples);
        let zcr = Self::calculate_zcr(samples);

        let energy_ratio = (rms / self.config.energy_threshold).clamp(0.0, 1.0);
        let zcr_ratio = (zcr / self.config.zcr_threshold).clamp(0.0, 1.0);

        // Weighted confidence score
        let confidence = (energy_ratio * 0.75) + (zcr_ratio * 0.25);

        if rms >= self.config.energy_threshold {
            self.consecutive_silent_frames = 0;
            self.is_in_speech = true;
            confidence
        } else {
            self.consecutive_silent_frames += 1;
            if self.is_in_speech && self.consecutive_silent_frames <= self.config.hangover_frames {
                // Hangover grace period keeps speech active across brief inter-word pauses
                confidence.max(0.5)
            } else {
                self.is_in_speech = false;
                confidence * 0.2
            }
        }
    }

    fn is_speech(&mut self, samples: &[f32], sample_rate: u32) -> bool {
        self.detect_speech(samples, sample_rate) >= 0.5
    }

    fn reset(&mut self) {
        self.consecutive_silent_frames = 0;
        self.is_in_speech = false;
    }
}
