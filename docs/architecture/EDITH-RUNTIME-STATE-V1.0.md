# E.D.I.T.H. Runtime State, Self-Knowledge & Self-Control Architecture (V1.0)

## 1. Executive Summary

This specification establishes **Phase 8** of the approved E.D.I.T.H. AI Core Architecture roadmap (`docs/architecture/EDITH-AI-CORE-ARCHITECTURE-V1.1.md`). 

Phase 8 introduces the **Runtime State & Coordination Layer** (`EdithRuntimeState`) alongside structured **Self-Knowledge** and bounded, policy-governed **Self-Control** tools under the `edith.*` domain in the Universal Tool Runtime (`ToolRouter`).

```
AI / Agent / ConversationCore
             │
             ▼
   ToolRequest("edith.*")
             │
             ▼
        ToolRouter (Universal Tool Runtime)
             │
             ▼
       PolicyEngine (Host Boundary) ──[Requires Approval]──► Operator Prompt (HITL)
             │ (Allowed / Pre-Authorized)
             ▼
      EdithDomainExecutor
             │
             ▼
      EdithRuntimeState (Coordination & Read-Model Projection)
             ├──► ConversationCore (Active turns, streaming)
             ├──► TaskRuntime (Active & queued background tasks)
             ├──► ToolRegistry (Registered tools, schemas)
             ├──► CancellationRegistry (In-flight executions)
             ├──► ProviderRegistry (AI adapters, models, capabilities)
             ├──► BrowserState & GLOBAL_CONTROL_MGR (Open tabs, control ownership)
             ├──► GLOBAL_COMPUTER_CONTROL_MGR (Desktop control ownership & takeover)
             └──► PolicyEngine & ApprovalStore (Active constraints & pending approvals)
```

---

## 2. Core Architectural Principles

### A. Non-Duplicating Observation Layer
The runtime state layer is strictly an **observation and coordination read-model**. It aggregates canonical state from authoritative domain owners without duplicating or becoming a competing owner of domain state:
- **ConversationCore** remains the sole owner of conversational turns and streaming context.
- **TaskRuntime** remains the sole owner of background tasks and task lifecycle states.
- **ToolRegistry** remains the sole owner of tool definitions and parameter schemas.
- **ToolRouter / CancellationRegistry** remains the sole owner of in-flight tool cancellation tokens.
- **ProviderRegistry** remains the sole owner of AI provider adapters and model capabilities.
- **BrowserState** and **BrowserControlManager** remain the sole owners of browser tabs and tab control.
- **ComputerControlManager** remains the sole owner of desktop input ownership and takeover preemption.
- **PolicyEngine** and **ApprovalStore** remain the sole owners of security policies and operator approvals.

### B. Structured Self-Knowledge vs. Prompt Bloat
Instead of inflating system prompts with large, stale textual descriptions or hallucinating current capabilities, E.D.I.T.H. queries live runtime state through typed, bounded tools (`edith.get_runtime_status`, `edith.get_capabilities`, `edith.list_active_tasks`, etc.).

### C. Bounded Self-Control & Least Privilege
Self-control tools allow E.D.I.T.H. to coordinate her own operations (e.g. `edith.cancel_task`, `edith.cancel_tool_execution`), but are strictly subordinate to the Host Policy Engine (`PolicyEngine`). E.D.I.T.H. cannot modify security policies, approve her own pending actions, alter credentials, access raw secrets, or self-escalate privileges.

---

## 3. State Ownership Matrix

