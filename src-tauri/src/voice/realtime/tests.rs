use super::*;
use crate::ai::{Capability, CapabilitySet, ProviderRegistry};
use crate::conversation::ConversationCore;
use crate::events::ConversationId;
use crate::policy::engine::PolicyEngine;
use crate::tools::{DomainExecutorRegistry, ToolDefinition, ToolDomain, ToolRegistry, ToolRouter};
use crate::voice::capture::{AudioCaptureDriver, MockAudioCaptureDriver};
use crate::voice::output::MockAudioOutputDriver;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

// Helper: Setup in-memory test environment
fn setup_test_env() -> (
    Arc<ConversationCore>,
    Arc<ToolRouter>,
    Arc<MockAudioOutputDriver>,
    Arc<MockAudioCaptureDriver>,
    Arc<RealtimeVoiceEngine>,
) {
    let registry = ProviderRegistry::standard_builtins();
    let conversation_core = Arc::new(ConversationCore::mock(registry));
    let tool_registry = Arc::new(ToolRegistry::new());

    // Register a mock tool
    let test_tool = ToolDefinition::new(
        "get_weather",
        ToolDomain::System,
        "Returns weather forecast",
        json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
        false,
        5000,
    );
    let _ = tool_registry.register(test_tool);

    let executors = Arc::new(DomainExecutorRegistry::new());
    let policy_engine = Arc::new(PolicyEngine::new(None));
    let tool_router = Arc::new(ToolRouter::with_defaults(
        tool_registry,
        executors,
        policy_engine,
        None,
    ));

    let audio_output = Arc::new(MockAudioOutputDriver::new());
    let capture_driver = Arc::new(MockAudioCaptureDriver::new());
    let engine = Arc::new(RealtimeVoiceEngine::new(
        Arc::clone(&conversation_core),
        Arc::clone(&tool_router),
        Arc::clone(&audio_output) as Arc<dyn crate::voice::AudioOutputDriver>,
        Arc::clone(&capture_driver) as Arc<dyn crate::voice::AudioCaptureDriver>,
        None,
    ));

    (
        conversation_core,
        tool_router,
        audio_output,
        capture_driver,
        engine,
    )
}

#[tokio::test]
async fn test_01_realtime_capability_detection() {
    let mut caps = CapabilitySet::new();
    assert!(!caps.has(Capability::RealtimeAudio));
    caps.insert(Capability::RealtimeAudio);
    assert!(caps.has(Capability::RealtimeAudio));
}

#[tokio::test]
async fn test_02_transport_abstraction_lifecycle() {
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    assert_eq!(transport.state(), TransportState::Connected);

    let frame = AudioFrame::new(
        crate::events::VoiceSessionId::new(),
        1,
        0,
        16000,
        1,
        vec![0.1, -0.2, 0.3],
        FrameDirection::Input,
        1,
    );

    let send_res = transport.send_frame(frame.clone()).await;
    assert!(send_res.is_ok());

    let popped = transport.pop_outbound_frame().await;
    assert_eq!(popped, Some(frame));

    let _ = transport.close(None).await;
    assert_eq!(transport.state(), TransportState::Closed);
}

#[tokio::test]
async fn test_03_authoritative_turn_ownership_in_conversation_core() {
    let (core, _, _, _, engine) = setup_test_env();
    let conv_id = ConversationId::new();
    let session = RealtimeVoiceSession::new(conv_id.clone(), "test-prov", "mock");

    // Initially no active turn in session
    assert!(session.active_turn_id.read().await.is_none());

    // Request authoritative turn through engine -> delegates to ConversationCore
    let turn_id = engine.ensure_active_turn(&session).await.unwrap();
    assert_eq!(session.active_turn_id.read().await.as_ref(), Some(&turn_id));

    // Calling ensure_active_turn again returns the same existing TurnId without creating another
    let turn_id_2 = engine.ensure_active_turn(&session).await.unwrap();
    assert_eq!(turn_id, turn_id_2);

    // Verify turn exists in ConversationCore with status Processing
    let status = core.get_turn_status(&turn_id).await.unwrap();
    assert_eq!(status.status, crate::conversation::TurnStatus::Processing);

    // Finalize turn with user text and assistant response
    engine
        .finalize_active_turn(
            &session,
            Some("Hello EDITH".to_string()),
            Some("Hello user".to_string()),
        )
        .await
        .unwrap();

    // Verify turn completed in ConversationCore
    let status_completed = core.get_turn_status(&turn_id).await.unwrap();
    assert_eq!(
        status_completed.status,
        crate::conversation::TurnStatus::Completed
    );
    assert_eq!(
        status_completed.final_response,
        Some("Hello user".to_string())
    );

    // Active turn in session should now be None
    assert!(session.active_turn_id.read().await.is_none());
}

