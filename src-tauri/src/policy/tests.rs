use super::*;
use crate::events::emitter::EventEmitter;
use crate::events::envelope::EventCorrelation;
use serde_json::json;
use std::path::PathBuf;

fn sample_context(mode: SecurityMode) -> PolicyContext {
    PolicyContext::new_with_scope(
        Some("session-123".to_string()),
        Some("turn-456".to_string()),
        Some("task-789".to_string()),
        ActionSource::AutonomousAgent,
        mode,
        vec![PathBuf::from(r"C:\safe_workspace")],
    )
}

#[tokio::test]
async fn test_safe_diagnostics_allowed_in_standard_mode() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Standard);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "whoami",
        vec!["/user".to_string()],
        json!({ "command": "whoami /user" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Allow);
    assert_eq!(decision.risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_shell_interpreter_invocation_strictly_blocked() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "cmd.exe",
        vec!["/c".to_string(), "dir".to_string()],
        json!({ "command": "cmd.exe /c dir" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_shell_operator_chaining_strictly_blocked() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "whoami",
        vec!["&".to_string(), "dir".to_string()],
        json!({ "command": "whoami & dir" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_destructive_command_requires_confirmation() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["-rf".to_string(), "/tmp".to_string()],
        json!({ "command": "rm -rf /tmp" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert!(decision.approval_id.is_some());
    assert_eq!(decision.risk_level, RiskLevel::High);
}

#[tokio::test]
async fn test_browser_unsafe_javascript_scheme_blocked() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_url(
        "browser",
        "navigate",
        "javascript:alert(1)",
        json!({ "url": "javascript:alert(1)" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_browser_file_scheme_blocked() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_url(
        "browser",
        "navigate",
        "file:///C:/Windows/System32/cmd.exe",
        json!({ "url": "file:///C:/Windows/System32/cmd.exe" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_browser_safe_navigation_allowed() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Standard);

    let req = ActionRequest::new_url(
        "browser",
        "navigate",
        "https://crates.io",
        json!({ "url": "https://crates.io" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Allow);
    assert_eq!(decision.risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_browser_passive_observation_allowed() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Standard);

    let req = ActionRequest::new(
        "browser",
        "screenshot",
        ActionTarget::None,
        json!({}),
        EventCorrelation::default(),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Allow);
    assert_eq!(decision.risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_browser_password_input_requires_confirmation() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new(
        "browser",
        "type",
        ActionTarget::BrowserElement {
            selector: Some("#password_input".to_string()),
            element_id: None,
            text: None,
        },
        json!({
            "selector": "#password_input",
            "text": "my_super_secret",
            "is_password": true
        }),
        EventCorrelation::default(),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
    assert!(decision.approval_id.is_some());
}

#[tokio::test]
async fn test_browser_financial_button_requires_confirmation() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new(
        "browser",
        "click",
        ActionTarget::BrowserElement {
            selector: Some("#checkout-btn".to_string()),
            element_id: None,
            text: Some("Buy Now - $199".to_string()),
        },
        json!({
            "selector": "#checkout-btn",
            "text": "Buy Now - $199"
        }),
        EventCorrelation::default(),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::High);
    assert!(decision.approval_id.is_some());
}

#[tokio::test]
async fn test_approval_lifecycle_and_single_use_consumption() {
    let engine = PolicyEngine::default();
    let mut ctx = sample_context(SecurityMode::Autonomous);

    // 1. Initial request requires confirmation
    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["-f".to_string(), "temp.log".to_string()],
        json!({ "command": "rm -f temp.log" }),
    );

    let decision1 = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision1.outcome, PolicyOutcome::ConfirmationRequired);
    let approval_id = decision1.approval_id.expect("Expected approval ID");

    // 2. Operator resolves approval
    let resolve_res = engine
        .resolve_approval(
            &approval_id,
            OperatorDecision::Approve,
        )
        .await;
    assert!(resolve_res.is_ok());

    // 3. Execution with valid approval token -> ALLOW
    ctx.active_approval_id = Some(approval_id.clone());
    let decision2 = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision2.outcome, PolicyOutcome::Allow);

    // 4. Replay attempt with consumed approval token -> BLOCKED
    let decision3 = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision3.outcome, PolicyOutcome::Blocked);
    assert!(decision3.reason.contains("consumed") || decision3.reason.contains("Invalid authorization"));
}

#[tokio::test]
async fn test_tampered_arguments_rejected_despite_valid_approval_id() {
    let engine = PolicyEngine::default();
    let mut ctx = sample_context(SecurityMode::Autonomous);

    // 1. Request confirmation for deleting specific file
    let original_req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["temp.txt".to_string()],
        json!({ "command": "rm temp.txt" }),
    );

    let decision1 = engine.evaluate(&original_req, &ctx).await;
    let approval_id = decision1.approval_id.expect("Approval ID expected");

    // 2. Operator approves deletion of temp.txt
    engine
        .resolve_approval(
            &approval_id,
            OperatorDecision::Approve,
        )
        .await
        .unwrap();

    // 3. Attacker modifies argument to delete entire filesystem
    let tampered_req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["/system32".to_string()],
        json!({ "command": "rm /system32" }),
    );

    ctx.active_approval_id = Some(approval_id);
    let decision2 = engine.evaluate(&tampered_req, &ctx).await;

    // Must be strictly BLOCKED due to SHA-256 hash mismatch
    assert_eq!(decision2.outcome, PolicyOutcome::Blocked);
    assert!(decision2.reason.contains("hash") || decision2.reason.contains("parameters do not match"));
}

#[tokio::test]
async fn test_denied_approval_cannot_authorize_action() {
    let engine = PolicyEngine::default();
    let mut ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["important.db".to_string()],
        json!({ "command": "rm important.db" }),
    );

    let decision1 = engine.evaluate(&req, &ctx).await;
    let approval_id = decision1.approval_id.unwrap();

    // Operator explicitly DENIES
    engine
        .resolve_approval(
            &approval_id,
            OperatorDecision::Deny,
        )
        .await
        .unwrap();

    // Attempting execution with denied approval
    ctx.active_approval_id = Some(approval_id);
    let decision2 = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision2.outcome, PolicyOutcome::Blocked);
    assert!(decision2.reason.contains("denied"));
}

#[tokio::test]
async fn test_policy_update_invalidates_prior_approvals() {
    let engine = PolicyEngine::default();
    let mut ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["scratch.log".to_string()],
        json!({ "command": "rm scratch.log" }),
    );

    let decision1 = engine.evaluate(&req, &ctx).await;
    let approval_id = decision1.approval_id.unwrap();

    engine
        .resolve_approval(&approval_id, OperatorDecision::Approve)
        .await
        .unwrap();

    // Policy constraints changed (policy_version incremented)
    let mut new_constraints = engine.get_constraints().await;
    new_constraints.allow_command_execution = false;
    engine.update_constraints(new_constraints).await;

    // Execution must be rejected due to policy version mismatch
    ctx.active_approval_id = Some(approval_id);
    let decision2 = engine.evaluate(&req, &ctx).await;
    assert_eq!(decision2.outcome, PolicyOutcome::Blocked);
    assert!(decision2.reason.contains("version"));
}

#[tokio::test]
async fn test_audit_trail_redacts_sensitive_credentials() {
    let engine = PolicyEngine::default();
    let ctx = sample_context(SecurityMode::Standard);

    let req = ActionRequest::new(
        "browser",
        "type",
        ActionTarget::None,
        json!({
            "username": "admin@example.com",
            "password": "SuperSecretPassword123!",
            "api_key": "sk-1234567890abcdef",
            "nested": {
                "token": "bearer-secret-token"
            }
        }),
        EventCorrelation::default(),
    );

    engine.evaluate(&req, &ctx).await;

    let audit_log = engine.get_audit_log(10).await;
    assert!(!audit_log.is_empty());

    let entry = &audit_log[0];
    let sanitized_args = &entry.sanitized_arguments;

    assert_eq!(sanitized_args["username"], "admin@example.com");
    assert_eq!(sanitized_args["password"], "[REDACTED]");
    assert_eq!(sanitized_args["api_key"], "[REDACTED]");
    assert_eq!(sanitized_args["nested"]["token"], "[REDACTED]");
}

#[tokio::test]
async fn test_event_emitter_correlated_lifecycle() {
    let mock_emitter = EventEmitter::mock();
    let engine = PolicyEngine::new(Some(mock_emitter.clone()));
    let ctx = sample_context(SecurityMode::Autonomous);

    let req = ActionRequest::new_command(
        "system",
        "execute_command",
        "rm",
        vec!["test.tmp".to_string()],
        json!({ "command": "rm test.tmp" }),
    );

    let decision = engine.evaluate(&req, &ctx).await;
    let approval_id = decision.approval_id.unwrap();

    // Verify ApprovalRequested event was emitted
    let events = mock_emitter.get_mock_events();
    assert!(events.len() >= 2); // ApprovalRequested + PolicyEvaluated

    // Resolve approval
    engine
        .resolve_approval(&approval_id, OperatorDecision::Approve)
        .await
        .unwrap();

    // Verify ApprovalResolved event was emitted
    let updated_events = mock_emitter.get_mock_events();
    assert!(updated_events.len() >= 3);
}
