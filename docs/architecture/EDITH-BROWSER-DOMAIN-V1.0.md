# E.D.I.T.H. Browser Domain Architecture (Phase 6)

**Version:** 1.0  
**Author:** Principal Software Architect + Senior Rust/Tauri Engineer  
**Status:** Approved & Implemented  
**Date:** September 2026  

---

## 1. Executive Summary

Phase 6 of the E.D.I.T.H. architecture roadmap converges all **AI-driven browser capabilities** into the **Universal Tool Runtime (UTR)** and centralized **Policy Engine** established in Phases 4 and 5, while maintaining **100% functionality and zero regressions** for the direct human browser experience (`BrowserView.tsx`, multi-tab WebView2, omnibox, bookmarks, history, downloads).

Prior to Phase 6, browser automation existed in isolated silos:
- `BrowserAgent` directly invoked `execute_browser_tool`, bypassing centralized policy and task management.
- `BrowserRiskEngine` maintained an ad-hoc in-memory pending approvals map that predated the Phase 4 `PolicyEngine` and `ApprovalStore`.
- Multi-step tasks maintained private cancellation flags disconnected from the Phase 3 `TaskRuntime`.

With Phase 6:
- The **Browser Domain** is a first-class domain in the Universal Tool Runtime with 24 stable tools.
- Every AI-driven browser interaction strictly routes through:
  $$\text{AI / ConversationCore / BrowserAgent} \longrightarrow \text{ToolRequest} \longrightarrow \text{ToolRouter} \longrightarrow \text{PolicyEngine} \longrightarrow \text{BrowserDomainExecutor} \longrightarrow \text{WebView2 Engine}$$
- The host-enforced `PolicyEngine` is the single source of truth for authorization.
- `BrowserDomainExecutor` executes pre-authorized tools on WebView2 while enforcing human takeover semaphores.
- `BrowserAgent` operates as a reasoning and orchestration layer, delegating execution to `ToolRouter` and lifecycle/cancellation to `TaskRuntime`.

---

## 2. Target Execution Architecture

```
                 +-----------------------------------+
                 | ConversationCore / BrowserAgent   |
                 +-----------------------------------+
                                   |
                                   | ToolRequest(browser.*)
                                   v
                 +-----------------------------------+
                 |           ToolRouter              |
                 +-----------------------------------+
                                   |
                                   | ActionRequest
                                   v
                 +-----------------------------------+
                 |          PolicyEngine             |
                 +-----------------------------------+
                                   |
         +-------------------------+-------------------------+
         |                         |                         |
         v                         v                         v
   [ Outcome: Allow ]     [ ConfirmationRequired ]     [ Outcome: Blocked ]
         |                         |                         |
         |                         | Pauses & requests       v
         |                         | operator approval       ToolExecutionResult
         |                         | via ApprovalStore       (Fail-Closed Error)
         |                         v
         |               [ Single-Use Token Validated ]
         +-------------------------+
                                   |
                                   v
                 +-----------------------------------+
                 |       BrowserDomainExecutor       |
                 +-----------------------------------+
                                   |
                                   | execute_browser_tool_authorized
                                   v
                 +-----------------------------------+
                 |         WebView2 Engine           |
                 |      (BrowserState Singleton)     |
                 +-----------------------------------+
                                   |
                                   | Observation / Result
                                   v
                 +-----------------------------------+
                 |        ToolExecutionResult        |
                 +-----------------------------------+
                                   |
                                   v
                 +-----------------------------------+
                 |     Correlated Lifecycle Events   |
                 |  (ConversationCore / TaskRuntime) |
                 +-----------------------------------+
```

---

## 3. The 24 Universal Browser Tools

The browser domain provides 24 deterministic universal tools registered in `ToolRegistry` under `ToolDomain::Browser`:

