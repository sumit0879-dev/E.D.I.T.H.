use super::browser::{get_browser_definitions, BrowserDomainExecutor};
use crate::browser_agent::to_universal_tool_name;
use crate::events::envelope::EventCorrelation;
use crate::events::{EventEmitter, TaskId};
use crate::policy::context::PolicyContext;
use crate::policy::{OperatorDecision, PolicyEngine};
use crate::policy::types::{ActionRequest, ActionTarget, PolicyOutcome, RiskLevel};
use crate::task::runtime::TaskRuntime;
use crate::task::types::{TaskOwner, TaskType};
use crate::tools::executor::DomainExecutorRegistry;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use crate::tools::types::{ToolDomain, ToolRequest, ToolStatus};
use crate::tools::validator::ArgumentValidator;
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_browser_tool_registration() {
    let registry = ToolRegistry::new();
    let defs = get_browser_definitions();
    assert_eq!(defs.len(), 24, "Expected 24 registered browser tools");

    for def in &defs {
        assert_eq!(def.domain, ToolDomain::Browser);
        assert!(def.name.starts_with("browser."));
        assert!(registry.register(def.clone()).is_ok());
    }

    assert_eq!(registry.count(), 24);

    // Verify key operations exist
    assert!(registry.get("browser.observe").is_some());
    assert!(registry.get("browser.screenshot").is_some());
    assert!(registry.get("browser.get_tabs").is_some());
    assert!(registry.get("browser.navigate").is_some());
    assert!(registry.get("browser.click").is_some());
    assert!(registry.get("browser.type").is_some());
    assert!(registry.get("browser.scroll").is_some());
    assert!(registry.get("browser.press_key").is_some());
    assert!(registry.get("browser.focus").is_some());
    assert!(registry.get("browser.wait").is_some());
    assert!(registry.get("browser.select_option").is_some());
    assert!(registry.get("browser.history_recent").is_some());
    assert!(registry.get("browser.history_search").is_some());
    assert!(registry.get("browser.bookmarks_list").is_some());
    assert!(registry.get("browser.bookmarks_search").is_some());
    assert!(registry.get("browser.downloads_recent").is_some());
    assert!(registry.get("browser.download_get").is_some());
}

#[test]
fn test_browser_tool_schema_validation() {
    let registry = ToolRegistry::new();
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }

    let nav_def = registry.get("browser.navigate").unwrap();

    // Valid arguments
    let valid_args = json!({
        "tab_id": "tab_1",
        "url": "https://example.com"
    });
    assert!(ArgumentValidator::validate(&valid_args, &nav_def.parameters_schema).is_ok());

    // Missing required url
    let missing_url = json!({
        "tab_id": "tab_1"
    });
    assert!(ArgumentValidator::validate(&missing_url, &nav_def.parameters_schema).is_err());

    // Missing required tab_id
    let missing_tab = json!({
        "url": "https://example.com"
    });
    assert!(ArgumentValidator::validate(&missing_tab, &nav_def.parameters_schema).is_err());

    // Click validation
    let click_def = registry.get("browser.click").unwrap();
    let valid_click = json!({
        "tab_id": "tab_1",
        "element_id": "btn_submit"
    });
    assert!(ArgumentValidator::validate(&valid_click, &click_def.parameters_schema).is_ok());

    let invalid_click = json!({
        "tab_id": "tab_1"
    });
    assert!(ArgumentValidator::validate(&invalid_click, &click_def.parameters_schema).is_err());
}

