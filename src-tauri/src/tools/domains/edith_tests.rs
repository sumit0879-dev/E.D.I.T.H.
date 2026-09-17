//! edith_tests.rs — Comprehensive unit test suite for Phase 8 E.D.I.T.H. Runtime State,
//! Self-Knowledge & Self-Control Domain.
//!
//! Validates:
//! 1. Catalog registration and parameter schema enforcement (11 active tools)
//! 2. Bounded, sanitized Self-Knowledge queries (status, capabilities, tasks, providers, browser, computer, security, health)
//! 3. Credential scrubbing (zero secret leakage)
//! 4. Bounded, policy-governed Self-Control (cancel_task, cancel_tool_execution)
//! 5. Ownership & authorization scope enforcement (Guardrail 1)
//! 6. Formal deferral of pause_task/resume_task (Guardrail 2: Fail Closed)
//! 7. Anti-escalation guarantees (Privilege modifications fail closed as Critical)
//! 8. AutonomyState dynamic state derivation hierarchy
//! 9. Concurrency & lock-free read safety under multi-threaded loads

use crate::ai::ProviderRegistry;
use crate::computer_control::GLOBAL_COMPUTER_CONTROL_MGR;
use crate::conversation::ConversationCore;
use crate::events::{EventCorrelation, EventEmitter};
use crate::policy::context::{PolicyContext, SecurityMode};
use crate::policy::engine::PolicyEngine;
use crate::policy::types::{ActionRequest, ActionTarget, PolicyOutcome, RiskLevel};
use crate::runtime::projections::scrub_url;
use crate::runtime::{AutonomyState, EdithRuntimeState};
use crate::task::types::{TaskOwner, TaskType};
use crate::task::TaskRuntime;
use crate::tools::cancellation::CancellationRegistry;
use crate::tools::domains::edith::{get_edith_definitions, EdithDomainExecutor};
use crate::tools::executor::DomainExecutorRegistry;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use crate::tools::types::ToolRequest;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test fixture setting up an isolated, headless E.D.I.T.H. runtime environment.
async fn setup_edith_fixture() -> (
    Arc<EdithRuntimeState>,
    Arc<ToolRouter>,
    Arc<TaskRuntime>,
    Arc<PolicyEngine>,
    Arc<ConversationCore>,
) {
    let emitter = EventEmitter::mock();
    let task_runtime = Arc::new(TaskRuntime::new(emitter.clone()));
    let provider_registry = Arc::new(RwLock::new(ProviderRegistry::standard_builtins()));
    let conversation_core = Arc::new(ConversationCore::new(
        ProviderRegistry::standard_builtins(),
        emitter.clone(),
        None,
        None,
    ));
    let policy_engine = Arc::new(PolicyEngine::new(Some(emitter.clone())));

    let tool_registry = Arc::new(ToolRegistry::new());
    for def in get_edith_definitions() {
        tool_registry.register(def).unwrap();
    }

    let domain_executors = Arc::new(DomainExecutorRegistry::new());
    let cancellation = Arc::new(CancellationRegistry::new());

    let tool_router = Arc::new(ToolRouter::new(
        tool_registry.clone(),
        domain_executors.clone(),
        policy_engine.clone(),
        Some(emitter.clone()),
        cancellation.clone(),
    ));

    let runtime_state = Arc::new(EdithRuntimeState::mock(
        conversation_core.clone(),
        task_runtime.clone(),
        tool_registry.clone(),
        tool_router.clone(),
        provider_registry.clone(),
        policy_engine.clone(),
    ));

    let edith_executor = Arc::new(EdithDomainExecutor::new(runtime_state.clone()));
    domain_executors.register(edith_executor);

    (
        runtime_state,
        tool_router,
        task_runtime,
        policy_engine,
        conversation_core,
    )
}

