//! computer_control.rs — E.D.I.T.H. Computer Control Ownership & Human Takeover State Machine
//!
//! Provides authoritative control ownership over desktop input actions.
//! Ensures human actions immediately preempt and pause autonomous AI input.

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Discrete states of desktop input control ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComputerControlState {
    /// Human user is in control; AI input simulation is inactive.
    UserControlled,
    /// AI is actively authorized and driving computer actions.
    AiControlled,
    /// AI execution has been paused due to human interaction or manual intervention.
    AiPaused,
    /// Operation is paused awaiting human-in-the-loop (HITL) authorization.
    WaitingForApproval,
    /// State transition in progress.
    Transitioning,
}

/// Snapshot of the current computer control state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerControlInfo {
    pub control_state: ComputerControlState,
    pub last_transition: u64,
    pub ai_task_id: Option<String>,
    pub reason: Option<String>,
}

pub struct ComputerControlManager {
    control_info: Mutex<ComputerControlInfo>,
}

impl Default for ComputerControlManager {
    fn default() -> Self {
        Self {
            control_info: Mutex::new(ComputerControlInfo {
                control_state: ComputerControlState::UserControlled,
                last_transition: current_timestamp_ms(),
                ai_task_id: None,
                reason: None,
            }),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_COMPUTER_CONTROL_MGR: Arc<ComputerControlManager> =
        Arc::new(ComputerControlManager::default());
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

impl ComputerControlManager {
    /// Returns the current control state snapshot.
    pub fn get_control_info(&self) -> ComputerControlInfo {
        let guard = self.control_info.lock().unwrap();
        guard.clone()
    }

    /// Returns the current control state, defaulting to UserControlled.
    pub fn get_control_state(&self) -> ComputerControlState {
        let guard = self.control_info.lock().unwrap();
        guard.control_state
    }

    /// Returns whether autonomous AI input actions are permitted in the current state.
    pub fn is_ai_execution_permitted(&self) -> bool {
        let guard = self.control_info.lock().unwrap();
        matches!(guard.control_state, ComputerControlState::AiControlled)
    }

    /// Requests transition to AI control for an authorized task.
    pub fn request_ai_control(
        &self,
        task_id: Option<String>,
        reason: Option<String>,
    ) -> Result<(), String> {
        let mut guard = self.control_info.lock().unwrap();
        if guard.control_state == ComputerControlState::AiPaused {
            return Err(
                "Cannot enter AI control directly while paused by operator. Resume first."
                    .to_string(),
            );
        }
        guard.control_state = ComputerControlState::AiControlled;
        guard.last_transition = current_timestamp_ms();
        guard.ai_task_id = task_id;
        guard.reason = reason;
        Ok(())
    }

    /// Pauses AI control immediately upon human operator intervention.
    pub fn pause_ai_control(&self, reason: Option<String>) -> Result<(), String> {
        let mut guard = self.control_info.lock().unwrap();
        guard.control_state = ComputerControlState::AiPaused;
        guard.last_transition = current_timestamp_ms();
        guard.reason = reason.or_else(|| Some("Operator initiated human takeover.".to_string()));
        Ok(())
    }

    /// Resumes AI control after a pause if authorized.
    pub fn resume_ai_control(&self) -> Result<(), String> {
        let mut guard = self.control_info.lock().unwrap();
        if guard.control_state != ComputerControlState::AiPaused {
            return Err("Control is not currently in paused state.".to_string());
        }
        guard.control_state = ComputerControlState::AiControlled;
        guard.last_transition = current_timestamp_ms();
        guard.reason = Some("Operator resumed autonomous execution.".to_string());
        Ok(())
    }

    /// Releases AI control back to the human operator upon task completion or cancellation.
    pub fn release_ai_control(&self) -> Result<(), String> {
        let mut guard = self.control_info.lock().unwrap();
        guard.control_state = ComputerControlState::UserControlled;
        guard.last_transition = current_timestamp_ms();
        guard.ai_task_id = None;
        guard.reason = Some("AI control released back to user.".to_string());
        Ok(())
    }

    /// Sets the state to WaitingForApproval during a HITL prompt.
    pub fn wait_for_approval(&self, reason: Option<String>) {
        let mut guard = self.control_info.lock().unwrap();
        guard.control_state = ComputerControlState::WaitingForApproval;
        guard.last_transition = current_timestamp_ms();
        guard.reason = reason;
    }
}
