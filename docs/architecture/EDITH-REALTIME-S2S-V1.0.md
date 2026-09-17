# E.D.I.T.H. Realtime Duplex Speech-to-Speech (S2S) Architecture Specification (v1.0)
## Phase 10: Realtime Audio Transport, Streaming Voice Engine, & Fallback Coexistence

---

## 1. Executive Architecture Summary

Phase 10 introduces native **Full-Duplex Speech-to-Speech (S2S)** capability to E.D.I.T.H., enabling natural, conversational interaction with ultra-low latency, bidirectional audio streaming, real-time interruptions (barge-in), and conversational tool invocation, while rigorously preserving the turn-based **Phase 9 Fallback Voice Pipeline** (`STT → ConversationCore → LLM → TTS`).

### Coexisting Dual-Pipeline Architecture:

```
                                      ┌─────────────────────────────────────────────────────────┐
                                      │                   PATH A: REALTIME S2S                  │
                                      │                                                         │
                                      │   Microphone Capture Driver                             │
                                      │              │                                          │
                                      │              ▼ (Raw PCM Frames)                         │
                                      │     AudioFrameTransport (WebSocket / WebRTC)            │
                                      │              │                                          │
                                      │              ▼ (Duplex Stream)                          │
                                      │     Realtime Audio Provider (e.g., Gemini Live S2S)     │
                                      │              │                                          │
                                      │      ┌───────┴────────┐                                 │
                                      │      │ Tool Calls     │ Streaming Audio Frames          │
                                      │      ▼                ▼                                 │
                                      │     UTR / Policy   AudioOutputDriver (Rodio)            │
                                      │      │                │                                 │
                                      └──────┼────────────────┼─────────────────────────────────┘
                                             │                │
                                      ┌──────┼────────────────┼─────────────────────────────────┐
                                      │      │                │                                 │
                                      │      ▼                ▼                                 │
                                      │   Authoritative ConversationCore   Hardware Speaker     │
                                      │      ▲                                                  │
                                      │      │                                                  │
                                      │      │ (Transcripts / Turn Finalization)                │
                                      │      │                                                  │
                                      │   STT Adapter / Web Speech Bridge                       │
                                      │      ▲                                                  │
                                      │      │                                                  │
                                      │   Microphone (Fallback Mode)                            │
                                      │                                                         │
                                      │                  PATH B: FALLBACK VOICE                 │
                                      └─────────────────────────────────────────────────────────┘
```

---

## 2. The 18 Non-Negotiable Architectural Invariants

| # | Invariant | Enforcement Mechanism |
|:---|:---|:---|
| **1** | **Strict ConversationCore Turn Ownership** | `RealtimeVoiceEngine` possesses **zero authority** to invent independent turns. Logical user utterances allocate an authoritative `TurnId` via `ConversationCore::start_realtime_turn`. Normal completion persists through `ConversationCore::complete_realtime_turn`. |
| **2** | **Universal Tool Runtime (UTR) Enforcement** | Realtime providers **never** execute tools directly. Inbound tool calls from the model are converted to `ToolRequest` with the active `TurnId`, evaluated against `PolicyEngine`, and executed by `ToolRouter`. Results are fed back over the realtime session. |
| **3** | **Single Microphone Capture Owner** | Hardware microphone capture enforces mutual exclusion via `AudioCaptureDriver`. Path A and Path B can never capture concurrently. |
| **4** | **Single Playback Driver Owner** | The Rust `AudioOutputDriver` (Rodio) is the **sole** hardware soundcard sink. All assistant audio frames route through this sink. |
| **5** | **Decoupled Transport Neutrality** | Realtime streaming is decoupled behind `AudioFrameTransport`. WebSocket, WebRTC, IPC, and Mock implementations conform to the identical contract. |
| **6** | **Dual Backpressure Control** | Output: Stale assistant audio (`generation_id < active_generation_id`) is discarded immediately. Input: Live mic frames are buffered intact; if backpressure exceeds timeout (>500ms stall), an explicit Stream Discontinuity is triggered, invalidating the turn and gracefully falling back to Phase 9. |
| **7** | **Authoritative Barge-in & Interruption** | User speech interrupts active playback within milliseconds: `AudioOutputDriver::stop_immediately()` halts hardware output, active turn in `ConversationCore` is cancelled, and `generation_id` increments. |
| **8** | **Stale Audio Drop by Generation** | In-flight assistant audio packets arriving over the network tagged with older generation IDs are dropped prior to the playback driver. |
| **9** | **Phase 9 Pipeline Preservation** | Phase 9 fallback (`STT → ConversationCore → LLM → TTS`) remains fully intact with zero regressions and operates as the automated fallback on network or provider failure. |
| **10** | **Monotonic Audio Sequencing** | Every input and output `AudioFrame` carries a monotonically increasing sequence number per session direction. |
| **11** | **Canonical Audio Frame Model** | Input audio is normalized to 16kHz mono `f32`; output audio is normalized to 24kHz mono `f32`. Linear resampling is applied when necessary. |
| **12** | **Zero Credential Leaking** | Realtime session configuration and provider authentication tokens are secured in the backend Rust environment and never exposed over frontend events or IPC payload dumps. |
| **13** | **Correlated Event Taxonomy** | All realtime voice events use `EventCorrelation::for_voice` tagged with `session_id`, `conversation_id`, and authoritative `turn_id`. |
| **14** | **Bounded Reconnection Strategy** | Disconnections perform exponential backoff up to `max_reconnect_attempts`. If reconnection fails, automatic fallback to Phase 9 is triggered. |
| **15** | **No Locks Held Across Await** | All synchronization primitives (`RwLock`, `Mutex`) release guard locks prior to asynchronous network or hardware calls. |
| **16** | **Cooperative Cancellation** | Stopping a realtime voice session stops capture, flushes audio buffers, stops playback, and cancels any in-flight turns cleanly. |
| **17** | **Voice Status State Projection** | `VoiceController` and Tauri IPC project unified `VoiceStatusSummary` including `mode: "realtime" | "fallback"`, provider ID, transport type, and active turn ID. |
| **18** | **Graceful Fallback Handover** | Triggering fallback cleanly stops the realtime engine, releases capture locks, and transitions the session to Path B without audio glitches or orphan processes. |

