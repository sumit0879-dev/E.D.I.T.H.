use super::state::TaskStatus;
use super::types::{TaskError, TaskOwner, TaskProgress, TaskSnapshot, TaskType};
use crate::events::{EventCorrelation, EventEmitter, TaskId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// A lightweight, thread-safe cancellation token with atomic cooperative signaling.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signals cancellation to all holders of this token or its clones.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Checks if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Resets the cancellation signal (useful for pooling or retry runs).
    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

/// Helper function to retrieve the current UTC timestamp in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Internal handle representing the full mutable state of a managed task.
pub struct TaskHandle {
    pub task_id: TaskId,
    pub task_type: TaskType,
    pub owner: TaskOwner,
    pub goal: String,
    pub status: TaskStatus,
    pub progress: TaskProgress,
    pub correlation: EventCorrelation,
    pub cancellation_token: CancellationToken,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub error: Option<String>,
    pub result_summary: Option<String>,
}

impl TaskHandle {
    pub fn to_snapshot(&self) -> TaskSnapshot {
        TaskSnapshot {
            task_id: self.task_id.to_string(),
            task_type: self.task_type.clone(),
            owner: self.owner.clone(),
            goal: self.goal.clone(),
            status: self.status,
            progress: self.progress.clone(),
            correlation: self.correlation.clone(),
            created_at_ms: self.created_at_ms,
            started_at_ms: self.started_at_ms,
            completed_at_ms: self.completed_at_ms,
            error: self.error.clone(),
            result_summary: self.result_summary.clone(),
        }
    }
}

/// General-purpose Task Runtime managing asynchronous work independent from human dialogue.
#[derive(Clone)]
pub struct TaskRuntime {
    tasks: Arc<RwLock<HashMap<String, Arc<RwLock<TaskHandle>>>>>,
    emitter: EventEmitter,
}

impl TaskRuntime {
    /// Creates a TaskRuntime bound to an active Tauri application EventEmitter.
    pub fn new(emitter: EventEmitter) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            emitter,
        }
    }

    /// Creates an in-memory mock TaskRuntime for headless testing.
    pub fn mock() -> Self {
        Self::new(EventEmitter::mock())
    }

    /// Returns a reference to the event emitter.
    pub fn emitter(&self) -> &EventEmitter {
        &self.emitter
    }

    /// Registers a new asynchronous task and returns its authoritative TaskId.
    pub async fn create_task(
        &self,
        task_type: TaskType,
        goal: impl Into<String>,
        mut correlation: EventCorrelation,
        owner: TaskOwner,
    ) -> TaskId {
        let task_id = TaskId::new();
        correlation.task_id = Some(task_id.to_string());

        let handle = TaskHandle {
            task_id: task_id.clone(),
            task_type,
            owner,
            goal: goal.into(),
            status: TaskStatus::Created,
            progress: TaskProgress::default(),
            correlation,
            cancellation_token: CancellationToken::new(),
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            error: None,
            result_summary: None,
        };

        let mut lock = self.tasks.write().await;
        lock.insert(task_id.to_string(), Arc::new(RwLock::new(handle)));
        task_id
    }

    /// Registers a task with an explicitly provided authoritative TaskId (e.g. from orchestrator).
    pub async fn create_task_with_id(
        &self,
        task_id: TaskId,
        task_type: TaskType,
        goal: impl Into<String>,
        mut correlation: EventCorrelation,
        owner: TaskOwner,
    ) -> Result<TaskId, TaskError> {
        correlation.task_id = Some(task_id.to_string());

        let mut lock = self.tasks.write().await;
        if lock.contains_key(task_id.as_str()) {
            return Err(TaskError::AlreadyExists(task_id.to_string()));
        }

        let handle = TaskHandle {
            task_id: task_id.clone(),
            task_type,
            owner,
            goal: goal.into(),
            status: TaskStatus::Created,
            progress: TaskProgress::default(),
            correlation,
            cancellation_token: CancellationToken::new(),
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            error: None,
            result_summary: None,
        };

        lock.insert(task_id.to_string(), Arc::new(RwLock::new(handle)));
        Ok(task_id)
    }

    /// Transitions a task to the `Running` state and emits a correlated `TaskPayload::Started` event.
    pub async fn start_task(&self, task_id: &TaskId) -> Result<(), TaskError> {
        let handle_arc = {
            let lock = self.tasks.read().await;
            lock.get(task_id.as_str())
                .cloned()
                .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?
        };

        let mut task = handle_arc.write().await;
        if !task.status.can_transition_to(&TaskStatus::Running) {
            return Err(TaskError::InvalidStateTransition {
                current: task.status.to_string(),
                attempted: TaskStatus::Running.to_string(),
            });
        }

        task.status = TaskStatus::Running;
        task.started_at_ms = Some(now_ms());

        let _ =
            self.emitter
                .emit_task_started(&task.correlation, task.task_id.as_str(), &task.goal);

        Ok(())
    }

    /// Updates dynamic progress for an active task and emits a correlated `TaskPayload::StepProgress` event.
    pub async fn update_progress(
        &self,
        task_id: &TaskId,
        step: u32,
        max_steps: u32,
        status_text: impl Into<String>,
    ) -> Result<(), TaskError> {
        let handle_arc = {
            let lock = self.tasks.read().await;
            lock.get(task_id.as_str())
                .cloned()
                .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?
        };

        let text = status_text.into();
        let mut task = handle_arc.write().await;
        task.progress = TaskProgress {
            step,
            max_steps,
            status_text: text.clone(),
        };

        let _ = self.emitter.emit_task_step_progress(
            &task.correlation,
            task.task_id.as_str(),
            step,
            max_steps,
            &text,
        );

        Ok(())
    }

    /// Marks a task as successfully completed and emits a correlated `TaskPayload::Finished` event.
    pub async fn complete_task(
        &self,
        task_id: &TaskId,
        summary: impl Into<String>,
    ) -> Result<(), TaskError> {
        let handle_arc = {
            let lock = self.tasks.read().await;
            lock.get(task_id.as_str())
                .cloned()
                .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?
        };

        let summary_str = summary.into();
        let mut task = handle_arc.write().await;
        if !task.status.can_transition_to(&TaskStatus::Completed) {
            return Err(TaskError::InvalidStateTransition {
                current: task.status.to_string(),
                attempted: TaskStatus::Completed.to_string(),
            });
        }

        task.status = TaskStatus::Completed;
        task.completed_at_ms = Some(now_ms());
        task.result_summary = Some(summary_str.clone());

        let _ = self.emitter.emit_task_finished(
            &task.correlation,
            task.task_id.as_str(),
            true,
            &summary_str,
        );

        Ok(())
    }

    /// Marks a task as failed and emits a correlated `TaskPayload::Failed` event.
    pub async fn fail_task(
        &self,
        task_id: &TaskId,
        error: impl Into<String>,
    ) -> Result<(), TaskError> {
        let handle_arc = {
            let lock = self.tasks.read().await;
            lock.get(task_id.as_str())
                .cloned()
                .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?
        };

        let error_str = error.into();
        let mut task = handle_arc.write().await;
        if !task.status.can_transition_to(&TaskStatus::Failed) {
            return Err(TaskError::InvalidStateTransition {
                current: task.status.to_string(),
                attempted: TaskStatus::Failed.to_string(),
            });
        }

        task.status = TaskStatus::Failed;
        task.completed_at_ms = Some(now_ms());
        task.error = Some(error_str.clone());

        let _ = self
            .emitter
            .emit_task_failed(&task.correlation, task.task_id.as_str(), &error_str);

        Ok(())
    }

    /// Cancels a running or queued task using its scoped cancellation token and emits `TaskPayload::Cancelled`.
    pub async fn cancel_task(
        &self,
        task_id: &TaskId,
        reason: Option<String>,
    ) -> Result<(), TaskError> {
        let handle_arc = {
            let lock = self.tasks.read().await;
            lock.get(task_id.as_str())
                .cloned()
                .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?
        };

        let mut task = handle_arc.write().await;
        if !task.status.can_transition_to(&TaskStatus::Cancelled) {
            return Err(TaskError::InvalidStateTransition {
                current: task.status.to_string(),
                attempted: TaskStatus::Cancelled.to_string(),
            });
        }

        // Trigger cooperative atomic cancellation
        task.cancellation_token.cancel();
        task.status = TaskStatus::Cancelled;
        task.completed_at_ms = Some(now_ms());
        task.error = reason.clone();

        let _ = self
            .emitter
            .emit_task_cancelled(&task.correlation, task.task_id.as_str(), reason);

        Ok(())
    }

    /// Retrieves an immutable snapshot of the task state by its TaskId.
    pub async fn get_task(&self, task_id: &TaskId) -> Option<TaskSnapshot> {
        let lock = self.tasks.read().await;
        let handle_arc = lock.get(task_id.as_str())?.clone();
        let task = handle_arc.read().await;
        Some(task.to_snapshot())
    }

    /// Obtains a clone of the task's scoped cancellation token for worker loops.
    pub async fn get_cancellation_token(&self, task_id: &TaskId) -> Option<CancellationToken> {
        let lock = self.tasks.read().await;
        let handle_arc = lock.get(task_id.as_str())?.clone();
        let task = handle_arc.read().await;
        Some(task.cancellation_token.clone())
    }

    /// Lists snapshots of all currently active (non-terminal) tasks.
    pub async fn list_active_tasks(&self) -> Vec<TaskSnapshot> {
        let lock = self.tasks.read().await;
        let mut active = Vec::new();
        for handle_arc in lock.values() {
            let task = handle_arc.read().await;
            if !task.status.is_terminal() {
                active.push(task.to_snapshot());
            }
        }
        active
    }

    /// Lists snapshots of all tasks in the runtime regardless of lifecycle state.
    pub async fn list_all_tasks(&self) -> Vec<TaskSnapshot> {
        let lock = self.tasks.read().await;
        let mut all = Vec::new();
        for handle_arc in lock.values() {
            let task = handle_arc.read().await;
            all.push(task.to_snapshot());
        }
        all
    }
}
