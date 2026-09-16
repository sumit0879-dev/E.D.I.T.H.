# E.D.I.T.H. Universal Tool Runtime (UTR) Specification
**Document Version:** 1.0.0  
**Phase:** 5 (Universal Tool Runtime)  
**Status:** Approved Architecture Implementation  
**Baseline:** Phases 1–4 Merged (`ai`, `events`, `conversation`, `task`, `policy`)  
**Target Branch:** `feature/phase-5-tool-runtime`

---

## 1. Executive Summary & Purpose

The **Universal Tool Runtime (UTR)** is the single, authoritative execution gateway for all tool invocations within E.D.I.T.H. Prior to Phase 5, tool logic (such as browser automation in `browser_tools.rs`) was loosely coupled to commands with fragmented argument handling and without strict host-enforced pre-execution policy gating. 

Phase 5 establishes a centralized, domain-agnostic, and secure execution runtime. UTR decouples tool definitions and schemas from low-level subsystem drivers, unifies argument validation against JSON Schema, strictly integrates with the Phase 4 `PolicyEngine` before dispatching any domain operation, guarantees scoped cancellation, and emits end-to-end correlated audit events into the Phase 2 Event Infrastructure.

```
AI Model / Conversation Core / Task Runtime
                 │
                 ▼
          [ ToolRequest ]
                 │
                 ▼
          ┌──────────────┐
          │ ToolRegistry │ (lookup definition, schema, risk level)
          └──────┬───────┘
                 │
                 ▼
      ┌──────────────────────┐
      │  ArgumentValidator   │ (JSON Schema validation, types, enums, bounds)
      └──────────┬───────────┘
                 │
                 ▼
      ┌──────────────────────┐
      │     PolicyEngine     │ (Phase 4 mandatory security boundary)
      └──────────┬───────────┘
                 │
     ┌───────────┴──────────────────────────────┐
     ▼                                          ▼
[ALLOW / RESTRICTED]                [CONFIRMATION_REQUIRED / BLOCKED]
     │                                          │
     ▼                                          ▼
┌──────────────────┐               ┌───────────────────────────────┐
│  ToolRouter      │               │ Return ApprovalRequired/Error │
│  (dispatch)      │               │ (Domain Executor NOT invoked) │
└────────┬─────────┘               └───────────────────────────────┘
         │
         ▼
┌──────────────────┐
│  DomainExecutor  │ (BrowserDomainExecutor: 15 stable tools)
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ ToolExecutionRes │ ──► Emits Correlated Events (Phase 2)
└──────────────────┘
```

---

## 2. Non-Negotiable Security Invariants

1. **Mandatory Pre-Execution Policy Boundary:**  
   Every tool execution request MUST traverse `PolicyEngine::evaluate` before any domain executor is invoked. No tool can bypass policy evaluation.
2. **AI Cannot Authorize Its Own Tools:**  
   AI models, prompts, agents, and conversational turns possess zero authority to approve or elevate permissions. Only host policies, human operator confirmations, or deterministic security modes can grant execution authorization.
3. **Single-Use Consumable Approvals:**  
   Operator approval tokens (`ApprovalId`) are strictly single-use. Once an approved tool executes, the token is cryptographically marked as `Consumed`. Replaying tokens or re-executing with previously used tokens is rejected immediately.
4. **Policy Version Invalidation:**  
   If the active security policy constraints or mode changes, all pending approvals created under prior versions are invalidated (`PolicyOutcome::Blocked` or `ApprovalStatus::Expired`).
5. **Fail-Closed Security Posture:**  
   Any validation failure, missing tool, policy denial, unknown argument, or executor crash results in an explicit, typed `ToolExecutionError`. Execution fails closed with zero side effects.
6. **No Self-Modification or Dynamic Execution of Arbitrary Code:**  
   UTR does not allow dynamic compilation, script evaluation, or shell command execution outside approved domain-specific handlers.

---

## 3. Architecture Hierarchy & Boundaries

UTR occupies the execution layer between higher-level orchestrators (`ConversationCore`, `TaskRuntime`) and low-level subsystem drivers:

