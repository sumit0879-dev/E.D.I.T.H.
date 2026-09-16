use serde::{Deserialize, Serialize};

/// Top-level taxonomy categories for E.D.I.T.H. runtime events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "category", content = "data")]
pub enum EdithPayload {
    #[serde(rename = "stream")]
    Stream(StreamPayload),

    #[serde(rename = "task")]
    Task(TaskPayload),

    #[serde(rename = "tool")]
    Tool(ToolPayload),

    #[serde(rename = "voice")]
    Voice(VoicePayload),

    #[serde(rename = "runtime")]
    Runtime(RuntimePayload),

    #[serde(rename = "security_policy")]
    SecurityPolicy(SecurityPolicyPayload),
}

/// Lifecycle events for streaming LLM generations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stream_event", content = "data")]
pub enum StreamPayload {
    #[serde(rename = "started")]
    Started { model: String },

    #[serde(rename = "chunk")]
    Chunk {
        text: String,
        sequence_number: u64,
        is_final: bool,
    },

    #[serde(rename = "finished")]
    Finished {
        total_tokens: Option<u32>,
        finish_reason: Option<String>,
    },

    #[serde(rename = "failed")]
    Failed {
        error: String,
        error_type: Option<String>,
    },

    #[serde(rename = "cancelled")]
    Cancelled { reason: Option<String> },
}

/// Lifecycle events for autonomous background tasks (browser, dev agent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "task_event", content = "data")]
pub enum TaskPayload {
    #[serde(rename = "started")]
    Started { task_id: String, goal: String },

    #[serde(rename = "step_progress")]
    StepProgress {
        task_id: String,
        step: u32,
        max_steps: u32,
        status_text: String,
    },

    #[serde(rename = "finished")]
    Finished {
        task_id: String,
        success: bool,
        summary: String,
    },

    #[serde(rename = "failed")]
    Failed { task_id: String, error: String },

    #[serde(rename = "cancelled")]
    Cancelled {
        task_id: String,
        reason: Option<String>,
    },
}

/// Lifecycle events for tool proposals and execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool_event", content = "data")]
pub enum ToolPayload {
    #[serde(rename = "proposed")]
    Proposed {
        execution_id: String,
        tool_name: String,
        risk_level: String,
        summary: String,
    },

    #[serde(rename = "started")]
    Started {
        execution_id: String,
        tool_name: String,
    },

    #[serde(rename = "completed")]
    Completed {
        execution_id: String,
        tool_name: String,
        success: bool,
        duration_ms: u64,
        result_summary: Option<String>,
    },

    #[serde(rename = "failed")]
    Failed {
        execution_id: String,
        tool_name: String,
        error: String,
    },

    #[serde(rename = "approval_required")]
    ApprovalRequired {
        execution_id: String,
        tool_name: String,
        approval_id: String,
        reason: String,
    },

    #[serde(rename = "approved")]
    Approved {
        execution_id: String,
        tool_name: String,
        approval_id: String,
    },

    #[serde(rename = "denied")]
    Denied {
        execution_id: String,
        tool_name: String,
        reason: String,
    },

    #[serde(rename = "progress")]
    Progress {
        execution_id: String,
        tool_name: String,
        step: u32,
        status_text: String,
    },

    #[serde(rename = "cancelled")]
    Cancelled {
        execution_id: String,
        tool_name: String,
        reason: Option<String>,
    },
}

/// Events for voice capture and realtime/pipeline speech sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "voice_event", content = "data")]
pub enum VoicePayload {
    #[serde(rename = "session_started")]
    SessionStarted { session_id: String },

    #[serde(rename = "state_changed")]
    StateChanged {
        state: String,
        decibel: Option<u32>,
    },

    #[serde(rename = "barge_in")]
    BargeInTriggered { interrupted_source: String },

    #[serde(rename = "session_ended")]
    SessionEnded {
        session_id: String,
        reason: Option<String>,
    },
}

/// Global runtime status and error notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "runtime_event", content = "data")]
pub enum RuntimePayload {
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        details: Option<String>,
    },

    #[serde(rename = "notification")]
    Notification { level: String, message: String },
}

/// Events for host-enforced security evaluation, permission checks, and human approvals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "security_event", content = "data")]
pub enum SecurityPolicyPayload {
    #[serde(rename = "policy_evaluated")]
    PolicyEvaluated {
        action_domain: String,
        action_operation: String,
        risk_level: String,
        outcome: String,
        reason: String,
        approval_id: Option<String>,
    },

    #[serde(rename = "approval_requested")]
    ApprovalRequested {
        approval_id: String,
        action_domain: String,
        action_operation: String,
        risk_level: String,
        reason: String,
        expires_at_ms: u64,
    },

    #[serde(rename = "approval_resolved")]
    ApprovalResolved {
        approval_id: String,
        status: String,
        notes: Option<String>,
    },
}

