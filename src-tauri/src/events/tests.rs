use super::*;
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn test_event_id_uniqueness() {
    let mut set = HashSet::new();
    for _ in 0..10_000 {
        let id = EventId::new();
        assert!(set.insert(id.0), "Collision detected in EventId generation!");
    }
    assert_eq!(set.len(), 10_000);
}

#[test]
fn test_envelope_serialization_roundtrip() {
    let correlation = EventCorrelation::for_stream(
        "conv-123".to_string(),
        "turn-456".to_string(),
        "stream-789".to_string(),
    );
    let payload = EdithPayload::Stream(StreamPayload::Chunk {
        text: "Hello, world!".to_string(),
        sequence_number: 1,
        is_final: false,
    });

    let envelope = EdithEventEnvelope::new(correlation, payload);
    let json = serde_json::to_string(&envelope).unwrap();

    let deserialized: EdithEventEnvelope<EdithPayload> = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope.event_id, deserialized.event_id);
    assert_eq!(envelope.timestamp_ms, deserialized.timestamp_ms);
    assert_eq!(envelope.correlation, deserialized.correlation);
    assert_eq!(envelope.payload, deserialized.payload);
}

#[test]
fn test_optional_correlation_omitted() {
    let correlation = EventCorrelation {
        conversation_id: Some("conv-1".to_string()),
        turn_id: None,
        stream_id: None,
        task_id: None,
        tool_execution_id: None,
        voice_session_id: None,
    };
    let payload = EdithPayload::Runtime(RuntimePayload::Notification {
        level: "info".to_string(),
        message: "System initialized".to_string(),
    });

    let envelope = EdithEventEnvelope::new(correlation, payload);
    let json = serde_json::to_string(&envelope).unwrap();

    // Verify omitted fields are not serialized as null in JSON
    assert!(!json.contains("\"turn_id\":null"));
    assert!(!json.contains("\"stream_id\":null"));
    assert!(!json.contains("\"task_id\":null"));
    assert!(json.contains("\"conversation_id\":\"conv-1\""));
}

#[test]
fn test_stream_lifecycle_success() {
    let emitter = EventEmitter::mock();
    let correlation = EventCorrelation::for_stream(
        "conv-test".to_string(),
        "turn-1".to_string(),
        "stream-alpha".to_string(),
    );

    // 1. Started
    emitter.emit_stream_started(&correlation, "llama-3.3-70b-versatile").unwrap();

    // 2. Chunks (sequence 1, 2, 3)
    emitter.emit_stream_chunk(&correlation, "The", 1, false).unwrap();
    emitter.emit_stream_chunk(&correlation, " quick", 2, false).unwrap();
    emitter.emit_stream_chunk(&correlation, " brown", 3, true).unwrap();

    // 3. Finished
    emitter.emit_stream_finished(&correlation, Some(3), Some("stop".to_string())).unwrap();

    let events = emitter.get_mock_events();
    assert_eq!(events.len(), 5);

    match &events[0].payload {
        EdithPayload::Stream(StreamPayload::Started { model }) => {
            assert_eq!(model, "llama-3.3-70b-versatile");
        }
        other => panic!("Expected Started, got {:?}", other),
    }

    match &events[1].payload {
        EdithPayload::Stream(StreamPayload::Chunk { text, sequence_number, is_final }) => {
            assert_eq!(text, "The");
            assert_eq!(*sequence_number, 1);
            assert!(!is_final);
        }
        other => panic!("Expected Chunk 1, got {:?}", other),
    }

    match &events[3].payload {
        EdithPayload::Stream(StreamPayload::Chunk { text, sequence_number, is_final }) => {
            assert_eq!(text, " brown");
            assert_eq!(*sequence_number, 3);
            assert!(is_final);
        }
        other => panic!("Expected Chunk 3, got {:?}", other),
    }

    match &events[4].payload {
        EdithPayload::Stream(StreamPayload::Finished { finish_reason, .. }) => {
            assert_eq!(finish_reason.as_deref(), Some("stop"));
        }
        other => panic!("Expected Finished, got {:?}", other),
    }
}

#[test]
fn test_stream_lifecycle_failed() {
    let emitter = EventEmitter::mock();
    let correlation = EventCorrelation::for_stream(
        "conv-test".to_string(),
        "turn-1".to_string(),
        "stream-err".to_string(),
    );

    emitter.emit_stream_started(&correlation, "gemini-2.5-flash").unwrap();
    emitter.emit_stream_chunk(&correlation, "Incomplete...", 1, false).unwrap();
    emitter.emit_stream_failed(&correlation, "Connection timed out", Some("timeout".to_string())).unwrap();

    let events = emitter.get_mock_events();
    assert_eq!(events.len(), 3);

    match &events[2].payload {
        EdithPayload::Stream(StreamPayload::Failed { error, error_type }) => {
            assert_eq!(error, "Connection timed out");
            assert_eq!(error_type.as_deref(), Some("timeout"));
        }
        other => panic!("Expected Failed, got {:?}", other),
    }
}

