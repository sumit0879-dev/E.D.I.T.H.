//! session.rs — VoiceSession lifecycle, turn correlation, and VoiceController.
//!
//! Enforces:
//! 1. Authoritative TurnId generation in ConversationCore (VoiceSession associates TurnId only after submit_turn).
//! 2. Convergence on ConversationCore (voice does not create an isolated agent or LLM loop).
//! 3. Turn Interruption / Barge-in (starting a new turn halts active TTS playback immediately).
//! 4. Dual STT input modes (Web Speech bridge vs Raw-Audio STT).
//! 5. Single authoritative hardware playback sink.

use super::audio::AudioBuffer;
use super::capture::{AudioCaptureDriver, CaptureOwner};
use super::errors::VoiceError;
use super::output::AudioOutputDriver;
use super::stt::{STTAdapter, STTOptions, WebSpeechSTTBridge};
use super::tts::{TTSAdapter, TTSOptions};
use crate::conversation::{ConversationCore, TurnSubmissionRequest};
use crate::events::{
    ConversationId, EdithPayload, EventCorrelation, EventEmitter, TurnId, VoicePayload,
    VoiceSessionId,
};
use crate::task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Operational lifecycle states of a voice session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceSessionState {
    Idle,
    Listening,
    Transcribing,
    CoreExecution,
    Synthesizing,
    Speaking,
    Cancelled,
    Ended,
    Failed(String),
}

/// A bounded, ephemeral voice session representing a single spoken turn.
#[derive(Clone)]
pub struct VoiceSession {
    pub id: VoiceSessionId,
    pub conversation_id: ConversationId,
    /// Populated ONLY after ConversationCore::submit_turn creates the authoritative turn.
    pub turn_id: Option<TurnId>,
    pub state: VoiceSessionState,
    pub cancellation_token: CancellationToken,
    pub started_at: Instant,
    pub capture_owner: CaptureOwner,
    pub stt_provider: String,
    pub tts_provider: String,
}

impl VoiceSession {
    pub fn new(
        conversation_id: ConversationId,
        capture_owner: CaptureOwner,
        stt_provider: impl Into<String>,
        tts_provider: impl Into<String>,
    ) -> Self {
        Self {
            id: VoiceSessionId::new(),
            conversation_id,
            turn_id: None,
            state: VoiceSessionState::Idle,
            cancellation_token: CancellationToken::new(),
            started_at: Instant::now(),
            capture_owner,
            stt_provider: stt_provider.into(),
            tts_provider: tts_provider.into(),
        }
    }
}

fn default_voice_mode() -> String {
    "fallback".to_string()
}

/// Read-model summary of voice state projected to EdithRuntimeState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceStatusSummary {
    pub is_active: bool,
    #[serde(default = "default_voice_mode")]
    pub mode: String,
    pub state: String,
    pub session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub stt_provider: String,
    pub tts_provider: String,
    #[serde(default)]
    pub realtime_provider: Option<String>,
    #[serde(default)]
    pub transport_type: Option<String>,
    #[serde(default)]
    pub reconnect_attempt: Option<u32>,
    pub is_muted: bool,
    pub last_error: Option<String>,
}

/// Central controller orchestrating both Realtime S2S (Path A) and Fallback (Path B) voice pipelines.
pub struct VoiceController {
    conversation_core: Arc<ConversationCore>,
    stt_adapter: Arc<dyn STTAdapter>,
    tts_adapter: Arc<dyn TTSAdapter>,
    audio_output: Arc<dyn AudioOutputDriver>,
    capture_driver: Arc<dyn AudioCaptureDriver>,
    event_emitter: Option<Arc<EventEmitter>>,
    active_session: Arc<RwLock<Option<VoiceSession>>>,
    realtime_engine: Arc<RwLock<Option<Arc<super::realtime::RealtimeVoiceEngine>>>>,
}

impl VoiceController {
    pub fn new(
        conversation_core: Arc<ConversationCore>,
        stt_adapter: Arc<dyn STTAdapter>,
        tts_adapter: Arc<dyn TTSAdapter>,
        audio_output: Arc<dyn AudioOutputDriver>,
        capture_driver: Arc<dyn AudioCaptureDriver>,
        event_emitter: Option<Arc<EventEmitter>>,
    ) -> Self {
        Self {
            conversation_core,
            stt_adapter,
            tts_adapter,
            audio_output,
            capture_driver,
            event_emitter,
            active_session: Arc::new(RwLock::new(None)),
            realtime_engine: Arc::new(RwLock::new(None)),
        }
    }

