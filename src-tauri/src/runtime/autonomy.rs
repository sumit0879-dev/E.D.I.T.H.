//! autonomy.rs — High-level autonomy and coordination state machine for E.D.I.T.H.
//!
//! Synthesizes the overall operational state of the assistant from live subsystem states
//! (ConversationCore, TaskRuntime, ToolRouter, ApprovalStore, BrowserControlManager, ComputerControlManager).

use serde::{Deserialize, Serialize};

/// High-level operational autonomy state of the E.D.I.T.H. runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AutonomyState {
    /// Ready and waiting for user input or requests. No active tasks or streaming turns.
    Idle,
    /// Actively processing or streaming a conversational turn with the user.
    Conversing,
    /// Actively executing one or more tools through the Universal Tool Runtime.
    ExecutingTool,
    /// Executing an autonomous multi-step background task (e.g., BrowserAgent, DevAgent).
    RunningTask,
    /// Execution is paused awaiting operator review for a high-risk action in the ApprovalStore.
    WaitingForApproval,
    /// A human operator has physically taken over desktop input or browser controls.
    UserTakeover,
    /// Operations have been manually paused by the operator or system.
    Paused,
    /// Subsystem is experiencing a degraded or error condition.
    Error,
}

impl AutonomyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutonomyState::Idle => "IDLE",
            AutonomyState::Conversing => "CONVERSING",
            AutonomyState::ExecutingTool => "EXECUTING_TOOL",
            AutonomyState::RunningTask => "RUNNING_TASK",
            AutonomyState::WaitingForApproval => "WAITING_FOR_APPROVAL",
            AutonomyState::UserTakeover => "USER_TAKEOVER",
            AutonomyState::Paused => "PAUSED",
            AutonomyState::Error => "ERROR",
        }
    }

    /// Whether this state represents active AI operation driving the system.
    pub fn is_active_operation(&self) -> bool {
        matches!(
            self,
            AutonomyState::Conversing | AutonomyState::ExecutingTool | AutonomyState::RunningTask
        )
    }

    /// Whether this state indicates the system is blocked waiting for human input or intervention.
    pub fn is_waiting_on_human(&self) -> bool {
        matches!(
            self,
            AutonomyState::WaitingForApproval | AutonomyState::UserTakeover
        )
    }
}

impl std::fmt::Display for AutonomyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
