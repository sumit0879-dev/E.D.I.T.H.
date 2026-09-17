//! tests.rs — Comprehensive unit and mock integration tests for E.D.I.T.H. Fallback Voice.

use super::*;
use crate::ai::{CapabilitySet, GenerateRequest, GenerateResponse, ModelMetadata, Provider, ProviderError, ProviderRegistry, TextGenerationCapability};
use crate::conversation::ConversationCore;
use crate::events::{ConversationId, EventEmitter, VoiceSessionId};
use crate::task::CancellationToken;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// -----------------------------------------------------------------------------
// Test Helpers: Mock Provider for ConversationCore
// -----------------------------------------------------------------------------

#[derive(Debug)]
struct MockVoiceTestProvider;

impl Provider for MockVoiceTestProvider {
    fn id(&self) -> &str {
        "mock-voice-provider"
    }
    fn name(&self) -> &str {
        "Mock Voice Provider"
    }
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::new()
    }
    fn models(&self) -> Vec<ModelMetadata> {
        vec![]
    }
    fn default_model(&self) -> Option<String> {
        Some("test-model".to_string())
    }
    fn as_text_generation(&self) -> Option<&dyn TextGenerationCapability> {
        Some(self)
    }
}

impl TextGenerationCapability for MockVoiceTestProvider {
    fn generate<'a>(
        &'a self,
        _req: &'a GenerateRequest,
        _creds: &'a Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(GenerateResponse {
                text: "Hello! Tactical systems online and operational.".to_string(),
                model: "test-model".to_string(),
                finish_reason: Some("stop".to_string()),
            })
        })
    }
}

fn create_test_conversation_core() -> Arc<ConversationCore> {
    let mut registry = ProviderRegistry::new();
    let prov: Arc<dyn Provider> = Arc::new(MockVoiceTestProvider);
    registry.register(prov);
    let emitter = EventEmitter::mock();
    Arc::new(ConversationCore::new(registry, emitter, None, None))
}

// -----------------------------------------------------------------------------
// 1. Audio Buffer Normalization & PCM Conversion Tests
// -----------------------------------------------------------------------------

#[test]
fn test_audio_buffer_normalization() {
    let samples = vec![0.0, 0.5, -0.5, 1.2, -1.5];
    let buf = AudioBuffer::new(CANONICAL_STT_SAMPLE_RATE, 1, samples);

    // Verify clamping
    assert_eq!(buf.samples[3], 1.0);
    assert_eq!(buf.samples[4], -1.0);
    assert_eq!(buf.sample_rate, 16000);
    assert_eq!(buf.channels, 1);
    assert_eq!(buf.len(), 5);

    // Duration calculation
    let sr = 16000;
    let one_sec = AudioBuffer::new(sr, 1, vec![0.0; sr as usize]);
    assert_eq!(one_sec.duration_ms(), 1000);

    // Mono conversion
    let stereo = AudioBuffer::new(sr, 2, vec![0.2, 0.4, -0.2, -0.4]);
    let mono = stereo.to_mono();
    assert_eq!(mono.channels, 1);
    assert_eq!(mono.len(), 2);
    assert!((mono.samples[0] - 0.3).abs() < 1e-5);
    assert!((mono.samples[1] - (-0.3)).abs() < 1e-5);

    // PCM i16 Round-trip
    let orig = AudioBuffer::new(16000, 1, vec![0.0, 0.5, -0.5, 1.0, -1.0]);
    let bytes = orig.to_i16_pcm();
    assert_eq!(bytes.len(), 10); // 5 samples * 2 bytes
    let decoded = AudioBuffer::from_i16_pcm(&bytes, 16000, 1);
    assert_eq!(decoded.len(), orig.len());
    for (a, b) in orig.samples.iter().zip(decoded.samples.iter()) {
        assert!((a - b).abs() < 0.001);
    }
}

#[test]
fn test_audio_buffer_linear_resampling() {
    let original_sr = 16000;
    let target_sr = 24000;
    let buf = AudioBuffer::new(original_sr, 1, vec![0.0, 0.5, 1.0, 0.5, 0.0]);
    let resampled = buf.resample_linear(target_sr);
    assert_eq!(resampled.sample_rate, target_sr);
    assert!(resampled.len() > buf.len());
}

// -----------------------------------------------------------------------------
// 2. STT Adapter & Web Speech Bridge Tests
// -----------------------------------------------------------------------------

#[test]
fn test_web_speech_bridge_normalization() {
    // Normal input
    let res = WebSpeechSTTBridge::normalize_transcript(
        "  Status report on system diagnostics  ".to_string(),
        Some(0.95),
        Some("en-US".to_string()),
        true,
    );
    assert!(res.is_ok());
    let t = res.unwrap();
    assert_eq!(t.text, "Status report on system diagnostics");
    assert_eq!(t.confidence, Some(0.95));
    assert_eq!(t.language, "en-US");
    assert!(t.is_final);

    // Empty input fails cleanly
    let empty_res = WebSpeechSTTBridge::normalize_transcript("   ".to_string(), None, None, true);
    assert!(matches!(empty_res, Err(VoiceError::EmptyTranscript)));
}