---

## 3. Audio Frame & Transport Layer

### Canonical `AudioFrame` Structure

```rust
pub struct AudioFrame {
    pub session_id: VoiceSessionId,
    pub sequence_number: u64,
    pub timestamp_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
    pub direction: FrameDirection,
    pub generation_id: u64,
}
```

- **Sample Rate**: 16,000 Hz for input capture (ASR standard); 24,000 Hz for output playback (high-fidelity neural synthesis).
- **Format**: Normalized `f32` samples in `[-1.0, 1.0]`.
- **Direction**: `FrameDirection::Input` (mic → provider) or `FrameDirection::Output` (provider → speaker).
- **RMS Energy**: Calculated via `calculate_rms()` for visualizer feedback and client silence detection.

### `AudioFrameTransport` Abstraction

Decouples network socket implementation from the audio processing pipeline:

```rust
pub trait AudioFrameTransport: Send + Sync {
    fn connect<'a>(&'a self, url: &'a str, auth_token: Option<&'a str>)
        -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;
    
    fn send_frame<'a>(&'a self, frame: AudioFrame)
        -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;
    
    fn recv_event<'a>(&'a self)
        -> Pin<Box<dyn Future<Output = Result<Option<TransportEvent>, VoiceError>> + Send + 'a>>;
    
    fn close<'a>(&'a self)
        -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;
    
    fn state<'a>(&'a self)
        -> Pin<Box<dyn Future<Output = TransportState> + Send + 'a>>;
}
```

---

## 4. Authoritative ConversationCore Turn Ownership

Realtime conversations maintain strict compliance with E.D.I.T.H.'s Phase 3 architecture:

```
Realtime Voice Engine                ConversationCore                    Policy / UTR
       │                                     │                                 │
       ├──[User utterance detected]          │                                 │
       │                                     │                                 │
       ├──[start_realtime_turn()]───────────>│                                 │
       │<─[Authoritative TurnId]─────────────│                                 │
       │                                     │                                 │
       ├──[Deltas: transcript/audio]         │                                 │
       │                                     │                                 │
       ├──[Tool Call from Provider]          │                                 │
       │  (carries TurnId)                   │                                 │
       │──────────────────────────────────────────────────────────────────────>│
       │<──────────────────────────────────────────────────────────────────────┤
       │                                     │                                 │
       ├──[Turn Finished]                    │                                 │
       ├──[complete_realtime_turn()]────────>│                                 │
       │                                     ├──[Persist message to DB]        │
       │                                     └──[Mark Turn Completed]          │
```

### Turn Lifecycle Invariants:
1. **No Parallel Identity**: `RealtimeVoiceSession` only caches `active_turn_id: Arc<RwLock<Option<TurnId>>>`. It never creates turn IDs independently.
2. **Interruption / Barge-in**: When user speech interrupts an active assistant response, `ConversationCore::cancel_turn(turn_id)` is invoked. The turn transitions to `Cancelled`, and the next user utterance requests a fresh `TurnId`.
3. **Database Convergence**: Upon completion, user transcripts and assistant replies are saved to SQLite via `save_session_message`, ensuring session history is unified across voice and text interactions.