#[tokio::test]
async fn test_04_audio_frame_monotonic_sequence_and_duration() {
    let sid = crate::events::VoiceSessionId::new();
    let session = RealtimeVoiceSession::new(ConversationId::new(), "test-prov", "mock");

    let seq1 = session.next_input_sequence();
    let seq2 = session.next_input_sequence();
    assert_eq!(seq1, 1);
    assert_eq!(seq2, 2);

    // 160 samples at 16,000 Hz = 10ms
    let samples = vec![0.05f32; 160];
    let frame = AudioFrame::new(sid, seq1, 0, 16000, 1, samples, FrameDirection::Input, 1);
    assert_eq!(frame.duration_ms(), 10);
}

#[tokio::test]
async fn test_05_stale_assistant_audio_rejection_on_generation_increment() {
    let (_core, _, output, _, engine) = setup_test_env();
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport.clone()));
    let conv_id = ConversationId::new();

    let sid = engine
        .start_session(conv_id, "mock-prov", adapter.clone())
        .await
        .unwrap();

    // Inject a frame with generation_id = 1 (active)
    let frame_gen_1 = AudioFrame::new(
        sid.clone(),
        1,
        100,
        24000,
        1,
        vec![0.1f32; 240],
        FrameDirection::Output,
        1,
    );
    transport
        .inject_inbound_event(TransportEvent::Audio(frame_gen_1))
        .await
        .unwrap();

    // Give event loop time to process
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(output.get_played_buffers_count(), 1);

    // Trigger barge-in: increments generation to 2 and halts output
    let session_arc = engine.active_session().read().await.clone().unwrap();
    engine
        .handle_barge_in(&session_arc, adapter.as_ref(), "user_interrupt")
        .await;

    assert_eq!(session_arc.current_generation(), 2);
    assert!(output.get_stop_count() >= 1);

    // Now inject a stale frame with generation_id = 1
    let stale_frame = AudioFrame::new(
        sid.clone(),
        2,
        200,
        24000,
        1,
        vec![0.2f32; 240],
        FrameDirection::Output,
        1, // Stale!
    );
    transport
        .inject_inbound_event(TransportEvent::Audio(stale_frame))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stale frame was discarded! Played buffers count did NOT increment!
    assert_eq!(output.get_played_buffers_count(), 1);

    // Inject a fresh frame with generation_id = 2
    let fresh_frame = AudioFrame::new(
        sid.clone(),
        3,
        300,
        24000,
        1,
        vec![0.3f32; 240],
        FrameDirection::Output,
        2, // Fresh!
    );
    transport
        .inject_inbound_event(TransportEvent::Audio(fresh_frame))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(output.get_played_buffers_count(), 2);

    engine.stop_session().await.unwrap();
}

#[tokio::test]
async fn test_06_barge_in_cancels_turn_in_conversation_core() {
    let (core, _, _, _, engine) = setup_test_env();
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport.clone()));
    let conv_id = ConversationId::new();

    engine
        .start_session(conv_id, "mock-prov", adapter.clone())
        .await
        .unwrap();

    let session_arc = engine.active_session().read().await.clone().unwrap();

    // Start turn
    let turn_id = engine.ensure_active_turn(&session_arc).await.unwrap();
    let status_before = core.get_turn_status(&turn_id).await.unwrap();
    assert_eq!(
        status_before.status,
        crate::conversation::TurnStatus::Processing
    );

    // User speaks -> triggers barge-in
    engine
        .handle_barge_in(&session_arc, adapter.as_ref(), "user_barge_in")
        .await;

    // Turn in ConversationCore was cancelled!
    let status_after = core.get_turn_status(&turn_id).await.unwrap();
    assert_eq!(
        status_after.status,
        crate::conversation::TurnStatus::Cancelled
    );

    // Session's active turn is reset to None
    assert!(session_arc.active_turn_id.read().await.is_none());

    // Next utterance allocates fresh turn
    let next_turn_id = engine.ensure_active_turn(&session_arc).await.unwrap();
    assert_ne!(turn_id, next_turn_id);

    engine.stop_session().await.unwrap();
}