#[tokio::test]
async fn test_01_edith_tool_registration() {
    let defs = get_edith_definitions();
    assert_eq!(defs.len(), 11, "Must declare exactly 11 active tools");

    let expected_names = [
        "edith.get_runtime_status",
        "edith.get_capabilities",
        "edith.list_active_tasks",
        "edith.get_task_details",
        "edith.list_providers",
        "edith.get_browser_status",
        "edith.get_computer_status",
        "edith.get_security_status",
        "edith.get_system_health",
        "edith.cancel_task",
        "edith.cancel_tool_execution",
    ];

    for name in &expected_names {
        assert!(
            defs.iter().any(|d| &d.name == name),
            "Missing definition for tool '{}'",
            name
        );
    }
}

#[tokio::test]
async fn test_02_edith_tool_schema_validation() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    // Missing required argument `task_id` for edith.cancel_task
    let req = ToolRequest::simple("edith.cancel_task", json!({ "reason": "Just because" }));

    let result = router.execute(req).await;
    assert!(!result.is_success());
    assert_eq!(result.error_code, Some("INVALID_ARGUMENTS".to_string()));
}

#[tokio::test]
async fn test_03_self_knowledge_runtime_status() {
    let (runtime_state, router, _, _, _) = setup_edith_fixture().await;

    // Direct read-model query reflects IDLE when no tool or turn is running
    let direct_status = runtime_state.get_runtime_status(None).await;
    assert_eq!(direct_status.autonomy_state, AutonomyState::Idle);
    assert_eq!(direct_status.active_task_count, 0);
    assert_eq!(direct_status.active_execution_count, 0);

    // When dispatched via tool router, it accurately reflects EXECUTING_TOOL for the in-flight tool
    let req = ToolRequest::simple("edith.get_runtime_status", json!({}));

    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("Must return payload");
    assert_eq!(data["autonomy_state"], "EXECUTING_TOOL");
    assert_eq!(data["active_task_count"], 0);
    assert_eq!(data["active_execution_count"], 1);
    assert_eq!(data["pending_approval_count"], 0);
    assert!(data["uptime_seconds"].as_u64().is_some());
}

#[tokio::test]
async fn test_04_self_knowledge_capabilities() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    let req = ToolRequest::simple("edith.get_capabilities", json!({}));

    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("Must return payload");
    assert!(data["total_tools"].as_u64().unwrap() >= 11);
    let domains = data["domains"].as_array().expect("domains list");
    assert!(domains.iter().any(|d| d["domain"] == "edith"));
}