    /// Associates the RealtimeVoiceEngine for Path A duplex voice.
    pub fn with_realtime_engine(self, engine: Arc<super::realtime::RealtimeVoiceEngine>) -> Self {
        *self.realtime_engine.try_write().unwrap() = Some(engine);
        self
    }

    /// Sets the realtime engine on an existing Arc/shared controller.
    pub async fn set_realtime_engine(&self, engine: Arc<super::realtime::RealtimeVoiceEngine>) {
        let mut lock = self.realtime_engine.write().await;
        *lock = Some(engine);
    }

    /// Helper to emit correlated voice events via EventEmitter if configured.
    fn emit_voice_event(
        &self,
        session: &VoiceSession,
        payload: VoicePayload,
    ) {
        if let Some(ref emitter) = self.event_emitter {
            let mut correlation = EventCorrelation::for_voice(
                session.id.to_string(),
                Some(session.conversation_id.to_string()),
            );

            if let Some(ref tid) = session.turn_id {
                correlation.turn_id = Some(tid.to_string());
            }

            let _ = emitter.emit_payload(correlation, EdithPayload::Voice(payload));
        }
    }

    /// Commences a new voice session.
    /// If an active session is currently synthesizing or speaking, triggers barge-in interruption.
    pub async fn start_session(
        &self,
        conversation_id: ConversationId,
        capture_owner: CaptureOwner,
    ) -> Result<VoiceSessionId, VoiceError> {
        let mut lock = self.active_session.write().await;

        // Turn Interruption (Barge-in): If already speaking or synthesizing, halt immediately
        if let Some(ref mut active) = *lock {
            if matches!(
                active.state,
                VoiceSessionState::Synthesizing | VoiceSessionState::Speaking
            ) {
                // Cooperative cancellation
                active.cancellation_token.cancel();
                let _ = self.capture_driver.cancel_capture();
                let _ = self.audio_output.stop();

                self.emit_voice_event(
                    active,
                    VoicePayload::BargeInTriggered {
                        interrupted_source: "tts_playback".to_string(),
                    },
                );

                active.state = VoiceSessionState::Cancelled;
                self.emit_voice_event(
                    active,
                    VoicePayload::SessionEnded {
                        session_id: active.id.to_string(),
                        reason: Some("interrupted_by_user_barge_in".to_string()),
                    },
                );
            }
        }

        let mut session = VoiceSession::new(
            conversation_id,
            capture_owner,
            self.stt_adapter.name(),
            self.tts_adapter.name(),
        );

        session.state = VoiceSessionState::Listening;
        let session_id = session.id.clone();

        self.emit_voice_event(
            &session,
            VoicePayload::SessionStarted {
                session_id: session_id.to_string(),
            },
        );

        self.emit_voice_event(
            &session,
            VoicePayload::StateChanged {
                state: "listening".to_string(),
                decibel: None,
            },
        );

        // Start capture on designated driver
        self.capture_driver.start_capture(&session_id)?;

        *lock = Some(session);
        Ok(session_id)
    }

