//! echo.rs — Acoustic Echo Cancellation (AEC) and software ducking boundary.
//!
//! Enforces:
//! - Explicit trait boundary for acoustic feedback mitigation.
//! - Software ducking & barge-in energy threshold elevation implemented for Phase 11.
//! - Clear definition of native hardware/multi-mic AEC as a future native integration boundary.

/// Trait defining acoustic echo cancellation and playback ducking.
pub trait EchoCanceller: Send + Sync {
    /// In-place processing of captured microphone audio in the presence of reference speaker audio.
    fn process_duplex_frame(
        &mut self,
        mic_samples: &mut [f32],
        reference_playback_samples: &[f32],
        is_playback_active: bool,
    );

    /// Current software ducking attenuation factor applied during active playback [0.0, 1.0].
    fn ducking_factor(&self) -> f32;

    /// Multiplier applied to VAD energy threshold during active assistant playback to prevent false barge-in.
    fn energy_threshold_multiplier(&self, is_playback_active: bool) -> f32;
}

/// Production software ducking echo mitigation filter.
/// Attenuates mic gain and elevates barge-in threshold when assistant is actively speaking.
pub struct SoftwareDuckingEchoCanceller {
    ducking_factor: f32,
    threshold_elevation_multiplier: f32,
}

impl SoftwareDuckingEchoCanceller {
    pub fn new(ducking_factor: f32, threshold_elevation_multiplier: f32) -> Self {
        Self {
            ducking_factor: ducking_factor.clamp(0.0, 1.0),
            threshold_elevation_multiplier: threshold_elevation_multiplier.max(1.0),
        }
    }
}

impl Default for SoftwareDuckingEchoCanceller {
    fn default() -> Self {
        Self {
            ducking_factor: 0.85, // Mild mic attenuation during playback
            threshold_elevation_multiplier: 2.2, // Barge-in requires deliberate, louder voice
        }
    }
}

impl EchoCanceller for SoftwareDuckingEchoCanceller {
    fn process_duplex_frame(
        &mut self,
        mic_samples: &mut [f32],
        _reference_playback_samples: &[f32],
        is_playback_active: bool,
    ) {
        if is_playback_active && self.ducking_factor < 1.0 {
            for sample in mic_samples.iter_mut() {
                *sample *= self.ducking_factor;
            }
        }
    }

    fn ducking_factor(&self) -> f32 {
        self.ducking_factor
    }

    fn energy_threshold_multiplier(&self, is_playback_active: bool) -> f32 {
        if is_playback_active {
            self.threshold_elevation_multiplier
        } else {
            1.0
        }
    }
}
