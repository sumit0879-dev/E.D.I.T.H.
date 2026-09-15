#[cfg(test)]
mod tests {
    use super::super::context::{ContextAssembler, StaticMemoryRetriever, UserProfile};
    use super::super::core::ConversationCore;
    use super::super::errors::ConversationError;
    use super::super::turn::TurnStatus;
    use super::super::types::TurnSubmissionRequest;
    use crate::ai::capabilities::{Capability, CapabilitySet};
    use crate::ai::errors::ProviderError;
    use crate::ai::model::ModelMetadata;
    use crate::ai::provider::{
        ChatMessage, GenerateRequest, GenerateResponse, Provider, StreamChunk,
        StreamingTextCapability,
    };
    use crate::ai::ProviderRegistry;
    use crate::events::payload::{EdithPayload, StreamPayload};
    use crate::events::{StreamId, TurnId};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockStreamingProvider {
        id: String,
        name: String,
        chunks: Vec<String>,
        should_fail: bool,
    }

    impl MockStreamingProvider {
        fn new(id: impl Into<String>, chunks: Vec<&str>) -> Self {
            Self {
                id: id.into(),
                name: "Mock Provider".to_string(),
                chunks: chunks.into_iter().map(String::from).collect(),
                should_fail: false,
            }
        }

        fn failing(id: impl Into<String>) -> Self {
            Self {
                id: id.into(),
                name: "Failing Mock Provider".to_string(),
                chunks: Vec::new(),
                should_fail: true,
            }
        }
    }

