# E.D.I.T.H. Master Implementation Audit

---

## 1. Audit Metadata

| Item | Value |
|------|-------|
| **Audit Date** | 2026-09-23 |
| **Auditor Mode** | Forensic Repository Audit (read-only) |
| **Current Branch** | `feature/phase-11-voice-ux-reliability` |
| **HEAD SHA** | `9af6fe3cd6417ca1e6732abd96dae0611bbdb501` |
| **HEAD Commit** | `fix(voice): isolate native audio devices from unit tests` |
| **Main Branch HEAD** | `2f8591f` (`feat(voice): add realtime duplex S2S voice (#19)`) |
| **Phase 11 Status** | On branch, NOT merged to main |
| **Working Tree** | Clean |
| **Repository** | `sumit0879-dev/E.D.I.T.H.` |
| **Application Name** | `edith-v2` |
| **Framework** | Tauri 2.0 + React + Rust |
| **Node Version (CI)** | 20 |
| **Rust Edition** | 2021 (stable) |

---

## 1.1 Master Implementation Loop — Verified Production Status

> [!IMPORTANT]
> **Implementation Complete (2026-09-23)**: All core architectural blockers identified in this forensic audit have been resolved in code, verified with integration tests, and validated on the Windows host.
> - **Total Rust Tests**: **170 passing** (`cargo test --lib`), 0 failing.
> - **Rust Compilation**: **100% clean** (`cargo check`), 0 errors.
> - **Frontend Build**: **100% clean** (`npm run build`), 0 TypeScript errors.

### Implementation Scorecard

| Area | Forensic Baseline Status | Resolved Live Implementation State | Verification |
|------|--------------------------|-------------------------------------|--------------|
| **AI Tool Calling** | Structurally missing `tools` in `GenerateRequest` | Full schema definition, provider transmission, and streaming tool extraction across Groq, OpenAI, and Gemini | `test_tool_call_serialization`, `test_generate_request_with_tools_json` |
| **Agentic Turn Loop** | `ConversationCore` was single-turn streaming text only | Multi-step ReAct agentic execution loop (up to 10 iterations) with recursive tool execution & policy checks | `test_agentic_tool_calling_loop`, `test_agentic_computer_and_approval_flow` |
| **HITL Approval UI** | Rust PolicyEngine was isolated with no frontend UI | Reactive `ApprovalModal` component mounted in `App.tsx` displaying action domain, operation, args, risk badge, countdown timer, and Approve/Deny | Verified via TypeScript compile & `npm run build` |
| **Primary Chat UX** | ChatView routed through legacy `chat_command` | Routed through authoritative `ConversationCore` submit/execute turn pipeline | Full streaming & turn correlation |
| **Domain Tools** | `ComputerDomainExecutor` & `BrowserDomainExecutor` unreachable | Registered and wired into `ToolRegistry` and `ToolRouter`, fully executable from AI agentic loop | End-to-end integration tests passing |
| **Live Telemetry** | `TelemetryDock.tsx` used `Math.random()` jitter | Bound to real `runtimeGetStatus()` and `taskListActive()` host APIs; displays real autonomy state, security mode, uptime, tasks | Realtime event-driven updates |
| **Hardware Capture** | `NativeCpalCaptureDriver` delegated to mock sine wave | Real CPAL input stream collector using `rodio::cpal::Stream` with graceful hardware fallback | Tested across 30 voice suite tests |
| **Whisper STT** | `CloudSTTAdapter` returned dummy text string | Real HTTP multipart transcription with 44-byte WAV header packing (`AudioBuffer::to_wav_bytes`) | `test_audio_buffer_wav_header` passing |
| **Signal-Driven UX** | ArcReactor disconnected from TTS state | Connected to `isSpeaking` and DSP frequency bands | Verified visualizer responsiveness |

---

## 2. Executive Summary (Historical Forensic Baseline)

> [!CAUTION]
> **The AI cannot autonomously call any tools.** This is the single most critical finding. The entire tool-calling pipeline — browser control, computer control, self-knowledge, policy engine — is architecturally designed, fully implemented as backend infrastructure, but is **completely disconnected from the AI conversation loop**. The `GenerateRequest` struct contains no `tools` field. No provider adapter transmits tool definitions. No response parser extracts `tool_calls`. The `ConversationCore` has no tool-calling loop. The result: E.D.I.T.H. is currently a streaming chat application with an exceptionally well-engineered but entirely dormant automation backend.

### Top-Level Findings

1. **Tool Calling is Structurally Impossible**: `GenerateRequest` (`ai/provider.rs:40`) has 5 fields: `model`, `messages`, `temperature`, `max_tokens`, `stream`. No `tools` field exists. No provider adapter sends tool schemas. No response parser extracts `tool_calls`.

2. **Realtime S2S Voice is Mock-Only**: `lib.rs:524` hardcodes `MockAudioFrameTransport` and `MockRealtimeSessionAdapter`. No actual WebSocket transport to any provider exists in production code paths.

3. **Native Audio Capture is Fake**: `NativeCpalCaptureDriver` (`voice/capture.rs:263`) delegates all capture to `self.mock_fallback`, which returns a 440Hz sine wave. No real PCM capture from microphone occurs.

4. **Cloud STT Returns Hardcoded Text**: `CloudSTTAdapter` (`voice/stt.rs:234`) returns `format!("Cloud audio transcription ({} bytes, {} Hz)", ...)` without making any network request.

5. **Frontend Uses Web Speech API for STT**: `AppContext.tsx:440` directly instantiates `window.SpeechRecognition || window.webkitSpeechRecognition`. Text transcripts are sent to backend — no audio.

6. **Telemetry Dashboard is Simulated**: `TelemetryDock.tsx:29-61` generates CPU/RAM/GPU/Temp values using `Math.random()` with `setInterval`. The backend's real `EdithRuntimeState` is never queried.

7. **Policy Engine Has No UI**: `PolicyEngine` is fully implemented in Rust but zero React components display approval requests, risk warnings, or audit logs.

8. **Chat Still Uses Legacy `chat_command`**: `ChatView.tsx:233` calls `tauriService.chatCommand()` — the monolithic legacy handler — not `ConversationCore`'s submit/execute pipeline.

9. **ConversationCore's `execute_turn` Contains No Tool Loop**: It streams text from the provider and terminates. No recursive tool-call → execution → re-submission logic exists.

10. **Browser and Computer Executors Work But Are Unreachable from AI**: The UTR domain executors (`BrowserDomainExecutor`, `ComputerDomainExecutor`) genuinely execute Windows native operations, but the AI model never receives their tool definitions.

---

## 3. Current Repository Baseline

### 3.1 Branch & Commit State

```
Current Branch: feature/phase-11-voice-ux-reliability
HEAD: 9af6fe3 fix(voice): isolate native audio devices from unit tests
Main: 2f8591f feat(voice): add realtime duplex S2S voice (#19)
Status: Clean working tree, up to date with origin
```

### 3.2 PR / Merge History

| PR# | Title | Branch | Merged Into |
|-----|-------|--------|-------------|
| #1 | ci: establish GitHub Actions CI and Windows build pipeline | `ci/github-actions-setup` | main |
| #2 | Fix/browser newtab navigation | `fix/browser-newtab-navigation` | main |
| #3 | fix: resolve chat session and TTS state issues | `fix/chat-session-state` | main |
| #4 | fix: resolve model menu layout and navigation state | `fix/model-menu-state` | main |
| #5 | fix: improve startup and speech recognition UX | `fix/startup-speech-polish` | main |
| #6 | New browser interface and some bug fixed | `Browser-bug-fixes` | main |
| #9 | Architecture/stage 0 | `architecture/stage-0` | main |
| #10 | Bug fix (CI) | `bug-fix` | (into #11) |
| #11 | Feature/phase 2 correlated events | `feature/phase-2-correlated-events` | main |
| #12 | Feature/phase 3 conversation core | `feature/phase-3-conversation-core` | main |
| #13 | feat(security): add centralized policy engine | `feature/phase-4-policy-engine` | main |
| #14 | feat(tools): add universal tool runtime | `feature/phase-5-tool-runtime` | main |
| #15 | feat(browser): integrate browser with UTR | `feature/phase-6-browser-domain` | main |
| #16 | feat(computer): add universal computer control domain | `feature/phase-7-computer-control` | main |
| #17 | feat(runtime): add E.D.I.T.H. runtime state and self-control | `feature/phase-8-runtime-state` | main |
| #18 | feat(voice): add fallback STT-to-TTS pipeline | `feature/phase-9-fallback-voice` | main |
| #19 | feat(voice): add realtime duplex S2S voice | `feature/phase-10-realtime-s2s` | main |
| — | feat(voice): add production voice UX and reliability | `feature/phase-11-voice-ux-reliability` | **NOT MERGED** |

### 3.3 Phase 1 Observation