| Tool Name | Domain | Category | Risk Level | Approval Required | Read-Only | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `browser.observe` | Browser | Observation | Safe | No | Yes | Observe live rendered DOM, interactive elements, forms, and headings. |
| `browser.screenshot` | Browser | Observation | Safe | No | Yes | Capture viewport screenshot of a tab as Base64 data URL. |
| `browser.get_tabs` | Browser | Observation | Safe | No | Yes | List all open tabs, titles, URLs, and active focus state. |
| `browser.get_active_tab` | Browser | Observation | Safe | No | Yes | Retrieve state and URL of currently active tab. |
| `browser.navigate` | Browser | Navigation | Low | No | No | Navigate tab to an HTTP/HTTPS destination URL. |
| `browser.new_tab` | Browser | Navigation | Low | No | No | Open a new tab with an optional initial URL. |
| `browser.close_tab` | Browser | Navigation | Low | No | No | Close specified browser tab and release resources. |
| `browser.switch_tab` | Browser | Navigation | Safe | No | No | Switch active user-facing focus to specified tab. |
| `browser.back` | Browser | Navigation | Safe | No | No | Navigate backward in tab history. |
| `browser.forward` | Browser | Navigation | Safe | No | No | Navigate forward in tab history. |
| `browser.reload` | Browser | Navigation | Safe | No | No | Reload active page in a browser tab. |
| `browser.click` | Browser | Interaction | Medium | Conditional | No | Click interactive element identified by deterministic `element_id`. |
| `browser.type` | Browser | Interaction | Medium | Conditional | No | Type text into input field or textarea (passwords require confirmation). |
| `browser.scroll` | Browser | Interaction | Safe | No | No | Scroll browser viewport in direction with bounded pixel increment. |
| `browser.press_key` | Browser | Interaction | Low | No | No | Dispatch keyboard event from allowed key enum to active element. |
| `browser.focus` | Browser | Interaction | Safe | No | No | Focus element on active page identified by `element_id`. |
| `browser.wait` | Browser | Interaction | Safe | No | No | Wait for page load, element presence, URL change, or bounded timeout. |
| `browser.select_option`| Browser | Interaction | Low | No | No | Select option from dropdown HTML select element. |
| `browser.history_recent`| Browser| Storage | Safe | No | Yes | Retrieve recent browsing history entries (newest first). |
| `browser.history_search`| Browser| Storage | Safe | No | Yes | Search browsing history by URL or title query. |
| `browser.bookmarks_list`| Browser| Storage | Safe | No | Yes | Retrieve all saved browser bookmarks. |
| `browser.bookmarks_search`|Browser|Storage | Safe | No | Yes | Search bookmarks by title or URL query. |
| `browser.downloads_recent`|Browser|Downloads| Safe | No | Yes | Retrieve recent downloads with progress, size, and status. |
| `browser.download_get` | Browser | Downloads | Safe | No | Yes | Get detailed metadata and progress for a download ID. |

---

## 4. Centralized Policy Integration & Confirmation Flow

### 4.1 Single Authorization Authority
- Standalone risk checks in `BrowserRiskEngine` no longer compete with `PolicyEngine`.
- When `BrowserDomainExecutor` invokes `execute_browser_tool_authorized`, the action has already been verified and authorized by `PolicyEngine`.
- The duplicate in-memory pending approvals map is bypassed during UTR execution in favor of `ApprovalStore`.

### 4.2 Security Rules Enforced by BrowserAdapter
1. **URI Scheme Enforcement:**
   - `javascript:`: Blocked immediately with `RiskLevel::Critical`.
   - `file:`: Blocked immediately with `RiskLevel::Critical`.
   - `data:text/html`: Blocked immediately with `RiskLevel::Critical`.
   - Native protocols without `http://`, `https://`, or `about:`: Blocked with `RiskLevel::High`.
2. **Credential & Sensitive Field Protection:**
   - Any typing action targeting fields named `password`, `passwd`, `pwd` or flagged with `is_password: true` strictly returns `ConfirmationRequired` with `RiskLevel::Critical`.
   - Fields or text involving `cvv`, `cvc`, `creditcard`, or `cardnumber` strictly return `ConfirmationRequired`.