#[tokio::test]
async fn test_07_realtime_tool_call_delegation_to_universal_tool_runtime() {
    let (_core, _, _, _, engine) = setup_test_env();
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport.clone()));
    let conv_id = ConversationId::new();

    engine
        .start_session(conv_id, "mock-prov", adapter.clone())
        .await
        .unwrap();

    // Inject a tool call event from provider
    let tool_call_event = TransportEvent::ToolCall {
        call_id: "call_abc123".to_string(),
        tool_name: "get_weather".to_string(),
        arguments: json!({ "location": "New York" }),
    };

    transport
        .inject_inbound_event(tool_call_event)
        .await
        .unwrap();

    // Allow event loop to dispatch through ToolRouter
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Turn was created in ConversationCore
    let session_arc = engine.active_session().read().await.clone().unwrap();
    assert!(session_arc.active_turn_id.read().await.is_some());

    engine.stop_session().await.unwrap();
}

#[tokio::test]
async fn test_08_input_backpressure_timeout_triggers_stream_discontinuity() {
    let (core, _, _, _, engine) = setup_test_env();
    // Transport with 1 capacity so second send will block/timeout
    let transport = Arc::new(MockAudioFrameTransport::new(1));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport.clone()));
    let conv_id = ConversationId::new();

    let session = Arc::new(RealtimeVoiceSession::new(conv_id, "mock-prov", "mock"));

    // Fill the 1-capacity transport
    let frame1 = AudioFrame::new(
        session.id.clone(),
        1,
        0,
        16000,
        1,
        vec![0.1; 160],
        FrameDirection::Input,
        1,
    );
    adapter.send_audio(frame1).await.unwrap();

    // Send next frame with a tiny timeout to simulate transport saturation
    let mut config_engine = RealtimeVoiceEngine::new(
        Arc::clone(&core),
        engine.tool_router().clone(),
        engine.audio_output().clone(),
        engine.capture_driver().clone(),
        None,
    );
    config_engine.config_mut().backpressure_timeout_ms = 50; // 50ms timeout

    let res = config_engine
        .dispatch_input_frame(&session, adapter.as_ref(), vec![0.2; 160], 16000)
        .await;

    // Must return backpressure error with stream discontinuity
    assert!(res.is_err());
    assert!(config_engine.is_fallback_needed());
}

#[tokio::test]
async fn test_09_mutual_exclusion_of_microphone_capture() {
    let (_, _, _, capture, engine) = setup_test_env();
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport));

    assert_eq!(capture.state(), crate::voice::CaptureState::Idle);

    // Start realtime session -> acquires capture lock
    let sid = engine
        .start_session(ConversationId::new(), "mock-prov", adapter)
        .await
        .unwrap();

    assert_eq!(capture.state(), crate::voice::CaptureState::Recording);

    // Attempting to start another capture conflicts
    let conflict = capture.start_capture(&sid);
    assert!(conflict.is_err());

    // Stop session -> releases capture lock
    engine.stop_session().await.unwrap();
    assert_eq!(capture.state(), crate::voice::CaptureState::Idle);
}

#[tokio::test]
async fn test_10_cooperative_cancellation_propagates_cleanly() {
    let (_, _, _output, capture, engine) = setup_test_env();
    let transport = Arc::new(MockAudioFrameTransport::new(16));
    let adapter = Arc::new(MockRealtimeSessionAdapter::new(transport));

    engine
        .start_session(ConversationId::new(), "mock-prov", adapter)
        .await
        .unwrap();

    assert_eq!(capture.state(), crate::voice::CaptureState::Recording);

    // Cancel session
    engine.stop_session().await.unwrap();

    // Session state should be cleared, capture stopped, and output stopped
    assert!(engine.active_session().read().await.is_none());
    assert_eq!(capture.state(), crate::voice::CaptureState::Idle);
}
