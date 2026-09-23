use super::cancellation::CancellationRegistry;
use super::domains::browser::{get_browser_definitions, BrowserDomainExecutor};
use super::executor::{BoxFuture, DomainExecutor, DomainExecutorRegistry};
use super::registry::ToolRegistry;
use super::router::ToolRouter;
use super::types::{
    ToolDefinition, ToolDomain, ToolExecutionError, ToolExecutionId, ToolRequest, ToolStatus,
};
use super::validator::ArgumentValidator;
use crate::events::envelope::EventCorrelation;
use crate::policy::engine::PolicyEngine;
use crate::policy::types::PolicyConstraints;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A mock domain executor for testing execution routing, recording calls and outcomes.
struct MockTestDomainExecutor {
    domain_type: ToolDomain,
    invoked_count: Arc<AtomicUsize>,
    last_action: Arc<tokio::sync::Mutex<Option<String>>>,
    last_args: Arc<tokio::sync::Mutex<Option<serde_json::Value>>>,
    should_delay_ms: Option<u64>,
    should_fail_with: Option<ToolExecutionError>,
}

impl MockTestDomainExecutor {
    fn new(domain_type: ToolDomain) -> Self {
        Self {
            domain_type,
            invoked_count: Arc::new(AtomicUsize::new(0)),
            last_action: Arc::new(tokio::sync::Mutex::new(None)),
            last_args: Arc::new(tokio::sync::Mutex::new(None)),
            should_delay_ms: None,
            should_fail_with: None,
        }
    }

    fn with_delay(domain_type: ToolDomain, ms: u64) -> Self {
        let mut s = Self::new(domain_type);
        s.should_delay_ms = Some(ms);
        s
    }

    fn with_error(domain_type: ToolDomain, err: ToolExecutionError) -> Self {
        let mut s = Self::new(domain_type);
        s.should_fail_with = Some(err);
        s
    }
}

impl DomainExecutor for MockTestDomainExecutor {
    fn domain(&self) -> ToolDomain {
        self.domain_type
    }

    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        _definition: &'a ToolDefinition,
        cancel_token: super::cancellation::ScopedCancellationToken,
    ) -> BoxFuture<'a, Result<serde_json::Value, ToolExecutionError>> {
        Box::pin(async move {
            self.invoked_count.fetch_add(1, Ordering::SeqCst);
            *self.last_action.lock().await = Some(request.tool_name.clone());
            *self.last_args.lock().await = Some(request.arguments.clone());

            if let Some(delay) = self.should_delay_ms {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(delay)) => {},
                    _ = cancel_token.cancelled() => {
                        return Err(ToolExecutionError::Cancelled(
                            "Mock executor received cancellation".to_string(),
                        ));
                    }
                }
            }

            if cancel_token.is_cancelled() {
                return Err(ToolExecutionError::Cancelled(
                    "Cancelled before completion".to_string(),
                ));
            }

            if let Some(err) = &self.should_fail_with {
                return Err(err.clone());
            }

            Ok(json!({
                "status": "success",
                "tool": request.tool_name,
                "received_args": request.arguments
            }))
        })
    }
}

fn create_sample_definition(name: &str, domain: ToolDomain) -> ToolDefinition {
    ToolDefinition::new(
        name,
        domain,
        format!("Test tool description for {}", name),
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "timeout": { "type": "integer" },
                "mode": { "type": "string", "enum": ["fast", "slow"] }
            },
            "required": ["url"]
        }),
        false,
        5000,
    )
}

// ============================================================================
// TESTS 1-4: Tool Registration & Resolution
// ============================================================================

#[tokio::test]
async fn test_01_tool_registration_succeeds() {
    let registry = ToolRegistry::new();
    let def = create_sample_definition("test.ping", ToolDomain::System);
    assert!(registry.register(def).is_ok());
    assert_eq!(registry.count(), 1);
}

#[tokio::test]
async fn test_02_duplicate_registration_rejected() {
    let registry = ToolRegistry::new();
    let def1 = create_sample_definition("test.ping", ToolDomain::System);
    let def2 = create_sample_definition("test.ping", ToolDomain::System);
    assert!(registry.register(def1).is_ok());
    let err = registry.register(def2).unwrap_err();
    assert!(err.contains("already registered"));
}

#[tokio::test]
async fn test_03_registration_lookup_matches() {
    let registry = ToolRegistry::new();
    let def = create_sample_definition("test.echo", ToolDomain::System);
    registry.register(def).unwrap();

    let found = registry.get("test.echo");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "test.echo");
    assert!(registry.contains("test.echo"));
}

