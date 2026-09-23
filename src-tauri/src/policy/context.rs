use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Originating entity or workflow requesting the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSource {
    /// Action triggered directly by explicit user request or manual UI prompt
    UserInitiated,
    /// Action initiated autonomously by an AI generation loop
    AiAutonomous,
    /// Action initiated by an autonomous agent
    AutonomousAgent,
    /// Action initiated from an asynchronous background task
    BackgroundTask,
    /// Action initiated by the developer agent
    DevAgent,
    /// Action initiated by the autonomous browser agent
    BrowserAgent,
    /// Action initiated by automated test runner
    TestRunner,
}

/// Global or scoped system security mode affecting policy thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    /// Balanced default desktop security mode
    Standard,
    /// Maximum isolation: every mutating action requires confirmation
    Strict,
    /// Developer sandbox: trusted local developer tools are pre-authorized
    Developer,
    /// Autonomous execution mode with automated safety policies
    Autonomous,
    /// High-security restricted operational profile
    Restricted,
}

impl Default for SecurityMode {
    fn default() -> Self {
        SecurityMode::Standard
    }
}

/// Dynamic contextual environment evaluated by the Policy Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub session_id: Option<String>,
    pub conversation_id: Option<String>,
    pub turn_id: Option<String>,
    pub task_id: Option<String>,
    pub source: ActionSource,
    pub security_mode: SecurityMode,
    pub workspace_roots: Vec<PathBuf>,
    pub active_approval_id: Option<String>,
}

impl Default for PolicyContext {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            session_id: None,
            conversation_id: None,
            turn_id: None,
            task_id: None,
            source: ActionSource::UserInitiated,
            security_mode: SecurityMode::Standard,
            workspace_roots: vec![current_dir],
            active_approval_id: None,
        }
    }
}

impl PolicyContext {
    pub fn new(source: ActionSource, security_mode: SecurityMode) -> Self {
        Self {
            source,
            security_mode,
            ..Default::default()
        }
    }

    pub fn new_with_scope(
        session_id: Option<String>,
        turn_id: Option<String>,
        task_id: Option<String>,
        source: ActionSource,
        security_mode: SecurityMode,
        workspace_roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            session_id,
            conversation_id: None,
            turn_id,
            task_id,
            source,
            security_mode,
            workspace_roots,
            active_approval_id: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_turn(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl AsRef<Path>) -> Self {
        self.workspace_roots.push(root.as_ref().to_path_buf());
        self
    }

    pub fn with_approval(mut self, approval_id: impl Into<String>) -> Self {
        self.active_approval_id = Some(approval_id.into());
        self
    }
}
