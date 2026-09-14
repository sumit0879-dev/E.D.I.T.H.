use super::envelope::{EdithEventEnvelope, EventCorrelation};
use super::payload::{EdithPayload, StreamPayload, TaskPayload, ToolPayload};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const EDITH_EVENT_CHANNEL: &str = "edith-event";
pub const LEGACY_CHAT_CHUNK_CHANNEL: &str = "chat-chunk";

/// Central event emitter coordinating typed, correlated event emission across Tauri IPC.
#[derive(Clone)]
pub struct EventEmitter {
    app: Option<AppHandle>,
    mock_events: Option<Arc<Mutex<Vec<EdithEventEnvelope<EdithPayload>>>>>,
}

impl EventEmitter {
    /// Creates an emitter tied to the active Tauri application runtime.
    pub fn new(app: AppHandle) -> Self {
        Self {
            app: Some(app),
            mock_events: None,
        }
    }

    /// Convenience constructor taking an AppHandle reference.
    pub fn from_app(app: &AppHandle) -> Self {
        Self::new(app.clone())
    }

    /// Creates an in-memory mock emitter for headless automated unit testing.
    pub fn mock() -> Self {
        Self {
            app: None,
            mock_events: Some(Arc::new(Mutex::new(Vec::new()))),
        }
    }

    /// Returns recorded mock events (available only in mock mode).
    pub fn get_mock_events(&self) -> Vec<EdithEventEnvelope<EdithPayload>> {
        if let Some(ref m) = self.mock_events {
            m.lock().unwrap().clone()
        } else {
            Vec::new()
        }
    }

    /// Clears recorded mock events.
    pub fn clear_mock_events(&self) {
        if let Some(ref m) = self.mock_events {
            m.lock().unwrap().clear();
        }
    }

    /// Emits a typed envelope across Tauri IPC.
    pub fn emit_envelope<T: Serialize + Clone>(
        &self,
        envelope: &EdithEventEnvelope<T>,
    ) -> Result<(), String> {
        if let Some(ref app) = self.app {
            app.emit(EDITH_EVENT_CHANNEL, envelope)
                .map_err(|e| format!("Failed to emit event {}: {}", envelope.event_id, e))?;
        }
        Ok(())
    }

    /// Emits a generic correlated payload wrapped in an EdithEventEnvelope.
    pub fn emit_payload(
        &self,
        correlation: EventCorrelation,
        payload: EdithPayload,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        let envelope = EdithEventEnvelope::new(correlation, payload);

        if let Some(ref mock) = self.mock_events {
            mock.lock().unwrap().push(envelope.clone());
        }

        self.emit_envelope(&envelope)?;
        Ok(envelope)
    }

    // -------------------------------------------------------------------------
    // Stream Lifecycle Emitters
    // -------------------------------------------------------------------------

    pub fn emit_stream_started(
        &self,
        correlation: &EventCorrelation,
        model: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Stream(StreamPayload::Started {
                model: model.into(),
            }),
        )
    }

    pub fn emit_stream_chunk(
        &self,
        correlation: &EventCorrelation,
        text: impl Into<String>,
        sequence_number: u64,
        is_final: bool,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        let text_str = text.into();

        // 1. Emit correlated event
        let env = self.emit_payload(
            correlation.clone(),
            EdithPayload::Stream(StreamPayload::Chunk {
                text: text_str.clone(),
                sequence_number,
                is_final,
            }),
        )?;

        // 2. Compatibility Bridge: Emit on legacy chat-chunk channel
        if let Some(ref app) = self.app {
            let _ = app.emit(LEGACY_CHAT_CHUNK_CHANNEL, text_str);
        }

        Ok(env)
    }

    pub fn emit_stream_finished(
        &self,
        correlation: &EventCorrelation,
        total_tokens: Option<u32>,
        finish_reason: Option<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Stream(StreamPayload::Finished {
                total_tokens,
                finish_reason,
            }),
        )
    }

    pub fn emit_stream_failed(
        &self,
        correlation: &EventCorrelation,
        error: impl Into<String>,
        error_type: Option<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Stream(StreamPayload::Failed {
                error: error.into(),
                error_type,
            }),
        )
    }

    pub fn emit_stream_cancelled(
        &self,
        correlation: &EventCorrelation,
        reason: Option<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Stream(StreamPayload::Cancelled { reason }),
        )
    }

    // -------------------------------------------------------------------------
    // Task Lifecycle Emitters
    // -------------------------------------------------------------------------

    pub fn emit_task_started(
        &self,
        correlation: &EventCorrelation,
        task_id: impl Into<String>,
        goal: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Task(TaskPayload::Started {
                task_id: task_id.into(),
                goal: goal.into(),
            }),
        )
    }

    pub fn emit_task_step_progress(
        &self,
        correlation: &EventCorrelation,
        task_id: impl Into<String>,
        step: u32,
        max_steps: u32,
        status_text: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Task(TaskPayload::StepProgress {
                task_id: task_id.into(),
                step,
                max_steps,
                status_text: status_text.into(),
            }),
        )
    }

    pub fn emit_task_finished(
        &self,
        correlation: &EventCorrelation,
        task_id: impl Into<String>,
        success: bool,
        summary: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Task(TaskPayload::Finished {
                task_id: task_id.into(),
                success,
                summary: summary.into(),
            }),
        )
    }

    pub fn emit_task_failed(
        &self,
        correlation: &EventCorrelation,
        task_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Task(TaskPayload::Failed {
                task_id: task_id.into(),
                error: error.into(),
            }),
        )
    }

    pub fn emit_task_cancelled(
        &self,
        correlation: &EventCorrelation,
        task_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Task(TaskPayload::Cancelled {
                task_id: task_id.into(),
                reason,
            }),
        )
    }

    // -------------------------------------------------------------------------
    // Tool Lifecycle Emitters
    // -------------------------------------------------------------------------

    pub fn emit_tool_proposed(
        &self,
        correlation: &EventCorrelation,
        execution_id: impl Into<String>,
        tool_name: impl Into<String>,
        risk_level: impl Into<String>,
        summary: impl Into<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Tool(ToolPayload::Proposed {
                execution_id: execution_id.into(),
                tool_name: tool_name.into(),
                risk_level: risk_level.into(),
                summary: summary.into(),
            }),
        )
    }

    pub fn emit_tool_completed(
        &self,
        correlation: &EventCorrelation,
        execution_id: impl Into<String>,
        tool_name: impl Into<String>,
        success: bool,
        duration_ms: u64,
        result_summary: Option<String>,
    ) -> Result<EdithEventEnvelope<EdithPayload>, String> {
        self.emit_payload(
            correlation.clone(),
            EdithPayload::Tool(ToolPayload::Completed {
                execution_id: execution_id.into(),
                tool_name: tool_name.into(),
                success,
                duration_ms,
                result_summary,
            }),
        )
    }
}