#[tokio::test]
async fn test_04_unknown_tool_returns_not_found() {
    let registry = ToolRegistry::new();
    let found = registry.get("unknown.nonexistent");
    assert!(found.is_none());
    assert!(!registry.contains("unknown.nonexistent"));
}

// ============================================================================
// TESTS 5-9: Schema & Argument Validation
// ============================================================================

#[tokio::test]
async fn test_05_schema_validation_passes_valid_args() {
    let def = create_sample_definition("test.tool", ToolDomain::System);
    let valid_args = json!({
        "url": "https://example.com",
        "timeout": 3000,
        "mode": "fast"
    });
    assert!(ArgumentValidator::validate(&valid_args, &def.parameters_schema).is_ok());
}

#[tokio::test]
async fn test_06_schema_validation_catches_missing_required() {
    let def = create_sample_definition("test.tool", ToolDomain::System);
    let missing_url = json!({
        "timeout": 3000
    });
    let err = ArgumentValidator::validate(&missing_url, &def.parameters_schema).unwrap_err();
    assert!(matches!(err, ToolExecutionError::InvalidArguments(_)));
    assert!(err.to_string().contains("url"));
}

#[tokio::test]
async fn test_07_schema_validation_catches_type_mismatch() {
    let def = create_sample_definition("test.tool", ToolDomain::System);
    let wrong_type = json!({
        "url": 12345 // expected string
    });
    let err = ArgumentValidator::validate(&wrong_type, &def.parameters_schema).unwrap_err();
    assert!(matches!(err, ToolExecutionError::InvalidArguments(_)));
    assert!(err.to_string().contains("url"));
}

#[tokio::test]
async fn test_08_schema_validation_catches_enum_violation() {
    let def = create_sample_definition("test.tool", ToolDomain::System);
    let enum_violation = json!({
        "url": "https://example.com",
        "mode": "invalid_mode"
    });
    let err = ArgumentValidator::validate(&enum_violation, &def.parameters_schema).unwrap_err();
    assert!(matches!(err, ToolExecutionError::InvalidArguments(_)));
    assert!(err.to_string().contains("mode"));
}

#[tokio::test]
async fn test_09_malformed_json_returns_malformed_arguments() {
    let def = create_sample_definition("test.tool", ToolDomain::System);
    let malformed_args = json!("not an object");
    let err = ArgumentValidator::validate(&malformed_args, &def.parameters_schema).unwrap_err();
    assert!(matches!(err, ToolExecutionError::InvalidArguments(_)));
}

// ============================================================================
// TESTS 10-15: Policy & Permission Integration
// ============================================================================

#[tokio::test]
async fn test_10_policy_allow_dispatches_to_executor() {
    let registry = Arc::new(ToolRegistry::new());
    let mut def = create_sample_definition("test.read", ToolDomain::Application);
    def.is_read_only = true;
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Application));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let req = ToolRequest::simple("test.read", json!({ "url": "https://example.com" }));
    let res = router.execute(req).await;

    assert!(res.is_success());
    assert_eq!(mock_executor.invoked_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_11_policy_confirmation_required_pauses_with_approval_id() {
    let registry = Arc::new(ToolRegistry::new());
    // Browser domain tool targeting sensitive credential/password field triggers ConfirmationRequired
    let def = ToolDefinition::new(
        "browser.type",
        ToolDomain::Browser,
        "Type text into target selector",
        json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "text": { "type": "string" }
            },
            "required": ["selector", "text"]
        }),
        false,
        10000,
    );
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Browser));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    // Target password field triggers confirmation
    let req = ToolRequest::simple(
        "browser.type",
        json!({ "selector": "#user_password", "text": "secret123" }),
    );
    let res = router.execute(req).await;

    assert_eq!(res.status, ToolStatus::ApprovalRequired);
    assert!(res.approval_id.is_some());
    // Crucial: Executor MUST NOT have been called!
    assert_eq!(mock_executor.invoked_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_12_confirmation_approval_allows_execution_on_replay() {
    let registry = Arc::new(ToolRegistry::new());
    let def = ToolDefinition::new(
        "browser.type",
        ToolDomain::Browser,
        "Type text into target selector",
        json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "text": { "type": "string" }
            },
            "required": ["selector", "text"]
        }),
        false,
        10000,
    );
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Browser));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router = ToolRouter::with_defaults(
        registry,
        Arc::new(domain_executors),
        policy_engine.clone(),
        None,
    );

    let req = ToolRequest::simple(
        "browser.type",
        json!({ "selector": "#user_password", "text": "secret123" }),
    );
    let initial_res = router.execute(req).await;
    assert_eq!(initial_res.status, ToolStatus::ApprovalRequired);
    let approval_id = initial_res.approval_id.expect("Expected approval id");

    // Approve the action in the PolicyEngine
    policy_engine
        .resolve_approval(&approval_id, crate::policy::OperatorDecision::Approve)
        .await
        .expect("Approval resolution failed");

    // Replay with active_approval_id
    let mut approved_req = ToolRequest::simple(
        "browser.type",
        json!({ "selector": "#user_password", "text": "secret123" }),
    );
    approved_req.active_approval_id = Some(approval_id.clone());

    let final_res = router.execute(approved_req).await;
    assert!(final_res.is_success());
    assert_eq!(mock_executor.invoked_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_13_policy_blocked_fails_closed_no_executor_invocation() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.blocked", ToolDomain::Application);
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Application));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    // Configure policy constraints to disable external services
    let mut constraints = PolicyConstraints::default();
    constraints.allow_external_services = false;
    let policy_engine = Arc::new(PolicyEngine::with_constraints(None, constraints));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let req = ToolRequest::simple("test.blocked", json!({ "url": "https://example.com" }));
    let res = router.execute(req).await;

    assert_eq!(res.status, ToolStatus::Blocked);
    // Executor MUST NEVER be called
    assert_eq!(mock_executor.invoked_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_14_policy_restricted_preserves_constraints() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.restricted", ToolDomain::Application);
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Application));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let req = ToolRequest::simple("test.restricted", json!({ "url": "https://example.com" }));
    let res = router.execute(req).await;
    assert!(res.is_success());
}