| Entity | Canonical Owner | Source of Truth | Runtime State Visibility | Permitted Mutators |
|:---|:---|:---|:---|:---|
| **Conversation Turn** | `ConversationCore` | `ConversationCore::turns` | Read-only projection | User prompt, `ConversationCore` stream runner |
| **Autonomous Task** | `TaskRuntime` | `TaskRuntime::tasks` | Read-only projection | Task worker loops, operator, `edith.cancel_task` |
| **Tool Registry** | `ToolRegistry` | `ToolRegistry::tools` | Read-only projection | Domain plugins during startup setup |
| **In-Flight Tool Execution** | `ToolRouter` | `CancellationRegistry::executions` | Read-only projection | Tool router execution lifecycle, `edith.cancel_tool_execution` |
| **AI Providers & Models** | `ProviderRegistry` & `DbState` | `ProviderRegistry::providers` & SQLite `app_settings` | Read-only projection (Sanitized) | Settings UI (user configured) |
| **Browser Tabs & State** | `BrowserState` | `BrowserState::tabs`, `active_tab_id` | Read-only projection | User in `BrowserView`, `BrowserDomainExecutor` |
| **Browser Control State** | `BrowserControlManager` | `GLOBAL_CONTROL_MGR` | Read-only projection | Human interaction, `BrowserControlManager` |
| **Computer Control State** | `ComputerControlManager` | `GLOBAL_COMPUTER_CONTROL_MGR` | Read-only projection | Human interaction preemption, `ComputerControlManager` |
| **Policy & Security Mode** | `PolicyEngine` | `PolicyEngine::constraints` | Read-only projection | Host operator settings only (**Never AI**) |
| **Pending Approvals** | `ApprovalStore` | `ApprovalStore::approvals` | Read-only projection (No secrets) | `PolicyEngine` (request), Host operator (resolve) |
| **Overall Autonomy State** | `EdithRuntimeState` | Derived dynamically from above subsystems | Read-only projection | Pure derived calculation based on live subsystem states |

---

## 4. Operational Autonomy State Machine

The overall operational state of E.D.I.T.H. is synthesized dynamically by `EdithRuntimeState::get_autonomy_state()` using an unambiguous priority hierarchy:

```
                      ┌──────────────┐
                      │     Idle     │
                      └──────┬───────┘
                             │ User Prompt
                             ▼
                      ┌──────────────┐
                      │  Conversing  │◄────────────┐
                      └──────┬───────┘             │
                             │ Tool Call           │
                             ▼                     │ Tool
                      ┌──────────────┐             │ Result
                      │ExecutingTool ├─────────────┘
                      └──────┬───────┘
                             │ Autonomous Task Spawn
                             ▼
                      ┌──────────────┐
                      │ RunningTask  │◄────────────┐
                      └──────┬───────┘             │
                             │ High Risk Action    │ Operator
                             ▼                     │ Approved
                      ┌──────────────────┐         │
                      │WaitingForApproval├─────────┘
                      └──────┬───────────┘
                             │ Physical Mouse/Key Activity
                             ▼
                      ┌──────────────┐
                      │ UserTakeover │
                      └──────────────┘
```

### Derivation Priority Order:
1. **`UserTakeover`** (Highest Precedence): Triggered if `GLOBAL_COMPUTER_CONTROL_MGR` is `AiPaused` (physical mouse/key takeover) or any browser tab is `AiPaused`. Halts all autonomous execution immediately.
2. **`WaitingForApproval`**: Triggered if `ApprovalStore` contains at least one pending operator approval.
3. **`RunningTask`**: Triggered if `TaskRuntime` has one or more non-terminal tasks running or queued.
4. **`ExecutingTool`**: Triggered if `CancellationRegistry` contains active in-flight tool executions.
5. **`Conversing`**: Triggered if `ConversationCore` has an active non-terminal dialogue streaming turn.
6. **`Idle`**: Default state when no background or foreground operations are executing.

---

## 5. Tool Catalog (`edith.*`)

### A. Self-Knowledge Tools (Read-Only)

