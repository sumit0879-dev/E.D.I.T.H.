# E.D.I.T.H. Correlated Event Infrastructure Specification V1.0

**Status:** IMPLEMENTED & VERIFIED  
**Phase:** Phase 2 — Correlated Event Infrastructure  
**Baseline:** `docs/architecture/EDITH-AI-CORE-ARCHITECTURE-V1.1.md` (Section 6: Event Infrastructure)  
**Date:** September 2026  
**Target Branch:** `feature/phase-2-correlated-events`

---

## 1. Executive Summary & Purpose

In the original E.D.I.T.H. architecture, real-time asynchronous streaming across Tauri IPC relied on an untyped, global event channel named `chat-chunk`. The frontend listener operated under the fragile assumption:
> *"Append incoming chunk text to whichever message currently has `isStreaming: true`."*

This architecture suffered from critical vulnerabilities:
1. **Stream Cross-Talk:** If the developer ran an autonomous DevAgent or BrowserAgent while actively chatting, streaming tokens from both sources were interleaved into the active chat UI.
2. **Race Conditions & Interleaving:** Multiple simultaneous queries or rapid turn submissions could corrupt each other's message buffers.
3. **No Lifecycle Determinism:** The client had no typed notification of when a stream started, finished, errored, or was cancelled, relying instead on the outer IPC command completion.
4. **No Sequence Verification:** Chunks could arrive out-of-order or duplicate without detection.

**Phase 2 resolves these challenges by introducing a typed, correlated event infrastructure.** Every event in the system is wrapped in an `EdithEventEnvelope`, stamped with monotonic sequence numbers, tagged with precise correlation context (`conversation_id`, `turn_id`, `stream_id`, `task_id`), and dispatched via dedicated backend and frontend routers.

---

## 2. Universal Event Envelope & Correlation Model

### 2.1 Strong Types & IDs
All identifiers are strongly typed newtypes wrapping UUID v4 strings in Rust and string primitives in TypeScript:
- `EventId`: Unique ID generated per event envelope.
- `ConversationId` / `SessionId`: Identifies the conversation session.
- `TurnId`: Identifies an individual user-assistant turn in a conversation.
- `StreamId`: Identifies an individual streaming token emission lifecycle.
- `TaskId`: Identifies an autonomous background task (e.g. `dev_agent`, `browser_agent`).
- `ToolExecutionId`: Identifies a specific tool call proposal/execution.
- `VoiceSessionId`: Identifies an audio/speech session.

### 2.2 Scoped Correlation (`EventCorrelation`)
Events only carry identifiers relevant to their scope. Unused correlation fields are serialized as omitted (not null) to minimize IPC serialization overhead:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventCorrelation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_session_id: Option<String>,
}
```

### 2.3 Universal Envelope (`EdithEventEnvelope<T>`)
Dispatched over the Tauri IPC channel `"edith-event"`:

```json
{
  "event_id": "7f8b9e12-4c3a-48d6-91b5-90123456789a",
  "timestamp_ms": 1788514800000,
  "correlation": {
    "conversation_id": "session_8812",
    "turn_id": "turn_3391",
    "stream_id": "stream_1104"
  },
  "payload": {
    "category": "stream",
    "data": {
      "stream_event": "chunk",
      "data": {
        "text": "Hello, Commander.",
        "sequence_number": 1,
        "is_final": false
      }
    }
  }
}
```

---

## 3. Tagged Event Taxonomy

All payloads are categorized into top-level discriminant categories:

| Category | Description | Primary Correlation Keys |
|---|---|---|
| `stream` | LLM token streaming lifecycle | `conversation_id`, `turn_id`, `stream_id` |
| `task` | Autonomous agent lifecycle & step progress | `task_id`, `conversation_id` |
| `tool` | Tool call proposals, approval, and execution | `tool_execution_id`, `task_id` |
| `voice` | Realtime voice capture, vad, and barge-in | `voice_session_id`, `conversation_id` |
| `runtime` | System notifications, errors, health | Optional global correlation |

### 3.1 Stream Lifecycle State Machine
```
[Started] ──> [Chunk 1] ──> [Chunk 2] ──...──> [Chunk N] ──> [Finished]
    │                                                              ▲
    ├───> [Failed] ────────────────────────────────────────────────┘
    │                                                              ▲
    └───> [Cancelled] ─────────────────────────────────────────────┘
