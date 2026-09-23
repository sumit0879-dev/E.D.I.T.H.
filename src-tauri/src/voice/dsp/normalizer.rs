//! normalizer.rs — Input audio conditioning, high-pass filtering, and dynamic range limiting.

/// Trait defining in-place audio pre-processing and conditioning.
pub trait AudioPreprocessor: Send + Sync {
    /// In-place audio preconditioning (DC rejection filter, soft limiter).
    fn process_input_frame(&mut self, samples: &mut [f32], sample_rate: u32);
}

/// Standard audio normalizer implementing single-pole high-pass filter and soft limiter.
pub struct AudioNormalizer {
    prev_input: f32,
    prev_output: f32,
    #[allow(dead_code)]
    target_rms: f32,
}

impl AudioNormalizer {
    pub fn new(target_rms: f32) -> Self {
        Self {
            prev_input: 0.0,
            prev_output: 0.0,
            target_rms: target_rms.clamp(0.01, 0.5),
        }
    }
}

impl Default for AudioNormalizer {
    fn default() -> Self {
        Self::new(0.12)
    }
}

impl AudioPreprocessor for AudioNormalizer {
    fn process_input_frame(&mut self, samples: &mut [f32], sample_rate: u32) {
        if samples.is_empty() || sample_rate == 0 {
            return;
        }

        // 1. Single-pole High-Pass Filter (~80 Hz cutoff for DC rumble rejection)
        // RC = 1 / (2 * pi * fc), alpha = RC / (RC + dt)
        let fc = 80.0f32;
        let dt = 1.0f32 / sample_rate as f32;
        let rc = 1.0f32 / (2.0f32 * std::f32::consts::PI * fc);
        let alpha = rc / (rc + dt);

        for sample in samples.iter_mut() {
            let input = *sample;
            let output = alpha * (self.prev_output + input - self.prev_input);
            self.prev_input = input;
            self.prev_output = output;
            *sample = output;
        }

        // 2. Soft-knee peak limiting (prevents harsh digital clipping)
        for sample in samples.iter_mut() {
            if *sample > 0.95 {
                *sample = 0.95 + 0.05 * ((*sample - 0.95) / 0.05).tanh();
            } else if *sample < -0.95 {
                *sample = -0.95 - 0.05 * ((-0.95 - *sample) / 0.05).tanh();
            }
        }
    }
}