| Tool Name | Parameters | Description | Risk Level | Redaction & Sanitization |
|:---|:---|:---|:---|:---|
| `edith.get_runtime_status` | None | Returns overall operational status: autonomy state, active session/turn, task count, tool execution count, security mode, and uptime. | Low | Bounded summary; zero secrets. |
| `edith.get_capabilities` | `domain?: string` | Returns catalog of available tool domains, tools per domain, and active provider capabilities. | Low | Excludes internal debug hooks. |
| `edith.list_active_tasks` | `limit?: integer` | Returns bounded list of running/queued tasks with progress steps and status. | Low | Sensitive text patterns scrubbed. |
| `edith.get_task_details` | `task_id: string` | Returns detailed snapshot and progress of a specific task. | Low | Errors and summaries scrubbed. |
| `edith.list_providers` | None | Lists registered AI provider adapters and supported capabilities. | Low | Excludes all API keys, headers, and endpoints. |
| `edith.get_browser_status` | None | Returns summary of open browser tabs, active tab ID, visibility, and control state. | Low | Query params (`token`, `key`, `auth`, `password`) scrubbed to `[REDACTED]`. |
| `edith.get_computer_status` | None | Returns desktop control ownership state and takeover reason. | Low | Sensitive window titles scrubbed. |
| `edith.get_security_status` | None | Returns policy version, security mode, and pending confirmation count. | Low | Excludes cryptographic hashes and raw arguments. |
| `edith.get_system_health` | None | Diagnostic health check across database, task runtime, tool runtime, provider registry, browser, and computer control. | Low | No stack traces or database connection strings. |

### B. Self-Control Tools

| Tool Name | Parameters | Description | Risk Level | Ownership Scope & Security Rule |
|:---|:---|:---|:---|:---|
| `edith.cancel_task` | `task_id: string`, `reason?: string` | Cancels an active background task. | Medium | Caller can cancel tasks initiated within its active turn or session. Cancelling foreign or protected System/User tasks requires operator approval. |
| `edith.cancel_tool_execution` | `execution_id: string`, `reason?: string` | Cancels an in-flight tool execution. | Medium | Scoped strictly to the caller's active turn/task in `CancellationRegistry`. Foreign executions are rejected. |

### C. Deferred Capabilities (Guardrail 2)
Inspection of `src-tauri/src/task/state.rs` verifies that `TaskStatus` currently supports `Created`, `Queued`, `Running`, `Completing`, `Completed`, `Failed`, and `Cancelled`. True cooperative task suspension (`TaskStatus::Paused`, pause/resume event taxonomies) does not yet exist in `TaskRuntime`. In strict adherence to architectural guardrails, `pause_task` and `resume_task` are formally deferred until `TaskRuntime` introduces cooperative suspension, and fail closed with `TASK_PAUSE_NOT_SUPPORTED`.

---

## 6. Security Boundaries & Anti-Escalation Guarantees

1. **Host Policy Engine Invariant**: Every `edith.*` tool execution passes through `ToolRouter::execute` and is authorized by `PolicyEngine::evaluate`.
2. **Prohibited Operations**: Any attempt to call `modify_policy`, `grant_permission`, `set_security_mode`, `reveal_secrets`, or `bypass_approval` fails closed immediately with `PolicyDecision::Blocked` and `RiskLevel::Critical`.
3. **Data Redaction**:
   - URLs: `scrub_url()` automatically redacts `token`, `key`, `auth`, `password`, `code`, `secret` query parameters. Local `file://` URLs are masked.
   - Strings: `scrub_sensitive_text()` removes common API key patterns (`gsk_`, `AIza`, `sk-`, `Bearer `).
   - Providers: `ProviderSummary` contains only `id`, `name`, `capabilities`, `model_count`, `default_model`.

---

## 7. Concurrency & Performance Strategy

1. **Lock-Free Read Models**: `EdithRuntimeState` does not maintain long-lived locks. State queries take brief, read-only locks on internal maps, clone lightweight DTOs, and immediately release guards.
2. **No Deadlocks**: Subsystems are queried sequentially without holding multiple locks simultaneously.
3. **Async Safety**: All returned projections implement `Clone + Send + Sync + Serialize + Deserialize`.