#[test]
fn test_concurrent_stream_isolation() {
    let emitter = Arc::new(EventEmitter::mock());

    let corr_a = EventCorrelation::for_stream(
        "session-1".to_string(),
        "turn-a".to_string(),
        "stream-a".to_string(),
    );
    let corr_b = EventCorrelation::for_stream(
        "session-2".to_string(),
        "turn-b".to_string(),
        "stream-b".to_string(),
    );

    let emitter_a = Arc::clone(&emitter);
    let handle_a = std::thread::spawn(move || {
        emitter_a.emit_stream_started(&corr_a, "model-a").unwrap();
        for i in 1..=50 {
            emitter_a
                .emit_stream_chunk(&corr_a, format!("A{}", i), i, i == 50)
                .unwrap();
        }
        emitter_a.emit_stream_finished(&corr_a, Some(50), None).unwrap();
    });

    let emitter_b = Arc::clone(&emitter);
    let handle_b = std::thread::spawn(move || {
        emitter_b.emit_stream_started(&corr_b, "model-b").unwrap();
        for i in 1..=50 {
            emitter_b
                .emit_stream_chunk(&corr_b, format!("B{}", i), i, i == 50)
                .unwrap();
        }
        emitter_b.emit_stream_finished(&corr_b, Some(50), None).unwrap();
    });

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    let all_events = emitter.get_mock_events();
    // 52 events from Stream A (Started + 50 chunks + Finished) + 52 from Stream B = 104
    assert_eq!(all_events.len(), 104);

    // Filter Stream A
    let stream_a_events: Vec<_> = all_events
        .iter()
        .filter(|e| e.correlation.stream_id.as_deref() == Some("stream-a"))
        .collect();

    assert_eq!(stream_a_events.len(), 52);
    // Verify Stream A sequence numbers are strictly 1..=50 and contain only A payloads
    let mut expected_seq = 1;
    for env in &stream_a_events[1..51] {
        match &env.payload {
            EdithPayload::Stream(StreamPayload::Chunk { text, sequence_number, .. }) => {
                assert_eq!(*sequence_number, expected_seq);
                assert_eq!(text, &format!("A{}", expected_seq));
                assert_eq!(env.correlation.conversation_id.as_deref(), Some("session-1"));
                assert_eq!(env.correlation.turn_id.as_deref(), Some("turn-a"));
                expected_seq += 1;
            }
            other => panic!("Expected chunk, got {:?}", other),
        }
    }

    // Filter Stream B
    let stream_b_events: Vec<_> = all_events
        .iter()
        .filter(|e| e.correlation.stream_id.as_deref() == Some("stream-b"))
        .collect();

    assert_eq!(stream_b_events.len(), 52);
    let mut expected_seq = 1;
    for env in &stream_b_events[1..51] {
        match &env.payload {
            EdithPayload::Stream(StreamPayload::Chunk { text, sequence_number, .. }) => {
                assert_eq!(*sequence_number, expected_seq);
                assert_eq!(text, &format!("B{}", expected_seq));
                assert_eq!(env.correlation.conversation_id.as_deref(), Some("session-2"));
                assert_eq!(env.correlation.turn_id.as_deref(), Some("turn-b"));
                expected_seq += 1;
            }
            other => panic!("Expected chunk, got {:?}", other),
        }
    }
}

#[test]
fn test_cancellation_isolation() {
    let emitter = EventEmitter::mock();

    let corr_active = EventCorrelation::for_stream(
        "session-1".to_string(),
        "turn-active".to_string(),
        "stream-active".to_string(),
    );
    let corr_cancelled = EventCorrelation::for_stream(
        "session-2".to_string(),
        "turn-cancelled".to_string(),
        "stream-cancelled".to_string(),
    );

    // Both start
    emitter.emit_stream_started(&corr_active, "model-1").unwrap();
    emitter.emit_stream_started(&corr_cancelled, "model-2").unwrap();

    // Stream 1 emits chunk
    emitter.emit_stream_chunk(&corr_active, "Token 1", 1, false).unwrap();

    // Stream 2 is cancelled
    emitter.emit_stream_cancelled(&corr_cancelled, Some("User cancelled".to_string())).unwrap();

    // Stream 1 continues unaffected
    emitter.emit_stream_chunk(&corr_active, "Token 2", 2, true).unwrap();
    emitter.emit_stream_finished(&corr_active, Some(2), Some("stop".to_string())).unwrap();

    let events = emitter.get_mock_events();
    assert_eq!(events.len(), 6);

    // Verify Stream 2 has only Started and Cancelled
    let s2_events: Vec<_> = events
        .iter()
        .filter(|e| e.correlation.stream_id.as_deref() == Some("stream-cancelled"))
        .collect();
    assert_eq!(s2_events.len(), 2);
    match &s2_events[1].payload {
        EdithPayload::Stream(StreamPayload::Cancelled { reason }) => {
            assert_eq!(reason.as_deref(), Some("User cancelled"));
        }
        other => panic!("Expected Cancelled, got {:?}", other),
    }

    // Verify Stream 1 finished successfully
    let s1_events: Vec<_> = events
        .iter()
        .filter(|e| e.correlation.stream_id.as_deref() == Some("stream-active"))
        .collect();
    assert_eq!(s1_events.len(), 4);
    match &s1_events[3].payload {
        EdithPayload::Stream(StreamPayload::Finished { finish_reason, .. }) => {
            assert_eq!(finish_reason.as_deref(), Some("stop"));
        }
        other => panic!("Expected Finished, got {:?}", other),
    }
}