---

## 5. Universal Tool Runtime (UTR) Integration

Realtime voice models operate purely as reasoning engines and tool callers; they are **never** permitted to execute tools directly.

### Execution Flow:
1. Provider emits `TransportEvent::ToolCall { call_id, tool_name, arguments }`.
2. Engine ensures active turn exists (`ensure_active_turn`).
3. Engine constructs a correlated `ToolRequest`:
   ```rust
   let tool_request = ToolRequest::new(tool_name, arguments, correlation)
       .with_execution_id(ToolExecutionId::from_string(exec_id));
   ```
4. `ToolRouter::execute` evaluates `PolicyEngine`:
   - If `Allow`: Executor executes immediately; result serialized and returned to provider via `adapter.send_tool_result`.
   - If `ConfirmationRequired`: Human approval requested; session pauses or informs user.
   - If `Blocked`: Security block returned; tool execution prevented.

---

## 6. VAD, Barge-in, and Stale Audio Drop

To achieve instantaneous conversational responsiveness:

1. **Barge-in Trigger**:
   - Provider emits `RealtimeProviderEvent::Interrupted`.
   - OR client VAD / Engine input frame energy exceeds threshold during playback.
2. **Generation Counter**:
   - `RealtimeVoiceSession` increments `generation_id: AtomicU64`.
3. **Immediate Hardware Halt**:
   - `AudioOutputDriver::stop_immediately()` drops all pending audio in the hardware sink.
4. **Stale Frame Filter**:
   - Inbound assistant audio frames check:
     ```rust
     if frame.generation_id < session.current_generation() {
         // Discard stale audio
         return;
     }
     ```
5. **Turn Cancellation**:
   - The active turn is cancelled in `ConversationCore`.

---

## 7. Dual Backpressure Control

Audio streaming over unstable network connections requires asymmetric backpressure strategies:

### Output Backpressure (Provider → Speaker)
- Stale or superseded audio frames have zero value to the user once interrupted.
- Any frame with `generation_id < active_generation` is immediately discarded without buffering.

### Input Backpressure (Microphone → Provider)
- Dropping random frames from live user speech destroys acoustic phonemes and causes severe ASR corruption.
- Input frames are buffered intact up to a bounded jitter buffer limit (~640ms / 32 frames).
- If the transport blocks and timeouts exceed `backpressure_timeout_ms` (500ms):
  1. An explicit **Stream Discontinuity** is declared.
  2. The current in-flight turn is cancelled.
  3. `RealtimeFallbackTriggered` event is emitted.
  4. The engine initiates controlled fallback to the Phase 9 pipeline.

---

## 8. Verification & Test Suite

The realtime voice subsystem is validated by an automated unit and integration test suite (`src-tauri/src/voice/realtime/tests.rs`):

- **Test 01**: `test_01_realtime_capability_detection` — Capability flag detection and downcast verification.
- **Test 02**: `test_02_transport_abstraction_lifecycle` — Full transport connect, frame send, event receive, and close lifecycle.
- **Test 03**: `test_03_authoritative_turn_ownership_in_conversation_core` — Verification that `start_realtime_turn` and `complete_realtime_turn` manage lifecycle strictly in `ConversationCore`.
- **Test 04**: `test_04_audio_frame_monotonic_sequence_and_duration` — Monotonic sequence counter and frame duration verification.
- **Test 05**: `test_05_stale_assistant_audio_rejection_on_generation_increment` — Confirmation that stale frames are discarded when generation increments.
- **Test 06**: `test_06_barge_in_cancels_turn_in_conversation_core` — Verification that barge-in transitions active turn to `Cancelled` and clears session turn.
- **Test 07**: `test_07_realtime_tool_call_delegation_to_universal_tool_runtime` — Confirmation that provider tool calls route through `ToolRouter`.
- **Test 08**: `test_08_input_backpressure_timeout_triggers_stream_discontinuity` — Verification that network stalls trigger stream discontinuity and signal fallback.
- **Test 09**: `test_09_mutual_exclusion_of_microphone_capture` — Mutual exclusion of microphone capture driver between concurrent sessions.
- **Test 10**: `test_10_cooperative_cancellation_propagates_cleanly` — Clean cooperative cancellation of sessions, capture, and audio output.

---

## 9. Phase 11 Boundary

The following capabilities are reserved for Phase 11 and beyond:
- Native local audio neural network inference (Whisper ONNX / Kokoro Rust embedded).
- Custom hardware wake-word neural models.
- Multimodal video and screen-share frame injection into duplex sessions.
