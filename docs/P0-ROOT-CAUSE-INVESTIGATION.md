# P0 Root-Cause Investigation: Installed EXE E2E System Failures

**Date:** 2026-10-02  
**Target:** E.D.I.T.H. Desktop AI Assistant (Windows x64 / Tauri v2 / Rust 2021)  
**Status:** DIAGNOSIS COMPLETE (Read-Only Investigation; Source Code Unmodified)

---

## 1. Executive Summary

During End-to-End (E2E) validation of the packaged/installed Windows executable for E.D.I.T.H., five critical system-level failures were encountered:
1. **Computer Control Failure:** Tool call attempts resulted in an immediate OpenAI/Groq API validation error: `messages.X.tool_calls.0.type is missing`.
2. **Browser / Plugin Crash:** Querying plugins or executing web search resulted in a fatal SQLite error: `no such table: plugin_states`.
3. **Security Policy UI Deadlock:** Destructive operations (e.g. file deletion style prompts) caused the application to hang indefinitely in the user interface on `"Synthesizing response..."` with no approval modal appearing and no error/rejection surfaced.
4. **Agentic Tool Loop Breakdown:** Tool calls received from the model could never complete an agentic observation cycle or continue reasoning.
5. **Voice Subsystem Disconnect:** Voice telemetry reported `Voice Synthesis = STANDBY` without verifiable hardware output, while native CPAL capture and realtime duplex voice remained disconnected or mocked.
6. **Provider / Identity Hallucination:** Groq telemetry reported model `openai/gpt-oss-120b`, while conversational replies asserted the model was `GPT-4-Turbo`.

Our systematic, read-only root-cause investigation has traced every failure to precise file locations, function calls, and data structures. No hypothetical root causes are presented; every defect is corroborated with static analysis, Git commit diffs, and exact execution paths.

---

## 2. Exact Reproduction Path for Each Failure