#[tokio::test]
async fn test_stt_adapter_contract() {
    let adapter = MockSTTAdapter::new("test-mock-stt");
    adapter.set_transcript("Operational readiness 100%");

    let cancel = CancellationToken::new();
    let audio = AudioBuffer::new(16000, 1, vec![0.1; 1600]);
    let opts = STTOptions::default();

    let res = adapter.transcribe(&audio, &opts, &cancel).await;
    assert!(res.is_ok());
    let t = res.unwrap();
    assert_eq!(t.text, "Operational readiness 100%");
    assert_eq!(t.confidence, Some(0.98));

    // Test cancellation
    cancel.cancel();
    let res_cancelled = adapter.transcribe(&audio, &opts, &cancel).await;
    assert!(matches!(res_cancelled, Err(VoiceError::Cancelled(_))));
}

// -----------------------------------------------------------------------------
// 3. TTS Adapter Contract Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_tts_adapter_contract() {
    let adapter = MockTtsAdapter::new("test-mock-tts");
    let cancel = CancellationToken::new();
    let opts = TTSOptions::default();

    let res = adapter.synthesize("System ready.", &opts, &cancel).await;
    assert!(res.is_ok());
    let buf = res.unwrap();
    assert!(!buf.is_empty());
    assert_eq!(buf.sample_rate, CANONICAL_TTS_SAMPLE_RATE);

    // Voices list
    let voices = adapter.list_voices().await.unwrap();
    assert!(!voices.is_empty());

    // Test cancellation
    cancel.cancel();
    let res_cancelled = adapter.synthesize("System ready.", &opts, &cancel).await;
    assert!(matches!(res_cancelled, Err(VoiceError::Cancelled(_))));
}

// -----------------------------------------------------------------------------
// 4. Capture Ownership & Exclusivity Tests
// -----------------------------------------------------------------------------

#[test]
fn test_capture_ownership_exclusivity() {
    let bridge = BrowserCaptureBridge::new();
    let sess_id = VoiceSessionId::new();

    // Start browser capture
    assert!(bridge.start_capture(&sess_id).is_ok());
    assert_eq!(bridge.owner(), CaptureOwner::BrowserWebSpeech);
    assert_eq!(bridge.state(), CaptureState::Recording);

    // Conflicting second capture must fail
    let conflict = bridge.start_capture(&sess_id);
    assert!(matches!(conflict, Err(VoiceError::CaptureConflict(_))));

    // Stopping releases lock
    assert!(bridge.stop_capture().is_ok());
    assert_eq!(bridge.state(), CaptureState::Idle);

    // Can capture again cleanly
    assert!(bridge.start_capture(&sess_id).is_ok());
    assert!(bridge.cancel_capture().is_ok());
    assert_eq!(bridge.state(), CaptureState::Idle);
}

// -----------------------------------------------------------------------------
// 5. Single Authoritative Playback Sink & Stop Tests
// -----------------------------------------------------------------------------

#[test]
fn test_duplicate_playback_prevention_and_stop() {
    let driver = MockAudioOutputDriver::new();
    let sess_id = VoiceSessionId::new();
    let buf = AudioBuffer::new(24000, 1, vec![0.1; 2400]);

    assert!(!driver.is_playing());
    assert!(driver.play(buf, sess_id.clone()).is_ok());
    assert!(driver.is_playing());
    assert_eq!(driver.get_played_buffers_count(), 1);
    assert_eq!(driver.last_played_session(), Some(sess_id));

    // Stop halts playback
    assert!(driver.stop().is_ok());
    assert!(!driver.is_playing());
    assert_eq!(driver.get_stop_count(), 1);
}

