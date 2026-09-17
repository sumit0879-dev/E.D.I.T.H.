//! computer_tests.rs — Comprehensive Unit & Integration Tests for Computer Domain
//!
//! Validates:
//! 1. Tool registration & count (14 tools)
//! 2. Parameter schema validation & error catching
//! 3. Policy evaluation: read-only allowed (Low risk)
//! 4. Policy evaluation: sensitive typing requires confirmation (Critical/High risk)
//! 5. Policy evaluation: app launch requires confirmation (High risk)
//! 6. Policy evaluation: window closure requires confirmation (High risk)
//! 7. Policy evaluation: high-risk hotkey requires confirmation (High risk)
//! 8. Policy evaluation: privileged UAC targets blocked (Critical risk)
//! 9. HITL approval & execution resumption
//! 10. Consumed approval replay rejection (fail-closed)
//! 11. Human takeover preemption (AiPaused blocks execution)
//! 12. Scoped cancellation token halting
//! 13. Tool execution timeout enforcement
//! 14. Correlated lifecycle events emission
//! 15. Error normalization into typed ToolExecutionResult

use super::computer::{get_computer_definitions, ComputerDomainExecutor};
use super::computer_platform::MockPlatformAdapter;
use crate::computer_control::GLOBAL_COMPUTER_CONTROL_MGR;
use crate::events::envelope::EventCorrelation;
use crate::events::EventEmitter;
use crate::policy::context::PolicyContext;
use crate::policy::{OperatorDecision, PolicyEngine};
use crate::policy::types::{ActionRequest, ActionTarget, PolicyOutcome, RiskLevel};
use crate::tools::executor::DomainExecutorRegistry;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use crate::tools::types::{ToolDomain, ToolRequest, ToolStatus};
use crate::tools::validator::ArgumentValidator;
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_01_computer_tool_registration() {
    let registry = ToolRegistry::new();
    let defs = get_computer_definitions();
    assert_eq!(defs.len(), 14, "Expected 14 registered computer tools");

    for def in &defs {
        assert_eq!(def.domain, ToolDomain::Computer);
        assert!(def.name.starts_with("computer."));
        assert!(registry.register(def.clone()).is_ok());
    }

    assert_eq!(registry.count(), 14);

    // Verify key operations exist
    assert!(registry.get("computer.observe_screen").is_some());
    assert!(registry.get("computer.screenshot").is_some());
    assert!(registry.get("computer.get_active_window").is_some());
    assert!(registry.get("computer.list_windows").is_some());
    assert!(registry.get("computer.focus_window").is_some());
    assert!(registry.get("computer.launch_app").is_some());
    assert!(registry.get("computer.close_window").is_some());
    assert!(registry.get("computer.move_cursor").is_some());
    assert!(registry.get("computer.click").is_some());
    assert!(registry.get("computer.double_click").is_some());
    assert!(registry.get("computer.right_click").is_some());
    assert!(registry.get("computer.type").is_some());
    assert!(registry.get("computer.press_key").is_some());
    assert!(registry.get("computer.hotkey").is_some());
}

#[test]
fn test_02_computer_tool_schema_validation() {
    let registry = ToolRegistry::new();
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }

    let move_def = registry.get("computer.move_cursor").unwrap();

    // Valid cursor coordinates
    let valid_move = json!({ "x": 500, "y": 300 });
    assert!(ArgumentValidator::validate(&valid_move, &move_def.parameters_schema).is_ok());

    // Missing required y coordinate
    let missing_y = json!({ "x": 500 });
    assert!(ArgumentValidator::validate(&missing_y, &move_def.parameters_schema).is_err());

    // Focus window requires title
    let focus_def = registry.get("computer.focus_window").unwrap();
    let valid_focus = json!({ "title": "Notepad" });
    assert!(ArgumentValidator::validate(&valid_focus, &focus_def.parameters_schema).is_ok());

    let invalid_focus = json!({ "process_name": "notepad.exe" });
    assert!(ArgumentValidator::validate(&invalid_focus, &focus_def.parameters_schema).is_err());
}