```
┌─────────────────────────────────────────────────────────────┐
│ High-Level Orchestration (Phase 3)                          │
│   • ConversationCore (Turns, Chat Lifecycle, Context)       │
│   • TaskRuntime (Async Tasks, Background Jobs)              │
└──────────────────────────────┬──────────────────────────────┘
                               │ ToolRequest
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Universal Tool Runtime (Phase 5 - Core)                     │
│   • ToolRegistry (Authoritative schemas, metadata, domains) │
│   • ArgumentValidator (Strict JSON Schema validation)       │
│   • ToolRouter (Policy gating, timeout racing, lifecycle)   │
│   • CancellationRegistry (Scoped cancellation trees)        │
└──────────────────────────────┬──────────────────────────────┘
                               │
                ┌──────────────┴──────────────┐
                │ Policy Boundary (Phase 4)   │
                │   • PolicyEngine            │
                │   • ApprovalStore           │
                │   • SecurityAuditTrail      │
                └──────────────┬──────────────┘
                               │ ALLOW / RESTRICTED
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Domain Executors (Subsystem Drivers)                        │
│   • BrowserDomainExecutor (15 tools: observe, click, nav...)│
│   • SystemDomainExecutor (Future: Phase 6)                  │
│   • Custom / Plugin Executors (Future: Phase 6)             │
└──────────────────────────────┬──────────────────────────────┘
                               │ Result / Error
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Correlated Event Infrastructure (Phase 2)                   │
│   • ToolPayload (Started, Completed, Failed, Cancelled)     │
│   • EventEmitter (IPC bridge to Tauri frontend)             │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Universal Tool Runtime (UTR) Components

- **`ToolRegistry` (`src-tauri/src/tools/registry.rs`):**  
  Thread-safe concurrent registry storing `ToolDefinition`s mapped by canonical names. Supports listing, querying, domain filtering, and duplicate-prevention.
- **`ArgumentValidator` (`src-tauri/src/tools/validator.rs`):**  
  Validates inbound arguments against parameter schemas (type checking, required fields, bounds, enums, formats).
- **`CancellationRegistry` (`src-tauri/src/tools/cancellation.rs`):**  
  Maintains execution-level, turn-level, task-level, and session-level cancellation tokens backed by native `tokio::sync::watch` broadcast channels.
- **`DomainExecutor` Trait (`src-tauri/src/tools/executor.rs`):**  
  Asynchronous, object-safe trait contract (`BoxFuture`) implemented by each subsystem domain.
- **`ToolRouter` (`src-tauri/src/tools/router.rs`):**  
  The central coordinator. Resolves definitions, runs validation, executes policy checks, creates cancellation tokens, applies execution timeouts, invokes domain executors, and records audit events.
- **`BrowserDomainExecutor` (`src-tauri/src/tools/domains/browser.rs`):**  
  The initial domain executor adapting the 15 stable browser automation actions to UTR.

---

## 5. Authoritative Tool Registry & Schema System

Every tool registered in UTR must declare a comprehensive `ToolDefinition`:

```rust
pub struct ToolDefinition {
    pub name: String,
    pub domain: ToolDomain,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub is_read_only: boolean,
    pub requires_approval: boolean,
    pub default_timeout_ms: u64,
    pub risk_level: RiskLevel,
}
```

- **Canonical Naming:** Names are dot-notated by convention: `<domain>.<action>` (e.g., `browser.navigate`, `browser.click`).
- **Domain Categorization:** Tools belong to typed domains (`Browser`, `System`, `File`, `Git`, `Network`, `Custom(String)`).
- **Schema Contracts:** Parameters must adhere to standard JSON Schema specification. Dynamic or untyped tools are prohibited.

---

## 6. Declarative Argument Validation Engine

The `ArgumentValidator` guarantees that no malformed or hostile input reaches domain executors:

1. **Top-Level Type Check:** Input arguments must be a JSON Object.
2. **Required Fields:** All keys declared in the schema's `required` array must exist in the arguments and not be `null`.
3. **Property Type Matching:**
   - `"string"` $\to$ `Value::String` (rejects non-strings).
   - `"number"` $\to$ `Value::Number`.
   - `"integer"` $\to$ `Value::Number` where `is_i64()` or `is_u64()`.
   - `"boolean"` $\to$ `Value::Bool`.
   - `"array"` $\to$ `Value::Array`.
   - `"object"` $\to$ `Value::Object`.
4. **Enum Constraint Verification:** If schema specifies `"enum": [...]`, input value must match one of the permitted enum values.
5. **Length and Value Bounds:** Validates `minLength` and numeric ranges if specified.

---

## 7. Hierarchical Execution Routing

Execution dispatch follows a deterministic state transition pipeline:

```
[ToolRequest]
     │
     ├─► 1. Lookup in ToolRegistry ──► (Fail: ToolExecutionError::NotFound)
     │
     ├─► 2. Argument Validation ──────► (Fail: ToolExecutionError::ValidationFailed)
     │
     ├─► 3. PolicyEngine Evaluate ──► (Denied: ToolExecutionError::AccessDenied)
     │                              └─► (ApprovalReq: ToolStatus::ApprovalRequired)
     │
     ├─► 4. Resolve Domain Executor ─► (Fail: ToolExecutionError::DomainUnavailable)
     │
     ├─► 5. Register Cancellation Token
     │
     ├─► 6. Race Domain Execution vs Timeout vs Cancellation
     │        │
     │        ├─► Success ──► ToolStatus::Success
     │        ├─► Timeout ──► ToolExecutionError::Timeout
     │        ├─► Cancel ───► ToolExecutionError::Cancelled
     │        └─► Err ──────► ToolExecutionError::ExecutionFailed
     │
     └─► 7. Emit Correlated Event & Cleanup Cancellation Token
