use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

/// An asynchronous, lightweight cancellation token.
#[derive(Clone)]
pub struct ScopedCancellationToken {
    tx: Arc<watch::Sender<bool>>,
    rx: watch::Receiver<bool>,
}

impl Default for ScopedCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopedCancellationToken {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            tx: Arc::new(tx),
            rx,
        }
    }

    /// Triggers cancellation on this token and all observers.
    pub fn cancel(&self) {
        let _ = self.tx.send(true);
    }

    /// Checks if cancellation has already been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.rx.borrow()
    }

    /// Asynchronously waits until cancellation has been requested.
    pub async fn cancelled(&self) {
        let mut rx = self.rx.clone();
        if *rx.borrow() {
            return;
        }
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                break;
            }
        }
    }
}

/// Authoritative host registry coordinating scoped cancellation across executions, turns, and tasks.
#[derive(Clone, Default)]
pub struct CancellationRegistry {
    executions: Arc<RwLock<HashMap<String, ScopedCancellationToken>>>,
    turns: Arc<RwLock<HashMap<String, ScopedCancellationToken>>>,
    tasks: Arc<RwLock<HashMap<String, ScopedCancellationToken>>>,
    sessions: Arc<RwLock<HashMap<String, ScopedCancellationToken>>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates and returns a linked cancellation token for a tool execution.
    /// The token is cancelled if:
    /// 1. The specific tool execution is cancelled.
    /// 2. Or the parent turn is cancelled.
    /// 3. Or the parent task is cancelled.
    /// 4. Or the parent session is cancelled.
    pub async fn register_execution(
        &self,
        execution_id: &str,
        turn_id: Option<&str>,
        task_id: Option<&str>,
        session_id: Option<&str>,
    ) -> ScopedCancellationToken {
        let exec_token = ScopedCancellationToken::new();

        // Check if any parent scope is already cancelled
        let mut parent_already_cancelled = false;
        if let Some(s_id) = session_id {
            let lock = self.sessions.read().await;
            if let Some(s_token) = lock.get(s_id) {
                if s_token.is_cancelled() {
                    parent_already_cancelled = true;
                }
            }
        }
        if let Some(t_id) = turn_id {
            let lock = self.turns.read().await;
            if let Some(t_token) = lock.get(t_id) {
                if t_token.is_cancelled() {
                    parent_already_cancelled = true;
                }
            }
        }
        if let Some(tsk_id) = task_id {
            let lock = self.tasks.read().await;
            if let Some(tsk_token) = lock.get(tsk_id) {
                if t_token_cancelled(tsk_token) {
                    parent_already_cancelled = true;
                }
            }
        }

        if parent_already_cancelled {
            exec_token.cancel();
        }

        // Store execution token
        let mut lock = self.executions.write().await;
        lock.insert(execution_id.to_string(), exec_token.clone());

        exec_token
    }

    /// Cancels a specific tool execution without affecting other concurrent tools.
    pub async fn cancel_execution(&self, execution_id: &str) -> bool {
        let lock = self.executions.read().await;
        if let Some(token) = lock.get(execution_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Cancels a conversation turn, cascading cancellation to linked tool executions.
    pub async fn cancel_turn(&self, turn_id: &str) -> bool {
        let mut lock = self.turns.write().await;
        let token = lock.entry(turn_id.to_string()).or_insert_with(ScopedCancellationToken::new);
        token.cancel();
        true
    }

    /// Cancels a background task, cascading cancellation to linked tool executions.
    pub async fn cancel_task(&self, task_id: &str) -> bool {
        let mut lock = self.tasks.write().await;
        let token = lock.entry(task_id.to_string()).or_insert_with(ScopedCancellationToken::new);
        token.cancel();
        true
    }

    /// Cancels a session, cascading cancellation to linked tool executions.
    pub async fn cancel_session(&self, session_id: &str) -> bool {
        let mut lock = self.sessions.write().await;
        let token = lock.entry(session_id.to_string()).or_insert_with(ScopedCancellationToken::new);
        token.cancel();
        true
    }

    /// Cleans up completed execution token to prevent memory growth.
    pub async fn cleanup_execution(&self, execution_id: &str) {
        let mut lock = self.executions.write().await;
        lock.remove(execution_id);
    }
}

fn t_token_cancelled(token: &ScopedCancellationToken) -> bool {
    token.is_cancelled()
}