    impl StreamingTextCapability for MockStreamingProvider {
        fn stream<'a>(
            &'a self,
            _req: &'a GenerateRequest,
            _creds: &'a Option<String>,
            on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync + 'a>,
        ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, ProviderError>> + Send + 'a>> {
            Box::pin(async move {
                if self.should_fail {
                    return Err(ProviderError::RateLimited {
                        retry_after_secs: None,
                        message: "Simulated rate limit".to_string(),
                    });
                }
                for c in &self.chunks {
                    on_chunk(StreamChunk {
                        text: c.clone(),
                        is_done: false,
                    });
                }
                Ok(GenerateResponse {
                    text: self.chunks.join(""),
                    model: "mock-model".to_string(),
                    finish_reason: Some("stop".to_string()),
                })
            })
        }
    }

    impl Provider for MockStreamingProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet::from_slice(&[Capability::Streaming, Capability::TextGeneration])
        }

        fn models(&self) -> Vec<ModelMetadata> {
            Vec::new()
        }

        fn default_model(&self) -> Option<String> {
            Some("mock-model".to_string())
        }

        fn as_streaming_text(&self) -> Option<&dyn StreamingTextCapability> {
            Some(self)
        }
    }

    #[tokio::test]
    async fn test_turn_creation_authoritative_turn_id() {
        let registry = ProviderRegistry::new();
        let core = ConversationCore::mock(registry);

        let req = TurnSubmissionRequest {
            session_id: "sess-123".to_string(),
            message: "Hello E.D.I.T.H.".to_string(),
            provider_id: Some("mock".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        };

        let result = core.submit_turn(req).await.expect("Turn creation must succeed");

        assert!(!result.turn_id.is_empty());
        assert_eq!(result.session_id, "sess-123");
        assert_eq!(result.user_message_text, "Hello E.D.I.T.H.");

        let turn_id = TurnId::from_string(result.turn_id.clone());
        let status = core.get_turn_status(&turn_id).await.expect("Turn status must exist");
        assert_eq!(status.status, TurnStatus::InputAccepted);
        assert_eq!(status.turn_id, result.turn_id);
    }

    #[test]
    fn test_turn_lifecycle_valid_transitions() {
        assert!(TurnStatus::Created.can_transition_to(&TurnStatus::InputAccepted));
        assert!(TurnStatus::InputAccepted.can_transition_to(&TurnStatus::Processing));
        assert!(TurnStatus::Processing.can_transition_to(&TurnStatus::Streaming));
        assert!(TurnStatus::Streaming.can_transition_to(&TurnStatus::Completed));
        assert!(TurnStatus::Streaming.can_transition_to(&TurnStatus::Failed));
        assert!(TurnStatus::Streaming.can_transition_to(&TurnStatus::Cancelled));
    }

    #[test]
    fn test_turn_lifecycle_invalid_transitions_rejected() {
        assert!(!TurnStatus::Completed.can_transition_to(&TurnStatus::Streaming));
        assert!(!TurnStatus::Failed.can_transition_to(&TurnStatus::InputAccepted));
        assert!(!TurnStatus::Cancelled.can_transition_to(&TurnStatus::Completed));
        assert!(!TurnStatus::Created.can_transition_to(&TurnStatus::Completed));
    }

    #[tokio::test]
    async fn test_context_assembly_with_memory_and_profile() {
        let profile = UserProfile {
            nickname: Some("Tony Stark".to_string()),
            occupation: Some("Engineer".to_string()),
            more_about_you: Some("Builder of armor".to_string()),
        };
        let memory = Arc::new(StaticMemoryRetriever::new(vec![
            "Project Mark 85 completed".to_string(),
            "Arc reactor efficiency at 99.8%".to_string(),
        ]));

        let assembler = ContextAssembler::new(
            Some("Stark AI directive.".to_string()),
            Some(profile),
            memory,
        );

        let history = vec![
            ChatMessage { role: "user".to_string(), content: "Status report".to_string() },
            ChatMessage { role: "assistant".to_string(), content: "All systems green".to_string() },
        ];

        let assembled = assembler.assemble_messages(&history, "Optimize power").await;

        assert_eq!(assembled.len(), 4);
        assert_eq!(assembled[0].role, "system");
        assert!(assembled[0].content.contains("Stark AI directive"));
        assert!(assembled[0].content.contains("Tony Stark"));
        assert!(assembled[0].content.contains("Arc reactor efficiency at 99.8%"));

        assert_eq!(assembled[1].role, "user");
        assert_eq!(assembled[1].content, "Status report");

        assert_eq!(assembled[2].role, "assistant");
        assert_eq!(assembled[2].content, "All systems green");

        assert_eq!(assembled[3].role, "user");
        assert_eq!(assembled[3].content, "Optimize power");
    }

    #[tokio::test]
    async fn test_provider_routing_and_streaming_correlation() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockStreamingProvider::new(
            "mock_stream",
            vec!["Hello ", "there, ", "Tony!"],
        )));

        let core = ConversationCore::mock(registry);

        let req = TurnSubmissionRequest {
            session_id: "session-alpha".to_string(),
            message: "Greetings".to_string(),
            provider_id: Some("mock_stream".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        };

        let sub_res = core.submit_turn(req).await.unwrap();
        let turn_id = TurnId::from_string(sub_res.turn_id.clone());
        let stream_id = StreamId::from_string(sub_res.stream_id.clone());

        let result = core.execute_turn(&turn_id, &stream_id, None).await.unwrap();
        assert_eq!(result, "Hello there, Tony!");

        let turn_status = core.get_turn_status(&turn_id).await.unwrap();
        assert_eq!(turn_status.status, TurnStatus::Completed);
        assert_eq!(turn_status.final_response.as_deref(), Some("Hello there, Tony!"));

        // Verify emitted events carry proper correlation
        let events = core.emitter().get_mock_events();
        assert!(events.iter().any(|e| {
            e.correlation.turn_id.as_deref() == Some(turn_id.as_str())
                && e.correlation.stream_id.as_deref() == Some(stream_id.as_str())
                && matches!(&e.payload, EdithPayload::Stream(StreamPayload::Started { .. }))
        }));
        assert!(events.iter().any(|e| {
            e.correlation.turn_id.as_deref() == Some(turn_id.as_str())
                && matches!(&e.payload, EdithPayload::Stream(StreamPayload::Finished { .. }))
        }));
    }

    #[tokio::test]
    async fn test_turn_scoped_cancellation() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockStreamingProvider::new(
            "mock_stream",
            vec!["Part 1", "Part 2"],
        )));

        let core = ConversationCore::mock(registry);

        let req = TurnSubmissionRequest {
            session_id: "sess-cancel".to_string(),
            message: "Run calculation".to_string(),
            provider_id: Some("mock_stream".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        };

        let sub_res = core.submit_turn(req).await.unwrap();
        let turn_id = TurnId::from_string(sub_res.turn_id.clone());

        // Cancel before execution finishes
        assert!(core.cancel_turn(&turn_id, Some("User cancelled".to_string())).await.is_ok());

        let status = core.get_turn_status(&turn_id).await.unwrap();
        assert_eq!(status.status, TurnStatus::Cancelled);

        // Attempting to execute a cancelled turn returns Cancellation error
        let stream_id = StreamId::new();
        let exec_res = core.execute_turn(&turn_id, &stream_id, None).await;
        assert!(matches!(exec_res, Err(ConversationError::Cancellation(_))));
    }

    #[tokio::test]
    async fn test_concurrent_turn_isolation() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockStreamingProvider::new(
            "mock_stream",
            vec!["Chunk A", "Chunk B"],
        )));

        let core = ConversationCore::mock(registry);

        let sub_1 = core.submit_turn(TurnSubmissionRequest {
            session_id: "sess-1".to_string(),
            message: "Turn 1".to_string(),
            provider_id: Some("mock_stream".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        }).await.unwrap();

        let sub_2 = core.submit_turn(TurnSubmissionRequest {
            session_id: "sess-2".to_string(),
            message: "Turn 2".to_string(),
            provider_id: Some("mock_stream".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        }).await.unwrap();

        let id1 = TurnId::from_string(sub_1.turn_id);
        let id2 = TurnId::from_string(sub_2.turn_id);

        assert_ne!(id1, id2);

        // Cancel Turn 1
        assert!(core.cancel_turn(&id1, Some("Stop Turn 1".to_string())).await.is_ok());

        // Execute Turn 2 - must complete successfully without being affected by Turn 1
        let stream_2 = StreamId::from_string(sub_2.stream_id);
        let res2 = core.execute_turn(&id2, &stream_2, None).await;
        assert!(res2.is_ok());

        assert_eq!(core.get_turn_status(&id1).await.unwrap().status, TurnStatus::Cancelled);
        assert_eq!(core.get_turn_status(&id2).await.unwrap().status, TurnStatus::Completed);
    }

    #[tokio::test]
    async fn test_normalized_error_propagation() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockStreamingProvider::failing("mock_fail")));

        let core = ConversationCore::mock(registry);

        let sub = core.submit_turn(TurnSubmissionRequest {
            session_id: "sess-err".to_string(),
            message: "Trigger error".to_string(),
            provider_id: Some("mock_fail".to_string()),
            model_id: Some("mock-model".to_string()),
            temperature: Some(0.7),
            client_turn_id: None,
        }).await.unwrap();

        let turn_id = TurnId::from_string(sub.turn_id);
        let stream_id = StreamId::from_string(sub.stream_id);

        let res = core.execute_turn(&turn_id, &stream_id, None).await;
        assert!(matches!(res, Err(ConversationError::RateLimit(_))));

        let status = core.get_turn_status(&turn_id).await.unwrap();
        assert_eq!(status.status, TurnStatus::Failed);
    }
}