#[tokio::test]
async fn test_05_self_knowledge_active_tasks_and_details() {
    let (_, router, task_runtime, _, _) = setup_edith_fixture().await;

    // Create and start a test task
    let task_id = task_runtime
        .create_task(
            TaskType::BrowserAgent,
            "Automated research task",
            EventCorrelation::default(),
            TaskOwner::System,
        )
        .await;
    task_runtime.start_task(&task_id).await.unwrap();
    task_runtime
        .update_progress(&task_id, 1, 5, "Navigating to portal")
        .await
        .unwrap();

    // Query active tasks
    let req = ToolRequest::simple("edith.list_active_tasks", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("tasks payload");
    assert_eq!(data["total_active"], 1);

    // Query task details
    let req_details = ToolRequest::simple("edith.get_task_details", json!({ "task_id": task_id.as_str() }));
    let res_details = router.execute(req_details).await;
    assert!(res_details.is_success());
    let details_data = res_details.data.expect("details payload");
    assert_eq!(details_data["task_id"], task_id.as_str());
    assert_eq!(details_data["progress"]["step"], 1);
}

#[tokio::test]
async fn test_06_self_knowledge_provider_redaction() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    let req = ToolRequest::simple("edith.list_providers", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("providers payload");
    let arr = data.as_array().expect("array of providers");
    assert!(!arr.is_empty());

    // Strict Data Contract: Verify no secrets or credentials leaked
    let json_str = serde_json::to_string(&data).unwrap();
    assert!(!json_str.contains("api_key"));
    assert!(!json_str.contains("secret"));
    assert!(!json_str.contains("bearer"));
    assert!(!json_str.contains("authorization"));
}

#[tokio::test]
async fn test_07_self_knowledge_browser_status_sanitization() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    // Direct unit test of scrub_url function
    let dirty_url = "https://service.corp/callback?token=secret123&auth=bearer456&user=alice";
    let cleaned_url = scrub_url(dirty_url);
    assert!(!cleaned_url.contains("secret123"));
    assert!(!cleaned_url.contains("bearer456"));
    assert!(cleaned_url.contains("token=%5BREDACTED%5D") || cleaned_url.contains("token=[REDACTED]"));
    assert!(cleaned_url.contains("user=alice"));

    // Tool execution test (headless environment defaults to 0 open tabs safely)
    let req = ToolRequest::simple("edith.get_browser_status", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("browser status data");
    assert_eq!(data["open_tabs_count"], 0);
}

#[tokio::test]
async fn test_08_self_knowledge_computer_status() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    let req = ToolRequest::simple("edith.get_computer_status", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("computer status data");
    assert_eq!(data["control_state"], "USER_CONTROLLED");
    assert_eq!(data["is_ai_controlled"], false);
}

#[tokio::test]
async fn test_09_self_knowledge_security_status() {
    let (_, router, _, policy_engine, _) = setup_edith_fixture().await;

    // Create a mock pending approval
    let action_req = ActionRequest::new(
        "computer",
        "launch_app",
        ActionTarget::SystemTarget("powershell.exe".to_string()),
        json!({ "app_name": "powershell.exe" }),
        EventCorrelation::default(),
    );
    policy_engine
        .approvals()
        .create_request_direct(
            &action_req,
            RiskLevel::High,
            "Restricted app execution".to_string(),
            Some("sess-1".to_string()),
            Some("turn-1".to_string()),
            None,
            policy_engine.get_policy_version(),
            300,
        )
        .await;

    let req = ToolRequest::simple("edith.get_security_status", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("security status data");
    assert_eq!(data["pending_approval_count"], 1);
    let approvals = data["pending_approvals"].as_array().expect("approvals list");
    assert_eq!(approvals[0]["operation"], "launch_app");

    // Verify cryptographic hashes and raw arguments are not dumped
    let json_str = serde_json::to_string(&data).unwrap();
    assert!(!json_str.contains("request_hash"));
}

#[tokio::test]
async fn test_10_self_knowledge_system_health() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    let req = ToolRequest::simple("edith.get_system_health", json!({}));
    let result = router.execute(req).await;
    assert!(result.is_success());
    let data = result.data.expect("health data");
    assert_eq!(data["overall_healthy"], true);
    assert!(data["subsystems"]["task_runtime"]["healthy"].as_bool().unwrap());
    assert!(data["subsystems"]["tool_runtime"]["healthy"].as_bool().unwrap());
    assert!(data["subsystems"]["policy_engine"]["healthy"].as_bool().unwrap());
}

#[tokio::test]
async fn test_11_self_control_cancel_task_ownership_authorized() {
    let (_, router, task_runtime, _, _) = setup_edith_fixture().await;

    let mut corr = EventCorrelation::default();
    corr.conversation_id = Some("active-session-123".to_string());
    corr.turn_id = Some("turn-456".to_string());

    let task_id = task_runtime
        .create_task(
            TaskType::BrowserAgent,
            "Task initiated in turn",
            corr.clone(),
            TaskOwner::Turn("turn-456".to_string()),
        )
        .await;
    task_runtime.start_task(&task_id).await.unwrap();

    // Call edith.cancel_task from within the authorized turn scope
    let req = ToolRequest::new(
        "edith.cancel_task",
        json!({
            "task_id": task_id.as_str(),
            "reason": "Turn objective changed"
        }),
        corr,
    );

    let result = router.execute(req).await;
    assert!(result.is_success(), "Cancellation must succeed: {:?}", result.error);

    // Verify task is cancelled in TaskRuntime
    let task_snapshot = task_runtime.get_task(&task_id).await.unwrap();
    assert_eq!(task_snapshot.status.to_string(), "cancelled");
}

#[tokio::test]
async fn test_12_self_control_cancel_task_foreign_scope_confirmation() {
    let (_, router, task_runtime, _, _) = setup_edith_fixture().await;

    // Create a task belonging to a different session and owned by System
    let mut foreign_corr = EventCorrelation::default();
    foreign_corr.conversation_id = Some("session-alpha".to_string());

    let task_id = task_runtime
        .create_task(
            TaskType::Maintenance,
            "System database vacuum",
            foreign_corr,
            TaskOwner::System,
        )
        .await;

    // Attempt to cancel from a totally unrelated session and turn
    let mut caller_corr = EventCorrelation::default();
    caller_corr.conversation_id = Some("session-beta".to_string());
    caller_corr.turn_id = Some("turn-random".to_string());

    let req = ToolRequest::new(
        "edith.cancel_task",
        json!({
            "task_id": task_id.as_str(),
            "reason": "Trying to kill foreign system task"
        }),
        caller_corr,
    );

    let result = router.execute(req).await;
    assert!(!result.is_success());
    assert!(result.error.unwrap().contains("Authorization Violation"));
}

#[tokio::test]
async fn test_13_self_control_cancel_tool_execution() {
    let (_, router, _, _, _) = setup_edith_fixture().await;

    // Register an active in-flight tool execution token
    let exec_token = router
        .cancellation()
        .register_execution("in-flight-exec-999", None, None, None)
        .await;
    assert!(!exec_token.is_cancelled());

    // Cancel via edith.cancel_tool_execution
    let req = ToolRequest::simple(
        "edith.cancel_tool_execution",
        json!({
            "execution_id": "in-flight-exec-999",
            "reason": "Operator requested stop"
        }),
    );

    let result = router.execute(req).await;
    assert!(result.is_success());
    assert!(exec_token.is_cancelled(), "Scoped token must be cancelled");
}

#[tokio::test]
async fn test_14_self_control_deferred_pause_resume_fail_closed() {
    let (_, _, _, policy_engine, _) = setup_edith_fixture().await;

    // Guardrail 2: Verify that pause_task and resume_task fail closed with TASK_PAUSE_NOT_SUPPORTED
    let req_pause = ActionRequest::new(
        "edith",
        "pause_task",
        ActionTarget::SystemTarget("task:123".to_string()),
        json!({ "task_id": "123" }),
        EventCorrelation::default(),
    );
    let decision = policy_engine
        .evaluate(&req_pause, &PolicyContext::default())
        .await;

    assert_eq!(decision.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision.policy_code, "TASK_PAUSE_NOT_SUPPORTED");

    let req_resume = ActionRequest::new(
        "edith",
        "resume_task",
        ActionTarget::SystemTarget("task:123".to_string()),
        json!({ "task_id": "123" }),
        EventCorrelation::default(),
    );
    let decision_resume = policy_engine
        .evaluate(&req_resume, &PolicyContext::default())
        .await;

    assert_eq!(decision_resume.outcome, PolicyOutcome::Blocked);
    assert_eq!(decision_resume.policy_code, "TASK_PAUSE_NOT_SUPPORTED");
}

#[tokio::test]
async fn test_15_self_control_policy_strict_mode_confirmation() {
    let (_, router, task_runtime, policy_engine, _) = setup_edith_fixture().await;

    // Switch policy to Strict mode
    policy_engine.set_security_mode(SecurityMode::Strict).await;

    let task_id = task_runtime
        .create_task(
            TaskType::BrowserAgent,
            "Research task",
            EventCorrelation::default(),
            TaskOwner::System,
        )
        .await;

    let req = ToolRequest::simple(
        "edith.cancel_task",
        json!({ "task_id": task_id.as_str() }),
    );

    let result = router.execute(req).await;
    assert!(!result.is_success());
    assert_eq!(result.error_code, Some("CONFIRMATION_REQUIRED".to_string()));
}

#[tokio::test]
async fn test_16_anti_escalation_blocked() {
    let (_, _, _, policy_engine, _) = setup_edith_fixture().await;

    // Any attempt to modify security policies or escalate privileges fails closed
    let forbidden_ops = [
        "modify_policy",
        "grant_permission",
        "set_security_mode",
        "reveal_secrets",
    ];

    for op in forbidden_ops {
        let action_req = ActionRequest::new(
            "edith",
            op,
            ActionTarget::None,
            json!({}),
            EventCorrelation::default(),
        );

        let decision = policy_engine
            .evaluate(&action_req, &PolicyContext::default())
            .await;
        assert_eq!(decision.outcome, PolicyOutcome::Blocked);
        assert_eq!(decision.risk_level, RiskLevel::Critical);
        assert_eq!(decision.policy_code, "UNAUTHORIZED_SELF_ESCALATION");
    }
}

#[tokio::test]
async fn test_17_autonomy_state_transitions() {
    let (runtime_state, _, task_runtime, policy_engine, _) = setup_edith_fixture().await;

    // 1. Initial State: Idle
    assert_eq!(runtime_state.get_autonomy_state().await, AutonomyState::Idle);

    // 2. Active Task: RunningTask
    let task_id = task_runtime
        .create_task(
            TaskType::BrowserAgent,
            "Autonomous browser crawl",
            EventCorrelation::default(),
            TaskOwner::System,
        )
        .await;
    task_runtime.start_task(&task_id).await.unwrap();
    assert_eq!(
        runtime_state.get_autonomy_state().await,
        AutonomyState::RunningTask
    );

    // 3. Pending Approval: WaitingForApproval takes higher precedence
    let action_req = ActionRequest::new(
        "computer",
        "launch_app",
        ActionTarget::SystemTarget("powershell.exe".to_string()),
        json!({ "app_name": "powershell.exe" }),
        EventCorrelation::default(),
    );
    let approval = policy_engine
        .approvals()
        .create_request_direct(
            &action_req,
            RiskLevel::High,
            "Approval required".to_string(),
            None,
            None,
            None,
            policy_engine.get_policy_version(),
            300,
        )
        .await;
    assert_eq!(
        runtime_state.get_autonomy_state().await,
        AutonomyState::WaitingForApproval
    );

    // Resolve approval
    policy_engine
        .approvals()
        .resolve(&approval.approval_id, crate::policy::approval::OperatorDecision::Approve, None)
        .await
        .unwrap();

    // 4. Human Takeover Preemption: UserTakeover takes highest precedence
    GLOBAL_COMPUTER_CONTROL_MGR.pause_ai_control(Some("Physical human mouse movement detected".to_string())).unwrap();
    assert_eq!(
        runtime_state.get_autonomy_state().await,
        AutonomyState::UserTakeover
    );

    // Resume control
    GLOBAL_COMPUTER_CONTROL_MGR.release_ai_control().unwrap();
}

#[tokio::test]
async fn test_18_concurrent_runtime_reads() {
    let (runtime_state, _, task_runtime, _, _) = setup_edith_fixture().await;

    // Spawn concurrent reader tasks alongside concurrent task creator tasks
    let state = runtime_state.clone();
    let tasks = task_runtime.clone();

    let mut handles = Vec::new();

    // Task creator worker
    let t_clone = tasks.clone();
    handles.push(tokio::spawn(async move {
        for i in 0..20 {
            let tid = t_clone
                .create_task(
                    TaskType::Background,
                    format!("Worker task {}", i),
                    EventCorrelation::default(),
                    TaskOwner::System,
                )
                .await;
            let _ = t_clone.start_task(&tid).await;
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            let _ = t_clone.complete_task(&tid, "Finished").await;
        }
    }));

    // Concurrent status readers
    for _ in 0..5 {
        let s_clone = state.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..25 {
                let status = s_clone.get_runtime_status(None).await;
                assert!(status.uptime_seconds <= 100);
                let _ = s_clone.get_capabilities(None).await;
                let _ = s_clone.get_system_health().await;
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }));
    }

    // Await all without deadlock
    for h in handles {
        h.await.expect("Concurrent task failed");
    }
}