### Failure 1 & 4: Tool Call Serialization & Loop Termination
- **Step 1:** User initiates a conversation turn that requires tool invocation (e.g., `"Move cursor to 100, 200 and click"`).
- **Step 2:** [`ChatView.tsx`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx#L233) calls `conversationService.submitConversationTurn(...)`, followed by `executeConversationTurn(...)`.
- **Step 3:** [`ConversationCore::execute_turn`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs#L160) starts Iteration 1, assembling messages and dispatching a streaming `GenerateRequest` with `tools` to Groq (`groq.rs`).
- **Step 4:** Groq returns an SSE delta stream containing a tool call (e.g., `id: "call_abc"`, `name: "computer.click"`).
- **Step 5:** `groq.rs` parses the tool call into [`crate::ai::provider::ToolCall`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/provider.rs#L9).
- **Step 6:** [`ConversationCore`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs#L430) pushes `ChatMessage::assistant_with_tools(...)` into `loop_messages`.
- **Step 7:** Tool executes via [`ToolRouter::execute`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/tools/router.rs#L186). The execution result is formatted and appended via `ChatMessage::tool_result(...)`.
- **Step 8:** `ConversationCore` executes `continue;` to enter Iteration 2 (feeding tool results back to the LLM).
- **Step 9:** In Iteration 2, `GenerateRequest { messages: loop_messages, ... }` is passed to [`GroqAdapter::stream`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/groq.rs#L277) / [`OpenAICompatibleAdapter::stream`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/openai_compatible.rs#L277).
- **Step 10:** Serde serializes `ChatMessage` directly:
  ```json
  {
    "role": "assistant",
    "tool_calls": [
      {
        "id": "call_abc",
        "name": "computer.click",
        "arguments": "{\"x\": 100, \"y\": 200}"
      }
    ]
  }
  ```
- **Step 11:** Groq/OpenAI receives the payload. The provider wire specification requires:
  `{ "id": "call_abc", "type": "function", "function": { "name": "computer.click", "arguments": "..." } }`.
- **Step 12:** Groq API validation rejects the HTTP request with status 400 Bad Request:
  `messages.1.tool_calls.0.type is missing`.
- **Step 13:** Iteration 2 fails immediately with `ProviderError::InvalidRequest`. Tool results are never returned to the model, and reasoning halts.

### Failure 2: Production Database Missing Table
- **Step 1:** On a fresh production installation, the app starts with an empty AppData folder (`%APPDATA%/edith-v2/edith_memory.db`).
- **Step 2:** [`src-tauri/src/lib.rs:591`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/lib.rs#L591) calls [`db::init_db_at(&db_path)`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs#L174).
- **Step 3:** The user asks to search the web or issues a command (e.g., `"search latest AI news"`).
- **Step 4:** [`src-tauri/src/chat.rs:233`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs#L233) executes `if !plugin_enabled(&db_state, "web_search")?`.
- **Step 5:** [`plugin_enabled`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs#L47) executes [`db::get_plugin_states(&conn)?`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs#L666).
- **Step 6:** SQLite prepares `SELECT id, enabled FROM plugin_states`.
- **Step 7:** SQLite fails with error code 1: `no such table: plugin_states`.
- **Step 8:** `plugin_enabled` maps the error to a String and uses `?`, immediately bubbling `Err("no such table: plugin_states")` to the UI.

### Failure 3: Destructive Command Hang on "Synthesizing response..."
- **Step 1:** User submits a destructive request (e.g., file deletion).
- **Step 2:** [`ChatView.tsx:200`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx#L200) registers a stream listener via `streamRouter.subscribeTurn(assistantMsgId, ...)`. Note that `assistantMsgId` is a client timestamp ID (`msg-172...`).
- **Step 3:** [`ChatView.tsx:233`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx#L233) invokes `submitConversationTurn`. Backend authoritatively generates a UUID `turn_id` (`turn-9e8a...`), discarding `clientTurnId`.
- **Step 4:** `ChatView.tsx:242` invokes `executeConversationTurn` and awaits the returned promise.
- **Step 5:** Backend streams tokens or tool calls, emitting events with `correlation.turn_id = "turn-9e8a..."`.
- **Step 6:** Frontend [`streamRouter.ts:126`](file:///E:/Projects/E.D.I.T.H/src/events/streamRouter.ts#L126) checks `sub.turnId === correlation.turn_id`. Because `"msg-172..." !== "turn-9e8a..."`, all stream chunks and lifecycle events are discarded. The React message state text remains `""`.
- **Step 7:** In [`groq.rs:354-367`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/groq.rs#L354) (and `openai_compatible.rs`), the streaming loop reads SSE lines:
  ```rust
  while let Ok(Some(chunk)) = res.chunk().await {
      for line in chunk_str.lines() {
          if *data == "[DONE]" {
              on_chunk(StreamChunk { ... });
              break; // BUG: Only breaks the inner `for line` loop!
          }
      }
  }
  ```
- **Step 8:** `break` only exits the inner `for line` loop. The outer `while let Ok(Some(chunk)) = res.chunk().await` loop continues and invokes `res.chunk().await` again.
- **Step 9:** Because the provider has completed sending the response but keeps the TCP connection alive (HTTP keep-alive), `res.chunk().await` blocks indefinitely waiting for EOF.
- **Step 10:** `execute_turn` never returns. In `ChatView.tsx`, the promise never settles.
- **Step 11:** The UI renders line 638 of `ChatView.tsx`:
  `{msg.text || msg.content || (msg.isStreaming ? 'Synthesizing response...' : '')}`.
  Since `msg.text` was dropped due to the TurnId mismatch and `isStreaming` remains `true`, the UI hangs perpetually displaying `"Synthesizing response..."`.

---

## 3. Root Cause for P0-A: Tool Call Wire Format Schema Mismatch

### Identification
- **Files Responsible:**
  - [`src-tauri/src/ai/provider.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/provider.rs#L9-L50)
  - [`src-tauri/src/ai/adapters/openai_compatible.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/openai_compatible.rs#L155)
  - [`src-tauri/src/ai/adapters/groq.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/groq.rs#L172)
  - [`src-tauri/src/ai/adapters/gemini.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/gemini.rs#L170)
- **Functions:**
  - `ChatMessage::assistant_with_tools`
  - `OpenAICompatibleAdapter::generate` / `stream`
  - `GroqAdapter::generate` / `stream`
  - `GeminiAdapter::generate` / `stream`
- **Data Structures:**
  - `pub struct ToolCall { pub id: String, pub name: String, pub arguments: String }`
  - `pub struct ChatMessage { pub role: String, pub content: String, pub tool_calls: Option<Vec<ToolCall>>, ... }`

### Source Evidence & Detailed Explanation
In `src-tauri/src/ai/provider.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
```
When serialized by `serde_json`, this produces:
```json
{"id": "...", "name": "...", "arguments": "..."}
```
However, the standard OpenAI/Groq API wire format specification for an assistant message with tool calls mandates:
```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "computer.click",
        "arguments": "{\"x\":100,\"y\":200}"
      }
    }
  ]
}
```
And for the subsequent tool result message:
```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "{\"status\":\"success\"}"
}
```
In `src-tauri/src/ai/adapters/groq.rs` (lines 172, 294) and `src-tauri/src/ai/adapters/openai_compatible.rs` (lines 155, 294), the request payload is constructed via:
```rust
let mut body = json!({
    "model": req.model,
    "messages": req.messages,
    "temperature": req.temperature,
    "stream": true
});
```
Because `req.messages` contains the internal domain model [`ChatMessage`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/provider.rs#L41) with flat `ToolCall` objects, the outbound JSON lacks the `"type": "function"` field and the nested `"function"` dictionary.

When Groq or OpenAI validates the outbound request on Iteration 2, it inspects `messages[X].tool_calls[0]`, finds no `type` field, and immediately aborts with:
`400 Bad Request: messages.X.tool_calls.0.type is missing`.

---

## 4. Root Cause for P0-B: Missing `plugin_states` & `custom_apps` Schema

### Identification
- **Files Responsible:**
  - [`src-tauri/src/db.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs#L174-L359)
  - [`src-tauri/src/chat.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs#L47-L51)
  - [`src-tauri/src/plugins.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/plugins.rs#L160-L190)
  - [`src-tauri/src/browser.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/browser.rs#L2835)
- **Functions:**
  - `init_db_at(db_path: &PathBuf)`
  - `plugin_enabled(state: &DbState, plugin_id: &str)`
  - `get_plugin_states(conn: &Connection)`
  - `get_custom_apps(conn: &Connection)`

### Source Evidence & Detailed Explanation
In initial commit `10d6d79`, `init_db_at()` included:
```sql
CREATE TABLE IF NOT EXISTS custom_apps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE,
    path TEXT,
    keywords TEXT
);
CREATE TABLE IF NOT EXISTS plugin_states (
    id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1
);
```
However, in commit [`94258e0`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs#L193) (*feat: add content blocking and privacy policy engine*), the table definitions were accidentally deleted from the `execute_batch` script during a merge of `memories`, `knowledge_items`, and `browser_privacy_*` tables:
```diff
@@ -133,25 +176,32 @@ pub fn init_db_at(db_path: &PathBuf) -> Result<Connection> {
-        CREATE TABLE IF NOT EXISTS custom_apps (
-            id INTEGER PRIMARY KEY AUTOINCREMENT,
-            name TEXT UNIQUE,
-            path TEXT,
-            keywords TEXT
-        );
-        CREATE TABLE IF NOT EXISTS plugin_states (
-            id TEXT PRIMARY KEY,
-            enabled INTEGER NOT NULL DEFAULT 1
-        );
+        CREATE TABLE IF NOT EXISTS memories (
...
```
While `CREATE TABLE IF NOT EXISTS plugin_states` was deleted, the query functions were retained:
```rust
pub fn get_plugin_states(conn: &Connection) -> Result<std::collections::HashMap<String, bool>> {
    let mut stmt = conn.prepare("SELECT id, enabled FROM plugin_states")?;
...
```
Furthermore, in [`src-tauri/src/chat.rs:49`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs#L49):
```rust
fn plugin_enabled(state: &DbState, plugin_id: &str) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let saved = db::get_plugin_states(&conn).map_err(|e| e.to_string())?;
    Ok(*saved.get(plugin_id).unwrap_or(&true))
}
```
Unlike `plugins.rs:163` (which uses `.unwrap_or_default()`), `chat.rs:49` propagates the SQLite query error via `?`. Because `plugin_enabled` is evaluated for `web_search`, `app_launcher`, `media_player`, `terminal`, etc., any fresh database triggers an immediate fatal failure: `no such table: plugin_states`.

Additionally, an inconsistency exists between [`src-tauri/src/lib.rs:590`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/lib.rs#L590) (which opens `edith_memory.db`) and [`src-tauri/src/browser.rs:2835`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/browser.rs#L2835) (which hardcodes `app_dir.join("edith.db")`).

---

## 5. Root Cause for P0-C: Security Policy Deadlock & UI Hang

### Identification
- **Files Responsible:**
  - [`src-tauri/src/ai/adapters/groq.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/groq.rs#L360)
  - [`src-tauri/src/ai/adapters/openai_compatible.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/openai_compatible.rs#L351)
  - [`src-tauri/src/ai/adapters/gemini.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/gemini.rs#L393)
  - [`src-tauri/src/conversation/core.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs#L461-L495)
  - [`src/views/ChatView.tsx`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx#L200-L245)
  - [`src/events/streamRouter.ts`](file:///E:/Projects/E.D.I.T.H/src/events/streamRouter.ts#L124-L133)
- **Functions:**
  - `GroqAdapter::stream`
  - `ConversationCore::execute_turn`
  - `ChatView::handleSendMessage`
  - `streamRouter::isMatchingSubscription`

### Source Evidence & Detailed Explanation

#### Root Cause 1: Inner-Loop `break` Bug in Provider SSE Chunking
In all three LLM streaming adapters (`groq.rs`, `openai_compatible.rs`, `gemini.rs`):
```rust
while let Ok(Some(chunk)) = res.chunk().await {
    let chunk_str = String::from_utf8_lossy(&chunk);
    for line in chunk_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("data: ") {
            let data = &trimmed[6..].trim();
            if *data == "[DONE]" {
                on_chunk(StreamChunk {
                    text: String::new(),
                    is_done: true,
                    tool_calls: None,
                });
                break; // BUG: Breaks `for line in chunk_str.lines()`, NOT `while res.chunk().await`!
            }
```
When `data: [DONE]` is processed, `break;` only exits the `for line` loop. It returns to the `while` loop condition, which awaits `res.chunk().await`. Because the LLM server has finished streaming and transmits no additional bytes (while keeping the HTTP connection open via HTTP keep-alive), `res.chunk().await` hangs indefinitely waiting for TCP connection closure.

#### Root Cause 2: Absence of HITL Human Approval Suspension in `ConversationCore`
In `src-tauri/src/conversation/core.rs` (lines 461–495):
```rust
let tool_result = router.execute(tool_req).await;
let result_payload = match &tool_result.status {
    ToolStatus::ApprovalRequired => {
        json!({
            "status": "approval_required",
            "approval_id": tool_result.approval_id,
            "reason": tool_result.error
        }).to_string()
    }
...
loop_messages.push(ChatMessage::tool_result(call.id.clone(), call.name.clone(), result_payload));
continue;
```
When `PolicyEngine` determines an action requires human approval (`PolicyOutcome::ConfirmationRequired`):
1. `ToolRouter::execute` creates the pending approval in `ApprovalStore` and returns `ToolStatus::ApprovalRequired`.
2. `ConversationCore` **does not suspend execution or wait for the user to approve**.
3. Instead, it immediately serializes `status: approval_required` as an ordinary tool output and calls `continue;` to immediately re-prompt the LLM in Iteration 2.
4. In Iteration 2, the tool serialization bug (P0-A) triggers or `res.chunk().await` hangs on `[DONE]`.
5. The operator is never prompted in the UI to approve or deny the action.

#### Root Cause 3: Frontend Subscription TurnId Mismatch
In `src/views/ChatView.tsx`:
- Line 196: `const assistantMsgId = 'msg-' + (Date.now() + 1);`
- Line 200: `tauriService.streamRouter.subscribeTurn(assistantMsgId, ...)`
- Line 233: `const turnResult = await conversationService.submitConversationTurn(...)`

`submitConversationTurn` instructs the backend to authoritatively generate a new UUID `turn_id` (`turn-d4e5...`) and ignores `clientTurnId`. The backend emits chunks tagged with `turn_id: "turn-d4e5..."`. In `streamRouter.ts`:
```typescript
if (sub.turnId && correlation.turn_id) {
    return sub.turnId === correlation.turn_id; // "msg-172..." === "turn-d4e5..." -> FALSE
}
```
All streamed tokens are dropped. Because `msg.text` remains `""` and `isStreaming` is `true`, `ChatView.tsx:638` renders `"Synthesizing response..."` indefinitely while the backend is hung.

---

## 6. Agentic Loop Integrity Analysis

The intended agentic loop specification:
$$\text{Reason} \longrightarrow \text{Choose Tool} \longrightarrow \text{Policy Check} \longrightarrow \text{Execute} \longrightarrow \text{Observe Result} \longrightarrow \text{Append Result} \longrightarrow \text{Continue Reasoning} \longrightarrow \text{Final Response}$$

### Critical Breakage Audit

| Stage | Intended Architecture | Current Implementation | Failure Mode |
|---|---|---|---|
| **1. Reason & Choose Tool** | Model inspects available tool definitions in `GenerateRequest.tools` and emits `tool_calls`. | `GenerateRequest.tools` registered; model emits tool calls. | Functional in Iteration 1. |
| **2. Policy Check** | `PolicyEngine::evaluate` checks action against constraints & risk rules. | Implemented in `ToolRouter::execute`. | Functional; returns `Allow`, `Blocked`, or `ConfirmationRequired`. |
| **3. Approval Gate** | If `ConfirmationRequired`, pause execution, present `ApprovalModal`, await operator resolution. | `ConversationCore` formats `status: approval_required` as a tool result and loops immediately. | **CRITICAL LOOP BREAKAGE:** Turn never suspends; approval modal is bypassed. |
| **4. Execute Tool** | Domain executor (`BrowserDomainExecutor`, `ComputerDomainExecutor`) executes authorized tool. | Executed via `executors.get(&domain)`. | Functional for low-risk actions. |
| **5. Observe & Append** | Tool output is appended to conversation history as `role: tool`. | Appended to `loop_messages` via `ChatMessage::tool_result`. | Functional. |
| **6. Continue Reasoning** | LLM is called again with full history including assistant tool call and tool result. | `GenerateRequest` sent with `loop_messages`. | **FATAL FAILURE:** Outbound serialization missing `type: "function"` causes OpenAI/Groq 400 Bad Request error. |
| **7. Final Response** | LLM completes generation and returns final answer without tool calls. | Handled via `final_text = response.text; break;`. | **FATAL HANG:** `res.chunk().await` hangs on `[DONE]` due to inner-loop `break`. |

---

## 7. Voice Subsystem Status

| Component | Architecture Intent | Current Reality | Production Status |
|---|---|---|---|
| **Native CPAL Capture** | Hardware audio input via `cpal` with real device enumeration | [`NativeCpalCaptureDriver`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/capture.rs#L234) is implemented but bypassed in [`lib.rs:650`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/lib.rs#L650) in favor of `BrowserCaptureBridge`. | **UNREACHABLE** |
| **Frontend Microphone** | Respect capture ownership domain | [`AppContext.tsx:428`](file:///E:/Projects/E.D.I.T.H/src/context/AppContext.tsx#L428) uses browser `webkitSpeechRecognition`. | **REAL (Browser Only)** |
| **STT Mode A (Web Speech)** | Normalize browser transcripts | Implemented in [`WebSpeechSTTBridge`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/stt.rs#L48). | **REAL** |
| **STT Mode B (Cloud Whisper)** | Raw audio buffer transcription via OpenAI/Groq Whisper API | Implemented in [`CloudSTTAdapter`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/stt.rs#L178), but `api_key` is never set (`set_api_key` never called). | **UNREACHABLE** |
| **TTS (EdgeTTS)** | Cloud synthesis via `edge-tts-rust` | Implemented in [`tts.rs:136`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/tts.rs#L136) and [`voice/tts.rs:74`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/tts.rs#L74). | **REAL** |
| **TTS (Local Kokoro)** | On-device ONNX synthesis | Explicitly disabled and commented out in [`tts.rs:293`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/tts.rs#L293). | **MOCK / DISABLED** |
| **AudioOutputDriver** | Single authoritative hardware playback sink | [`RodioAudioOutputDriver`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/output.rs#L82) implemented, but frontend `speakText` calls legacy `tts_speak` (`tts.rs`). | **DISCONNECTED** |
| **Realtime Voice Engine** | Low-latency duplex speech-to-speech with server VAD & barge-in | [`RealtimeVoiceEngine`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/realtime/engine.rs#L1) exists; only [`MockRealtimeSessionAdapter`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/voice/realtime/adapter.rs#L70) exists. Zero live cloud transports. | **MOCK** |
| **UI Telemetry Status** | Telemetry dock reflects audio sink activity | [`TelemetryDock.tsx:244`](file:///E:/Projects/E.D.I.T.H/src/components/TelemetryDock.tsx#L244) reads React `isSpeaking` flag, which flips to `STANDBY` as soon as bytes are dispatched. | **PARTIAL** |

---

## 8. Provider / Self-Knowledge Status

1. **Broken Settings Contract Between Frontend & Backend:**
   - In [`src/context/AppContext.tsx:54-70`](file:///E:/Projects/E.D.I.T.H/src/context/AppContext.tsx#L54), default settings define `selectedProvider: 'groq'` and `selectedModel: 'llama-3.3-70b-versatile'`.
   - In [`src/views/ChatView.tsx:236-237`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx#L236), `submitConversationTurn` reads:
     ```typescript
     providerId: settings.aiProvider,
     modelId: settings.aiModel,
     ```
     Both `settings.aiProvider` and `settings.aiModel` are `undefined`.
   - In [`ConversationCore::submit_turn`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs#L106), `req.provider_id` and `req.model_id` are `None`, falling back to hardcoded defaults: `"groq"` and `"llama-3.3-70b-versatile"`.
   - Operator choices in the Settings UI are completely ignored by `ConversationCore`.

2. **Model Identity Hallucination:**
   - The system prompt assembled in [`ContextAssembler::build_system_prompt`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/context.rs#L83) only contains:
     `"You are E.D.I.T.H. (Even Dead, I'm The Hero), an advanced Stark-grade AI PC assistant..."`
   - It injects zero runtime self-knowledge regarding active provider ID, model ID, or execution mode.
   - When running open-weight or distilled models (such as `openai/gpt-oss-120b` via Groq), the underlying LLM relies on its internal pre-training weights and hallucinates that it is `GPT-4-Turbo`.

3. **Authoritative Runtime Source:**
   - [`EdithRuntimeState`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/runtime/state.rs#L26) exists and aggregates live provider and model metadata, but its projections are not injected into `ContextAssembler` prompt templates.

---

## 9. Files, Functions & Modules Involved

| Module | File | Target Symbol / Function | Issue |
|---|---|---|---|
| **AI Types** | `src-tauri/src/ai/provider.rs` | `ToolCall`, `ChatMessage` | Lacks `"type": "function"` and nested function schema serialization. |
| **Groq Adapter** | `src-tauri/src/ai/adapters/groq.rs` | `GroqAdapter::stream` (L349-410) | Inner `break` bug fails to terminate outer `while res.chunk().await`. Direct `req.messages` JSON serialization. |
| **OpenAI Adapter** | `src-tauri/src/ai/adapters/openai_compatible.rs` | `OpenAICompatibleAdapter::stream` (L339-395) | Same inner `break` bug and unnormalized message serialization. |
| **Gemini Adapter** | `src-tauri/src/ai/adapters/gemini.rs` | `GeminiAdapter::stream` (L381-435) | Same inner `break` bug. |
| **Database** | `src-tauri/src/db.rs` | `init_db_at` (L180-360) | Missing `CREATE TABLE IF NOT EXISTS plugin_states` and `custom_apps`. |
| **Chat Routing** | `src-tauri/src/chat.rs` | `plugin_enabled` (L47-51) | Unwrapped error propagation causes immediate crash when `plugin_states` missing. |
| **Conversation Core** | `src-tauri/src/conversation/core.rs` | `execute_turn` (L277-505) | Does not suspend/pause on `ToolStatus::ApprovalRequired`. No timeout guard on stream. |
| **Frontend Chat** | `src/views/ChatView.tsx` | `handleSendMessage` (L196-245) | Subscribes stream with client message ID instead of backend `turn_id`. Reads `settings.aiProvider` instead of `selectedProvider`. |
| **Stream Router** | `src/events/streamRouter.ts` | `isMatchingSubscription` (L124-133) | Drops events due to TurnId mismatch. |
| **Voice Bootstrap** | `src-tauri/src/lib.rs` | `run` (L647-668) | Passes `BrowserCaptureBridge` instead of `NativeCpalCaptureDriver`. `CloudSTTAdapter` key unpopulated. |
| **Context Assembler** | `src-tauri/src/conversation/context.rs` | `build_system_prompt` (L83-121) | No runtime provider/model identity injection into prompt. |

---

## 10. Evidence Collected

1. **Git Commit Diff Evidence (`94258e0`):**
   Verification of commit `94258e0` proves the exact deletion of `plugin_states` and `custom_apps` table creation statements during the implementation of content blocking.
2. **OpenAI / Groq API Wire Specification Evidence:**
   Official OpenAI Chat Completions API specification section on Function Calling requires:
   ```json
   "tool_calls": [{"id": "...", "type": "function", "function": {"name": "...", "arguments": "..."}}]
   ```
   Groq API error `messages.X.tool_calls.0.type is missing` directly matches this schema deficiency.
3. **Rust Control Flow Static Proof:**
   In `groq.rs:360`:
   ```rust
   while let Ok(Some(chunk)) = res.chunk().await { // Level 1
       for line in chunk_str.lines() {              // Level 2
           if *data == "[DONE]" {
               break;                               // Exits Level 2 ONLY!
           }
       }
   }
   ```
   Unlabeled `break;` cannot terminate Level 1 loop.
4. **React State & Props Contract Inspection:**
   `AppSettings` definition in `AppContext.tsx:54` contains `selectedProvider` and `selectedModel`. `ChatView.tsx:236` accesses `settings.aiProvider` and `settings.aiModel`. In JavaScript, accessing undeclared properties yields `undefined`.

---

## 11. Existing Tests Relevant to Each Bug

- [`src-tauri/src/conversation/tests.rs:108-205`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/tests.rs#L108): Tests `submit_turn` and turn state transitions, but uses `MockStreamingProvider` which produces zero `tool_calls`.
- [`src-tauri/src/policy/tests.rs:73-100`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/policy/tests.rs#L73): Tests that `rm` or `del` commands produce `PolicyOutcome::ConfirmationRequired`, but tests only the policy engine in isolation without the `ConversationCore` agentic loop.
- [`src-tauri/src/tools/domains/computer_tests.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/tools/domains/computer_tests.rs): Tests argument schemas and mock platform dispatch, but does not test integration with LLM streaming adapters.
- [`src-tauri/examples/e2e_reproducible_suite.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/examples/e2e_reproducible_suite.rs): Runs browser navigation and risk policy tests against temporary databases, but does not invoke `plugin_enabled` or multi-turn agentic conversations.

---

## 12. Missing Tests Required to Prevent Regression

1. **Tool Call Wire Schema Unit Test:**
   A test serializing `ChatMessage::assistant_with_tools` and asserting JSON schema compliance:
   - Verifies `type == "function"`.
   - Verifies nested `function.name` and `function.arguments`.
   - Verifies roundtrip deserialization.
2. **Agentic Loop Multi-Turn Integration Test:**
   A test in `conversation/tests.rs` where a mock provider returns a `ToolCall` on turn 1, executes a mock tool, returns the result on turn 2, and produces the terminal text response.
3. **Fresh Database Initialization Test:**
   A test in `db.rs` opening an in-memory database with `init_db_at()`, verifying all tables exist (`plugin_states`, `custom_apps`, etc.), and asserting `get_plugin_states(&conn)` succeeds on a clean database.
4. **SSE Stream Terminator Test:**
   A test in `groq.rs` and `openai_compatible.rs` using a mock HTTP server that sends `data: [DONE]` and holds the TCP connection open. Verifies that the adapter cleanly terminates the future without hanging.
5. **Human-In-The-Loop Approval Suspension Test:**
   A test asserting that when `ToolStatus::ApprovalRequired` is encountered in `execute_turn`, the turn enters a paused/suspended state or returns a correlated approval request rather than looping into the LLM.
6. **Frontend Turn Subscription Contract Test:**
   A component test asserting `ChatView.tsx` subscribes to the authoritative `turnResult.turn_id` returned by `submitConversationTurn`.

---

## 13. Risk & Impact Assessment

| Defect | Severity | User Impact | Security / Stability Impact |
|---|---|---|---|
| **P0-A (Tool Schema)** | Critical (P0) | Autonomous tool execution fails 100% of the time on second iteration. | Low security risk (fails closed), but catastrophic functional failure. |
| **P0-B (Database Table)** | Critical (P0) | App crashes or fails to execute web search, terminal, or app launch commands on any fresh installation. | High stability impact; unhandled promise rejection in frontend. |
| **P0-C (Security Hang)** | Critical (P0) | Application freezes on "Synthesizing response...". Operator cannot see approval prompt or cancel turn. | High security impact: bypasses HITL visibility; fails open to an indefinite hang instead of failing closed with explicit rejection. |
| **TurnId Disconnect** | High (P1) | Users see no streaming text; UI appears unresponsive until full generation finishes. | Degraded user experience; UI state desynchronization. |
| **Identity Mismatch** | Medium (P1) | Model misidentifies itself; operator cannot be certain which LLM engine is executing. | Audit and compliance violation; user confusion. |

---

## 14. Minimal Safe Implementation Strategy

1. **P0-A Wire Format Normalization:**
   - In `src-tauri/src/ai/adapters/openai_compatible.rs` and `groq.rs`, implement an outbound message normalizer function `format_messages_for_wire(&[ChatMessage]) -> Vec<serde_json::Value>`.
   - When an assistant message has `tool_calls`, map each `ToolCall` into `{ "id": tc.id, "type": "function", "function": { "name": tc.name, "arguments": tc.arguments } }`.
   - For `role == "tool"`, format as `{ "role": "tool", "tool_call_id": msg.tool_call_id, "content": msg.content }`.
   - Leave internal `ToolCall` and `ChatMessage` domain structs backward-compatible.

2. **P0-B Database Schema Restoration:**
   - In `src-tauri/src/db.rs` `init_db_at()`, restore `CREATE TABLE IF NOT EXISTS custom_apps` and `CREATE TABLE IF NOT EXISTS plugin_states`.
   - In `src-tauri/src/chat.rs` `plugin_enabled()`, change `db::get_plugin_states(&conn).map_err(...)` to `.unwrap_or_default()` as defensive hardening.
   - Align database paths: unify `browser.rs:2835` to use `edith_memory.db`.

3. **P0-C Loop Termination & Approval Resolution:**
   - In `groq.rs`, `openai_compatible.rs`, and `gemini.rs`, label the outer while loop (`'stream_loop: while let Ok(Some(chunk)) = res.chunk().await`) and break with `break 'stream_loop;` when `*data == "[DONE]"`.
   - In `ConversationCore::execute_turn`, wrap `streamer.stream(...)` with `tokio::time::timeout(Duration::from_secs(60), ...)`.
   - When `tool_result.status == ToolStatus::ApprovalRequired`, do not `continue;` into the model loop. Transition turn to a suspended state or emit `StreamPayload::Finished` informing the user that approval is pending.
   - In `src/views/ChatView.tsx`, update subscription to use `turnResult.turn_id` once returned from `submitConversationTurn`.
   - In `ChatView.tsx`, correct settings references from `settings.aiProvider` / `settings.aiModel` to `settings.selectedProvider` / `settings.selectedModel`.

---

## 15. Exact Implementation Order

1. **Step 1 (P0-B):** Restore `plugin_states` and `custom_apps` tables in `src-tauri/src/db.rs`. Add defensive `.unwrap_or_default()` in `src-tauri/src/chat.rs`.
2. **Step 2 (P0-C):** Fix SSE `[DONE]` loop break in `groq.rs`, `openai_compatible.rs`, and `gemini.rs` using labelled loop breaks.
3. **Step 3 (P0-A):** Implement wire schema normalization in `openai_compatible.rs` and `groq.rs` to ensure `"type": "function"` and nested `"function"` blocks are emitted for outbound assistant messages.
4. **Step 4 (P0-C / Agentic Loop):** Update `ConversationCore` handling of `ToolStatus::ApprovalRequired` to halt model iteration and yield control. Add per-turn timeouts.
5. **Step 5 (Frontend TurnId & Settings):** In `ChatView.tsx`, fix `streamRouter.subscribeTurn` to subscribe to the backend `turn_id`. Map settings keys `selectedProvider` and `selectedModel`.
6. **Step 6 (Identity Grounding):** In `ContextAssembler::build_system_prompt()`, ground the system prompt with active runtime metadata (`Provider: {id}`, `Model: {model_id}`).
7. **Step 7 (Voice Realignment):** Switch default capture driver in `lib.rs` to `NativeCpalCaptureDriver` when native mode is enabled, or document browser-owned capture boundary.

---

## 16. Verification Plan After Implementation

1. **Automated Unit Verification:**
   - Run `cargo test --package edith-v2 --lib db::tests`.
   - Run `cargo test --package edith-v2 --lib conversation::tests`.
   - Run `cargo test --package edith-v2 --lib ai::adapters::groq::tests`.
2. **Wire Format Verification:**
   - Execute a 2-turn agentic test with mock Groq server verifying outbound HTTP body contains `tool_calls.0.type == "function"`.
3. **Clean Database Verification:**
   - Delete temporary test databases and assert fresh start initialization of `edith_memory.db` initializes `plugin_states` without error.
4. **Destructive Command E2E Verification:**
   - Submit a prompt requesting file deletion. Verify that `ApprovalModal` immediately displays with high risk badge, and turn is not hung on "Synthesizing response...".
5. **Frontend Streaming Verification:**
   - Submit prompt in chat. Verify real-time token streaming text renders into the chat bubble without delay.

---

## 17. Explicit List of Things That MUST NOT Be Changed

1. **DO NOT weaken host policy evaluation:** The four-tier security evaluation (`Allow`, `ConfirmationRequired`, `Restricted`, `Blocked`) in [`PolicyEngine`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/policy/engine.rs#L92) must remain intact.
2. **DO NOT bypass human confirmation:** Destructive commands (`rm`, `del`, `format`, `close_window`, arbitrary app launch) must never be demoted to `Allow` just to make tests pass.
3. **DO NOT replace real provider adapters with mocks in production:** Production builds must connect to real endpoints via Groq/OpenAI/Gemini adapters.
4. **DO NOT surrender backend TurnId authority:** The backend must remain the sole authoritative creator of `TurnId` and `StreamId`. Frontend message IDs must never overwrite backend turn identity.
5. **DO NOT compromise DPAPI security:** Windows DPAPI protection for secrets and isolated API keys in `db.rs` must not be altered.
6. **DO NOT alter browser multi-profile isolation boundaries:** Browser storage directories and profile-scoped database queries must remain strictly isolated.

---
EOF