#[tokio::test]
async fn test_browser_navigation_policy_allow() {
    let engine = PolicyEngine::new(None);
    let req = ActionRequest::new(
        "browser",
        "navigate",
        ActionTarget::Url("https://example.com".to_string()),
        json!({
            "tab_id": "tab_1",
            "url": "https://example.com"
        }),
        EventCorrelation::default(),
    );
    let ctx = PolicyContext::default();
    let decision = engine.evaluate(&req, &ctx).await;

    assert_eq!(decision.outcome, PolicyOutcome::Allow);
    assert_eq!(decision.risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_browser_dangerous_scheme_blocked() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    // javascript: scheme blocked
    let js_req = ActionRequest::new(
        "browser",
        "navigate",
        ActionTarget::Url("javascript:alert(1)".to_string()),
        json!({ "url": "javascript:alert(1)" }),
        EventCorrelation::default(),
    );
    let js_decision = engine.evaluate(&js_req, &ctx).await;
    assert_eq!(js_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(js_decision.risk_level, RiskLevel::Critical);

    // file: scheme blocked
    let file_req = ActionRequest::new(
        "browser",
        "navigate",
        ActionTarget::Url("file:///etc/passwd".to_string()),
        json!({ "url": "file:///etc/passwd" }),
        EventCorrelation::default(),
    );
    let file_decision = engine.evaluate(&file_req, &ctx).await;
    assert_eq!(file_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(file_decision.risk_level, RiskLevel::Critical);

    // data:text/html scheme blocked
    let data_req = ActionRequest::new(
        "browser",
        "navigate",
        ActionTarget::Url("data:text/html,<h1>XSS</h1>".to_string()),
        json!({ "url": "data:text/html,<h1>XSS</h1>" }),
        EventCorrelation::default(),
    );
    let data_decision = engine.evaluate(&data_req, &ctx).await;
    assert_eq!(data_decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(data_decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_browser_password_field_requires_confirmation() {
    let engine = PolicyEngine::new(None);
    let ctx = PolicyContext::default();

    let pwd_req = ActionRequest::new(
        "browser",
        "type",
        ActionTarget::BrowserElement {
            selector: Some("input#password_field".to_string()),
            element_id: None,
            text: None,
        },
        json!({
            "tab_id": "tab_1",
            "element_id": "password_input",
            "text": "Secret123"
        }),
        EventCorrelation::default(),
    );
    let decision = engine.evaluate(&pwd_req, &ctx).await;
    assert_eq!(decision.outcome, PolicyOutcome::ConfirmationRequired);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_browser_confirmation_resumption() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine.clone(), None);

    // Initial request for password field requires confirmation
    let mut req = ToolRequest::new(
        "browser.type".to_string(),
        json!({
            "tab_id": "tab_1",
            "element_id": "pwd_field",
            "text": "Secret123!"
        }),
        EventCorrelation::default(),
    );

    let res = router.execute(req.clone()).await;
    assert_eq!(res.status, ToolStatus::ApprovalRequired);
    let approval_id = res.approval_id.expect("Expected approval_id to be populated");

    // Approve the request
    assert!(engine.resolve_approval(&approval_id, OperatorDecision::Approve).await.is_ok());

    // Replay request with approval token
    req.active_approval_id = Some(approval_id.clone());
    let second_res = router.execute(req.clone()).await;
    assert_eq!(second_res.status, ToolStatus::Completed);

    // Replaying again with consumed token must fail
    let replay_res = router.execute(req).await;
    assert_eq!(replay_res.status, ToolStatus::Blocked);
}

#[tokio::test]
async fn test_browser_replay_consumed_rejected() {
    let engine = Arc::new(PolicyEngine::new(None));
    let approval_req = ActionRequest::new(
        "browser",
        "type",
        ActionTarget::BrowserElement {
            selector: Some("input#password".to_string()),
            element_id: None,
            text: None,
        },
        json!({ "tab_id": "tab_1", "element_id": "pwd", "text": "secret" }),
        EventCorrelation::default(),
    );
    let ctx = PolicyContext::default();
    let initial_decision = engine.evaluate(&approval_req, &ctx).await;
    assert_eq!(initial_decision.outcome, PolicyOutcome::ConfirmationRequired);
    let approval_id = initial_decision.approval_id.expect("Expected approval_id");

    assert!(engine.resolve_approval(&approval_id, OperatorDecision::Approve).await.is_ok());

    // First consume succeeds
    let mut authed_ctx = ctx.clone();
    authed_ctx.active_approval_id = Some(approval_id.clone());
    let first_consume = engine.evaluate(&approval_req, &authed_ctx).await;
    assert_eq!(first_consume.outcome, PolicyOutcome::Allow);

    // Second consume fails closed
    let second_consume = engine.evaluate(&approval_req, &authed_ctx).await;
    assert_eq!(second_consume.outcome, PolicyOutcome::Blocked);
}

#[tokio::test]
async fn test_browser_click_execution() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    let req = ToolRequest::new(
        "browser.click".to_string(),
        json!({
            "tab_id": "tab_1",
            "element_id": "regular_link"
        }),
        EventCorrelation::default(),
    );

    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Completed);
    assert!(res.data.is_some());
}

#[tokio::test]
async fn test_browser_observation_structured_output() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    let req = ToolRequest::new(
        "browser.observe".to_string(),
        json!({
            "tab_id": "tab_1",
            "scope": "full_page"
        }),
        EventCorrelation::default(),
    );

    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Completed);
    let output = res.data.unwrap();
    assert_eq!(output["mock"], true);
    assert_eq!(output["tool"], "browser.observe");
}

#[tokio::test]
async fn test_browser_scoped_cancellation() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = Arc::new(ToolRouter::with_defaults(registry, exec_reg, engine, None));

    let req = ToolRequest::new(
        "browser.navigate".to_string(),
        json!({
            "tab_id": "tab_1",
            "url": "https://example.com"
        }),
        EventCorrelation::default(),
    );

    let exec_id = req.execution_id.to_string();
    let r_clone = router.clone();

    // Cancel prior to or during dispatch
    let cancel_handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        r_clone.cancel_execution(&exec_id).await
    });

    let res = router.execute(req).await;
    let _ = cancel_handle.await;

    // Execution should complete either as Completed or Cancelled depending on race
    assert!(res.status == ToolStatus::Completed || res.status == ToolStatus::Cancelled);
}