// -----------------------------------------------------------------------------
// 6. VoiceController Lifecycle & Authoritative TurnId Integration
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_conversation_core_convergence_and_turn_id() {
    let core = create_test_conversation_core();
    let stt = Arc::new(MockSTTAdapter::new("mock-stt"));
    stt.set_transcript("Run system self-check");
    let tts = Arc::new(MockTtsAdapter::new("mock-tts"));
    let output = Arc::new(MockAudioOutputDriver::new());
    let capture = Arc::new(BrowserCaptureBridge::new());
    let emitter = Arc::new(EventEmitter::mock());

    let controller = VoiceController::new(
        core.clone(),
        stt,
        tts,
        output.clone(),
        capture,
        Some(emitter.clone()),
    );

    let conv_id = ConversationId::from_string("session-voice-test-1");

    // 1. Start Session
    let session_id = controller
        .start_session(conv_id.clone(), CaptureOwner::BrowserWebSpeech)
        .await
        .unwrap();

    let summary = controller.status_summary().await;
    assert!(summary.is_active);
    assert_eq!(summary.state, "listening");
    assert_eq!(summary.active_turn_id, None); // TurnId is not yet created!

    // 2. Submit Transcript via Mode A Web Speech Bridge
    let res = controller
        .submit_web_speech_transcript(
            &session_id,
            "Run system self-check".to_string(),
            Some(0.99),
            Some("en-US".to_string()),
            Some("mock-voice-provider".to_string()),
            Some("test-model".to_string()),
            None,
        )
        .await;

    assert!(res.is_ok());
    let response_text = res.unwrap();
    assert_eq!(response_text, "Hello! Tactical systems online and operational.");

    // Verify authoritative TurnId association
    let final_summary = controller.status_summary().await;
    assert!(final_summary.active_turn_id.is_some());
    let authoritative_turn_id = final_summary.active_turn_id.unwrap();
    assert!(authoritative_turn_id.starts_with("turn-") || !authoritative_turn_id.is_empty());

    // Verify Audio was sent to single authoritative output driver
    assert_eq!(output.get_played_buffers_count(), 1);
    assert!(output.is_playing());

    // Verify correlated events were emitted
    let events = emitter.get_mock_events();
    assert!(!events.is_empty());

    let has_started = events.iter().any(|e| match &e.payload {
        crate::events::EdithPayload::Voice(crate::events::VoicePayload::SessionStarted { .. }) => true,
        _ => false,
    });
    assert!(has_started);
}

// -----------------------------------------------------------------------------
// 7. Barge-in / Interruption Test
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_barge_in_interruption() {
    let core = create_test_conversation_core();
    let stt = Arc::new(MockSTTAdapter::new("mock-stt"));
    let tts = Arc::new(MockTtsAdapter::new("mock-tts"));
    let output = Arc::new(MockAudioOutputDriver::new());
    let capture = Arc::new(BrowserCaptureBridge::new());
    let emitter = Arc::new(EventEmitter::mock());

    let controller = VoiceController::new(
        core,
        stt,
        tts,
        output.clone(),
        capture,
        Some(emitter.clone()),
    );

    let conv_id = ConversationId::from_string("session-barge-in-test");

    // Start initial session
    let s1 = controller
        .start_session(conv_id.clone(), CaptureOwner::BrowserWebSpeech)
        .await
        .unwrap();

    // Transition to speaking
    let _ = controller
        .submit_web_speech_transcript(
            &s1,
            "First question".to_string(),
            Some(0.95),
            None,
            Some("mock-voice-provider".to_string()),
            Some("test-model".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(output.is_playing());

    // User speaks / triggers barge-in by starting a new session
    let s2 = controller
        .start_session(conv_id.clone(), CaptureOwner::BrowserWebSpeech)
        .await
        .unwrap();

    assert_ne!(s1, s2);
    // Output driver stop was invoked by barge-in
    assert!(!output.is_playing());
    assert!(output.get_stop_count() >= 1);

    // Verify BargeIn event was emitted
    let events = emitter.get_mock_events();
    let has_barge_in = events.iter().any(|e| match &e.payload {
        crate::events::EdithPayload::Voice(crate::events::VoicePayload::BargeInTriggered { .. }) => true,
        _ => false,
    });
    assert!(has_barge_in);
}

// -----------------------------------------------------------------------------
// 8. Cooperative Cancellation Test
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_cooperative_cancellation() {
    let core = create_test_conversation_core();
    let stt = Arc::new(MockSTTAdapter::new("mock-stt"));
    let tts = Arc::new(MockTtsAdapter::new("mock-tts"));
    let output = Arc::new(MockAudioOutputDriver::new());
    let capture = Arc::new(BrowserCaptureBridge::new());
    let emitter = Arc::new(EventEmitter::mock());

    let controller = VoiceController::new(
        core,
        stt,
        tts,
        output.clone(),
        capture,
        Some(emitter.clone()),
    );

    let conv_id = ConversationId::from_string("session-cancel-test");
    let s1 = controller
        .start_session(conv_id.clone(), CaptureOwner::BrowserWebSpeech)
        .await
        .unwrap();

    // Cancel session
    assert!(controller
        .cancel_session(&s1, Some("user_aborted".to_string()))
        .await
        .is_ok());

    let summary = controller.status_summary().await;
    assert!(!summary.is_active);
    assert_eq!(summary.state, "cancelled");

    // Submitting on cancelled session fails
    let res = controller
        .submit_web_speech_transcript(
            &s1,
            "Query after cancel".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .await;

    assert!(matches!(res, Err(VoiceError::Cancelled(_))));
}
