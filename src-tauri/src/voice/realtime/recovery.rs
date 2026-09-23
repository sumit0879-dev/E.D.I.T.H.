//! recovery.rs — Realtime connection loss, exponential backoff, and fallback recovery manager.
//!
//! Enforces:
//! - Bounded retry budget: 3 attempts with exponential backoff.
//! - Deterministic transition to Phase 9 fallback on exhaustion.
//! - Non-blocking sleep/delay calculations.

use std::time::Duration;

/// Configuration for realtime connection recovery.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub max_reconnect_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 3,
            initial_delay_ms: 500,
            max_delay_ms: 4000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Current state of the reconnection and recovery machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryState {
    Connected,
    Reconnecting { attempt: u32, delay_ms: u64 },
    Exhausted { total_attempts: u32 },
    FallbackTriggered { reason: String },
}

/// Recovery machine governing retry delays and failover transitions.
pub struct RecoveryManager {
    config: RecoveryConfig,
    current_attempt: u32,
    is_exhausted: bool,
}

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            current_attempt: 0,
            is_exhausted: false,
        }
    }

    /// Resets attempt counters upon successful connection.
    pub fn on_connected(&mut self) {
        self.current_attempt = 0;
        self.is_exhausted = false;
    }

    /// Evaluates connection failure and computes next recovery step.
    pub fn on_disconnect(&mut self) -> RecoveryState {
        if self.current_attempt >= self.config.max_reconnect_attempts {
            self.is_exhausted = true;
            return RecoveryState::Exhausted {
                total_attempts: self.current_attempt,
            };
        }

        self.current_attempt += 1;
        let factor = self
            .config
            .backoff_multiplier
            .powi((self.current_attempt - 1) as i32);
        let raw_delay = (self.config.initial_delay_ms as f64 * factor) as u64;
        let delay_ms = raw_delay.min(self.config.max_delay_ms);

        RecoveryState::Reconnecting {
            attempt: self.current_attempt,
            delay_ms,
        }
    }

    /// Returns the delay Duration for a given attempt.
    pub fn delay_for_attempt(attempt: u32, initial_ms: u64, max_ms: u64) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        let factor = 2.0f64.powi((attempt - 1) as i32);
        let delay_ms = ((initial_ms as f64) * factor) as u64;
        Duration::from_millis(delay_ms.min(max_ms))
    }

    pub fn is_exhausted(&self) -> bool {
        self.is_exhausted
    }

    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new(RecoveryConfig::default())
    }
}