#[tokio::test]
async fn test_browser_execution_timeout() {
    let engine = Arc::new(PolicyEngine::new(None));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, None);

    let mut req = ToolRequest::new(
        "browser.wait".to_string(),
        json!({
            "tab_id": "tab_1",
            "condition": "page_load"
        }),
        EventCorrelation::default(),
    );
    // Set 0 timeout
    req.timeout_ms = Some(0);

    let res = router.execute(req).await;
    // With 0ms timeout, should immediately timeout or complete mock
    assert!(res.status == ToolStatus::Failed || res.status == ToolStatus::Completed);
}

#[test]
fn test_browser_agent_routes_through_tool_router() {
    assert_eq!(to_universal_tool_name("browser_open_url"), "browser.navigate");
    assert_eq!(to_universal_tool_name("browser_observe"), "browser.observe");
    assert_eq!(to_universal_tool_name("browser_click"), "browser.click");
    assert_eq!(to_universal_tool_name("browser_type"), "browser.type");
    assert_eq!(to_universal_tool_name("browser_scroll"), "browser.scroll");
    assert_eq!(to_universal_tool_name("browser_press_key"), "browser.press_key");
    assert_eq!(to_universal_tool_name("browser_focus"), "browser.focus");
    assert_eq!(to_universal_tool_name("browser_wait"), "browser.wait");
    assert_eq!(to_universal_tool_name("browser_select_option"), "browser.select_option");
    assert_eq!(to_universal_tool_name("browser_get_tabs"), "browser.get_tabs");
    assert_eq!(to_universal_tool_name("browser_get_active_tab"), "browser.get_active_tab");
    assert_eq!(to_universal_tool_name("browser_new_tab"), "browser.new_tab");
    assert_eq!(to_universal_tool_name("browser.custom_action"), "browser.custom_action");
}

#[tokio::test]
async fn test_browser_agent_task_runtime_integration() {
    let task_runtime = TaskRuntime::mock();
    let task_id = TaskId::from_string("task_test_browser_123");
    let correlation = EventCorrelation {
        task_id: Some(task_id.to_string()),
        ..Default::default()
    };

    let created_id = task_runtime
        .create_task_with_id(
            task_id.clone(),
            TaskType::BrowserAgent,
            "Research quantum computing".to_string(),
            correlation,
            TaskOwner::System,
        )
        .await
        .expect("Task creation failed");

    assert_eq!(created_id, task_id);

    // Start task
    assert!(task_runtime.start_task(&task_id).await.is_ok());

    // Update progress
    assert!(task_runtime
        .update_progress(&task_id, 1, 10, "Navigating to quantum paper")
        .await
        .is_ok());

    // Complete task
    assert!(task_runtime
        .complete_task(&task_id, "Found 3 relevant quantum computing papers")
        .await
        .is_ok());
}

#[tokio::test]
async fn test_browser_correlated_events_lifecycle() {
    let emitter = EventEmitter::mock();
    let engine = Arc::new(PolicyEngine::new(Some(emitter.clone())));
    let registry = Arc::new(ToolRegistry::new());
    for def in get_browser_definitions() {
        let _ = registry.register(def);
    }
    let exec_reg = Arc::new(DomainExecutorRegistry::new());
    exec_reg.register(Arc::new(BrowserDomainExecutor::new(None)));

    let router = ToolRouter::with_defaults(registry, exec_reg, engine, Some(emitter));

    let correlation = EventCorrelation {
        task_id: Some("task_789".to_string()),
        conversation_id: Some("conv_123".to_string()),
        turn_id: Some("turn_456".to_string()),
        ..Default::default()
    };

    let req = ToolRequest::new(
        "browser.get_tabs".to_string(),
        json!({}),
        correlation.clone(),
    );

    let res = router.execute(req).await;
    assert_eq!(res.status, ToolStatus::Completed);
}

#[test]
fn test_browser_direct_ui_regression_safety() {
    // Ensure get_browser_definitions and legacy catalog definitions don't collide
    let definitions = get_browser_definitions();
    let legacy_defs = crate::browser_tools::get_browser_tool_definitions();

    assert!(!definitions.is_empty());
    assert!(!legacy_defs.is_empty());

    // Verify all universal tools map to valid legacy names
    for def in &definitions {
        let mapped = BrowserDomainExecutor::map_tool_name(&def.name);
        assert!(!mapped.is_empty());
        assert!(!mapped.starts_with("browser."));
    }
}