3. **Destructive Action Protection:**
   - Financial buttons (`Buy Now`, `Checkout`, `Place Order`, `Pay`) require confirmation.
   - Destructive buttons (`Delete Account`, `Cancel Subscription`, `Wipe Data`) require confirmation.
   - Storage/download deletions (`history_delete`, `history_clear`, `bookmark_remove`, `download_cancel`, `download_start`) require confirmation.

### 4.3 Confirmation Lifecycle
1. `ToolRouter` evaluates `PolicyEngine::evaluate(&req, &ctx)`.
2. If `ConfirmationRequired`, `PolicyEngine::request_approval` generates an `ApprovalRequest` in `ApprovalStore` and emits `PolicyPayload::ApprovalRequired`.
3. Execution pauses and returns `ToolStatus::ApprovalRequired` with `approval_id`.
4. Operator approves via `policy_resolve_approval(approval_id, true)`.
5. AI or Task replays `ToolRequest` with `active_approval_id`.
6. `PolicyEngine::validate_and_consume_approval` consumes the token under **single-use semantics**.
7. Tool executes successfully. Any replay attempt of the consumed token immediately fails closed.

---

## 5. BrowserAgent Migration & Orchestration

### 5.1 Architecture
`BrowserAgent` is refactored from an autonomous execution silo into a pure reasoning and orchestration layer:
- **LLM Reasoning Loop:** Maintains context truncation, evidence validation (`verify_completion_evidence`), and repetition detection.
- **Tool Translation:** `to_universal_tool_name` maps legacy tool strings (`browser_open_url`) to canonical names (`browser.navigate`).
- **UTR Dispatch:** Tool invocations are submitted as `ToolRequest` via `ToolRouter::execute`.
- **Approval Pausing:** If `ToolStatus::ApprovalRequired` is returned, the agent sets status to `BrowserTaskStatus::Waiting`, notifies the frontend, and pauses without crashing.

### 5.2 TaskRuntime & Cancellation Integration
- Autonomous runs create an authoritative task in `TaskRuntime` with `TaskType::BrowserAgent` and `TaskOwner::Agent`.
- Each step updates task progress via `task_runtime.update_progress(&task_id, step, max_steps, status_text)`.
- Task cancellation propagates cooperatively: checking both the local atomic flag and `task_runtime.get_cancellation_token`.
- On termination, `task_runtime.complete_task`, `fail_task`, or `cancel_task` is called, ensuring unified task lifecycle visibility.

---

## 6. State Ownership & Frontend Integration

### 6.1 Authoritative State Ownership
- **`BrowserState` (Tauri State Singleton):** Authoritative owner of all WebView2 instances, tabs, active tab focus, window bounds, and zoom.
- **No Competing State:** `BrowserDomainExecutor` operates directly on `BrowserState`.
- **Human Takeover Semaphore:** `GLOBAL_CONTROL_MGR.verify_ai_action_permitted(&tab_id, tool_name)` is checked before every action. If a user physically takes control of a tab in the UI, AI execution halts safely.

### 6.2 Frontend Non-Regression
- `BrowserView.tsx`, omnibox navigation, tab bar, history, bookmarks, and downloads continue using direct Tauri commands (`browser_navigate_tab`, `browser_switch_tab`, etc.) without touching the tool runtime.
- AI actions executed through UTR update the same `BrowserState`, immediately reflecting on screen for the user.

---

## 7. Security Boundaries

1. **Untrusted Web Content:** Webpage DOM elements, text, and titles are treated as untrusted data. Extracted content cannot self-authorize tool executions or override policy constraints.
2. **Strict Sanitization:** URLs are validated and sanitized before navigation.
3. **No Secret Leaks:** Diagnostic logs and error messages omit raw passwords, payment values, and sensitive session tokens.
