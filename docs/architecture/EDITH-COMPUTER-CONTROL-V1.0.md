# E.D.I.T.H. Computer Control Domain Architecture Specification (v1.0)

## 1. Executive Summary

This document specifies the architecture, security enforcement, platform adapters, and tool models for the **Computer Control Domain** within Project E.D.I.T.H. 

The Computer Control Domain establishes desktop automation as a first-class domain inside the **Universal Tool Runtime** (Phase 5). All desktop operations—screen observation, window queries, focusing, launching applications, closing windows, pointer movements, mouse clicks, keyboard typing, and hotkeys—are dispatched through the centralized `ToolRouter`, guarded by the host-authoritative `PolicyEngine` (Phase 4), and correlated with the end-to-end event infrastructure (Phase 2).

---

## 2. Architectural Boundary & Execution Path

The execution path guarantees that AI models never execute desktop or OS commands directly:

```
AI Model / ConversationCore / TaskRuntime
                   │
                   ▼
       ToolRequest("computer.*")
                   │
                   ▼
              ToolRouter
                   │
                   ▼
             PolicyEngine  ──[ConfirmationRequired]──► Operator HITL Prompt
                   │ (Allow / Pre-Authorized)
                   ▼
         ComputerDomainExecutor
                   │
                   ▼
         ComputerPlatform Adapter
       ┌───────────┴───────────┐
       ▼                       ▼
WindowsPlatformAdapter     MockPlatformAdapter
(user32.dll / FFI)         (Tests / Headless)
       │
       ▼
 Desktop / OS Target
       │
       ▼
ToolExecutionResult (Normalized) & Correlated Events
```

---

## 3. Tool Inventory & JSON Schemas

The Computer domain provides **14 canonical tools** namespaced under `computer.*`:

| Tool Name | Domain | Read-Only | Timeout | Description |
|---|---|---|---|---|
| `computer.observe_screen` | Computer | Yes | 5,000ms | Inspect desktop screen dimensions, display count, cursor coordinates, and active window. |
| `computer.screenshot` | Computer | Yes | 10,000ms | Capture full visual screenshot of display or active window as base64 JPEG data URL. |
| `computer.get_active_window` | Computer | Yes | 5,000ms | Query currently focused top-level window title, process name, PID, and bounding box. |
| `computer.list_windows` | Computer | Yes | 5,000ms | List all open and visible desktop application windows with titles, process names, bounds. |
| `computer.focus_window` | Computer | No | 5,000ms | Activate and bring an application window into foreground focus by title substring. |
| `computer.launch_app` | Computer | No | 10,000ms | Launch an approved application from the registered application catalog. |
| `computer.close_window` | Computer | No | 5,000ms | Gracefully request an application window to close via `WM_CLOSE`. |
| `computer.move_cursor` | Computer | No | 5,000ms | Move mouse cursor to absolute desktop coordinates `(x, y)`. |
| `computer.click` | Computer | No | 5,000ms | Perform a mouse click (`left`, `right`, `middle`) at current or specified coordinate. |
| `computer.double_click` | Computer | No | 5,000ms | Perform a double left-click at current or specified desktop coordinate. |
| `computer.right_click` | Computer | No | 5,000ms | Perform a right-click at current or specified desktop coordinate to open context menus. |
| `computer.type` | Computer | No | 10,000ms | Type text into the currently focused desktop input field or window. |
| `computer.press_key` | Computer | No | 5,000ms | Press and release a keyboard key (`enter`, `tab`, `escape`, `backspace`, arrows, etc.). |
| `computer.hotkey` | Computer | No | 5,000ms | Execute a keyboard shortcut combination (`ctrl+c`, `ctrl+v`, `alt+tab`, etc.). |

---

## 4. Policy Engine Security Boundary & Risk Classification

The Host Security Policy Engine (`src-tauri/src/policy/adapters/computer.rs`) evaluates proposed computer operations independently of AI reasoning:

### Risk Tiers
1. **Low Risk (`PolicyOutcome::Allow`)**:
   - `observe_screen`, `screenshot`, `get_active_window`, `list_windows`, `move_cursor`, `scroll`, `wait`.
   - Actions are read-only or passive pointer movements.
2. **Medium Risk (`PolicyOutcome::Allow` in Standard Mode)**:
   - `click`, `double_click`, `right_click`, `focus_window`, `press_key`, safe `hotkey` combinations (`ctrl+c`, `ctrl+v`, `ctrl+a`, `ctrl+z`, `alt+tab`).
   - `type` with standard non-sensitive text into normal application windows.
3. **High Risk (`PolicyOutcome::ConfirmationRequired` - HITL)**:
   - `launch_app`: Launching applications creates new processes and requires operator confirmation.
   - `close_window`: Closing windows can terminate user work and requires confirmation.
   - `type` with sensitive patterns (passwords, credentials, API keys, `is_sensitive: true`).
   - High-consequence hotkeys (`alt+f4`, `ctrl+shift+esc`).
4. **Critical / Blocked (`PolicyOutcome::Blocked`)**:
   - Interacting with or targeting User Account Control (UAC) prompts or Windows Security dialogs.
   - Closing core system components (`explorer.exe`, Task Manager, E.D.I.T.H. runtime).
   - Privileged system hotkeys (`ctrl+alt+del`, `win+r`, `win+l`).
   - Any action attempted when `ComputerControlState::AiPaused`.

---