    /// Mode A: Ingests normalized transcript from WebView2 Web Speech bridge and executes full turn.
    pub async fn submit_web_speech_transcript(
        &self,
        session_id: &VoiceSessionId,
        raw_text: String,
        confidence: Option<f32>,
        language: Option<String>,
        provider_id: Option<String>,
        model_id: Option<String>,
        credentials: Option<String>,
    ) -> Result<String, VoiceError> {
        // 1. Normalize without fabricating dummy audio buffers
        let transcript = WebSpeechSTTBridge::normalize_transcript(raw_text, confidence, language, true)?;

        let mut lock = self.active_session.write().await;
        let session = match *lock {
            Some(ref mut s) if &s.id == session_id => s,
            _ => return Err(VoiceError::Internal("Voice session not found or expired".to_string())),
        };

        if session.cancellation_token.is_cancelled() {
            return Err(VoiceError::Cancelled("Session cancelled prior to execution".to_string()));
        }

        let _ = self.capture_driver.stop_capture();
        session.state = VoiceSessionState::CoreExecution;
        self.emit_voice_event(
            session,
            VoicePayload::StateChanged {
                state: "core_execution".to_string(),
                decibel: None,
            },
        );

        // 2. Submit turn to ConversationCore — backend creates AUTHORITATIVE TurnId
        let turn_submission = self
            .conversation_core
            .submit_turn(TurnSubmissionRequest {
                session_id: session.conversation_id.to_string(),
                message: transcript.text.clone(),
                provider_id,
                model_id,
                temperature: Some(0.7),
                client_turn_id: None,
            })
            .await
            .map_err(|e| VoiceError::Internal(format!("Failed to submit conversation turn: {}", e)))?;

        let authoritative_turn_id = TurnId::from_string(turn_submission.turn_id);

        // 3. Associate authoritative TurnId with VoiceSession
        session.turn_id = Some(authoritative_turn_id.clone());

        let session_clone = session.clone();
        drop(lock); // Release lock during long async execution

        // 4. Execute turn through ConversationCore (Inference + Policy + Universal Tool Runtime)
        let response_text = self
            .conversation_core
            .execute_turn(&authoritative_turn_id, credentials)
            .await
            .map_err(|e| VoiceError::Internal(format!("Conversation execution error: {}", e)))?;

        // Check cancellation
        if session_clone.cancellation_token.is_cancelled() {
            return Err(VoiceError::Cancelled("Session cancelled after turn execution".to_string()));
        }

        // 5. Synthesize Assistant Response via TTSAdapter
        {
            let mut lock = self.active_session.write().await;
            if let Some(ref mut s) = *lock {
                s.state = VoiceSessionState::Synthesizing;
                self.emit_voice_event(
                    s,
                    VoicePayload::StateChanged {
                        state: "synthesizing".to_string(),
                        decibel: None,
                    },
                );
            }
        }

        let tts_opts = TTSOptions::default();
        let audio_buffer = self
            .tts_adapter
            .synthesize(&response_text, &tts_opts, &session_clone.cancellation_token)
            .await?;

        if session_clone.cancellation_token.is_cancelled() {
            return Err(VoiceError::Cancelled("Session cancelled after synthesis".to_string()));
        }

        // 6. Play via single authoritative AudioOutputDriver
        {
            let mut lock = self.active_session.write().await;
            if let Some(ref mut s) = *lock {
                s.state = VoiceSessionState::Speaking;
                self.emit_voice_event(
                    s,
                    VoicePayload::StateChanged {
                        state: "speaking".to_string(),
                        decibel: None,
                    },
                );
            }
        }

        self.audio_output.play(audio_buffer, session_id.clone())?;

        Ok(response_text)
    }

    /// Mode B: Processes raw captured audio frames, transcribes via STTAdapter, and executes full turn.
    pub async fn submit_raw_audio(
        &self,
        session_id: &VoiceSessionId,
        audio_buffer: AudioBuffer,
        stt_options: STTOptions,
        provider_id: Option<String>,
        model_id: Option<String>,
        credentials: Option<String>,
    ) -> Result<String, VoiceError> {
        let mut lock = self.active_session.write().await;
        let session = match *lock {
            Some(ref mut s) if &s.id == session_id => s,
            _ => return Err(VoiceError::Internal("Voice session not found or expired".to_string())),
        };

        if session.cancellation_token.is_cancelled() {
            return Err(VoiceError::Cancelled("Session cancelled before STT".to_string()));
        }

        let _ = self.capture_driver.stop_capture();
        session.state = VoiceSessionState::Transcribing;
        self.emit_voice_event(
            session,
            VoicePayload::StateChanged {
                state: "transcribing".to_string(),
                decibel: None,
            },
        );

        let cancel_tok = session.cancellation_token.clone();
        drop(lock);

        // 1. Transcribe via STTAdapter
        let transcript = self
            .stt_adapter
            .transcribe(&audio_buffer, &stt_options, &cancel_tok)
            .await?;

        // 2. Converge on submit_web_speech_transcript with normalized text
        self.submit_web_speech_transcript(
            session_id,
            transcript.text,
            transcript.confidence,
            Some(transcript.language),
            provider_id,
            model_id,
            credentials,
        )
        .await
    }