```

---

## 8. Domain Executor Abstraction & Isolation

Each execution domain is strictly isolated behind the `DomainExecutor` interface:

```rust
pub trait DomainExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        cancellation_token: ScopedCancellationToken,
    ) -> BoxFuture<'a, Result<serde_json::Value, ToolExecutionError>>;
}
```

- **Zero Direct Host Access:** Higher layers cannot directly call subsystem APIs; everything passes through `ToolRouter`.
- **Async Object-Safety:** Uses native Rust 2021 `Pin<Box<dyn Future + Send + 'a>>` without external macro dependencies, matching project standards.
- **Cancellation-Aware:** Every domain executor receives a `ScopedCancellationToken` and must honor cancellation checkpoints during long-running I/O.

---

## 9. Browser Domain Migration (15 Tools)

All 15 existing browser automation actions from `browser_tools.rs` are migrated into `BrowserDomainExecutor`:

| Tool Name | Operation | Risk Level | Approval Required | Read-Only |
| :--- | :--- | :--- | :--- | :--- |
| `browser.observe` | DOM accessibility tree inspection | `Safe` | No | Yes |
| `browser.screenshot` | Viewport / tab visual capture | `Safe` | No | Yes |
| `browser.navigate` | URL navigation | `Low` | No | No |
| `browser.click` | DOM element click | `Medium` | Conditional | No |
| `browser.type` | Text insertion into input | `Medium` | Conditional | No |
| `browser.scroll` | Viewport scrolling | `Safe` | No | No |
| `browser.press_key` | Keyboard keypress | `Low` | No | No |
| `browser.get_tabs` | Query active browser tabs | `Safe` | No | Yes |
| `browser.get_active_tab`| Query selected tab details | `Safe` | No | Yes |
| `browser.new_tab` | Create browser tab | `Low` | No | No |
| `browser.close_tab` | Close browser tab | `Low` | No | No |
| `browser.switch_tab` | Activate tab index | `Safe` | No | No |
| `browser.back` | Navigate back | `Safe` | No | No |
| `browser.forward` | Navigate forward | `Safe` | No | No |
| `browser.reload` | Reload tab | `Safe` | No | No |

---

## 10. Policy & Permission Boundary Integration

`ToolRouter` translates `ToolRequest` into an `ActionRequest` and `PolicyContext` for Phase 4 `PolicyEngine`:

- **Domain Mapping:** `ToolDomain::Browser` $\to$ `"browser"`.
- **Target Derivation:**
  - `browser.navigate` $\to$ `ActionTarget::Url(url)`
  - `browser.click`, `browser.type` $\to$ `ActionTarget::BrowserElement(selector)`
- **Security Mode Integration:** Inherits mode (`Autonomous`, `Standard`, `Restricted`) from conversational turn or task context.
- **Fail-Closed Gate:** If `PolicyEngine` returns `PolicyOutcome::Blocked`, the executor is never invoked and an audit record is logged.

---

## 11. Approval Pausing, Resumption & Non-Replayability

When an action requires human verification:
1. `PolicyEngine::evaluate` generates a cryptographic `ApprovalRequest` with unique `approval_id`, stored in `ApprovalStore`.
2. `ToolRouter` returns `ToolExecutionResult` with `status: ToolStatus::ApprovalRequired` and `approval_id`.
3. Client / Operator reviews the request and submits an approval decision (`Approve` or `Deny`).
4. Re-invocation provides `ToolRequest.active_approval_id`.
5. `PolicyEngine` validates hash matching, policy version, and marks approval as `Consumed`.
6. Subsequent replays with the same `approval_id` are rejected immediately.