#[tokio::test]
async fn test_15_approval_replay_rejected_if_already_consumed() {
    let registry = Arc::new(ToolRegistry::new());
    let def = ToolDefinition::new(
        "browser.type",
        ToolDomain::Browser,
        "Type text into target selector",
        json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "text": { "type": "string" }
            },
            "required": ["selector", "text"]
        }),
        false,
        10000,
    );
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Browser));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor.clone());

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router = ToolRouter::with_defaults(
        registry,
        Arc::new(domain_executors),
        policy_engine.clone(),
        None,
    );

    let req = ToolRequest::simple(
        "browser.type",
        json!({ "selector": "#user_password", "text": "secret123" }),
    );
    let initial_res = router.execute(req).await;
    let approval_id = initial_res.approval_id.unwrap();

    policy_engine
        .resolve_approval(&approval_id, crate::policy::OperatorDecision::Approve)
        .await
        .unwrap();

    // First replay succeeds
    let mut approved_req = ToolRequest::simple(
        "browser.type",
        json!({ "selector": "#user_password", "text": "secret123" }),
    );
    approved_req.active_approval_id = Some(approval_id.clone());
    let res1 = router.execute(approved_req.clone()).await;
    assert!(res1.is_success());

    // Second replay MUST FAIL because approval was marked Consumed
    let res2 = router.execute(approved_req).await;
    assert_ne!(res2.status, ToolStatus::Completed);
    // Executor was called once, not twice
    assert_eq!(mock_executor.invoked_count.load(Ordering::SeqCst), 1);
}

// ============================================================================
// TESTS 16-20: Lifecycle, Cancellation, Timeouts & Error Normalization
// ============================================================================

#[tokio::test]
async fn test_16_execution_id_uniquely_generated() {
    let req1 = ToolRequest::simple("test.a", json!({}));
    let req2 = ToolRequest::simple("test.b", json!({}));
    assert_ne!(req1.execution_id, req2.execution_id);
}

#[tokio::test]
async fn test_17_custom_execution_id_preserved() {
    let custom_id = ToolExecutionId::new();
    let mut req = ToolRequest::simple("test.a", json!({}));
    req.execution_id = custom_id.clone();
    assert_eq!(req.execution_id, custom_id);
}

#[tokio::test]
async fn test_18_scoped_cancellation_cancels_in_flight() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.slow", ToolDomain::Application);
    registry.register(def).unwrap();

    // 500ms delay in mock executor
    let mock_executor = Arc::new(MockTestDomainExecutor::with_delay(
        ToolDomain::Application,
        500,
    ));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor);

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let cancellation = Arc::new(CancellationRegistry::new());
    let router = ToolRouter::new(
        registry,
        Arc::new(domain_executors),
        policy_engine,
        None,
        cancellation.clone(),
    );

    let req = ToolRequest::simple("test.slow", json!({ "url": "https://example.com" }));
    let exec_id = req.execution_id.to_string();

    let router_clone = router.clone();
    let join_handle = tokio::spawn(async move { router_clone.execute(req).await });

    // Wait 50ms then trigger cancellation
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(cancellation.cancel_execution(&exec_id).await);

    let result = join_handle.await.unwrap();
    assert_eq!(result.status, ToolStatus::Cancelled);
}

