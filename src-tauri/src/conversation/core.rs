use super::context::ContextAssembler;
use super::errors::ConversationError;
use super::turn::{Turn, TurnStatus};
use super::types::{ModelSelection, TurnSnapshot, TurnSubmissionRequest, TurnSubmissionResult};
use crate::ai::{ChatMessage, GenerateRequest, ProviderRegistry, ToolChoice};
use crate::events::{EventCorrelation, EventEmitter, StreamId, TurnId};
use crate::task::CancellationToken;
use crate::tools::types::{ToolExecutionId, ToolRequest, ToolStatus};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

fn current_timestamp_str() -> String {
    let now = chrono::Local::now();
    now.format("%H:%M").to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Central conversational runtime coordinating turns, context assembly, and streaming execution.
#[derive(Clone)]
pub struct ConversationCore {
    turns: Arc<RwLock<HashMap<String, Arc<RwLock<Turn>>>>>,
    registry: Arc<RwLock<ProviderRegistry>>,
    emitter: EventEmitter,
    db_conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    context_assembler: Arc<ContextAssembler>,
    tool_router: Arc<std::sync::RwLock<Option<Arc<crate::tools::ToolRouter>>>>,
    tool_registry: Arc<std::sync::RwLock<Option<Arc<crate::tools::ToolRegistry>>>>,
}

impl ConversationCore {
    /// Creates a new ConversationCore connected to the application runtime.
    pub fn new(
        registry: ProviderRegistry,
        emitter: EventEmitter,
        db_conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
        context_assembler: Option<ContextAssembler>,
    ) -> Self {
        Self {
            turns: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(registry)),
            emitter,
            db_conn,
            context_assembler: Arc::new(context_assembler.unwrap_or_default()),
            tool_router: Arc::new(std::sync::RwLock::new(None)),
            tool_registry: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// Creates an in-memory mock ConversationCore for headless automated unit testing.
    pub fn mock(registry: ProviderRegistry) -> Self {
        Self::new(registry, EventEmitter::mock(), None, None)
    }

    /// Attaches the Universal Tool Runtime router and registry for agentic tool execution.
    pub fn set_tools(
        &self,
        router: Arc<crate::tools::ToolRouter>,
        registry: Arc<crate::tools::ToolRegistry>,
    ) {
        if let Ok(mut r) = self.tool_router.write() {
            *r = Some(router);
        }
        if let Ok(mut reg) = self.tool_registry.write() {
            *reg = Some(registry);
        }
    }

    pub fn emitter(&self) -> &EventEmitter {
        &self.emitter
    }

    pub fn registry(&self) -> Arc<RwLock<ProviderRegistry>> {
        self.registry.clone()
    }

    /// Registers a new conversational turn, generates an authoritative TurnId,
    /// persists the user prompt, and prepares the turn state machine.
    pub async fn submit_turn(
        &self,
        req: TurnSubmissionRequest,
    ) -> Result<TurnSubmissionResult, ConversationError> {
        let trimmed_msg = req.message.trim().to_string();
        if trimmed_msg.is_empty() {
            return Err(ConversationError::Internal(
                "Message cannot be empty".to_string(),
            ));
        }

        // Backend is ALWAYS the sole authoritative creator and owner of TurnId.
        // Any client_turn_id in the request is treated strictly as a non-authoritative legacy hint:
        // it is never stored as TurnId, never emitted as TurnId, and never used for turn lookup.
        let turn_id = TurnId::new();
        let stream_id = StreamId::new();

        let model_selection = ModelSelection {
            provider_id: req.provider_id.unwrap_or_else(|| "groq".to_string()),
            model_id: req
                .model_id
                .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string()),
            temperature: req.temperature.unwrap_or(0.7),
        };

        let mut turn = Turn::new(
            turn_id.clone(),
            req.session_id.clone(),
            stream_id.clone(),
            trimmed_msg.clone(),
            model_selection,
        );

        // Transition: Created -> InputAccepted
        if !turn.status.can_transition_to(&TurnStatus::InputAccepted) {
            return Err(ConversationError::InvalidTurnState {
                current: turn.status.to_string(),
                target: TurnStatus::InputAccepted.to_string(),
            });
        }
        turn.status = TurnStatus::InputAccepted;

        // Persist user message to SQLite if db connection is present
        if let Some(ref db_conn) = self.db_conn {
            let conn = db_conn
                .lock()
                .map_err(|e| ConversationError::Internal(e.to_string()))?;
            let timestamp = current_timestamp_str();
            let _ = crate::db::save_session_message(
                &conn,
                &req.session_id,
                "user",
                &trimmed_msg,
                &timestamp,
            );
        }

        {
            let mut lock = self.turns.write().await;
            lock.insert(turn_id.to_string(), Arc::new(RwLock::new(turn)));
        }

        Ok(TurnSubmissionResult {
            turn_id: turn_id.to_string(),
            session_id: req.session_id,
            stream_id: stream_id.to_string(),
            user_message_text: trimmed_msg,
        })
    }

    /// Executes the LLM generation and streaming lifecycle for an accepted turn.
    /// The authoritative StreamId and cancellation token are retrieved directly from the Turn.
    pub async fn execute_turn(
        &self,
        turn_id: &TurnId,
        credentials: Option<String>,
    ) -> Result<String, ConversationError> {
        let turn_arc = {
            let lock = self.turns.read().await;
            lock.get(turn_id.as_str())
                .cloned()
                .ok_or_else(|| ConversationError::NotFound(turn_id.to_string()))?
        };

        // Extract turn parameters and check for early cancellation
        let (session_id, stream_id, user_message, model_selection, cancellation_token) = {
            let mut turn = turn_arc.write().await;
            if turn.status == TurnStatus::Cancelled || turn.cancellation_token.is_cancelled() {
                return Err(ConversationError::Cancellation(
                    "Turn was cancelled before execution".to_string(),
                ));
            }

            // Transition: InputAccepted -> Processing
            if !turn.status.can_transition_to(&TurnStatus::Processing) {
                return Err(ConversationError::InvalidTurnState {
                    current: turn.status.to_string(),
                    target: TurnStatus::Processing.to_string(),
                });
            }
            turn.status = TurnStatus::Processing;

            (
                turn.session_id.clone(),
                turn.stream_id.clone(),
                turn.user_message.clone(),
                turn.model_selection.clone(),
                turn.cancellation_token.clone(),
            )
        };

        // Load conversation history from SQLite if available
        let history_messages = if let Some(ref db_conn) = self.db_conn {
            let conn = db_conn
                .lock()
                .map_err(|e| ConversationError::Internal(e.to_string()))?;
            let db_msgs = crate::db::get_session_messages(&conn, &session_id).unwrap_or_default();
            db_msgs
                .into_iter()
                .map(|m| ChatMessage {
                    role: m.role,
                    content: m.text,
                    ..Default::default()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Context Assembly Boundary
        let assembled_messages = self
            .context_assembler
            .assemble_messages(&history_messages, &user_message)
            .await;

        // Resolve provider adapter from ProviderRegistry
        let adapter = {
            let reg = self.registry.read().await;
            reg.resolve_provider(&model_selection.provider_id)
                .map_err(ConversationError::from)?
        };

        // Correlation context for Phase 2 event infrastructure (authoritative TurnId and StreamId)
        let correlation = EventCorrelation::for_stream(
            Some(session_id.clone()),
            Some(turn_id.to_string()),
            Some(stream_id.to_string()),
        );

        // Fetch available tools from ToolRegistry if attached
        let (router_opt, registry_opt) = {
            let r = self.tool_router.read().ok().and_then(|g| g.clone());
            let reg = self.tool_registry.read().ok().and_then(|g| g.clone());
            (r, reg)
        };

        let available_tools = if let Some(ref reg) = registry_opt {
            let tools = reg.list();
            if tools.is_empty() {
                None
            } else {
                Some(tools)
            }
        } else {
            None
        };

        // Transition: Processing -> Streaming
        {
            let mut turn = turn_arc.write().await;
            if !turn.status.can_transition_to(&TurnStatus::Streaming) {
                return Err(ConversationError::InvalidTurnState {
                    current: turn.status.to_string(),
                    target: TurnStatus::Streaming.to_string(),
                });
            }
            turn.status = TurnStatus::Streaming;
        }

        let _ = self
            .emitter
            .emit_stream_started(&correlation, &model_selection.model_id);

        let mut loop_messages = assembled_messages;
        let mut final_text = String::new();
        let mut iteration = 0;
        const MAX_AGENTIC_ITERATIONS: usize = 10;
        let seq = Arc::new(AtomicU64::new(0));

        while iteration < MAX_AGENTIC_ITERATIONS {
            iteration += 1;

            if cancellation_token.is_cancelled() {
                let mut turn = turn_arc.write().await;
                if turn.status != TurnStatus::Cancelled {
                    turn.status = TurnStatus::Cancelled;
                    turn.completed_at_ms = Some(now_ms());
                    turn.error = Some("Turn cancelled by operator".to_string());
                    let _ = self.emitter.emit_stream_cancelled(
                        &correlation,
                        Some("Turn cancelled by operator".to_string()),
                    );
                }
                return Err(ConversationError::Cancellation(
                    "Turn cancelled by operator".to_string(),
                ));
            }

            let req = GenerateRequest {
                model: model_selection.model_id.clone(),
                messages: loop_messages.clone(),
                temperature: model_selection.temperature,
                max_tokens: None,
                stream: true,
                tools: available_tools.clone(),
                tool_choice: available_tools.as_ref().map(|_| ToolChoice::Auto),
            };

            // Stream execution
            let stream_cap = adapter.as_streaming_text();
            let response = if let Some(streamer) = stream_cap {
                let emitter_clone = self.emitter.clone();
                let correlation_clone = correlation.clone();
                let token_clone = cancellation_token.clone();
                let seq_clone = seq.clone();
                let accumulated = Arc::new(std::sync::Mutex::new(String::new()));
                let acc_clone = accumulated.clone();

                let stream_res = streamer
                    .stream(
                        &req,
                        &credentials,
                        Box::new(move |chunk| {
                            if token_clone.is_cancelled() {
                                return;
                            }
                            if !chunk.text.is_empty() {
                                if let Ok(mut text) = acc_clone.lock() {
                                    text.push_str(&chunk.text);
                                }
                                let n = seq_clone
                                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                                    + 1;
                                let _ = emitter_clone.emit_stream_chunk(
                                    &correlation_clone,
                                    chunk.text,
                                    n,
                                    false,
                                );
                            }
                        }),
                    )
                    .await;

                if cancellation_token.is_cancelled() {
                    let mut turn = turn_arc.write().await;
                    if turn.status != TurnStatus::Cancelled {
                        turn.status = TurnStatus::Cancelled;
                        turn.completed_at_ms = Some(now_ms());
                        turn.error = Some("Turn cancelled by operator".to_string());
                        let _ = self.emitter.emit_stream_cancelled(
                            &correlation,
                            Some("Turn cancelled by operator".to_string()),
                        );
                    }
                    return Err(ConversationError::Cancellation(
                        "Turn cancelled by operator".to_string(),
                    ));
                }

                match stream_res {
                    Ok(resp) => resp,
                    Err(e) => {
                        let err_str = e.to_string();
                        let _ = self
                            .emitter
                            .emit_stream_failed(&correlation, &err_str, None);
                        let mut turn = turn_arc.write().await;
                        turn.status = TurnStatus::Failed;
                        turn.completed_at_ms = Some(now_ms());
                        turn.error = Some(err_str);
                        return Err(ConversationError::from(e));
                    }
                }
            } else if let Some(gen) = adapter.as_text_generation() {
                let gen_res = gen.generate(&req, &credentials).await;

                if cancellation_token.is_cancelled() {
                    let mut turn = turn_arc.write().await;
                    if turn.status != TurnStatus::Cancelled {
                        turn.status = TurnStatus::Cancelled;
                        turn.completed_at_ms = Some(now_ms());
                        turn.error = Some("Turn cancelled by operator".to_string());
                        let _ = self.emitter.emit_stream_cancelled(
                            &correlation,
                            Some("Turn cancelled by operator".to_string()),
                        );
                    }
                    return Err(ConversationError::Cancellation(
                        "Turn cancelled by operator".to_string(),
                    ));
                }

                match gen_res {
                    Ok(resp) => {
                        let n = seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        let _ = self
                            .emitter
                            .emit_stream_chunk(&correlation, resp.text.clone(), n, false);
                        resp
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let _ = self
                            .emitter
                            .emit_stream_failed(&correlation, &err_str, None);
                        let mut turn = turn_arc.write().await;
                        turn.status = TurnStatus::Failed;
                        turn.completed_at_ms = Some(now_ms());
                        turn.error = Some(err_str);
                        return Err(ConversationError::from(e));
                    }
                }
            } else {
                let err = format!(
                    "Provider '{}' does not support text generation",
                    model_selection.provider_id
                );
                let _ = self.emitter.emit_stream_failed(&correlation, &err, None);
                let mut turn = turn_arc.write().await;
                turn.status = TurnStatus::Failed;
                turn.completed_at_ms = Some(now_ms());
                turn.error = Some(err.clone());
                return Err(ConversationError::ModelUnavailable(err));
            };

            // Check if model emitted tool calls
            if let Some(ref tool_calls) = response.tool_calls {
                if !tool_calls.is_empty() && router_opt.is_some() {
                    let router = router_opt.as_ref().unwrap();

                    // Append assistant message with tool calls
                    loop_messages.push(ChatMessage::assistant_with_tools(
                        response.text.clone(),
                        tool_calls.clone(),
                    ));

                    // Execute each requested tool through the Universal Tool Runtime
                    for call in tool_calls {
                        if cancellation_token.is_cancelled() {
                            break;
                        }

                        let args = serde_json::from_str::<serde_json::Value>(&call.arguments)
                            .unwrap_or_else(|_| json!({ "raw": call.arguments }));

                        let tool_req = ToolRequest::new(
                            call.name.clone(),
                            args,
                            correlation.clone(),
                        )
                        .with_execution_id(ToolExecutionId::from_string(call.id.clone()));

                        let tool_result = router.execute(tool_req).await;

                        let result_payload = match &tool_result.status {
                            ToolStatus::Completed => {
                                json!({
                                    "status": "success",
                                    "data": tool_result.data
                                })
                                .to_string()
                            }
                            ToolStatus::ApprovalRequired => {
                                json!({
                                    "status": "approval_required",
                                    "approval_id": tool_result.approval_id,
                                    "reason": tool_result.error
                                })
                                .to_string()
                            }
                            ToolStatus::Blocked => {
                                json!({
                                    "status": "blocked",
                                    "error": tool_result.error,
                                    "error_code": tool_result.error_code
                                })
                                .to_string()
                            }
                            _ => {
                                json!({
                                    "status": "error",
                                    "error": tool_result.error
                                })
                                .to_string()
                            }
                        };

                        loop_messages.push(ChatMessage::tool_result(
                            call.id.clone(),
                            call.name.clone(),
                            result_payload,
                        ));
                    }

                    // Feed tool outputs back to the model in the same conversational turn!
                    continue;
                }
            }

            // No tool calls — this is the terminal model response
            final_text = response.text;
            break;
        }

        let _ = self.emitter.emit_stream_finished(
            &correlation,
            None,
            Some("stop".to_string()),
        );

        // Transition: Streaming -> Completed
        {
            let mut turn = turn_arc.write().await;
            turn.status = TurnStatus::Completed;
            turn.completed_at_ms = Some(now_ms());
            turn.final_response = Some(final_text.clone());
        }

        // Persist assistant response to SQLite
        if let Some(ref db_conn) = self.db_conn {
            if let Ok(conn) = db_conn.lock() {
                let timestamp = current_timestamp_str();
                let _ = crate::db::save_session_message(
                    &conn,
                    &session_id,
                    "assistant",
                    &final_text,
                    &timestamp,
                );
            }
        }

        Ok(final_text)
    }

    /// Cancels a specific in-flight turn using its scoped cancellation token.
    /// Emits StreamCancelled with the authoritative session_id, turn_id, and stream_id.
    pub async fn cancel_turn(
        &self,
        turn_id: &TurnId,
        reason: Option<String>,
    ) -> Result<(), ConversationError> {
        let turn_arc = {
            let lock = self.turns.read().await;
            lock.get(turn_id.as_str())
                .cloned()
                .ok_or_else(|| ConversationError::NotFound(turn_id.to_string()))?
        };

        let mut turn = turn_arc.write().await;
        if turn.status.is_terminal() || !turn.status.can_transition_to(&TurnStatus::Cancelled) {
            return Err(ConversationError::InvalidTurnState {
                current: turn.status.to_string(),
                target: TurnStatus::Cancelled.to_string(),
            });
        }

        // Cooperative atomic signal
        turn.cancellation_token.cancel();
        turn.status = TurnStatus::Cancelled;
        turn.completed_at_ms = Some(now_ms());
        turn.error = reason
            .clone()
            .or_else(|| Some("Turn cancelled".to_string()));

        let correlation = EventCorrelation::for_stream(
            Some(turn.session_id.clone()),
            Some(turn.turn_id.to_string()),
            Some(turn.stream_id.to_string()),
        );
        let _ = self.emitter.emit_stream_cancelled(&correlation, reason);

        Ok(())
    }

    /// Retrieves an immutable snapshot of a turn's state.
    pub async fn get_turn_status(&self, turn_id: &TurnId) -> Option<TurnSnapshot> {
        let lock = self.turns.read().await;
        let turn_arc = lock.get(turn_id.as_str())?.clone();
        let turn = turn_arc.read().await;
        Some(turn.to_snapshot())
    }

    /// Retrieves the cancellation token for a turn.
    pub async fn get_cancellation_token(&self, turn_id: &TurnId) -> Option<CancellationToken> {
        let lock = self.turns.read().await;
        let turn_arc = lock.get(turn_id.as_str())?.clone();
        let turn = turn_arc.read().await;
        Some(turn.cancellation_token.clone())
    }

    /// Checks whether any conversational turns are currently in an active (non-terminal) state.
    pub async fn has_active_turns(&self) -> bool {
        let lock = self.turns.read().await;
        for turn_arc in lock.values() {
            let turn = turn_arc.read().await;
            if !turn.status.is_terminal() {
                return true;
            }
        }
        false
    }

    /// Lists TurnIds of currently in-flight turns.
    pub async fn get_active_turn_ids(&self) -> Vec<String> {
        let lock = self.turns.read().await;
        let mut active = Vec::new();
        for (id, turn_arc) in lock.iter() {
            let turn = turn_arc.read().await;
            if !turn.status.is_terminal() {
                active.push(id.clone());
            }
        }
        active
    }

    /// Authoritative turn creation specifically for duplex realtime conversational exchanges.
    /// Allocates an authoritative TurnId, initializes the Turn state machine in `Processing` state,
    /// and registers it in ConversationCore.
    pub async fn start_realtime_turn(
        &self,
        session_id: &str,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> Result<TurnId, ConversationError> {
        let turn_id = TurnId::new();
        let stream_id = StreamId::new();

        let model_selection = ModelSelection {
            provider_id: provider_id.unwrap_or_else(|| "gemini".to_string()),
            model_id: model_id.unwrap_or_else(|| "gemini-2.0-flash-exp".to_string()),
            temperature: 0.7,
        };

        let mut turn = Turn::new(
            turn_id.clone(),
            session_id.to_string(),
            stream_id,
            String::new(), // Populated dynamically as transcript deltas arrive
            model_selection,
        );
        turn.status = TurnStatus::Processing;

        let mut lock = self.turns.write().await;
        lock.insert(turn_id.to_string(), Arc::new(RwLock::new(turn)));

        Ok(turn_id)
    }

    /// Authoritative turn finalization for duplex realtime conversational exchanges.
    /// Transitions the turn to Completed, records final transcripts, and persists history.
    pub async fn complete_realtime_turn(
        &self,
        turn_id: &TurnId,
        user_transcript: Option<String>,
        assistant_response: Option<String>,
    ) -> Result<(), ConversationError> {
        let turn_arc = {
            let lock = self.turns.read().await;
            lock.get(turn_id.as_str())
                .cloned()
                .ok_or_else(|| ConversationError::NotFound(turn_id.to_string()))?
        };

        let mut turn = turn_arc.write().await;
        if turn.status.is_terminal() {
            return Ok(());
        }

        turn.status = TurnStatus::Completed;
        turn.completed_at_ms = Some(now_ms());

        let session_id = turn.session_id.clone();

        if let Some(user_text) = user_transcript {
            turn.user_message = user_text.clone();
            if let Some(ref db_conn) = self.db_conn {
                if let Ok(conn) = db_conn.lock() {
                    let timestamp = current_timestamp_str();
                    let _ = crate::db::save_session_message(
                        &conn,
                        &session_id,
                        "user",
                        &user_text,
                        &timestamp,
                    );
                }
            }
        }

        if let Some(assistant_text) = assistant_response {
            turn.final_response = Some(assistant_text.clone());
            if let Some(ref db_conn) = self.db_conn {
                if let Ok(conn) = db_conn.lock() {
                    let timestamp = current_timestamp_str();
                    let _ = crate::db::save_session_message(
                        &conn,
                        &session_id,
                        "assistant",
                        &assistant_text,
                        &timestamp,
                    );
                }
            }
        }

        Ok(())
    }
}