## 5. Human Takeover & Control Ownership Model

The system enforces unambiguous control ownership in `src-tauri/src/computer_control.rs` via `GLOBAL_COMPUTER_CONTROL_MGR`:

```
           ┌───────────────────────┐
           │    UserControlled     │ ◄── Default state
           └───────────┬───────────┘
                       │ (Task started / authorized)
                       ▼
           ┌───────────────────────┐
 ┌────────►│     AiControlled      │
 │         └───────────┬───────────┘
 │ (Operator           │ (Human intervention / mouse move / keypress)
 │  resumes)           ▼
 │         ┌───────────────────────┐
 └─────────┤       AiPaused        │ ◄── All AI execution immediately blocked
           └───────────────────────┘
```

- When `AiPaused`, any incoming tool request fails closed immediately with `ToolExecutionError::DomainError`.
- No dual-control conflict is permitted: human input strictly preempts autonomous actions.

---

## 6. Windows Native Platform Implementation

The platform layer (`src-tauri/src/tools/domains/computer_platform.rs`) links directly to `user32.dll` and `kernel32.dll` via safe, zero-dependency `extern "system"` FFI:

- **Window Management**: `GetForegroundWindow`, `GetWindowTextW`, `GetWindowThreadProcessId`, `EnumWindows`, `IsWindowVisible`, `SetForegroundWindow`, `ShowWindow`, `PostMessageW(WM_CLOSE)`.
- **Mouse Simulation**: `GetCursorPos`, `SetCursorPos`, `mouse_event` with coordinates clamped to `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`.
- **Keyboard Simulation**: `keybd_event` supporting standard virtual keys and Shift modifier simulation for uppercase ASCII characters.
- **Screen Capture**: Integrated with `screenshots::Screen` and `image` crate (capturing RGBA buffer, optional window bounds cropping, and encoding to JPEG data URLs).
- **Process Identification**: `OpenProcess` with `PROCESS_QUERY_LIMITED_INFORMATION` and `QueryFullProcessImageNameW`.

---

## 7. Application Launching Policy Reuse

The `computer.launch_app` tool reuses the existing `AppLauncherPolicy` in `src-tauri/src/security.rs`. It validates requested names against:
1. Built-in applications: Notepad, Calculator, Chrome, File Explorer, Command Prompt, Task Manager, Settings, Paint, VS Code.
2. Custom user-registered applications stored in SQLite database (`edith_memory.db`).
3. Arbitrary executables (e.g. `malicious.exe`, unlisted paths) are blocked by security policy.

---

## 8. Observe → Act → Verify Paradigm

Computer control adheres to the stateful automation loop:
1. **Observe**: AI queries `get_active_window` or `list_windows` to inspect window title, process name, and bounding box `(x, y, width, height)`.
2. **Act**: AI issues precise actions (`focus_window`, `click`, `type`, `press_key`).
3. **Verify**: The executor validates operational outcomes (e.g. confirming window focus succeeded, verifying window closed, checking process spawn).

---

## 9. Scoped Cancellation & Timeout Bounding

- Every computer action registers with the `CancellationRegistry` using `tokio_util::sync::CancellationToken`.
- Parent turn or task cancellation terminates in-flight computer actions cooperatively.
- `ToolRouter` enforces execution timeouts (default 5,000ms or 10,000ms) with `tokio::select!`.

---

## 10. Correlated Event Lifecycle

Every tool execution emits correlated telemetry envelopes:
- `ToolPayload::Requested`
- `ToolPayload::ApprovalRequired` (if HITL required)
- `ToolPayload::Started`
- `ToolPayload::Completed`
- `ToolPayload::Failed`
- `ToolPayload::Cancelled`

All events preserve `EventCorrelation` containing `conversation_id`, `turn_id`, `task_id`, and `tool_execution_id`.

---

## 11. Verification & Testing

The test suite in `src-tauri/src/tools/domains/computer_tests.rs` validates all 15 architectural invariants:

1. `test_01_computer_tool_registration`: 14 tools registered with `ToolDomain::Computer`.
2. `test_02_computer_tool_schema_validation`: Arguments validated against JSON schema.
3. `test_03_computer_policy_observation_allowed`: Read-only queries evaluated as Low risk.
4. `test_04_computer_policy_sensitive_type_confirmation`: Password entry requires confirmation.
5. `test_05_computer_policy_launch_app_confirmation`: App launch requires confirmation; arbitrary apps blocked.
6. `test_06_computer_policy_close_window_confirmation`: Window closure requires confirmation; system components blocked.
7. `test_07_computer_policy_high_risk_hotkey_confirmation`: Alt+F4 requires confirmation; safe hotkeys allowed.
8. `test_08_computer_policy_blocked_uac`: UAC targets and Ctrl+Alt+Del blocked.
9. `test_09_computer_confirmation_resumption`: Approved token allows execution on replay.
10. `test_10_computer_replay_consumed_rejected`: Replaying consumed token fails closed.
11. `test_11_computer_human_takeover_preemption`: Execution blocked when `AiPaused`.
12. `test_12_computer_scoped_cancellation`: In-flight action halts on cancellation token.
13. `test_13_computer_execution_timeout`: Router enforces 0ms execution timeout.
14. `test_14_computer_correlated_events_lifecycle`: Events maintain correlation context.
15. `test_15_computer_error_normalization`: Failures normalized into `ToolExecutionResult` with `DOMAIN_ERROR`.