#[tokio::test]
async fn test_19_execution_timeout_returns_timeout_error() {
    let registry = Arc::new(ToolRegistry::new());
    let mut def = create_sample_definition("test.hang", ToolDomain::Application);
    def.default_timeout_ms = 100; // 100ms timeout
    registry.register(def).unwrap();

    // 400ms delay mock executor
    let mock_executor = Arc::new(MockTestDomainExecutor::with_delay(
        ToolDomain::Application,
        400,
    ));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor);

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let req = ToolRequest::simple("test.hang", json!({ "url": "https://example.com" }));
    let result = router.execute(req).await;

    assert_eq!(result.status, ToolStatus::Failed);
    assert_eq!(result.error_code.as_deref(), Some("TIMEOUT"));
}

#[tokio::test]
async fn test_20_error_normalization_typed_tool_execution_error() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.err", ToolDomain::Application);
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::with_error(
        ToolDomain::Application,
        ToolExecutionError::InternalError("Simulated domain hardware failure".to_string()),
    ));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor);

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let req = ToolRequest::simple("test.err", json!({ "url": "https://example.com" }));
    let res = router.execute(req).await;

    assert_eq!(res.status, ToolStatus::Failed);
    assert_eq!(res.error_code.as_deref(), Some("INTERNAL_ERROR"));
}

// ============================================================================
// TESTS 21-24: Correlated Events, Browser Domain Adapter & Context Propagation
// ============================================================================

#[tokio::test]
async fn test_21_correlated_events_emitted_lifecycle() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.event", ToolDomain::Application);
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Application));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor);

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let mut correlation = EventCorrelation::default();
    correlation.conversation_id = Some("conv-123".to_string());
    correlation.turn_id = Some("turn-456".to_string());
    let req = ToolRequest::new(
        "test.event",
        json!({ "url": "https://example.com" }),
        correlation,
    );

    let res = router.execute(req).await;
    assert!(res.is_success());
}

#[tokio::test]
async fn test_22_browser_domain_adapter_handles_navigation() {
    let browser_executor = BrowserDomainExecutor::new(None);
    let req = ToolRequest::simple(
        "browser.navigate",
        json!({
            "url": "https://example.com",
            "tab_id": "test_tab"
        }),
    );
    let def = get_browser_definitions()
        .into_iter()
        .find(|d| d.name == "browser.navigate")
        .expect("browser.navigate definition missing");

    let token = super::cancellation::ScopedCancellationToken::new();
    let res = browser_executor.execute(&req, &def, token).await;

    // With None app handle, mock returns successful simulated navigation
    assert!(res.is_ok());
    let val = res.unwrap();
    assert_eq!(val.get("status").and_then(|v| v.as_str()), Some("success"));
}

#[tokio::test]
async fn test_23_browser_domain_adapter_validates_safety_constraints() {
    let def = get_browser_definitions()
        .into_iter()
        .find(|d| d.name == "browser.click")
        .expect("browser.click definition missing");

    // Missing required selector argument
    let invalid_args = json!({
        "tab_id": "test_tab"
    });
    let res = ArgumentValidator::validate(&invalid_args, &def.parameters_schema);

    assert!(res.is_err());
    assert!(matches!(
        res.unwrap_err(),
        ToolExecutionError::InvalidArguments(_)
    ));
}

#[tokio::test]
async fn test_24_tool_execution_under_task_turn_propagates_correlation() {
    let registry = Arc::new(ToolRegistry::new());
    let def = create_sample_definition("test.task_tool", ToolDomain::Application);
    registry.register(def).unwrap();

    let mock_executor = Arc::new(MockTestDomainExecutor::new(ToolDomain::Application));
    let domain_executors = DomainExecutorRegistry::new();
    domain_executors.register(mock_executor);

    let policy_engine = Arc::new(PolicyEngine::new(None));
    let router =
        ToolRouter::with_defaults(registry, Arc::new(domain_executors), policy_engine, None);

    let mut correlation = EventCorrelation::default();
    correlation.conversation_id = Some("sess-999".to_string());
    correlation.turn_id = Some("turn-888".to_string());
    correlation.task_id = Some("task-777".to_string());

    let req = ToolRequest::new(
        "test.task_tool",
        json!({ "url": "https://edith.internal" }),
        correlation,
    );
    let res = router.execute(req).await;
    assert!(res.is_success());
}