```

1. **`Started { model: String }`**: Emitted before the first token request starts. Notifies UI to initialize streaming state.
2. **`Chunk { text: String, sequence_number: u64, is_final: bool }`**: Monotonically numbered token emission. The frontend drops out-of-order or duplicate packets where `seq <= lastSeq`.
3. **`Finished { total_tokens: Option<u32>, finish_reason: Option<String> }`**: Normal completion.
4. **`Failed { error: String, error_type: Option<String> }`**: Provider failure, network drop, or rate limit.
5. **`Cancelled { reason: Option<String> }`**: User aborted generation.

---

## 4. Concurrency Isolation Architecture

### 4.1 Backend Dispatch (`EventEmitter`)
`src-tauri/src/events/emitter.rs` provides strongly typed helper methods:
- `emitter.emit_stream_started(&correlation, model)`
- `emitter.emit_stream_chunk(&correlation, text, seq, is_final)`
- `emitter.emit_stream_finished(&correlation, total_tokens, finish_reason)`
- `emitter.emit_stream_failed(&correlation, error, error_type)`
- `emitter.emit_stream_cancelled(&correlation, reason)`

For testability, `EventEmitter::mock()` creates an in-memory test harness with zero Tauri runtime dependencies, enabling blazing fast unit tests.

### 4.2 Frontend Demultiplexing (`StreamRouter`)
`src/events/streamRouter.ts` manages stream multiplexing on the client:
1. **Targeted Subscriptions:** Consumers subscribe via `subscribeTurn(turnId, onChunk, onLifecycle)` or `subscribeStream(streamId, onChunk, onLifecycle)`.
2. **Isolation Enforcement:** Incoming events are routed **only** to subscribers whose `turnId` or `streamId` matches the event correlation.
3. **Monotonic Sequence Enforcement:** If a chunk arrives with `seq <= lastSequenceNumber`, it is discarded with a console warning.
4. **Automatic Memory Cleanup:** Upon receiving `finished`, `failed`, or `cancelled`, the stream state is marked complete and purged after 5 seconds.

---

## 5. Backward Compatibility & Migration Strategy

Phase 2 ensures **100% backward compatibility** with unmigrated views and components:

1. **Dual Emission via Bridge:** When `emitter.emit_stream_chunk` is called, it emits the full typed `EdithEventEnvelope` on `"edith-event"` **and** emits the raw text on the legacy `"chat-chunk"` channel.
2. **Additive Tauri Command Signature:** `chat_command` accepts `turn_id: Option<String>`. If omitted (e.g. by legacy frontend callers), Tauri provides `None` and the backend auto-generates a fallback `TurnId`.
3. **Additive Response Type:** `ChatResponse` adds optional `stream_id` and `turn_id` fields without altering existing `response` and `type` fields.
4. **Cooperative Frontend Fallback:** In `ChatView.tsx`, the legacy `onChatChunk` listener yields precedence to `streamRouter`:
   ```typescript
   tauriService.onChatChunk((chunk) => {
     if (tauriService.streamRouter.getActiveStreamCount() > 0) return;
     // Legacy fallback...
   });
   ```

---

## 6. Implementation Artifacts

### Backend Files (`src-tauri/src/events/`)
- [`ids.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/ids.rs): Strongly typed ID newtypes with UUID v4 generators.
- [`envelope.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/envelope.rs): `EventCorrelation` and `EdithEventEnvelope<T>`.
- [`payload.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/payload.rs): Tagged enum taxonomy for Stream, Task, Tool, Voice, and Runtime.
- [`emitter.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/emitter.rs): `EventEmitter` with Tauri IPC dispatch and mock test engine.
- [`mod.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/mod.rs): Clean module exports.
- [`tests.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/events/tests.rs): 7 automated unit and concurrency isolation tests.

### Frontend Files (`src/events/`)
- [`types.ts`](file:///e:/Projects/E.D.I.T.H/src/events/types.ts): TypeScript type definitions matching backend models.
- [`eventRouter.ts`](file:///e:/Projects/E.D.I.T.H/src/events/eventRouter.ts): IPC listener for `"edith-event"` with category subscriber dispatch.
- [`streamRouter.ts`](file:///e:/Projects/E.D.I.T.H/src/events/streamRouter.ts): Turn-isolated stream subscriber and sequence order validator.
- [`index.ts`](file:///e:/Projects/E.D.I.T.H/src/events/index.ts): Barrel export.

### Migrated Call Sites
- [`src-tauri/src/lib.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/lib.rs): Registered `pub mod events;`.
- [`src-tauri/src/chat.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/chat.rs): Migrated `chat_command` to accept `turn_id`, emit correlated events with monotonic sequencing, and return correlation IDs.
- [`src-tauri/src/agent.rs`](file:///e:/Projects/E.D.I.T.H/src-tauri/src/agent.rs): Migrated `agent_chat` to emit correlated stream events with `task_id: "dev_agent"`.
- [`src/services/tauri.ts`](file:///e:/Projects/E.D.I.T.H/src/services/tauri.ts): Added `ChatCommandResult`, `onEdithEvent`, and re-exported `streamRouter`.
- [`src/views/ChatView.tsx`](file:///e:/Projects/E.D.I.T.H/src/views/ChatView.tsx): Replaced blind `prev[lastIdx]` chunk appending with targeted `streamRouter.subscribeTurn(assistantMsgId, ...)`.

---

## 7. Roadmap to Phase 3 (Conversation Core)

With Phase 2 merged:
1. **Single Source of Truth:** Phase 3 will introduce `ConversationCore` and `SessionManager` in Rust, taking over message persistence, title generation, and turn coordination.
2. **Turn Lifecycle Authority:** Turns will be created on the backend with an authoritative `TurnId`. The frontend will receive the `TurnId` and bind its UI elements directly to that ID.
3. **Tool Execution Events:** When the Universal Tool Runtime is implemented in Phase 4, tool proposals and results will emit via `EdithPayload::Tool`, correlated to the triggering `TurnId` and `StreamId`.
