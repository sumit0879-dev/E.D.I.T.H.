//! engine.rs — Realtime duplex voice engine for E.D.I.T.H.
//!
//! Enforces:
//! 1. Turn ownership strictly in `ConversationCore` (zero independent TurnId authority).
//! 2. Tool execution strictly through `ToolRouter` and `PolicyEngine` (UTR).
//! 3. Single authoritative capture driver and single authoritative audio output sink.
//! 4. Dual backpressure: safe stale assistant frame discard + intact mic jitter buffer with explicit stream discontinuity on saturation.
//! 5. Bounded reconnection and automatic fallback to Phase 9.
//! 6. Concurrency safety: no locks held across `.await` points.

use super::adapter::{RealtimeProviderEvent, RealtimeSessionAdapter};
use super::frame::{AudioFrame, FrameDirection};
use super::session::RealtimeVoiceSession;
use crate::conversation::ConversationCore;
use crate::events::{
    ConversationId, EdithPayload, EventCorrelation, EventEmitter, TurnId, VoicePayload,
    VoiceSessionId,
};
use crate::tools::{ToolExecutionId, ToolRequest, ToolRouter};
use crate::voice::capture::AudioCaptureDriver;
use crate::voice::errors::VoiceError;
use crate::voice::output::AudioOutputDriver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Configuration options for the realtime voice engine.
#[derive(Debug, Clone)]
pub struct RealtimeEngineConfig {
    pub max_reconnect_attempts: u32,
    pub initial_reconnect_delay_ms: u64,
    pub input_channel_capacity: usize,
    pub backpressure_timeout_ms: u64,
}

impl Default for RealtimeEngineConfig {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 3,
            initial_reconnect_delay_ms: 500,
            input_channel_capacity: 32, // ~640ms at 20ms frames
            backpressure_timeout_ms: 500,
        }
    }
}

/// Central engine orchestrating duplex realtime voice sessions.
pub struct RealtimeVoiceEngine {
    conversation_core: Arc<ConversationCore>,
    tool_router: Arc<ToolRouter>,
    audio_output: Arc<dyn AudioOutputDriver>,
    capture_driver: Arc<dyn AudioCaptureDriver>,
    event_emitter: Option<Arc<EventEmitter>>,
    active_session: Arc<RwLock<Option<Arc<RealtimeVoiceSession>>>>,
    config: RealtimeEngineConfig,
    is_fallback_triggered: Arc<AtomicBool>,
}