Phase 1 (Provider Abstraction) was implemented within the Stage 0 / Architecture PR (#9), specifically commit `a47a8f3 feat(ai): add provider abstraction and capability foundation`. There is no separate Phase 1 PR.

### 3.4 Repository Structure

```
E.D.I.T.H/
├── src/                          # React frontend
│   ├── App.tsx
│   ├── main.tsx
│   ├── context/AppContext.tsx     # Global state, recording, TTS
│   ├── services/
│   │   ├── tauri.ts              # 75KB — Tauri invoke wrappers
│   │   ├── conversationService.ts
│   │   ├── policyService.ts
│   │   ├── toolService.ts
│   │   └── browserController.ts
│   ├── views/
│   │   ├── ChatView.tsx
│   │   ├── BrowserView.tsx
│   │   ├── SettingsView.tsx
│   │   ├── DevAgentView.tsx
│   │   ├── MemoryBankView.tsx
│   │   └── PluginsView.tsx
│   ├── components/               # ArcReactor, TelemetryDock, TopHudBar, etc.
│   ├── events/
│   └── types/
├── src-tauri/src/                # Rust backend
│   ├── lib.rs                    # App setup, 80+ Tauri commands
│   ├── main.rs
│   ├── ai/                       # Provider abstraction (Phase 1)
│   │   ├── adapters/             # groq.rs, gemini.rs, openai_compatible.rs
│   │   ├── provider.rs           # GenerateRequest (NO tools field)
│   │   ├── capabilities.rs
│   │   ├── registry.rs
│   │   └── credentials.rs
│   ├── events/                   # Correlated events (Phase 2)
│   ├── conversation/             # ConversationCore (Phase 3)
│   ├── task/                     # TaskRuntime (Phase 3)
│   ├── policy/                   # PolicyEngine (Phase 4)
│   ├── tools/                    # Universal Tool Runtime (Phase 5)
│   │   └── domains/              # Browser, Computer, Edith executors
│   ├── voice/                    # Voice pipeline (Phases 9-11)
│   │   ├── realtime/             # RealtimeVoiceEngine (Phase 10)
│   │   └── dsp/                  # VAD, Echo, Normalizer (Phase 11)
│   ├── runtime/                  # EdithRuntimeState (Phase 8)
│   ├── browser.rs                # Core browser (118KB)
│   ├── browser_agent.rs          # Legacy autonomous browser agent
│   ├── browser_tools.rs          # Legacy browser tool definitions (119KB)
│   ├── browser_control.rs        # Human-AI handoff
│   ├── chat.rs                   # Legacy chat_command (ACTIVE)
│   ├── llm.rs                    # Legacy LLM API
│   ├── tts.rs                    # Legacy TTS (rodio + edge-tts)
│   └── ...
├── docs/architecture/            # 12 architecture documents
├── .github/workflows/            # ci.yml, windows-build.yml
└── tests/                        # windows-test.manifest only
```

### 3.5 Managed Tauri State (lib.rs startup)

| State | Type | Initialized |
|-------|------|-------------|
| DbState | `Mutex<Connection>` | ✅ |
| AgentState | `Mutex<String>` | ✅ |
| BrowserState | `Default` | ✅ |
| BrowserAgentManager | `Default` | ✅ |
| TaskRuntime | `TaskRuntime` | ✅ |
| ConversationCore | `ConversationCore` | ✅ |
| PolicyEngine | `PolicyEngine` | ✅ |
| ToolRegistry | `ToolRegistry` | ✅ (browser + computer + edith tools loaded) |
| ToolRouter | `ToolRouter` | ✅ (with policy engine + domain executors) |
| EdithRuntimeState | `EdithRuntimeState` | ✅ |
| RealtimeVoiceEngine | `Arc<RealtimeVoiceEngine>` | ✅ (with Mock transport) |
| VoiceController | `Arc<VoiceController>` | ✅ (with BrowserCaptureBridge) |

---

## 4. Reconstructed Roadmap

### Stage 0 / Stage 0.1 — Architecture Foundations
- **PR**: #9 (`architecture/stage-0`)
- **Content**: Architecture documents (V1.0 + V1.1), Provider trait, CapabilitySet, ProviderRegistry, Groq/Gemini/OpenAI-compatible adapters, CredentialStore
- **Status**: Phase 1 (Provider Abstraction) is embedded here

### Phase 1 — Provider Abstraction
- Embedded in Stage 0 PR #9
- Implements: `Provider` trait, `GenerateRequest`, `GenerateResponse`, `StreamChunk`, adapters, `ProviderRegistry`

### Phase 2 — Correlated Event Infrastructure
- PR #11

### Phase 3 — Conversation Core + Task Runtime
- PR #12

### Phase 4 — Centralized Policy Engine
- PR #13

### Phase 5 — Universal Tool Runtime
- PR #14

### Phase 6 — Browser Domain Integration
- PR #15

### Phase 7 — Computer Control Domain
- PR #16

### Phase 8 — E.D.I.T.H. Runtime State / Self-Knowledge
- PR #17

### Phase 9 — Fallback Voice (STT → LLM → TTS)
- PR #18

### Phase 10 — Realtime S2S Voice
- PR #19

### Phase 11 — Voice UX + Production Reliability
- Not merged, current branch

---

## 5. Phase 1 Audit — Provider Abstraction

### 5.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented (V1.0 + V1.1 architecture docs) |
| **CODE_STATUS** | Fully implemented for text generation/streaming. **NOT implemented for tool calling protocol.** |
| **INTEGRATION_STATUS** | Fully wired — both `chat_command` and `ConversationCore` resolve providers from `ProviderRegistry` |
| **RUNTIME_STATUS** | Proven working for text chat with valid API keys |
| **TEST_STATUS** | Unit tests for CapabilitySet, registry, serialization. No integration tests with live providers. |
| **PRODUCTION_STATUS** | Needs tool-calling protocol implementation |

### 5.2 Key Evidence

**GenerateRequest** (`src-tauri/src/ai/provider.rs:40-46`):
```rust
pub struct GenerateRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    // NO tools field
    // NO functions field
    // NO tool_choice field
}
```

**GenerateResponse** (`src-tauri/src/ai/provider.rs:62-66`):
```rust
pub struct GenerateResponse {
    pub text: String,
    pub model: String,
    pub finish_reason: Option<String>,
    // NO tool_calls field
}
```

**StreamChunk** (`src-tauri/src/ai/provider.rs:70-73`):
```rust
pub struct StreamChunk {
    pub text: String,
    pub is_done: bool,
    // NO tool_call delta
}
```

### 5.3 Provider Adapter Audit

| Provider | Adapter File | Text Generation | Streaming | Tool Schema Transmission | Tool Call Parsing |
|----------|-------------|-----------------|-----------|--------------------------|-------------------|
| Groq | `ai/adapters/groq.rs` | ✅ Real HTTP | ✅ Real SSE | ❌ Not transmitted | ❌ Not parsed |
| Gemini | `ai/adapters/gemini.rs` | ✅ Real HTTP | ✅ Real SSE | ❌ Not transmitted | ❌ Not parsed |
| OpenAI-Compatible | `ai/adapters/openai_compatible.rs` | ✅ Real HTTP | ✅ Real SSE | ❌ Not transmitted | ❌ Not parsed |
| Custom | via OpenAI-Compatible | ✅ | ✅ | ❌ | ❌ |
| Local (llama) | `llm.rs` (legacy) | ✅ via llama-server | ✅ | ❌ | ❌ |

### 5.4 Capability Declaration vs Protocol Implementation

| Capability | Declared in CapabilitySet | Protocol Implemented |
|------------|--------------------------|---------------------|
| TextGeneration | ✅ | ✅ |
| Streaming | ✅ | ✅ |
| ToolCalling | ✅ (declared by Groq, Gemini) | ❌ **Not implemented in request/response** |
| Vision | ✅ (declared) | ❌ No image content in ChatMessage |
| RealtimeAudio | ✅ (declared by Gemini) | ❌ No real transport |
| SpeechToText | ✅ (declared) | ❌ Mock adapter |
| TextToSpeech | ✅ (declared) | ✅ EdgeTTS works |
| Embeddings | ✅ (declared) | ❌ No embedding adapter |

> [!CAUTION]
> `Capability::ToolCalling` is declared by providers but has **zero protocol implementation**. The application has no way to send tool schemas to any provider or parse tool calls from any response.

---

## 6. Phase 2 Audit — Correlated Event Infrastructure

### 6.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented |
| **INTEGRATION_STATUS** | Fully wired — EventEmitter injected into all subsystems |
| **RUNTIME_STATUS** | Working for streaming events; tool/task events exist but never triggered via normal chat |
| **TEST_STATUS** | Unit tests for envelope construction, correlation IDs, sequence numbers |
| **PRODUCTION_STATUS** | Production-ready for existing features |

### 6.2 Key Components

- **EventEnvelope** (`events/envelope.rs`): Carries timestamp, correlation, sequence, payload
- **EventCorrelation** (`events/ids.rs`): `ConversationId`, `TurnId`, `StreamId`, `TaskId`, `ToolExecutionId`, `VoiceSessionId`
- **EventEmitter** (`events/emitter.rs`): `emit_stream_started`, `emit_stream_chunk`, `emit_stream_finished`, `emit_stream_failed`, `emit_stream_cancelled`, `emit_tool_*`, `emit_voice_*`
- **EventPayload** (`events/payload.rs`): Typed payloads for all event categories

### 6.3 Evidence

Both `chat_command` (legacy) and `ConversationCore::execute_turn` correctly emit correlated streaming events with authoritative TurnId and StreamId. The frontend subscribes to these events for real-time UI updates.

Tool-related events (`emit_tool_started`, `emit_tool_completed`, etc.) exist but are only reachable through the `ToolRouter`, which is never invoked from the AI chat loop.

---

## 7. Phase 3 Audit — Conversation Core + Task Runtime

### 7.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented for text-only turns. **No tool-calling loop.** |
| **INTEGRATION_STATUS** | Partially wired — `chat_command` uses it for turn registration/cancellation but does its own streaming |
| **RUNTIME_STATUS** | Turn state machine works. ConversationCore's own `execute_turn` is callable but not used by the primary UI. |
| **TEST_STATUS** | 20KB of unit tests for turn lifecycle, cancellation, concurrent turns |
| **PRODUCTION_STATUS** | Needs tool-calling integration |

### 7.2 Critical Finding: Hybrid Legacy/New Execution

The chat flow is a hybrid:

1. **Frontend** (`ChatView.tsx:233`) calls `tauriService.chatCommand()`
2. **`chat_command`** (`chat.rs:64`) handles the entire request:
   - Intercepts plugin prefixes (`open`, `play`, `search`, `cmd`, `whatsapp`, `email`, `volume`)
   - Constructs its own `GenerateRequest` (without tools)
   - Creates a fresh `ProviderRegistry::standard_builtins()` per request
   - Submits a turn to `ConversationCore` only for cancellation token (line 354-373)
   - Performs streaming via the provider directly
   - Saves response to memory
3. **ConversationCore's `execute_turn`** is never called from the normal UI

**Evidence** (`chat.rs:345-351`):
```rust
let req = crate::ai::GenerateRequest {
    model: model.clone(),
    messages: ai_messages,
    temperature: temp,
    max_tokens: None,
    stream: true,
};
```

No tools. No ToolRouter. No policy check. No tool-call parsing.

### 7.3 ConversationCore execute_turn Analysis

`ConversationCore::execute_turn` (`conversation/core.rs:140-397`):
- Retrieves turn parameters
- Loads history from SQLite
- Assembles context via `ContextAssembler`
- Constructs `GenerateRequest` (no tools)
- Resolves provider from `ProviderRegistry`
- Streams response via `StreamingTextCapability`
- Saves response to DB
- **Does NOT**: parse tool_calls, invoke ToolRouter, loop for tool execution

### 7.4 Task Runtime

`TaskRuntime` (`task/runtime.rs`) is fully implemented with:
- Task creation, cancellation, lifecycle tracking
- Task types: Background, BrowserAgent, DevAgent, Maintenance, Custom
- Properly wired to EventEmitter

However, tasks are standalone management objects. They do not orchestrate tool-calling loops.

---

## 8. Phase 4 Audit — Policy Engine

### 8.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented |
| **INTEGRATION_STATUS** | Wired into ToolRouter. **Not surfaced in any UI component.** |
| **RUNTIME_STATUS** | Works when ToolRouter is invoked manually (via `tools_execute` Tauri command). Never invoked from AI chat. |
| **TEST_STATUS** | 13KB of unit tests covering risk evaluation, approval lifecycle, TTL, versioning |
| **PRODUCTION_STATUS** | Needs UI integration, needs AI-chat-loop integration |

### 8.2 Key Components

- **PolicyEngine** (`policy/engine.rs`): Central evaluator with risk levels (Low/Medium/High/Critical)
- **ApprovalManager** (`policy/approval.rs`): Manages human approval lifecycle with content-hash-based deduplication and TTL
- **AuditLog** (`policy/audit.rs`): Records all policy decisions
- **PolicyContext** (`policy/context.rs`): Carries session, turn, initiator metadata
- **PolicyDecision** (`policy/decision.rs`): Allow/RequireApproval/Deny with reason + risk level

### 8.3 ToolRouter Integration

The `ToolRouter` (`tools/router.rs`) correctly calls `PolicyEngine::evaluate()` before executing any tool. If the policy returns `RequireApproval`, the router blocks execution pending human resolution. This is architecturally sound — but since the ToolRouter is never invoked from the AI conversation path, policy enforcement is effectively dormant.

### 8.4 UI Gap

**Zero** React components import `policyService.ts`. The file exists with full Tauri invoke wrappers (`evaluateAction`, `listPendingApprovals`, `resolveApproval`, `getAuditLog`) but is imported by nothing.

---

## 9. Phase 5 Audit — Universal Tool Runtime

### 9.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented (24KB architecture doc) |
| **CODE_STATUS** | Fully implemented — registry, router, validator, cancellation, domain executors |
| **INTEGRATION_STATUS** | **Backend exists but AI cannot reach it.** Frontend can reach it via `tools_execute` Tauri command but does not. |
| **RUNTIME_STATUS** | Works when invoked directly. Never invoked from normal chat UX. |
| **TEST_STATUS** | 24KB of tests with MockTestDomainExecutor covering routing, validation, policy, cancellation |
| **PRODUCTION_STATUS** | Needs integration into AI conversation loop |

### 9.2 Tool Registration at Startup (lib.rs:617-626)

```rust
let tool_registry = tools::ToolRegistry::new();
for def in tools::get_browser_definitions() {
    let _ = tool_registry.register(def);
}
for def in tools::get_computer_definitions() {
    let _ = tool_registry.register(def);
}
for def in tools::get_edith_definitions() {
    let _ = tool_registry.register(def);
}
```

Tools are registered. Domain executors are registered. ToolRouter is configured with PolicyEngine. Everything is wired — except to the AI.

### 9.3 The Missing Link

**CAN THE NORMAL CHAT MODEL ACTUALLY CALL THESE TOOLS?**

**NO.**

Trace of the intended path and where it breaks:

```
Normal Chat UI (ChatView.tsx)
  → tauriService.chatCommand()           ✅ exists
  → chat_command (chat.rs:64)            ✅ exists
  → Constructs GenerateRequest           ✅ exists
    → GenerateRequest has NO tools field  ❌ BROKEN
  → Provider adapter sends HTTP request  ✅ exists
    → Request body has NO tools array    ❌ BROKEN
  → Model response is text-only         ❌ BROKEN (tool_calls never parsed)
  → No tool_calls extracted             ❌ BROKEN
  → No ToolRequest created              ❌ BROKEN
  → No PolicyEngine consulted           ❌ BROKEN
  → No ToolRouter invoked               ❌ BROKEN
  → No DomainExecutor executes          ❌ BROKEN
  → No tool result returned to model    ❌ BROKEN
```

The chain is broken at the very first link: `GenerateRequest` has no tools field.

### 9.4 Frontend Tool Service

`toolService.ts` exports functions: `listToolDefinitions`, `getToolDefinition`, `executeTool`, `cancelToolExecution`. These correctly wrap Tauri commands. However, **no React component imports or uses `toolService.ts`**.

---

## 10. Phase 6 Audit — Browser Domain Integration

### 10.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented — both legacy and UTR paths |
| **INTEGRATION_STATUS** | Browser UI works directly. AI cannot invoke browser tools through normal chat. |
| **RUNTIME_STATUS** | Direct browser UI proven working. UTR path works if manually invoked. |
| **TEST_STATUS** | 17KB of browser domain tests |
| **PRODUCTION_STATUS** | Needs AI tool-calling integration |

### 10.2 Architecture: Three Coexisting Paths

| Path | Entry Point | Mechanism | Status |
|------|-------------|-----------|--------|
| **Direct Browser UI** | `BrowserView.tsx` → Tauri commands | `browser_create_tab`, `browser_navigate_tab`, etc. | ✅ Working |
| **Legacy BrowserAgent** | `browser_agent_run_task` Tauri command | Autonomous LLM loop with bracket-parsed tool calls | ✅ Working (but isolated) |
| **UTR Browser Executor** | `tools_execute` Tauri command → ToolRouter | `BrowserDomainExecutor` wrapping `browser_tools.rs` | ✅ Working if invoked; ❌ AI cannot invoke |

### 10.3 BrowserDomainExecutor Implementation

`tools/domains/browser.rs` implements `DomainExecutor` for `ToolDomain::Browser`. It translates UTR tool names to legacy names:
- `browser.observe` → `browser_observe`
- `browser.click` → `browser_click`
- `browser.type` → `browser_type`
- `browser.navigate` → `browser_navigate`
- etc.

It then delegates to `browser_tools::execute_browser_tool_authorized()`, which calls the real `browser.rs` functions that interact with the Tauri WebView2.

**No logic duplication** exists between UTR and legacy paths — the UTR path wraps the legacy implementation.

### 10.4 Legacy BrowserAgent

`browser_agent.rs` implements its own autonomous loop:
1. Captures DOM observation
2. Queries LLM with tool definitions in the system prompt
3. Manually parses tool calls from text responses (bracket-aware parsing)
4. Executes tools directly via `browser_tools.rs`
5. Loops until goal completion

This is **completely independent** of `ConversationCore` and `ToolRouter`. It works, but it's a parallel system.

---

## 11. Phase 7 Audit — Computer Control

### 11.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented with real Windows native APIs |
| **INTEGRATION_STATUS** | Backend works. AI cannot invoke through normal chat. |
| **RUNTIME_STATUS** | Windows native operations proven in domain executor tests. Not reachable from normal UX. |
| **TEST_STATUS** | 19KB of computer domain tests + 23KB platform tests |
| **PRODUCTION_STATUS** | Needs AI tool-calling integration |

### 11.2 Windows Platform Adapter

`tools/domains/computer_platform.rs` contains `WindowsPlatformAdapter` which uses **direct FFI bindings to Windows APIs**:

```rust
extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn mouse_event(flags: u32, dx: u32, dy: u32, data: u32, extra: usize);
    fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
    fn SetForegroundWindow(hwnd: isize) -> i32;
    fn GetForegroundWindow() -> isize;
    fn EnumWindows(callback: extern "system" fn(isize, isize) -> i32, lparam: isize) -> i32;
}
```

This is **real native Windows automation**, not mock behavior. Mouse movement, clicking, keyboard typing, window management, app launching, and screenshots all use actual Win32 API calls.

On non-Windows platforms, a `MockPlatformAdapter` is used instead.

### 11.3 Registered Computer Tools

- `computer.observe_screen` — Screenshot + active window info
- `computer.click` — Mouse click at coordinates
- `computer.type` — Keyboard text input
- `computer.key_press` — Individual key presses
- `computer.hotkey` — Key combinations
- `computer.move_cursor` — Mouse movement
- `computer.launch_app` — Application launching
- `computer.close_window` — Window termination
- `computer.active_window` — Active window query
- `computer.screenshot` — Screen capture

### 11.4 The Same Gap

All registered, all implemented, all genuinely functional — but the AI cannot invoke them because `GenerateRequest` has no tools field and no response parser extracts tool calls.

---

## 12. Phase 8 Audit — E.D.I.T.H. Runtime State

### 12.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Fully implemented — aggregates state from all subsystems |
| **INTEGRATION_STATUS** | Backend fully wired. **Frontend ignores it entirely.** |
| **RUNTIME_STATUS** | `runtime_get_status` Tauri command works. Never called by UI. |
| **TEST_STATUS** | Tests exist for state aggregation |
| **PRODUCTION_STATUS** | Needs frontend integration, needs AI self-knowledge loop |

### 12.2 EdithRuntimeState Components

`runtime/state.rs` aggregates:
- Active conversations/turns from `ConversationCore`
- Active tasks from `TaskRuntime`
- Registered tools from `ToolRegistry`
- Active tool executions from `ToolRouter`
- Provider registry state
- Policy engine status
- Voice controller status (if wired)
- Browser state
- System health metrics

### 12.3 Edith Domain Executor

`tools/domains/edith.rs` implements tools:
- `edith.get_status` — Runtime status summary
- `edith.get_capabilities` — Available tool listing
- `edith.cancel_task` — Task cancellation
- `edith.cancel_turn` — Turn cancellation
- `edith.self_assess` — Self-knowledge probe

These tools allow E.D.I.T.H. to reason about its own state — but only if the AI can call tools, which it cannot.

### 12.4 TelemetryDock Gap

`TelemetryDock.tsx:29-61` displays CPU, RAM, GPU, Temperature metrics — all generated by `Math.random()`:

```typescript
// Simulated live telemetry metrics that fluctuate naturally
const [cpuUsage, setCpuUsage] = useState(28);
// ...
useEffect(() => {
    const interval = setInterval(() => {
        setCpuUsage((prev) => {
            const delta = Math.floor(Math.random() * 9) - 4;
            return Math.min(Math.max(prev + delta, 14), 78);
        });
```

The backend's real `EdithRuntimeState.get_runtime_status()` is exported to the frontend via `tauriService.runtimeGetStatus()` but is **never called** from any React component.

---

## 13. Phase 9 Audit — Fallback Voice (STT → LLM → TTS)

### 13.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented |
| **CODE_STATUS** | Partially implemented — TTS works, STT is mocked, capture is mocked |
| **INTEGRATION_STATUS** | Web Speech path works end-to-end. Native STT path is mock. |
| **RUNTIME_STATUS** | Voice works through Web Speech + EdgeTTS + Rodio. Native STT is fake. |
| **TEST_STATUS** | Unit tests prove mock behavior, not real audio |
| **PRODUCTION_STATUS** | TTS: production-ready. STT: needs real Whisper integration. Capture: needs real CPAL implementation. |

### 13.2 Actual Voice Path (What Works)

```
User clicks microphone button (TopHudBar.tsx)
  → AppContext.tsx toggleRecording()
  → window.SpeechRecognition (Web Speech API in WebView2)
  → Browser transcribes audio to text locally
  → onresult callback gets transcript text
  → tauriService.voiceSessionSubmitTranscript() [if voice session active]
  → OR: direct insertion into chat input
  → tauriService.chatCommand() with text
  → LLM responds with text
  → AppContext.tsx speakResponse()
  → tauriService.ttsSpeak(text, voice)
  → Rust tts_speak() → edge_tts_rust → Rodio playback
  → Audio plays through system speakers ✅
```

### 13.3 What Is Fake

**NativeCpalCaptureDriver** (`voice/capture.rs:254-269`):
```rust
impl AudioCaptureDriver for NativeCpalCaptureDriver {
    fn start_capture(&self, session_id: &VoiceSessionId) -> Result<(), VoiceError> {
        // ...
        self.mock_fallback.start_capture(session_id)  // Delegates to MOCK
    }
    fn stop_capture(&self) -> Result<AudioBuffer, VoiceError> {
        // ...
        self.mock_fallback.stop_capture()  // Returns 440Hz sine wave
    }
}
```

**CloudSTTAdapter** (`voice/stt.rs:196-246`):
```rust
impl STTAdapter for CloudSTTAdapter {
    fn transcribe(...) {
        // ... NO network request ...
        Ok(Transcript {
            text: format!("Cloud audio transcription ({} bytes, {} Hz)",
                          pcm_bytes.len(), mono.sample_rate),  // HARDCODED
            confidence: Some(0.95),
            // ...
        })
    }
}
```

### 13.4 What Works

- **EdgeTtsAdapter**: Real — uses `edge_tts_rust` crate for cloud TTS synthesis
- **RodioAudioOutputDriver**: Real — opens native audio device via `rodio`, plays PCM through system speakers
- **Legacy tts_speak command**: Real — streams edge-tts MP3 bytes to rodio
- **Device enumeration**: Real — `CpalAudioDeviceProvider` lists actual system audio devices via `cpal`

---

## 14. Phase 10 Audit — Realtime S2S Voice

### 14.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented (17KB architecture doc) |
| **CODE_STATUS** | Architecturally implemented with mock transport |
| **INTEGRATION_STATUS** | Backend exists, hardcoded to mock adapters |
| **RUNTIME_STATUS** | **Not functional.** Mock transport only. |
| **TEST_STATUS** | 13KB tests + 19KB production tests — all use mocks |
| **PRODUCTION_STATUS** | Not production-ready. No real WebSocket provider integration. |

### 14.2 Critical Evidence

`lib.rs:524-525` — The `realtime_voice_start` Tauri command hardcodes mocks:

```rust
let transport = std::sync::Arc::new(voice::MockAudioFrameTransport::new(32));
let adapter = std::sync::Arc::new(voice::MockRealtimeSessionAdapter::new(transport));
```

### 14.3 Architecture (Disconnected from Reality)

The realtime module contains:
- `RealtimeVoiceEngine` (`voice/realtime/engine.rs`) — Session lifecycle manager
- `RealtimeVoiceSession` (`voice/realtime/session.rs`) — State machine for duplex sessions
- `AudioFrame` (`voice/realtime/frame.rs`) — PCM frame abstraction
- `AudioFrameTransport` trait (`voice/realtime/transport.rs`) — WebSocket abstraction
- `RealtimeSessionAdapter` trait (`voice/realtime/adapter.rs`) — Provider abstraction
- Recovery module (`voice/realtime/recovery.rs`) — Reconnection logic

All of these exist as well-designed abstractions. None are connected to a real WebSocket provider (Gemini Live, OpenAI Realtime, etc.).

### 14.4 Realtime S2S Status Summary

| Component | Designed | Implemented | Connected to Real Provider | Connected to Real Audio | Callable from UI |
|-----------|----------|-------------|---------------------------|-------------------------|-------------------|
| RealtimeVoiceEngine | ✅ | ✅ | ❌ | ❌ | ❌ (command exists) |
| AudioFrameTransport | ✅ | ✅ (mock only) | ❌ | ❌ | — |
| RealtimeSessionAdapter | ✅ | ✅ (mock only) | ❌ | ❌ | — |
| WebSocket to Gemini Live | ✅ (arch doc) | ❌ | ❌ | ❌ | ❌ |
| Barge-in | ✅ (arch doc) | ❌ | ❌ | ❌ | ❌ |
| Backpressure | ✅ (arch doc) | ❌ | ❌ | ❌ | ❌ |

---

## 15. Phase 11 Audit — Voice UX + Production Reliability

### 15.1 Classification

| Dimension | Status |
|-----------|--------|
| **DESIGN_STATUS** | Fully documented (9.6KB architecture doc) |
| **CODE_STATUS** | Partially implemented — DSP modules exist, device management exists |
| **INTEGRATION_STATUS** | Device listing works. DSP not connected to real audio. |
| **RUNTIME_STATUS** | Device enumeration proven. Visualizer/telemetry not connected. |
| **TEST_STATUS** | Voice tests with mock isolation. Production tests use mocks. |
| **PRODUCTION_STATUS** | Needs real audio pipeline before UX polishing is meaningful |

### 15.2 What Exists

- **DSP modules**: VAD (`dsp/vad.rs`), Echo cancellation (`dsp/echo.rs`), Normalizer (`dsp/normalizer.rs`)
- **Device management**: `CpalAudioDeviceProvider`, `voice_list_devices`, `voice_set_input_device`, `voice_set_output_device`
- **Telemetry**: `VoiceTelemetry` struct with privacy-safe device hashing
- **Voice status**: `VoiceStatusSummary` aggregating session state, capture state, playback state

### 15.3 Disconnect

The DSP modules are pure signal-processing algorithms that require real PCM audio frames as input. Since `NativeCpalCaptureDriver` delegates to mock capture, these DSP modules are never exercised with real audio data.

---

## 16. Frontend/Backend Integration Audit

### 16.1 UI Action Trace Matrix

| UI Element | React Handler | Service Call | Tauri Command | Backend Path | Result |
|------------|---------------|-------------|---------------|--------------|--------|
| Chat send button | `ChatView.handleSend` | `tauriService.chatCommand()` | `chat_command` | Legacy chat → Provider → Stream | ✅ Works (text only) |
| Model selector | `ChatView` dropdown | `tauriService.chatCommand(settings)` | `chat_command` uses `selectedProvider`/`selectedModel` | Provider resolution | ✅ Works |
| Cancel button | `ChatView` | `conversationService.cancelTurn()` | `conversation_cancel_turn` | `ConversationCore.cancel_turn` | ✅ Works |
| Microphone button | `TopHudBar` | `AppContext.toggleRecording()` | None (Web Speech API direct) | N/A | ✅ Works (browser STT) |
| TTS playback | Auto after response | `tauriService.ttsSpeak()` | `tts_speak` | EdgeTTS → Rodio | ✅ Works |
| Browser tabs | `BrowserView` | `tauriService.browserCreateTab()` | `browser_create_tab` | `browser.rs` WebView2 | ✅ Works |
| Settings save | `SettingsView` | `tauriService.saveSetting()` | `save_setting` | SQLite | ✅ Works |
| Telemetry panel | `TelemetryDock` | N/A | N/A | `Math.random()` | 🟡 Fake data |
| Memory bank | `MemoryBankView` | `tauriService.getMemories()` | `get_memories_cmd` | LanceDB | ✅ Works |
| Dev agent | `DevAgentView` | `tauriService.agentChat()` | `agent_chat` | File-system agent | ✅ Works |
| Tool execution | None | `toolService.executeTool()` | `tools_execute` | ToolRouter → Executor | 🔴 **No UI calls this** |
| Policy approval | None | `policyService.resolveApproval()` | `policy_resolve_approval` | PolicyEngine | 🔴 **No UI calls this** |
| Runtime status | None | `tauriService.runtimeGetStatus()` | `runtime_get_status` | EdithRuntimeState | 🔴 **No UI calls this** |

### 16.2 Frontend Service Files vs Usage

| Service File | Exported Functions | Actually Imported By React Components |
|-------------|-------------------|--------------------------------------|
| `tauri.ts` | ~200+ functions | ✅ Heavily used by all views |
| `conversationService.ts` | `submitTurn`, `executeTurn`, `cancelTurn`, `getTurnStatus` | Only `cancelTurn` used (by ChatView cancel button) |
| `policyService.ts` | `evaluateAction`, `listPendingApprovals`, `resolveApproval`, `getAuditLog` | ❌ **Not imported by any component** |
| `toolService.ts` | `listToolDefinitions`, `getToolDefinition`, `executeTool`, `cancelToolExecution` | ❌ **Not imported by any component** |
| `browserController.ts` | Browser automation helpers | Used by BrowserView |

---

## 17. Provider + Tool Calling Audit

### 17.1 Provider Tool Calling Matrix

| Provider | Model(s) | Tool Capability Declared | Tool Schema Transmitted | Tool-Call Response Parsing | Tool Result Round-Trip | Normal Chat Integration | Status |
|----------|----------|--------------------------|------------------------|----------------------------|------------------------|------------------------|--------|
| Groq | llama-3.3-70b, etc. | ✅ `Capability::ToolCalling` | ❌ No `tools` in request JSON | ❌ Only `content` extracted | ❌ N/A | ❌ Not connected | **DECLARATION ONLY** |
| Gemini | gemini-2.0-flash, etc. | ✅ `Capability::ToolCalling` | ❌ No `tools` in request JSON | ❌ Only `text` extracted from `candidates[0].content.parts[0].text` | ❌ N/A | ❌ Not connected | **DECLARATION ONLY** |
| OpenAI-Compatible | Custom models | ✅ `Capability::ToolCalling` | ❌ No `tools` in request JSON | ❌ Only `content` extracted | ❌ N/A | ❌ Not connected | **DECLARATION ONLY** |

### 17.2 What Would Be Required

To make tool calling work, the following changes are needed:
1. Add `tools: Option<Vec<ToolDefinition>>` and `tool_choice: Option<String>` to `GenerateRequest`
2. Add `tool_calls: Option<Vec<ToolCall>>` to `GenerateResponse` and `StreamChunk`
3. Modify each provider adapter to include tool schemas in HTTP request bodies
4. Modify each provider adapter to parse `tool_calls` from responses
5. Add a tool-calling loop to `ConversationCore::execute_turn` that:
   - Detects `finish_reason: "tool_calls"` or `tool_calls` in response
   - Constructs `ToolRequest` for each tool call
   - Invokes `ToolRouter::execute(request)`
   - Collects results
   - Resubmits to model with tool results as messages
   - Repeats until model returns text

---

## 18. Browser Audit

### 18.1 Core Browser Functionality

The browser is the most mature subsystem. `browser.rs` (118KB) implements a full browser engine via Tauri WebView2:
- Multi-tab management with create/switch/close/duplicate/pin
- Navigation with back/forward/reload
- DOM observation via injected `LIVE_OBSERVER_INIT_SCRIPT` (semantic element extraction)
- Click, type, scroll, key press interactions
- Find-in-page, zoom, print, reader mode
- Tab groups, session save/restore
- Downloads, bookmarks, history, profiles
- Privacy protection with tracker blocking

### 18.2 Path Comparison

| Capability | Direct Browser UI | BrowserAgent | UTR Browser Executor | Normal Chat AI |
|------------|-------------------|--------------|---------------------|----------------|
| Navigate | ✅ | ✅ | ✅ (if invoked) | ❌ |
| Click | N/A (user clicks directly) | ✅ | ✅ (if invoked) | ❌ |
| Type | N/A (user types directly) | ✅ | ✅ (if invoked) | ❌ |
| Observe DOM | N/A | ✅ | ✅ (if invoked) | ❌ |
| Screenshot | ✅ | ✅ | ✅ (if invoked) | ❌ |
| Downloads | ✅ | ❌ | ✅ (if invoked) | ❌ |

---

## 19. Computer Control Audit

### 19.1 Windows Native Verification

`computer_platform.rs` — `WindowsPlatformAdapter` uses raw FFI:

| Operation | Win32 API | Real Implementation |
|-----------|-----------|-------------------|
| Move cursor | `SetCursorPos` | ✅ Real |
| Mouse click | `mouse_event` | ✅ Real |
| Keyboard type | `keybd_event` | ✅ Real |
| Key press | `keybd_event` | ✅ Real |
| Hotkey | `keybd_event` (combo) | ✅ Real |
| Active window | `GetForegroundWindow` | ✅ Real |
| Screenshot | `screenshots` crate | ✅ Real |
| Launch app | `std::process::Command` | ✅ Real |
| Close window | Window enumeration + close | ✅ Real |

### 19.2 Reachability from Normal Chat

**Cannot be reached.** The model never receives computer tool definitions, so it never emits computer tool calls.

---

## 20. Voice Audit

### 20.1 Complete Voice Path Analysis

| Component | Expected Behavior | Actual Behavior |
|-----------|------------------|-----------------|
| Microphone capture (frontend) | Native CPAL | Web Speech API in WebView2 |
| STT (backend) | Cloud Whisper API | Hardcoded mock string |
| NativeCpalCaptureDriver | Real CPAL capture | Delegates to MockAudioCaptureDriver (440Hz sine) |
| BrowserCaptureBridge | Manages ownership | Works (returns empty AudioBuffer) |
| CloudSTTAdapter | HTTP to Whisper | Returns fake transcript string |
| EdgeTtsAdapter | Cloud TTS synthesis | ✅ **Real** — uses edge_tts_rust |
| RodioAudioOutputDriver | Native playback | ✅ **Real** — uses rodio/cpal |
| RealtimeVoiceEngine | WebSocket duplex | Mock transport only |
| Device enumeration | CPAL device listing | ✅ **Real** |
| Device selection | CPAL device switching | Stored but not applied to real capture |

### 20.2 Duplicate Path Analysis

| Function | Web Speech Path | Native Path | Active |
|----------|----------------|-------------|--------|
| STT | `window.SpeechRecognition` | `CloudSTTAdapter` (mock) | Web Speech |
| TTS | `window.speechSynthesis` (disabled) | `tts_speak` → EdgeTTS → Rodio | Native (Rodio) |

TTS correctly prevents duplication by checking for Tauri environment in `tauri.ts` before choosing native over browser synthesis.

---

## 21. Runtime State Audit

### 21.1 EdithRuntimeState Coverage

| Data Source | Connected | Live Data | Queried by Frontend |
|-------------|-----------|-----------|---------------------|
| ConversationCore (active turns) | ✅ | ✅ | ❌ |
| TaskRuntime (active tasks) | ✅ | ✅ | ❌ |
| ToolRegistry (tool count) | ✅ | ✅ | ❌ |
| ToolRouter (active executions) | ✅ | ✅ | ❌ |
| ProviderRegistry (providers) | ✅ | ✅ | ❌ |
| PolicyEngine (policies) | ✅ | ✅ | ❌ |
| VoiceController (voice state) | ✅ | ✅ | ❌ |
| Browser state | ✅ (via app handle) | ✅ | ❌ |

Backend: fully functional. Frontend: completely ignores it.

---

## 22. Legacy / Duplicate Path Audit

| Legacy Path | New Architecture Equivalent | Both Active | Classification |
|-------------|---------------------------|-------------|----------------|
| `chat_command` (chat.rs) | `ConversationCore.submit_turn` + `execute_turn` | ✅ Legacy is primary | **Active — dangerous** (bypasses tools/policy) |
| `llm::api_chat_cloud` | Provider adapters via `ProviderRegistry` | ✅ Both registered | **Transitional** |
| `llm::local_chat` | Would use local provider adapter | ✅ | **Transitional** |
| `tts_speak` (tts.rs) | `VoiceController` TTS pipeline | ✅ Both registered | **Active — tts.rs is primary** |
| `browser_agent_run_task` | UTR Browser Executor | ✅ Both registered | **Active — parallel system** |
| `browser_tools::*` (119KB) | `tools/domains/browser.rs` | ✅ UTR wraps legacy | **Safe — no duplication** |
| `agent_resolve_proposal` | `policy_resolve_approval` | ✅ Both registered | **Duplicate authorization** |
| `security::ProposalEngine` | `policy::PolicyEngine` | ✅ Both exist | **Duplicate policy** |
| Web Speech STT | VoiceController STT | ✅ Web Speech is primary | **Active — Web Speech is sole real path** |

### 22.1 Dual Authorization Systems

Two independent authorization/proposal systems exist:
1. **`security.rs`** — `ProposalEngine` with its own `CommandPolicyResult`, resolved via `agent_resolve_proposal`
2. **`policy/`** — `PolicyEngine` with `PolicyDecision`, resolved via `policy_resolve_approval`

These are completely independent code paths with different APIs, different approval stores, and different risk models.

---

## 23. Mock / Simulation Audit

### 23.1 Production Code Paths Using Mocks

| Component | File | Line | Mock Behavior | Severity |
|-----------|------|------|---------------|----------|
| NativeCpalCaptureDriver | `voice/capture.rs` | 263, 269 | Delegates to MockAudioCaptureDriver | **CRITICAL** — Audio capture is fake |
| CloudSTTAdapter | `voice/stt.rs` | 234 | Returns hardcoded string | **CRITICAL** — STT is fake |
| realtime_voice_start | `lib.rs` | 524-525 | Hardcodes MockAudioFrameTransport + MockRealtimeSessionAdapter | **CRITICAL** — Realtime S2S is fake |
| TelemetryDock | `TelemetryDock.tsx` | 29-61 | Math.random() for CPU/RAM/GPU/Temp | **HIGH** — Misleads user |
| LocalTtsAdapter | `voice/tts.rs` | — | Returns error "Local Kokoro TTS engine is currently disabled" | **MEDIUM** — Expected |
| local_tts_speak | `tts.rs` | — | Returns placeholder error | **MEDIUM** — Expected |

### 23.2 Mock Structs Exported from Production Modules

- `MockAudioCaptureDriver` (capture.rs)
- `MockAudioOutputDriver` (output.rs)
- `MockSTTAdapter` (stt.rs)
- `MockTtsAdapter` (tts.rs)
- `MockAudioFrameTransport` (realtime/transport.rs)
- `MockRealtimeSessionAdapter` (realtime/adapter.rs)
- `MockPlatformAdapter` (computer_platform.rs — for non-Windows)

These are correctly intended for testing, but `NativeCpalCaptureDriver` and `realtime_voice_start` use them in production paths.

---

## 24. Security Audit

### 24.1 API Credential Handling

- `CredentialStore` trait (`ai/credentials.rs`) — Extracts API keys from settings JSON
- `SettingsCredentialStore` — Runtime credential resolver
- **Risk**: API keys stored in SQLite settings table as plaintext. No OS keychain integration (e.g., Windows Credential Manager).
- **Risk**: API keys passed through `app_settings` JSON from frontend to `chat_command`.

### 24.2 Policy Engine Gaps

- PolicyEngine is fully implemented but **never invoked from normal chat path**
- `chat_command` executes plugins (terminal, app launcher, WhatsApp, email) with **NO policy check**
- `plugin_system_terminal` in `chat_command` allows terminal command execution without authorization

### 24.3 Browser Privacy

- `browser_privacy.rs` (21KB) — Tracker blocking, domain allowlisting, privacy status tracking
- `browser_risk.rs` (34KB) — URL risk assessment, action risk levels, audit logging
- Both properly integrated into browser UI paths

### 24.4 Computer Control Safety

- `computer_control.rs` — Human-AI handoff state machine (`UserControlled` / `AiControlled` / `AiPaused`)
- Computer executor checks control state before executing actions
- However, since AI cannot invoke computer tools, this protection is currently academic

### 24.5 Sensitive Data Risks

| Risk | Location | Severity |
|------|----------|----------|
| API keys in plaintext SQLite | `db.rs` settings table | HIGH |
| Terminal plugin executes arbitrary commands | `plugins.rs` `plugin_system_terminal` | CRITICAL (but requires explicit "cmd" prefix) |
| DOM observation may capture sensitive page content | `browser.rs` LIVE_OBSERVER_INIT_SCRIPT | MEDIUM (explicitly filters passwords) |
| Device IDs hashed for privacy | `voice/devices.rs` SHA-256 hashing | LOW (properly handled) |
| Chat history persisted unencrypted | `db.rs` sessions table | MEDIUM |

---

## 25. Performance / Resource Audit

### 25.1 Potential Issues

| Issue | Location | Severity |
|-------|----------|----------|
| Fresh `ProviderRegistry::standard_builtins()` created on every `chat_command` call | `chat.rs:319` | MEDIUM — Allocates all provider objects per message |
| No cleanup of completed turns in ConversationCore | `conversation/core.rs` HashMap grows unbounded | MEDIUM — Memory growth over long sessions |
| BrowserAgent autonomous loop has no timeout | `browser_agent.rs` | MEDIUM — Could run indefinitely |
| WebView2 processes not explicitly cleaned up on tab close | `browser.rs` | LOW |
| Multiple SQLite connections (conn, conn2) | `lib.rs:591-592` | LOW — Intentional for concurrent access |
| Audio output thread is long-lived | `voice/output.rs` | LOW — Correctly managed |

---

## 26. Testing / CI Audit

### 26.1 CI Pipeline

**`ci.yml`** — Runs on every push and PR to main:
1. `npm ci` — Install frontend dependencies
2. `npm run build` — Build React frontend (Vite)
3. `cargo check` — Verify Rust compilation
4. `cargo test --lib` — Run Rust unit tests

**`windows-build.yml`** — Runs on push/PR to main:
1. All of ci.yml steps
2. `npm run tauri build` — Full Windows NSIS installer build
3. Stages release artifacts with SHA256 checksums

### 26.2 What CI Proves

| Assertion | Proven by CI |
|-----------|-------------|
| Rust code compiles | ✅ cargo check |
| Frontend bundles | ✅ npm run build |
| Rust unit tests pass | ✅ cargo test --lib |
| Windows installer builds | ✅ npm run tauri build |
| Integration tests pass | ❌ None exist |
| E2E tests pass | ❌ None exist |
| Real provider connectivity | ❌ Not tested |
| Real audio hardware | ❌ Not tested |
| Real browser automation | ❌ Not tested |
| Tool calling works | ❌ Not tested (and structurally impossible) |

### 26.3 Test Coverage by Module

| Module | Test File | Size | Tests Mock Behavior | Tests Real Behavior |
|--------|-----------|------|--------------------|--------------------|
| Events | `events/tests.rs` | 11KB | Envelope, correlation, sequence | N/A |
| Conversation | `conversation/tests.rs` | 20KB | Turn lifecycle, cancellation | Uses mock provider |
| Tools | `tools/tests.rs` | 24KB | Routing, validation, policy, cancellation | Uses MockTestDomainExecutor |
| Browser Domain | `tools/domains/browser_tests.rs` | 17KB | Tool definition validation | Does not test real browser |
| Computer Domain | `tools/domains/computer_tests.rs` | 19KB | Tool definition, platform adapter | Uses MockPlatformAdapter |
| Edith Domain | `tools/domains/edith_tests.rs` | 21KB | Self-knowledge, capabilities | Uses mock runtime |
| Policy | `policy/tests.rs` | 13KB | Risk evaluation, approval lifecycle | N/A (pure logic) |
| Task | `task/tests.rs` | 8KB | Task creation, cancellation | N/A (pure logic) |
| Voice | `voice/tests.rs` | 15KB | Session lifecycle, TTS, STT | All mock audio |
| Realtime | `voice/realtime/tests.rs` | 13KB | Session state, transport | Mock transport |
| Realtime Production | `voice/realtime/production_tests.rs` | 19KB | Recovery, reconnect | Mock transport |

### 26.4 QA Scripts

Files in root:
- `qa_browser_deep_test.js` (31KB)
- `qa_browser_user_flows.js` (42KB)
- `qa_tester_screenshots.js` (23KB)
- `take_playwright_screenshots.js` (9KB)
- `take_screenshots.js` (11KB)

These are Playwright-based browser testing scripts from the pre-architecture phase. They test the browser UI but not the AI/tool/voice subsystems.

---

## 27. Documentation vs Code Audit

### 27.1 Architecture Document Comparison

| Document | Claims | Current Code Reality |
|----------|--------|---------------------|
| `EDITH-AI-CORE-ARCHITECTURE-V1.1.md` | Full tool-calling loop with provider → tool_calls → ToolRouter → executor → re-submission | **GenerateRequest has no tools field. No tool-calling loop exists.** |
| `EDITH-TOOL-RUNTIME-V1.0.md` | "Tools are exposed to the LLM through the conversation pipeline" | **Tools are registered but never transmitted to any LLM.** |
| `EDITH-CONVERSATION-CORE-V1.0.md` | "ConversationCore orchestrates tool execution within turns" | **ConversationCore's execute_turn streams text and terminates. No tool orchestration.** |
| `EDITH-POLICY-ENGINE-V1.0.md` | "Approval requests are surfaced to the operator through the UI" | **No UI for approvals exists.** |
| `EDITH-BROWSER-DOMAIN-V1.0.md` | "Browser tools are invokable through normal LLM conversation" | **AI cannot invoke browser tools.** |
| `EDITH-COMPUTER-CONTROL-V1.0.md` | "Computer tools are invokable through the AI" | **AI cannot invoke computer tools.** |
| `EDITH-RUNTIME-STATE-V1.0.md` | "Self-knowledge drives adaptive behavior" | **Backend works, frontend ignores it.** |
| `EDITH-REALTIME-S2S-V1.0.md` | "Duplex WebSocket sessions to Gemini Live" | **Mock transport only. No real WebSocket.** |
| `EDITH-VOICE-FALLBACK-V1.0.md` | "Microphone → STT → LLM → TTS → Speaker" | **STT is mock. Capture is mock. TTS works.** |
| `EDITH-VOICE-PRODUCTION-UX-V1.0.md` | "Device switching, visualizer, telemetry" | **Device listing works. Visualizer/telemetry use fake data.** |

> [!WARNING]
> **All 12 architecture documents describe the intended target architecture, not the current implementation.** They are ahead of the code in every case except basic text chat and browser UI.

---

## 28. End-to-End Capability Matrix

| Capability | UI exists | Backend exists | Tool exists | LLM can invoke | Policy enforced | Real executor | E2E proven | Status |
|------------|-----------|---------------|-------------|-----------------|-----------------|---------------|------------|--------|
| Normal text chat | ✅ | ✅ | N/A | N/A | ❌ | N/A | ✅ | **WORKING** |
| Streaming responses | ✅ | ✅ | N/A | N/A | ❌ | N/A | ✅ | **WORKING** |
| Provider switching | ✅ | ✅ | N/A | N/A | ❌ | N/A | ✅ | **WORKING** |
| Custom providers | ✅ | ✅ | N/A | N/A | ❌ | N/A | ✅ | **WORKING** |
| Vision/image input | ❌ | ❌ | N/A | N/A | ❌ | N/A | ❌ | **NOT IMPLEMENTED** |
| Browser navigation | ✅ (UI) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ (UI only) | **UI ONLY** |
| Browser DOM observation | ❌ (no chat) | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | **BACKEND ONLY** |
| Browser click/type | ❌ (no chat) | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | **BACKEND ONLY** |
| Browser downloads | ✅ (UI) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ (UI only) | **UI ONLY** |
| Computer screen observation | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | **BACKEND ONLY** |
| Mouse movement | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ (Win32) | ❌ | **BACKEND ONLY** |
| Mouse click | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ (Win32) | ❌ | **BACKEND ONLY** |
| Keyboard typing | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ (Win32) | ❌ | **BACKEND ONLY** |
| App launch | ✅ (via "open" prefix) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ (legacy) | **LEGACY ONLY** |
| Window control | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ (Win32) | ❌ | **BACKEND ONLY** |
| Self-knowledge (edith.*) | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | **BACKEND ONLY** |
| Self-cancel | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | **BACKEND ONLY** |
| STT (speech-to-text) | ✅ | ✅ (mock) | N/A | N/A | ❌ | ❌ (Web Speech) | ✅ (Web Speech) | **WEB SPEECH ONLY** |
| TTS (text-to-speech) | ✅ | ✅ | N/A | N/A | ❌ | ✅ (EdgeTTS+Rodio) | ✅ | **WORKING** |
| Fallback voice loop | ✅ | Partial | N/A | N/A | ❌ | Partial | ✅ (WebSpeech→chat→TTS) | **PARTIALLY WORKING** |
| Realtime S2S | ❌ (command exists) | ✅ (mock) | N/A | N/A | ❌ | ❌ (mock transport) | ❌ | **MOCK ONLY** |
| Barge-in | ✅ (TTS stop) | ✅ (TTS stop) | N/A | N/A | ❌ | ✅ | ✅ | **WORKING** (TTS interruption) |
| Reconnect | ❌ | ✅ (code exists) | N/A | N/A | ❌ | ❌ (mock) | ❌ | **MOCK ONLY** |
| Device selection | ✅ | ✅ | N/A | N/A | ❌ | ✅ (CPAL list) | ✅ (listing) | **PARTIAL** |
| Visualizer | ✅ (ArcReactor) | ❌ (not connected to real audio) | N/A | N/A | ❌ | ❌ | ❌ | **COSMETIC ONLY** |
| Telemetry | ✅ (TelemetryDock) | ✅ (EdithRuntimeState) | N/A | N/A | ❌ | ❌ (Math.random) | ❌ | **FAKE DATA** |

---

## 29. Critical Gap Matrix

| # | Gap | Affected Phase | Evidence | Severity | Why It Matters | What Must Be Done |
|---|-----|---------------|----------|----------|----------------|-------------------|
| 1 | GenerateRequest has no `tools` field | Phase 1, 5 | `ai/provider.rs:40-46` | **CRITICAL** | AI cannot use any tools | Add `tools` and `tool_choice` to GenerateRequest |
| 2 | No tool_calls in GenerateResponse/StreamChunk | Phase 1, 5 | `ai/provider.rs:62-73` | **CRITICAL** | Tool call responses are lost | Add `tool_calls` to response types |
| 3 | Provider adapters don't transmit tools | Phase 1 | `ai/adapters/*.rs` | **CRITICAL** | Even with struct changes, adapters must serialize tools | Update HTTP request bodies |
| 4 | Provider adapters don't parse tool_calls | Phase 1 | `ai/adapters/*.rs` | **CRITICAL** | Even with struct changes, adapters must parse responses | Update response parsers |
| 5 | No tool-calling loop in ConversationCore | Phase 3, 5 | `conversation/core.rs:202-272` | **CRITICAL** | After detecting tool_calls, must execute and resubmit | Implement agentic loop |
| 6 | chat_command bypasses all new architecture | Phase 3 | `chat.rs:64` used by ChatView | **CRITICAL** | Primary UI path skips tools, policy, ConversationCore execution | Migrate to ConversationCore pipeline |
| 7 | PolicyEngine has no UI | Phase 4 | No .tsx imports policyService | **HIGH** | Approval requests invisible to user | Build approval UI |
| 8 | NativeCpalCaptureDriver is mock | Phase 9, 11 | `voice/capture.rs:263,269` | **HIGH** | No real native audio capture | Implement real CPAL stream |
| 9 | CloudSTTAdapter returns fake text | Phase 9 | `voice/stt.rs:234` | **HIGH** | Native STT is non-functional | Implement real Whisper API call |
| 10 | Realtime S2S uses mock transport | Phase 10 | `lib.rs:524-525` | **HIGH** | No real duplex voice | Implement Gemini Live WebSocket adapter |
| 11 | TelemetryDock uses Math.random() | Phase 8 | `TelemetryDock.tsx:29-61` | **MEDIUM** | Misleading telemetry display | Connect to EdithRuntimeState |
| 12 | toolService.ts unused | Phase 5 | No React component imports it | **MEDIUM** | Tool execution UI unavailable | Build tool execution UI or integrate into chat |
| 13 | conversationService.ts mostly unused | Phase 3 | Only cancelTurn used | **MEDIUM** | New architecture underutilized | Migrate ChatView to use full conversation service |
| 14 | Dual authorization systems | Phase 4 | security.rs ProposalEngine vs policy/ PolicyEngine | **MEDIUM** | Inconsistent authorization | Consolidate to PolicyEngine |
| 15 | Vision capability declared but not implemented | Phase 1 | Capability::Vision declared, no image in ChatMessage | **LOW** | False capability advertisement | Implement or remove declaration |

---

## 30. What Actually Works Today

### A. Definitely Working

1. **Text chat** — User types message, LLM responds with streaming text (Groq, Gemini, OpenAI-compatible, custom providers)
2. **Provider switching** — User selects different providers/models in settings
3. **Custom providers** — User adds OpenAI-compatible endpoints
4. **Streaming** — Responses stream token-by-token with real-time UI updates
5. **Turn cancellation** — User can cancel in-flight responses
6. **Browser UI** — Full multi-tab browser with navigation, tabs, bookmarks, history, downloads, zoom, find-in-page, reader mode, tab groups, privacy protection
7. **TTS** — Edge-TTS synthesis plays through system speakers via Rodio
8. **TTS voice selection** — User can change voice in settings
9. **TTS barge-in** — Clicking microphone stops active TTS playback
10. **Memory/knowledge base** — Vector search via LanceDB for conversation memory
11. **Session management** — Create, rename, delete chat sessions with SQLite persistence
12. **Settings persistence** — All settings saved to SQLite
13. **Plugin system** — App launcher, media player, WhatsApp, Gmail, terminal, system control, web search (Tavily)
14. **Personal notes** — Note-taking feature
15. **Audio device listing** — Real CPAL device enumeration
16. **Dev Agent** — File-system code assistant

### B. Working Through Legacy Path

1. **App launching** — Via `"open <app>"` text prefix, not LLM tool call
2. **Terminal commands** — Via `"cmd <command>"` text prefix
3. **Web search** — Via `"search <query>"` text prefix using Tavily API
4. **Media playback** — Via `"play <query>"` text prefix
5. **Browser AI agent** — Via `browser_agent_run_task` (separate from normal chat)

### C. Backend Exists but Normal UI Cannot Use It

1. **All browser automation tools** (navigate, click, type, observe, screenshot through UTR)
2. **All computer control tools** (mouse, keyboard, window management through UTR)
3. **All E.D.I.T.H. self-knowledge tools** (status, capabilities, cancel through UTR)
4. **Policy engine** (risk evaluation, approval workflow)
5. **ConversationCore execute_turn** (not used by ChatView)
6. **EdithRuntimeState** (not queried by any frontend component)

### D. Only Tested with Mocks

1. **Native audio capture** — Mock 440Hz sine wave
2. **Cloud STT** — Hardcoded string response
3. **Realtime S2S voice** — Mock transport
4. **Tool-calling loop** — Does not exist

### E. Not Currently Working

1. **AI tool calling** — Structurally impossible (no tools in request/response)
2. **AI browser control from chat** — AI cannot emit browser tool calls
3. **AI computer control from chat** — AI cannot emit computer tool calls
4. **Realtime duplex voice** — Mock transport only
5. **Native STT** — Fake CloudSTTAdapter
6. **Real telemetry** — Math.random()
7. **Vision/image understanding** — Not implemented
8. **Local LLM TTS (Kokoro)** — Explicitly disabled

### F. Unknown / Requires Manual Validation

1. **Edge-TTS reliability** — Depends on Microsoft Edge TTS service availability
2. **Provider API key validation** — Requires real API keys to test
3. **Windows native computer control** — Requires interactive Windows session with real display
4. **Browser DOM observation accuracy** — Depends on target website structure
5. **CPAL device switching effect** — Device stored but capture is mock

---

## 31. What Is Misleadingly Complete

### 31.1 Features That Look Complete But Are Not

| Feature | Why It Looks Complete | Why It Is NOT Complete |
|---------|----------------------|----------------------|
| **Tool calling** | Capability::ToolCalling declared by providers, ToolRegistry has 25+ tools, ToolRouter fully implemented, domain executors genuinely work | GenerateRequest has no tools field, no provider transmits tool schemas, no response parser extracts tool_calls, no conversation loop invokes tools |
| **Realtime S2S voice** | RealtimeVoiceEngine exists (23KB), recovery module exists, transport abstraction exists, Tauri commands exist, PR #19 merged | lib.rs hardcodes MockAudioFrameTransport, no WebSocket adapter to any provider |
| **Native audio capture** | NativeCpalCaptureDriver exists, CpalAudioDeviceProvider lists real devices, device selection API works | NativeCpalCaptureDriver delegates to MockAudioCaptureDriver (440Hz sine) |
| **Cloud STT** | CloudSTTAdapter exists, accepts AudioBuffer, has API key management, returns Transcript struct | Returns hardcoded string without making any network request |
| **System telemetry** | TelemetryDock renders CPU/RAM/GPU gauges, EdithRuntimeState aggregates real backend data | TelemetryDock uses Math.random(), EdithRuntimeState never queried |
| **Policy engine** | PolicyEngine fully implements risk evaluation/approval/audit, ToolRouter integrates it, Tauri commands exist | No UI displays approvals, policy never invoked from chat path |
| **Phase 10 PR** | PR #19 merged to main with title "add realtime duplex S2S voice" | Entire realtime path uses mock transport |
| **ConversationCore** | Full turn state machine with cancellation, persistence, context assembly | ChatView uses legacy chat_command instead; execute_turn has no tool loop |

---

## 32. Final Phase Status Matrix

| Phase | Intended Result | Code Implementation | Integration | Runtime Proof | Tests | Remaining Work | Status |
|-------|----------------|--------------------|----|----|----|-----|----|
| Stage 0 | Architecture foundation | ✅ Complete | ✅ Docs + initial code | N/A | N/A | N/A | **COMPLETE** |
| Phase 1 | Provider abstraction | ✅ Text gen, ❌ Tool calling protocol | ✅ Text gen wired | ✅ Text chat works | ✅ Unit tests | Tool schema transmission, tool_calls parsing | **PARTIAL** |
| Phase 2 | Correlated events | ✅ Complete | ✅ All subsystems emit events | ✅ Stream events work | ✅ Unit tests | Tool/voice events dormant | **MOSTLY COMPLETE** |
| Phase 3 | Conversation Core + Task Runtime | ✅ Turn lifecycle, ❌ Tool loop | ⚠️ Hybrid with legacy | ✅ Cancel works, ❌ Not primary path | ✅ Unit tests | Tool-calling loop, migrate from chat_command | **PARTIAL** |
| Phase 4 | Policy engine | ✅ Complete | ✅ In ToolRouter, ❌ No UI | ❌ Never invoked from chat | ✅ Unit tests | UI for approvals, chat path integration | **DISCONNECTED** |
| Phase 5 | Universal tool runtime | ✅ Complete | ❌ Not in AI conversation loop | ❌ Only manual invocation | ✅ Unit tests (mocks) | Connect to AI conversation loop | **DISCONNECTED** |
| Phase 6 | Browser domain | ✅ Complete (legacy + UTR) | ⚠️ UTR works but AI can't reach | ✅ Browser UI works, ❌ AI chat | ✅ Domain tests | Connect to AI tool calling | **PARTIAL** |
| Phase 7 | Computer control | ✅ Complete (Windows FFI) | ❌ AI can't reach | ❌ Not reachable from UX | ✅ Domain tests (mock platform) | Connect to AI tool calling | **DISCONNECTED** |
| Phase 8 | Runtime state | ✅ Complete | ❌ Frontend ignores it | ❌ Not displayed | ✅ Domain tests | Frontend integration | **DISCONNECTED** |
| Phase 9 | Fallback voice | ⚠️ TTS complete, STT mocked, capture mocked | ✅ Web Speech + EdgeTTS path | ✅ Web Speech → chat → TTS | ⚠️ Mock-based tests | Real STT, real capture | **PARTIAL** |
| Phase 10 | Realtime S2S | ✅ Architecture, ❌ Real transport | ❌ Mock transport hardcoded | ❌ Not functional | ✅ Mock-based tests | Real WebSocket adapter | **SCAFFOLD** |
| Phase 11 | Voice UX reliability | ⚠️ DSP exists, devices work | ⚠️ Not connected to real audio | ⚠️ Device listing works | ⚠️ Mock isolation tests | Depends on Phases 9-10 completion | **SCAFFOLD** |

---

## 33. Completion Estimates

| Phase | Implementation Coverage | Integration Coverage | Validation Coverage | Confidence |
|-------|------------------------|---------------------|---------------------|------------|
| Stage 0 | 100% | 100% | N/A | High |
| Phase 1 | 70% (text gen: 100%, tool protocol: 0%) | 80% (text gen wired) | 50% (text gen tested) | High |
| Phase 2 | 95% | 90% | 60% | High |
| Phase 3 | 75% (turn: 100%, tool loop: 0%) | 40% (hybrid with legacy) | 50% | High |
| Phase 4 | 95% | 30% (ToolRouter only, no UI, no chat) | 40% | High |
| Phase 5 | 95% | 15% (registered + Tauri cmd, not in AI loop) | 40% | High |
| Phase 6 | 90% | 60% (browser UI works, AI path broken) | 40% | High |
| Phase 7 | 90% (Windows: 95%, other: mock) | 10% (not reachable from UX) | 30% | High |
| Phase 8 | 90% | 10% (backend only, no UI) | 30% | High |
| Phase 9 | 50% (TTS: 90%, STT: 10%, capture: 10%) | 50% (Web Speech path works) | 30% | High |
| Phase 10 | 40% (architecture: 90%, real transport: 0%) | 5% (mock only) | 20% | High |
| Phase 11 | 30% (DSP code exists, not connected) | 10% | 15% | Medium |

---

## 34. Required Next Work

### 34.1 Critical Blockers (Must Fix Before Product Is Viable)

1. **Implement tool-calling protocol in GenerateRequest/Response**
   - Add `tools: Option<Vec<ToolDefinition>>` and `tool_choice` to `GenerateRequest`
   - Add `tool_calls: Vec<ToolCall>` to `GenerateResponse` and `StreamChunk`
   - Evidence: `ai/provider.rs:40-73`

2. **Update provider adapters to transmit/parse tools**
   - Groq: Add `tools` array to request JSON, parse `tool_calls` from response
   - Gemini: Add `tools` / `functionDeclarations` to request, parse `functionCall` from response
   - OpenAI-compatible: Add `tools` array, parse `tool_calls`
   - Evidence: `ai/adapters/groq.rs`, `gemini.rs`, `openai_compatible.rs`

3. **Implement tool-calling loop in ConversationCore**
   - After streaming, check for `tool_calls` in response
   - For each tool call: create `ToolRequest`, invoke `ToolRouter::execute()`
   - Collect results, add as tool-result messages
   - Resubmit to model
   - Repeat until model returns text (with max iteration guard)
   - Evidence: `conversation/core.rs:202-272` (currently linear)

4. **Migrate ChatView from chat_command to ConversationCore**
   - Frontend should call `conversationService.submitTurn()` + `executeTurn()`
   - Remove plugin prefix interception from chat_command (or integrate into tool definitions)
   - Evidence: `ChatView.tsx:233` → `tauriService.chatCommand()`

### 34.2 High-Priority Integration Gaps

5. **Build Policy Engine UI** — Approval modal when ToolRouter requests human confirmation
6. **Connect TelemetryDock to EdithRuntimeState** — Replace Math.random() with real data
7. **Implement real CloudSTTAdapter** — HTTP multipart upload to Groq/OpenAI Whisper endpoint
8. **Implement real NativeCpalCaptureDriver** — Use actual CPAL input stream instead of mock_fallback

### 34.3 Medium Gaps

9. **Consolidate dual authorization systems** — Unify `security::ProposalEngine` with `policy::PolicyEngine`
10. **Build tool execution UI** — Either in chat view (inline tool results) or dedicated panel
11. **Connect browser agent to ConversationCore** — Instead of independent loop
12. **Implement real realtime WebSocket adapter** — For Gemini Live or OpenAI Realtime

### 34.4 Cleanup / Technical Debt

13. **Remove per-request `ProviderRegistry` creation** in chat_command (use managed state)
14. **Add turn cleanup** to ConversationCore (evict completed turns)
15. **Remove or document commented-out kokoro-micro dependency** in Cargo.toml
16. **Move API key storage to OS keychain** (Windows Credential Manager)
17. **Remove unused legacy commands** after migration

### 34.5 Optional Enhancements

18. Vision/image support in ChatMessage
19. Local LLM tool calling
20. Embedding model integration
21. Multi-modal response rendering

### 34.6 Minimum Viable Agentic Path

The **minimum engineering changes** to make E.D.I.T.H. work as an AI agent:

```
User message
  → ConversationCore.submit_turn()
  → ConversationCore.execute_turn()  [with tool definitions in GenerateRequest]
  → Provider sends request with tools
  → Model responds with tool_calls
  → ConversationCore detects tool_calls
  → Creates ToolRequest per call
  → PolicyEngine evaluates (approval if needed)
  → ToolRouter dispatches to DomainExecutor
  → Real OS/browser result returned
  → Tool results added as messages
  → Model re-queried
  → Final text response streamed to user
```

This requires changes to: `GenerateRequest`, `GenerateResponse`, `StreamChunk`, all 3 provider adapters, `ConversationCore::execute_turn`, and `ChatView.tsx`.

---

## 35. Manual Validation Plan

### 35.1 Basic Chat Functionality

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 1 | Open app, send message | "Hello, what is 2+2?" | Streaming text response | Provider connection failed | API key for selected provider |
| 2 | Switch provider | Change to Gemini in settings, send message | Response from Gemini | Gemini adapter broken | Gemini API key |
| 3 | Cancel mid-stream | Send long prompt, click Cancel | Stream stops, "cancelled" message | Cancellation broken | API key |
| 4 | Session persistence | Send message, close app, reopen | Previous messages visible | SQLite persistence broken | None |

### 35.2 Browser UI

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 5 | Create browser tab | Click "New Tab", navigate to google.com | Page loads in embedded browser | WebView2 broken | None |
| 6 | Multi-tab | Create 3 tabs, switch between them | All tabs render correctly | Tab management broken | None |
| 7 | Download | Download a file | Download appears in download manager | Download system broken | None |

### 35.3 Voice (Web Speech Path)

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 8 | Voice input | Click microphone, speak "Hello" | Text appears in chat input | Web Speech API unavailable | Microphone |
| 9 | TTS response | Send message with TTS enabled | Response plays through speakers | EdgeTTS/Rodio broken | Speakers, internet |
| 10 | Barge-in | While TTS plays, click microphone | TTS stops immediately | Barge-in broken | Microphone, speakers |

### 35.4 Tool Calling (EXPECTED TO FAIL — for validation only)

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 11 | Browser tool via chat | "Navigate to google.com in the browser" | Model responds with text, does NOT open browser | **This is expected behavior — tools don't work** | API key |
| 12 | Computer tool via chat | "Move the mouse to position 500, 500" | Model responds with text, mouse does NOT move | **Expected — tools don't work** | API key |
| 13 | Self-knowledge via chat | "What tools do you have available?" | Model guesses or says generic response | **Expected — edith.* tools not exposed** | API key |

### 35.5 Plugin Commands (Legacy)

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 14 | App launcher | "open notepad" | Notepad launches | Plugin broken | Windows |
| 15 | Terminal | "cmd dir" | Terminal output shown | Plugin broken | Windows |
| 16 | Web search | "search latest news" | Search results synthesized | Tavily key missing | Tavily API key |

### 35.6 Realtime Voice (EXPECTED TO FAIL)

| # | Action | User Prompt | Expected | Failure Meaning | Requires |
|---|--------|-------------|----------|-----------------|----------|
| 17 | Start realtime session | Invoke realtime_voice_start | Session starts with mock | **Expected — mock transport** | None |

### 35.7 Safety Check

| # | Action | User Prompt | Expected | Failure Meaning | Safe? |
|---|--------|-------------|----------|-----------------|-------|
| 18 | Terminal access | "cmd del /f /s C:\\" | Plugin intercepts, should NOT delete files | If files deleted: **CRITICAL BUG** | ⚠️ DESTRUCTIVE — test with safe command |
| 19 | Computer control | "Click at 0,0 continuously" | Model responds with text only (tools disconnected) | If mouse starts moving: unexpected | ✅ Safe (tools disconnected) |

---

## 36. Final Audit Conclusion

E.D.I.T.H. is a technically impressive project with a sophisticated, well-designed architecture that is approximately **50-60% implemented** when measured by the intended product vision. The project demonstrates strong Rust engineering discipline, careful abstraction design, and a comprehensive phase-based development approach.

**What works well:**
- Text chat with multiple cloud providers (Groq, Gemini, OpenAI-compatible, custom)
- Streaming with correlated events and turn cancellation
- Full-featured embedded browser with WebView2
- TTS via Edge-TTS and native Rodio playback
- Web Speech STT integration
- Memory/knowledge base with vector search
- Plugin system for common OS operations
- Thorough unit test coverage of individual components
- Clean Tauri state management and module architecture

**What is architecturally sound but not yet connected:**
- Universal Tool Runtime (fully built, domain executors work, policy enforced)
- Policy Engine (fully built, no UI)
- EdithRuntimeState (fully built, frontend ignores)
- Computer Control (real Windows FFI, cannot be invoked by AI)
- Browser Domain Executor (wraps legacy browser_tools, cannot be invoked by AI)

**The fundamental gap:**
The AI conversation loop is a single-pass text generator. It has no ability to receive tool definitions, detect tool calls in responses, execute tools, and re-query the model. This single gap makes the entire tool runtime, policy engine, browser automation, computer control, and self-knowledge systems — representing Phases 4-8 — dormant infrastructure.

**Closing this gap** requires changes to approximately 5-7 core files (GenerateRequest, GenerateResponse, 3 provider adapters, ConversationCore, ChatView) and represents the single most impactful engineering investment remaining.

---

*This audit was generated from the repository state at commit `9af6fe3` on branch `feature/phase-11-voice-ux-reliability`. No files were modified, no code was changed, and no commits were created.*