    /// Halts active hardware audio playback immediately.
    pub async fn stop_playback(&self) -> Result<(), VoiceError> {
        let _ = self.audio_output.stop();
        let mut lock = self.active_session.write().await;
        if let Some(ref mut session) = *lock {
            if session.state == VoiceSessionState::Speaking {
                session.state = VoiceSessionState::Ended;
                self.emit_voice_event(
                    session,
                    VoicePayload::SessionEnded {
                        session_id: session.id.to_string(),
                        reason: Some("playback_stopped_by_user".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    /// Cancels active voice session cooperatively.
    pub async fn cancel_session(
        &self,
        session_id: &VoiceSessionId,
        reason: Option<String>,
    ) -> Result<(), VoiceError> {
        let mut lock = self.active_session.write().await;
        if let Some(ref mut session) = *lock {
            if &session.id == session_id {
                session.cancellation_token.cancel();
                let _ = self.capture_driver.cancel_capture();
                let _ = self.audio_output.stop();

                if let Some(ref tid) = session.turn_id {
                    let _ = self.conversation_core.cancel_turn(tid, reason.clone()).await;
                }

                session.state = VoiceSessionState::Cancelled;
                self.emit_voice_event(
                    session,
                    VoicePayload::SessionEnded {
                        session_id: session.id.to_string(),
                        reason,
                    },
                );
            }
        }
        Ok(())
    }

    /// Produces a bounded, sanitized status summary for EdithRuntimeState.
    pub async fn status_summary(&self) -> VoiceStatusSummary {
        // 1. If Realtime S2S engine is active, project realtime status
        let rt_lock = self.realtime_engine.read().await;
        if let Some(ref rt) = *rt_lock {
            let session_lock = rt.active_session().read().await;
            if let Some(ref rt_session) = *session_lock {
                let is_active = matches!(
                    rt_session.state,
                    super::realtime::RealtimeSessionState::Connecting
                        | super::realtime::RealtimeSessionState::Connected
                        | super::realtime::RealtimeSessionState::Listening
                        | super::realtime::RealtimeSessionState::Processing
                        | super::realtime::RealtimeSessionState::Speaking
                        | super::realtime::RealtimeSessionState::Interrupted
                        | super::realtime::RealtimeSessionState::Reconnecting
                );
                let active_turn = rt_session.active_turn_id.read().await.as_ref().map(|t| t.to_string());
                return VoiceStatusSummary {
                    is_active,
                    mode: "realtime".to_string(),
                    state: format!("{:?}", rt_session.state).to_lowercase(),
                    session_id: Some(rt_session.id.to_string()),
                    active_turn_id: active_turn,
                    stt_provider: "realtime_audio".to_string(),
                    tts_provider: rt_session.provider_id.clone(),
                    realtime_provider: Some(rt_session.provider_id.clone()),
                    transport_type: Some(rt_session.transport_type.clone()),
                    reconnect_attempt: None,
                    is_muted: false,
                    last_error: match &rt_session.state {
                        super::realtime::RealtimeSessionState::Failed(msg) => Some(msg.clone()),
                        _ => None,
                    },
                };
            }
        }

        // 2. Otherwise project Fallback Voice status
        let lock = self.active_session.read().await;
        match *lock {
            Some(ref session) => VoiceStatusSummary {
                is_active: matches!(
                    session.state,
                    VoiceSessionState::Listening
                        | VoiceSessionState::Transcribing
                        | VoiceSessionState::CoreExecution
                        | VoiceSessionState::Synthesizing
                        | VoiceSessionState::Speaking
                ),
                mode: "fallback".to_string(),
                state: format!("{:?}", session.state).to_lowercase(),
                session_id: Some(session.id.to_string()),
                active_turn_id: session.turn_id.as_ref().map(|t| t.to_string()),
                stt_provider: session.stt_provider.clone(),
                tts_provider: session.tts_provider.clone(),
                realtime_provider: None,
                transport_type: None,
                reconnect_attempt: None,
                is_muted: false,
                last_error: match &session.state {
                    VoiceSessionState::Failed(msg) => Some(msg.clone()),
                    _ => None,
                },
            },
            None => VoiceStatusSummary {
                is_active: false,
                mode: "fallback".to_string(),
                state: "idle".to_string(),
                session_id: None,
                active_turn_id: None,
                stt_provider: self.stt_adapter.name().to_string(),
                tts_provider: self.tts_adapter.name().to_string(),
                realtime_provider: None,
                transport_type: None,
                reconnect_attempt: None,
                is_muted: false,
                last_error: None,
            },
        }
    }

    /// Triggers clean, controlled fallback from Realtime to Phase 9.
    pub async fn trigger_fallback(
        &self,
        conversation_id: ConversationId,
        _reason: &str,
    ) -> Result<VoiceSessionId, VoiceError> {
        // 1. If Realtime engine is running, halt it cleanly and release capture
        let rt_lock = self.realtime_engine.read().await;
        if let Some(ref rt) = *rt_lock {
            let _ = rt.stop_session().await;
        }
        drop(rt_lock);

        // 2. Start fallback session
        self.start_session(conversation_id, CaptureOwner::BrowserWebSpeech).await
    }

    pub fn audio_output(&self) -> &Arc<dyn AudioOutputDriver> {
        &self.audio_output
    }

    pub fn tts_adapter(&self) -> &Arc<dyn TTSAdapter> {
        &self.tts_adapter
    }

    pub fn stt_adapter(&self) -> &Arc<dyn STTAdapter> {
        &self.stt_adapter
    }

    pub fn realtime_engine(&self) -> Arc<RwLock<Option<Arc<super::realtime::RealtimeVoiceEngine>>>> {
        Arc::clone(&self.realtime_engine)
    }
}