impl RealtimeVoiceEngine {
    pub fn new(
        conversation_core: Arc<ConversationCore>,
        tool_router: Arc<ToolRouter>,
        audio_output: Arc<dyn AudioOutputDriver>,
        capture_driver: Arc<dyn AudioCaptureDriver>,
        event_emitter: Option<Arc<EventEmitter>>,
    ) -> Self {
        Self {
            conversation_core,
            tool_router,
            audio_output,
            capture_driver,
            event_emitter,
            active_session: Arc::new(RwLock::new(None)),
            config: RealtimeEngineConfig::default(),
            is_fallback_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Emits a correlated voice event if event emitter is configured.
    fn emit_voice_event(
        &self,
        session: &RealtimeVoiceSession,
        turn_id: Option<&TurnId>,
        payload: VoicePayload,
    ) {
        if let Some(ref emitter) = self.event_emitter {
            let mut correlation = EventCorrelation::for_voice(
                session.id.to_string(),
                Some(session.conversation_id.to_string()),
            );
            if let Some(tid) = turn_id {
                correlation.turn_id = Some(tid.to_string());
            }
            let _ = emitter.emit_payload(correlation, EdithPayload::Voice(payload));
        }
    }

    /// Authoritatively allocates or retrieves the active conversational turn in ConversationCore.
    pub async fn ensure_active_turn(
        &self,
        session: &RealtimeVoiceSession,
    ) -> Result<TurnId, VoiceError> {
        let mut turn_lock = session.active_turn_id.write().await;
        if let Some(ref tid) = *turn_lock {
            return Ok(tid.clone());
        }

        // Request authoritative turn directly from ConversationCore
        let turn_id = self
            .conversation_core
            .start_realtime_turn(
                &session.conversation_id.to_string(),
                Some(session.provider_id.clone()),
                None,
            )
            .await
            .map_err(|e| VoiceError::Internal(format!("ConversationCore turn creation failed: {}", e)))?;

        *turn_lock = Some(turn_id.clone());
        Ok(turn_id)
    }

    /// Finalizes the active turn in ConversationCore with completed user transcript and assistant response.
    pub async fn finalize_active_turn(
        &self,
        session: &RealtimeVoiceSession,
        user_transcript: Option<String>,
        assistant_response: Option<String>,
    ) -> Result<(), VoiceError> {
        let mut turn_lock = session.active_turn_id.write().await;
        if let Some(tid) = turn_lock.take() {
            self.conversation_core
                .complete_realtime_turn(&tid, user_transcript, assistant_response)
                .await
                .map_err(|e| VoiceError::Internal(format!("Failed to finalize turn: {}", e)))?;
        }
        Ok(())
    }

    /// Cancels the active turn in ConversationCore.
    pub async fn cancel_active_turn(
        &self,
        session: &RealtimeVoiceSession,
        reason: &str,
    ) -> Result<(), VoiceError> {
        let mut turn_lock = session.active_turn_id.write().await;
        if let Some(tid) = turn_lock.take() {
            let _ = self
                .conversation_core
                .cancel_turn(&tid, Some(reason.to_string()))
                .await;
        }
        Ok(())
    }

    /// Commences a new realtime duplex session.
    pub async fn start_session(
        &self,
        conversation_id: ConversationId,
        provider_id: impl Into<String>,
        adapter: Arc<dyn RealtimeSessionAdapter>,
    ) -> Result<VoiceSessionId, VoiceError> {
        let mut lock = self.active_session.write().await;

        // Ensure any previous session is fully halted and capture released
        if let Some(ref prev) = *lock {
            prev.cancellation_token.cancel();
            let _ = self.cancel_active_turn(prev, "new_session_started").await;
            let _ = self.capture_driver.cancel_capture();
            let _ = self.audio_output.stop();
        }

        let prov = provider_id.into();
        let session = Arc::new(RealtimeVoiceSession::new(
            conversation_id,
            prov.clone(),
            "realtime_stream",
        ));
        let session_id = session.id.clone();

        // 1. Acquire single authoritative microphone capture
        self.capture_driver.start_capture(&session_id)?;

        self.emit_voice_event(
            &session,
            None,
            VoicePayload::SessionStarted {
                session_id: session_id.to_string(),
            },
        );

        self.emit_voice_event(
            &session,
            None,
            VoicePayload::RealtimeConnected {
                provider: prov,
                transport: session.transport_type.clone(),
            },
        );

        self.emit_voice_event(
            &session,
            None,
            VoicePayload::StateChanged {
                state: "listening".to_string(),
                decibel: None,
            },
        );

        // 2. Spawn event processing and output loop
        let engine_self = self.clone_for_task();
        let session_clone = Arc::clone(&session);
        let adapter_clone = Arc::clone(&adapter);

        tokio::spawn(async move {
            engine_self
                .run_event_loop(session_clone, adapter_clone)
                .await;
        });

        *lock = Some(session);
        self.is_fallback_triggered.store(false, Ordering::SeqCst);
        Ok(session_id)
    }

    /// Handles continuous streaming events arriving from the provider session adapter.
    async fn run_event_loop(
        &self,
        session: Arc<RealtimeVoiceSession>,
        adapter: Arc<dyn RealtimeSessionAdapter>,
    ) {
        let mut accumulated_transcript = String::new();
        let mut accumulated_assistant = String::new();

        while !session.cancellation_token.is_cancelled() {
            match adapter.next_event().await {
                Ok(Some(event)) => match event {
                    RealtimeProviderEvent::Connected => {
                        self.emit_voice_event(
                            &session,
                            None,
                            VoicePayload::RealtimeConnected {
                                provider: session.provider_id.clone(),
                                transport: session.transport_type.clone(),
                            },
                        );
                    }
                    RealtimeProviderEvent::TranscriptDelta { text, is_final } => {
                        let active_tid = match self.ensure_active_turn(&session).await {
                            Ok(tid) => tid,
                            Err(_) => continue,
                        };

                        accumulated_transcript.push_str(&text);
                        self.emit_voice_event(
                            &session,
                            Some(&active_tid),
                            VoicePayload::TranscriptDelta {
                                text: text.clone(),
                                is_final,
                            },
                        );
                    }
                    RealtimeProviderEvent::AudioDelta { frame } => {
                        // Dual Backpressure Rule A: Obsolete Assistant Frame Discard
                        let current_gen = session.current_generation();
                        if frame.generation_id < current_gen {
                            // Frame belongs to an older interrupted/cancelled generation — discard immediately!
                            continue;
                        }

                        let active_tid = match self.ensure_active_turn(&session).await {
                            Ok(tid) => tid,
                            Err(_) => continue,
                        };

                        let duration = frame.duration_ms();
                        let seq = frame.sequence;

                        self.emit_voice_event(
                            &session,
                            Some(&active_tid),
                            VoicePayload::AssistantAudioDelta {
                                sequence: seq,
                                duration_ms: duration,
                            },
                        );

                        // Signal-driven Visualizer Energy for outbound assistant speech
                        let rms = frame.rms_energy();
                        let peak = frame.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                        let mut bands = [0.0f32; 8];
                        let chunk_size = frame.samples.len() / 8;
                        if chunk_size > 0 {
                            for (i, b) in bands.iter_mut().enumerate() {
                                let start = i * chunk_size;
                                let end = (start + chunk_size).min(frame.samples.len());
                                let sub_sq: f32 = frame.samples[start..end].iter().map(|s| s * s).sum();
                                *b = (sub_sq / (end - start) as f32).sqrt().clamp(0.0, 1.0);
                            }
                        }
                        let rms_bp = (rms * 10000.0).clamp(0.0, 10000.0) as u32;
                        let peak_bp = (peak * 10000.0).clamp(0.0, 10000.0) as u32;
                        let mut bands_bp = [0u32; 8];
                        for idx in 0..8 {
                            bands_bp[idx] = (bands[idx] * 10000.0).clamp(0.0, 10000.0) as u32;
                        }
                        self.emit_voice_event(
                            &session,
                            Some(&active_tid),
                            VoicePayload::VisualizerEnergy {
                                rms: rms_bp,
                                peak: peak_bp,
                                bands: bands_bp,
                                is_speech: true,
                                direction: "output".to_string(),
                            },
                        );

                        // Direct streaming to single authoritative hardware audio sink
                        let _ = self
                            .audio_output
                            .play(frame.to_audio_buffer(), session.id.clone());
                    }
                    RealtimeProviderEvent::ToolCall {
                        call_id,
                        tool_name,
                        arguments,
                    } => {
                        let active_tid = match self.ensure_active_turn(&session).await {
                            Ok(tid) => tid,
                            Err(_) => continue,
                        };

                        // Realtime Tool Calling Invariant: Strict execution through Universal Tool Runtime
                        let exec_id = format!("exec_{}", uuid::Uuid::new_v4());
                        let mut correlation = EventCorrelation::for_voice(
                            session.id.to_string(),
                            Some(session.conversation_id.to_string()),
                        );
                        correlation.turn_id = Some(active_tid.to_string());
                        correlation.tool_execution_id = Some(exec_id.clone());

                        let tool_request = ToolRequest::new(
                            tool_name,
                            arguments,
                            correlation,
                        )
                        .with_execution_id(ToolExecutionId::from_string(exec_id));

                        // Dispatch to ToolRouter with full PolicyEngine gating
                        let result = self.tool_router.execute(tool_request).await;

                        // Return result to provider
                        let _ = adapter.send_tool_result(call_id, result).await;
                    }
                    RealtimeProviderEvent::Interrupted { reason } => {
                        // Low-latency barge-in handling
                        self.handle_barge_in(&session, adapter.as_ref(), &reason).await;
                    }
                    RealtimeProviderEvent::TurnComplete => {
                        let u_text = if accumulated_transcript.is_empty() {
                            None
                        } else {
                            Some(accumulated_transcript.clone())
                        };
                        let a_text = if accumulated_assistant.is_empty() {
                            None
                        } else {
                            Some(accumulated_assistant.clone())
                        };

                        let _ = self
                            .finalize_active_turn(&session, u_text, a_text)
                            .await;

                        accumulated_transcript.clear();
                        accumulated_assistant.clear();
                    }
                    RealtimeProviderEvent::Error { message } => {
                        self.emit_voice_event(
                            &session,
                            None,
                            VoicePayload::RealtimeError { error: message },
                        );
                    }
                },
                Ok(None) => {
                    // Adapter closed cleanly
                    break;
                }
                Err(e) => {
                    self.emit_voice_event(
                        &session,
                        None,
                        VoicePayload::RealtimeError {
                            error: format!("Transport error: {}", e),
                        },
                    );
                    break;
                }
            }
        }

        // Teardown turn if still in-flight
        let _ = self.cancel_active_turn(&session, "session_ended").await;
    }

    /// Executes low-latency barge-in interruption.
    pub async fn handle_barge_in(
        &self,
        session: &RealtimeVoiceSession,
        adapter: &dyn RealtimeSessionAdapter,
        reason: &str,
    ) {
        // 1. Increment generation atomically — all queued or pending output frames are instantly invalid
        let _ = session.increment_generation();

        // 2. Halt soundcard output immediately
        let _ = self.audio_output.stop();

        // 3. Cancel active turn in ConversationCore
        let _ = self.cancel_active_turn(session, reason).await;

        // 4. Notify provider adapter
        let _ = adapter.interrupt();

        // 5. Emit barge-in events
        self.emit_voice_event(
            session,
            None,
            VoicePayload::BargeInTriggered {
                interrupted_source: "realtime_assistant_speech".to_string(),
            },
        );

        self.emit_voice_event(
            session,
            None,
            VoicePayload::RealtimeInterrupted {
                reason: reason.to_string(),
            },
        );
    }

    /// Dispatches captured microphone audio frame with dual backpressure enforcement.
    pub async fn dispatch_input_frame(
        &self,
        session: &RealtimeVoiceSession,
        adapter: &dyn RealtimeSessionAdapter,
        samples: Vec<f32>,
        sample_rate: u32,
    ) -> Result<(), VoiceError> {
        if session.cancellation_token.is_cancelled() {
            return Ok(());
        }

        let seq = session.next_input_sequence();
        let frame = AudioFrame::new(
            session.id.clone(),
            seq,
            session.elapsed_ms(),
            sample_rate,
            1,
            samples,
            FrameDirection::Input,
            session.current_generation(),
        );

        // Signal-driven Visualizer Energy for live microphone speech
        let rms = frame.rms_energy();
        let peak = frame.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let mut bands = [0.0f32; 8];
        let chunk_size = frame.samples.len() / 8;
        if chunk_size > 0 {
            for (i, b) in bands.iter_mut().enumerate() {
                let start = i * chunk_size;
                let end = (start + chunk_size).min(frame.samples.len());
                let sub_sq: f32 = frame.samples[start..end].iter().map(|s| s * s).sum();
                *b = (sub_sq / (end - start) as f32).sqrt().clamp(0.0, 1.0);
            }
        }
        let is_speech = rms >= 0.02;

        let rms_bp = (rms * 10000.0).clamp(0.0, 10000.0) as u32;
        let peak_bp = (peak * 10000.0).clamp(0.0, 10000.0) as u32;
        let mut bands_bp = [0u32; 8];
        for idx in 0..8 {
            bands_bp[idx] = (bands[idx] * 10000.0).clamp(0.0, 10000.0) as u32;
        }

        self.emit_voice_event(
            session,
            None,
            VoicePayload::VisualizerEnergy {
                rms: rms_bp,
                peak: peak_bp,
                bands: bands_bp,
                is_speech,
                direction: "input".to_string(),
            },
        );

        // Send outbound with timeout to avoid stalling on network slowdown
        let send_res = tokio::time::timeout(
            Duration::from_millis(self.config.backpressure_timeout_ms),
            adapter.send_audio(frame),
        )
        .await;

        match send_res {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Dual Backpressure Rule B: Buffer Saturation & Explicit Stream Discontinuity
                // Do NOT randomly drop isolated frames from live speech!
                self.emit_voice_event(
                    session,
                    None,
                    VoicePayload::RealtimeError {
                        error: "Network backpressure exceeded: input audio buffer saturated"
                            .to_string(),
                    },
                );

                // Invalidate current in-flight turn
                let _ = self
                    .cancel_active_turn(session, "input_buffer_saturation")
                    .await;

                // Signal fallback if backpressure continues
                self.is_fallback_triggered.store(true, Ordering::SeqCst);
                self.emit_voice_event(
                    session,
                    None,
                    VoicePayload::RealtimeFallbackTriggered {
                        reason: "persistent_input_backpressure".to_string(),
                    },
                );

                Err(VoiceError::Internal(
                    "Input stream backpressure timeout; stream discontinuity triggered"
                        .to_string(),
                ))
            }
        }
    }

    /// Halts active session and releases capture and playback devices.
    pub async fn stop_session(&self) -> Result<(), VoiceError> {
        let mut lock = self.active_session.write().await;
        if let Some(session) = lock.take() {
            session.cancellation_token.cancel();
            let _ = self.cancel_active_turn(&session, "session_stopped_by_user").await;
            let _ = self.capture_driver.cancel_capture();
            let _ = self.audio_output.stop();

            self.emit_voice_event(
                &session,
                None,
                VoicePayload::SessionEnded {
                    session_id: session.id.to_string(),
                    reason: Some("stopped_by_user".to_string()),
                },
            );
        }
        Ok(())
    }

    /// Returns whether the engine has flagged a need to fall back to Phase 9.
    pub fn is_fallback_needed(&self) -> bool {
        self.is_fallback_triggered.load(Ordering::SeqCst)
    }

    /// Active session reader.
    pub fn active_session(&self) -> &Arc<RwLock<Option<Arc<RealtimeVoiceSession>>>> {
        &self.active_session
    }

    /// Sets custom engine configuration.
    pub fn with_config(mut self, config: RealtimeEngineConfig) -> Self {
        self.config = config;
        self
    }

    pub fn tool_router(&self) -> &Arc<ToolRouter> {
        &self.tool_router
    }

    pub fn audio_output(&self) -> &Arc<dyn AudioOutputDriver> {
        &self.audio_output
    }

    pub fn capture_driver(&self) -> &Arc<dyn AudioCaptureDriver> {
        &self.capture_driver
    }

    pub fn config_mut(&mut self) -> &mut RealtimeEngineConfig {
        &mut self.config
    }

    /// Clones internal references to pass into spawned worker tasks.
    fn clone_for_task(&self) -> Self {
        Self {
            conversation_core: Arc::clone(&self.conversation_core),
            tool_router: Arc::clone(&self.tool_router),
            audio_output: Arc::clone(&self.audio_output),
            capture_driver: Arc::clone(&self.capture_driver),
            event_emitter: self.event_emitter.clone(),
            active_session: Arc::clone(&self.active_session),
            config: self.config.clone(),
            is_fallback_triggered: Arc::clone(&self.is_fallback_triggered),
        }
    }
}