---

## 12. Scoped Cancellation Engine

Cancellation is organized in a hierarchical cascade:
- **Execution Level:** `cancel_execution(execution_id)` terminates a specific running tool.
- **Turn Level:** Cancelling a turn automatically cancels all active child tool executions.
- **Task Level:** Cancelling a task cascades to all turns and running tools.
- **Session Level:** Closing a session cancels all associated active operations.

Tokens use non-blocking `tokio::sync::watch` channels to trigger immediate wake-ups in waiting async futures.

---

## 13. Execution Lifecycle & Status State Machine

```
   [ Queued ]
        │
        ▼
   [ Running ] ───────► [ ApprovalRequired ]
        │                       │
        │                       ├─► Approved ──► [ Running ]
        │                       └─► Denied ────► [ Failed ]
        │
        ├───────────────────────┬───────────────────────┐
        ▼                       ▼                       ▼
   [ Success ]             [ Failed ]              [ Cancelled ]
```

---

## 14. Error Taxonomy & Structured Normalization

UTR categorizes all errors into typed `ToolExecutionError` variants:
- `NotFound { tool_name }`: Tool is not registered.
- `AlreadyExists { tool_name }`: Tool registration collision.
- `ValidationFailed { reason }`: Argument does not match schema.
- `MalformedArguments { reason }`: Argument parsing error.
- `AccessDenied { reason, risk_level }`: Policy rejection.
- `ApprovalRequired { approval_id, reason }`: Execution paused awaiting approval.
- `Timeout { timeout_ms }`: Execution exceeded time budget.
- `Cancelled { reason }`: Operator or runtime cancelled execution.
- `DomainUnavailable { domain, reason }`: Subsystem driver not mounted.
- `ExecutionFailed { reason }`: Domain handler encountered error.
- `Internal { reason }`: Unexpected host or hardware failure.

---

## 15. Execution Isolation & Sandboxing Philosophy

UTR enforces strict boundaries around execution:
- Executions cannot mutate global runtime state directly.
- All disk, network, and system operations must be bounded by active `PolicyConstraints` (e.g. `sandbox_workspace_only`, `allow_network_access`).
- Any future plugin tools will run inside process-isolated boundaries.

---

## 16. Correlated Event Infrastructure Integration

Every tool lifecycle change emits correlated events into the Phase 2 Event Infrastructure:
- `ToolPayload::Started`: Emitted when dispatch begins.
- `ToolPayload::Progress`: Emitted during intermediate steps (optional).
- `ToolPayload::ApprovalRequired`: Emitted when paused for approval.
- `ToolPayload::Approved` / `Denied`: Emitted when operator resolves approval.
- `ToolPayload::Completed`: Emitted upon success.
- `ToolPayload::Failed`: Emitted upon failure.
- `ToolPayload::Cancelled`: Emitted upon cancellation.

All events carry trace metadata: `conversation_id`, `turn_id`, `task_id`, and `trace_id`.

---

## 17. Task Runtime & Conversation Core Interoperability

- **Conversation Core:** AI models produce tool call intents during turn generation. Conversation Core submits `ToolRequest` to `ToolRouter`, waits for execution result, and feeds the outcome back into the model context.
- **Task Runtime:** Long-running background tasks orchestrate multiple sequential or parallel `ToolRequest` invocations, bound to the parent `TaskId`.

---

## 18. Future Extensibility & Plugin Readiness (Phase 6/7)

The UTR architecture is designed for forward-compatibility:
- Phase 6 (Developer & System Tools) will mount `SystemDomainExecutor` and `GitDomainExecutor`.
- Phase 7 (Computer Control) will mount `OSControlDomainExecutor` adhering to the same pre-execution policy boundary.
- Third-party plugins will implement `DomainExecutor` with declared schemas.

---

## 19. Threat Model & Abuse Vector Defenses