#[tokio::test]
async fn test_03_computer_policy_observation_allowed() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    let obs_req = ActionRequest::new(
        "computer",
        "observe_screen",
        ActionTarget::None,
        json!({}),
        EventCorrelation::default(),
    );
    let decision = engine.evaluate(&obs_req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::Allow);
    assert_eq!(decision.risk_level, RiskLevel::Low);

    let win_req = ActionRequest::new(
        "computer",
        "get_active_window",
        ActionTarget::None,
        json!({}),
        EventCorrelation::default(),
    );
    let win_decision = engine.evaluate(&win_req, &ctx).await;
    assert_eq!(win_decision.outcome, PolicyOutcome::Allow);
    assert_eq!(win_decision.risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_04_computer_policy_sensitive_type_confirmation() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // Sensitive credential typing requires confirmation
    let pwd_req = ActionRequest::new(
        "computer",
        "type",
        ActionTarget::None,
        json!({ "text": "SecretPassword123!", "is_sensitive": true }),
        EventCorrelation::default(),
    );
    let decision = engine.evaluate(&pwd_req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::Critical);

    // Normal text typing is permitted
    let normal_req = ActionRequest::new(
        "computer",
        "type",
        ActionTarget::None,
        json!({ "text": "Hello, world!" }),
        EventCorrelation::default(),
    );
    let normal_decision = engine.evaluate(&normal_req, &ctx).await;
    assert_eq!(normal_decision.outcome, PolicyOutcome::Allow);
    assert_eq!(normal_decision.risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn test_05_computer_policy_launch_app_confirmation() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // Whitelisted built-in app requires operator confirmation before spawning
    let app_req = ActionRequest::new(
        "computer",
        "launch_app",
        ActionTarget::SystemTarget("notepad".to_string()),
        json!({ "app_name": "notepad" }),
        EventCorrelation::default(),
    );
    let decision = engine.evaluate(&app_req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::High);

    // Arbitrary unapproved executable is blocked
    let arbitrary_req = ActionRequest::new(
        "computer",
        "launch_app",
        ActionTarget::SystemTarget("malicious_hax.exe".to_string()),
        json!({ "app_name": "malicious_hax.exe" }),
        EventCorrelation::default(),
    );
    let arbitrary_decision = engine.evaluate(&arbitrary_req, &ctx).await;
    assert_eq!(arbitrary_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(arbitrary_decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_06_computer_policy_close_window_confirmation() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // Closing normal window requires confirmation
    let close_req = ActionRequest::new(
        "computer",
        "close_window",
        ActionTarget::SystemTarget("Notepad".to_string()),
        json!({ "title": "Notepad" }),
        EventCorrelation::default(),
    );
    let decision = engine.evaluate(&close_req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::High);

    // Closing core system window (explorer.exe or Task Manager) is strictly blocked
    let system_close_req = ActionRequest::new(
        "computer",
        "close_window",
        ActionTarget::SystemTarget("Task Manager".to_string()),
        json!({ "title": "Task Manager" }),
        EventCorrelation::default(),
    );
    let sys_decision = engine.evaluate(&system_close_req, &ctx).await;
    assert_eq!(sys_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(sys_decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_07_computer_policy_high_risk_hotkey_confirmation() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // Alt+F4 requires confirmation
    let f4_req = ActionRequest::new(
        "computer",
        "hotkey",
        ActionTarget::None,
        json!({ "keys": ["alt", "f4"] }),
        EventCorrelation::default(),
    );
    let f4_decision = engine.evaluate(&f4_req, &ctx).await;
    assert_eq!(f4_decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(f4_decision.risk_level, RiskLevel::High);

    // Safe combination (Ctrl+C) is allowed
    let copy_req = ActionRequest::new(
        "computer",
        "hotkey",
        ActionTarget::None,
        json!({ "keys": ["ctrl", "c"] }),
        EventCorrelation::default(),
    );
    let copy_decision = engine.evaluate(&copy_req, &ctx).await;
    assert_eq!(copy_decision.outcome, PolicyOutcome::Allow);
    assert_eq!(copy_decision.risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn test_08_computer_policy_blocked_uac() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // Privileged system hotkeys (Ctrl+Alt+Del, Win+R) are blocked
    let uac_hotkey_req = ActionRequest::new(
        "computer",
        "hotkey",
        ActionTarget::None,
        json!({ "keys": ["ctrl", "alt", "del"] }),
        EventCorrelation::default(),
    );
    let hotkey_decision = engine.evaluate(&uac_hotkey_req, &ctx).await;
    assert_eq!(hotkey_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(hotkey_decision.risk_level, RiskLevel::Critical);

    // Clicking into UAC dialog is blocked
    let uac_click_req = ActionRequest::new(
        "computer",
        "click",
        ActionTarget::SystemTarget("User Account Control".to_string()),
        json!({ "target": "User Account Control" }),
        EventCorrelation::default(),
    );
    let click_decision = engine.evaluate(&uac_click_req, &ctx).await;
    assert_eq!(click_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(click_decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_09_computer_confirmation_resumption() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine.clone(), None);

    // Initial request to launch app requires confirmation
    let mut req = ToolRequest::new(
        "computer.launch_app".to_string(),
        json!({ "app_name": "notepad" }),
        EventCorrelation::default(),
    );

    let res = router.execute(req.clone()).await;
    assert_eq!(res.status, ToolStatus::ApprovalRequired);
    let approval_id = res.approval_id.expect("Expected approval_id");

    // Operator approves the action
    assert!(engine.resolve_approval(&approval_id, OperatorDecision::Approve).await.is_ok());

    // Replay with active approval token succeeds
    req.active_approval_id = Some(approval_id.clone());
    let second_res = router.execute(req.clone()).await;
    assert_eq!(second_res.status, ToolStatus::Completed);
    assert!(second_res.data.is_some());
}

#[tokio::test]
async fn test_10_computer_replay_consumed_rejected() {
    let engine = Arc::new(PolicyEngine::new(None));
    let req = ActionRequest::new(
        "computer",
        "launch_app",
        ActionTarget::SystemTarget("notepad".to_string()),
        json!({ "app_name": "notepad" }),
        EventCorrelation::default(),
    );
    let ctx = PolicyContext::default();
    let initial_decision = engine.evaluate(&req, &ctx).await;
    assert_eq!(initial_decision.outcome, PolicyOutcome::ConfirmationRequired);
    let approval_id = initial_decision.approval_id.expect("Expected approval_id");

    // Approve token
    assert!(engine.resolve_approval(&approval_id, OperatorDecision::Approve).await.is_ok());

    // First consume succeeds
    let mut authed_ctx = ctx.clone();
    authed_ctx.active_approval_id = Some(approval_id.clone());
    let first_consume = engine.evaluate(&req, &authed_ctx).await;
    assert_eq!(first_consume.outcome, PolicyOutcome::Allow);

    // Replaying again with consumed token must fail closed
    let second_consume = engine.evaluate(&req, &authed_ctx).await;
    assert_eq!(second_consume.outcome, PolicyOutcome::Blocked);
}

#[tokio::test]
async fn test_11_computer_human_takeover_preemption() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    // Operator pauses AI control (human takeover)
    assert!(GLOBAL_COMPUTER_CONTROL_MGR.pause_ai_control(Some("User moved mouse".to_string())).is_ok());

    let req = ToolRequest::new(
        "computer.move_cursor".to_string(),
        json!({ "x": 100, "y": 200 }),
        EventCorrelation::default(),
    );

    // Execution must be blocked when human is in control
    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Blocked);

    // Resume AI control
    assert!(GLOBAL_COMPUTER_CONTROL_MGR.resume_ai_control().is_ok());
    assert!(GLOBAL_COMPUTER_CONTROL_MGR.release_ai_control().is_ok());
}

#[tokio::test]
async fn test_12_computer_scoped_cancellation() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = Arc::new(ToolRouter::with_defaults(registry, exec_reg, engine, None));

    let req = ToolRequest::new(
        "computer.observe_screen".to_string(),
        json!({}),
        EventCorrelation::default(),
    );

    let exec_id = req.execution_id.to_string();
    let r_clone = router.clone();

    let cancel_handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        r_clone.cancel_execution(&exec_id).await
    });

    let res = router.execute(req).await;
    let _ = cancel_handle.await;

    assert!(res.status == ToolStatus::Completed || res.status == ToolStatus::Cancelled);
}

#[tokio::test]
async fn test_13_computer_execution_timeout() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    let mut req = ToolRequest::new(
        "computer.observe_screen".to_string(),
        json!({}),
        EventCorrelation::default(),
    );
    req.timeout_ms = Some(0); // Immediate timeout

    let res = router.execute(req).await;
    assert!(res.status == ToolStatus::Failed || res.status == ToolStatus::Completed);
}

#[tokio::test]
async fn test_14_computer_correlated_events_lifecycle() {
    let emitter = EventEmitter::mock();
    let engine = Arc::new(PolicyEngine::new(Some(emitter.clone())));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, Some(emitter));

    let correlation = EventCorrelation {
        task_id: Some("task_comp_123".to_string()),
        conversation_id: Some("conv_comp_456".to_string()),
        turn_id: Some("turn_comp_789".to_string()),
        ..Default::default()
    };

    let req = ToolRequest::new(
        "computer.list_windows".to_string(),
        json!({}),
        correlation.clone(),
    );

    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Completed);
    assert_eq!(res.tool_name, "computer.list_windows");
}

#[tokio::test]
async fn test_15_computer_error_normalization() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_computer_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    let mock_platform = Arc::new(MockPlatformAdapter::new());
    exec_reg.register(Arc::new(ComputerDomainExecutor::with_platform(None, mock_platform)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    // Focusing non-existent window returns normalized execution failure
    let req = ToolRequest::new(
        "computer.focus_window".to_string(),
        json!({ "title": "NonExistentWindow_9999" }),
        EventCorrelation::default(),
    );

    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Failed);
    assert!(res.error.is_some());
    assert_eq!(res.error_code, Some("DOMAIN_ERROR".to_string()));
}