| Threat / Abuse Vector | Mitigation Strategy |
| :--- | :--- |
| **Prompt Injection Tool Invocation** | Mandatory PolicyEngine evaluation, strict schema type-checking, confirmation required for high-risk actions. |
| **Replay of Prior Approvals** | Cryptographic hash binding, atomic status mutation to `Consumed`, policy version checks. |
| **Denial of Service / Infinite Hangs** | Scoped execution timeouts (`default_timeout_ms`), async cancellation tokens. |
| **State Tampering / In-flight Mutation** | Tool definitions and requests are immutable during execution (`Arc<ToolDefinition>`). |
| **Unauthorized File/Network Ingress** | Policy-enforced domain allowlists and path sandboxing. |

---

## 20. Verification & Test Matrix (24 Automated Tests)

The UTR test suite (`src-tauri/src/tools/tests.rs`) verifies 24 critical behaviors:

1. `test_01_tool_registration_succeeds`: Valid tool registers successfully.
2. `test_02_duplicate_registration_rejected`: Rejects registration of existing name.
3. `test_03_registration_lookup_matches`: Query returns correct tool definition.
4. `test_04_unknown_tool_returns_not_found`: Query of non-existent tool returns None.
5. `test_05_schema_validation_passes_valid_args`: Conforming arguments pass.
6. `test_06_schema_validation_catches_missing_required`: Catches missing required parameters.
7. `test_07_schema_validation_catches_type_mismatch`: Rejects wrong type (string vs number).
8. `test_08_schema_validation_catches_enum_violation`: Catches illegal enum value.
9. `test_09_malformed_json_returns_malformed_arguments`: Rejects non-object arguments.
10. `test_10_policy_allow_dispatches_to_executor`: ALLOW policy dispatches and returns success.
11. `test_11_policy_confirmation_required_pauses_with_approval_id`: Pause with approval ID; executor not invoked.
12. `test_12_confirmation_approval_allows_execution_on_replay`: Resumes upon approval resolution.
13. `test_13_policy_blocked_fails_closed_no_executor_invocation`: BLOCKED fails closed; executor never invoked.
14. `test_14_policy_restricted_preserves_constraints`: RESTRICTED policy applies constraints.
15. `test_15_approval_replay_rejected_if_already_consumed`: Replay of consumed approval is rejected.
16. `test_16_execution_id_uniquely_generated`: Default execution IDs are unique.
17. `test_17_custom_execution_id_preserved`: Caller-provided execution ID preserved.
18. `test_18_scoped_cancellation_cancels_in_flight`: In-flight cancellation terminates execution.
19. `test_19_execution_timeout_returns_timeout_error`: Timeout terminates hanging execution.
20. `test_20_error_normalization_typed_tool_execution_error`: Normalizes domain errors into typed errors.
21. `test_21_correlated_events_emitted_lifecycle`: Correlation context preserved end-to-end.
22. `test_22_browser_domain_adapter_handles_navigation`: Browser adapter navigates correctly.
23. `test_23_browser_domain_adapter_validates_safety_constraints`: Browser adapter enforces input safety.
24. `test_24_tool_execution_under_task_turn_propagates_correlation`: Multi-level correlation propagation.

---

## 21. Frontend Data Contracts & Service Integration

Frontend TypeScript contracts (`src/services/toolService.ts`) provide type safety and brand isolation:
- Branded type: `ToolExecutionId`.
- Typed structures: `ToolDefinition`, `ToolRequest`, `ToolExecutionResult`, `ToolExecutionError`.
- Service APIs: `listTools()`, `getTool()`, `executeTool()`, `cancelTool()`.
- Safe browser-mode mock fallbacks when not running in Tauri window.

---

## 22. Out-of-Scope Explicit Prohibitions

The following capabilities are strictly forbidden in Phase 5:
- **No Computer Control:** Mouse/keyboard hardware injection at the OS level (deferred to Phase 7).
- **No Direct Shell / Command Execution Framework:** OS automation and terminal execution are Phase 6.
- **No Browser Domain Rewrite:** Phase 5 encapsulates the existing 15 stable browser tools; it does not replace the browser frontend or browser controller.
- **No Audio / Speech / STT / TTS Alterations:** Realtime voice audio remains unchanged.

---

## 23. Transition & Operational Readiness

- **Backward Compatibility:** Existing `browser_tools::browser_execute_tool_cmd` remains operational while new orchestrators use `tools_execute`.
- **Zero Breaking Changes:** Pre-existing tests in `security`, `policy`, `events`, and `conversation` continue passing with zero regressions.
- **Merge Readiness:** All code builds cleanly on Windows without duplicate manifest errors or linker issues. Branch `feature/phase-5-tool-runtime` is ready for review.
