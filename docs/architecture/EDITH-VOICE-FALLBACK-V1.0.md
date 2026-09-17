# E.D.I.T.H. Fallback Voice Pipeline Architecture Specification (v1.0)
## Phase 9: STT → ConversationCore → LLM → TTS

---

## 1. Executive Architecture Summary

Phase 9 establishes the turn-based **Fallback Voice Pipeline** for E.D.I.T.H.
It provides robust, spoken dialogue interaction over the existing `ConversationCore`, `PolicyEngine`, and `Universal Tool Runtime`:

$$\text{Microphone} \longrightarrow \text{Audio Capture} \longrightarrow \text{STT} \longrightarrow \text{Transcript} \longrightarrow \text{ConversationCore} \longrightarrow \text{LLM / Tools} \longrightarrow \text{Assistant Response} \longrightarrow \text{TTS} \longrightarrow \text{Audio Output} \longrightarrow \text{Speaker}$$

### Key Architectural Invariants:
1. **ConversationCore Turn Convergence**: Voice input does **NOT** spawn a secondary voice agent or bypassing LLM loop. Spoken queries normalize into transcripts and enter `ConversationCore::submit_turn`.
2. **Backend Authoritative TurnId**: The backend `ConversationCore` authoritatively assigns the `TurnId`. The active `VoiceSession` associates and tracks this `TurnId` only after submission.
3. **Single Authoritative Playback Sink**: Rust `AudioOutputDriver` (managing Rodio) is the **sole hardware audio output owner**. Dual-path playback (simultaneous Rodio and browser HTML5 Audio) is permanently eliminated.
4. **Dual-Mode STT Modality**:
   - **Mode A (Web Speech Bridge)**: Client-side WebView2 `SpeechRecognition` normalizes directly into a `Transcript` contract without synthesizing dummy audio buffers.
   - **Mode B (Raw-Audio STT)**: Hardware `AudioCaptureDriver` captures `AudioBuffer` frames and transcribes via remote or local speech recognition adapters (`STTAdapter`).
5. **Single Capture Owner Exclusivity**: There is never concurrent capture between WebView2 and the backend. When browser capture is active, the backend recording device remains closed. When native capture is active, browser capture is deactivated.
6. **Cooperative Interruption (Barge-in)**: Triggering a new voice turn while E.D.I.T.H. is synthesizing or speaking immediately cancels active synthesis and halts hardware playback, preventing overlapping speech.
7. **Phase 10 Boundary**: Full-duplex realtime speech-to-speech (S2S), WebRTC, and bidirectional audio streaming are strictly preserved for Phase 10.

---

## 2. Voice Pipeline Sequence

```
User               Client UI               VoiceController         ConversationCore           TTS & Audio
 │                     │                          │                       │                        │
 │──[Speaks Input]────>│                          │                       │                        │
 │                     │──[onresult: transcript]─>│                       │                        │
 │                     │                          │──[submit_turn]───────>│                        │
 │                     │                          │<─[authoritative TurnId]│                       │
 │                     │                          │                       │                        │
 │                     │                          │──[execute_turn]──────>│                        │
 │                     │                          │                       ├──[Policy / Tools]      │
 │                     │                          │                       │                        │
 │                     │                          │<─[text response]──────│                        │
 │                     │                          │                                                │
 │                     │                          │──[synthesize]─────────────────────────────────>│
 │                     │                          │<─[canonical AudioBuffer]───────────────────────│
 │                     │                          │                                                │
 │                     │                          │──[play(AudioBuffer)]──────────────────────────>│
 │<────────────────────┴──────────────────────────┴────────────────────────────────────────────────┼──[Hardware Speaker]
```

---

## 3. Canonical Audio Format

To ensure interchangeability across STT models, VAD, and speech synthesis, the voice subsystem enforces a standardized internal representation:

| Parameter | Specification | Purpose |
|:---|:---|:---|
| **STT Sample Rate** | `16,000 Hz` | Industry standard for ASR models (Whisper, Vosk, SAPI) |
| **TTS Sample Rate** | `24,000 Hz` | High-fidelity vocal clarity for EdgeTTS and Kokoro |
| **Channel Count** | `1` (Mono) | Voice interaction is strictly monaural |
| **Sample Format** | Linear PCM `f32` (`[-1.0, 1.0]`) | Floating-point dynamic range prevents clipping during scaling |
| **Wire Encoding** | 16-bit signed integer (LE) | Standard conversion for cloud APIs and PCM byte buffers |
| **Frame Duration** | `20 ms` (320 samples @ 16kHz) | Standard packetization chunk |

### `AudioBuffer` Structure:
```rust
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}
```
Provides helpers:
- `to_mono(&self) -> AudioBuffer`: Averages interleaved multi-channel frames to mono.
- `to_i16_pcm(&self) -> Vec<u8>`: Converts normalized `f32` samples to little-endian 16-bit PCM bytes.
- `from_i16_pcm(bytes, sr, ch) -> AudioBuffer`: Decodes raw PCM bytes into normalized `f32`.
- `resample_linear(&self, target_sr) -> AudioBuffer`: Standard linear interpolation resampler.

---

## 4. Audio Capture Architecture & Ownership

Microphone acquisition enforces strict mutual exclusion:
```rust
pub enum CaptureOwner {
    BrowserWebSpeech,
    NativeDriver,
    None,
}
```

- **Browser-Active Mode (Default Desktop Mode)**:
  - WebView2 owns the microphone device.
  - The backend `AudioCaptureDriver` (`BrowserCaptureBridge`) tracks state and forbids the backend from opening the hardware recording device simultaneously.
  - Web Speech events are ingested and normalized directly via `WebSpeechSTTBridge`.
- **Native-Active Mode (Raw Audio)**:
  - Backend driver (`MockAudioCaptureDriver` / OS audio driver) acquires the recording device.
  - Browser `SpeechRecognition` is disabled.
  - Produces raw `AudioBuffer` frames dispatched to `STTAdapter::transcribe`.

---

## 5. STT Abstraction

The STT subsystem defines two distinct input modalities without fabricating dummy audio buffers:

### Mode A: Web Speech Transcript Bridge
```rust
pub struct WebSpeechSTTBridge;

impl WebSpeechSTTBridge {
    pub fn normalize_transcript(
        raw_text: String,
        confidence: Option<f32>,
        language: Option<String>,
        is_final: bool,
    ) -> Result<Transcript, VoiceError>;
}
```
Validates text, clamps confidence, normalizes BCP-47 language tags, and fails closed on empty speech (`VoiceError::EmptyTranscript`).

### Mode B: Raw-Audio STT Adapter
```rust
pub trait STTAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioBuffer,
        options: &'a STTOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Transcript, VoiceError>> + Send + 'a>>;
}
```
Implementations:
- `CloudSTTAdapter`: Encodes `AudioBuffer` into WAV/PCM and queries Whisper-compatible APIs.
- `MockSTTAdapter`: Deterministic mock supporting simulated delays and cancellation checks for unit and integration testing.

---

## 6. TTS Abstraction & Single Playback Sink

```rust
pub trait TTSAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        options: &'a TTSOptions,
        cancellation: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AudioBuffer, VoiceError>> + Send + 'a>>;
    fn list_voices<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceDescriptor>, VoiceError>> + Send + 'a>>;
}
```

Implementations:
- `EdgeTtsAdapter`: Integrates Microsoft Azure EdgeTTS via `edge_tts_rust`, decoding MP3 streams directly into canonical `AudioBuffer` frames.
- `MockTtsAdapter`: Generates 440 Hz test tones for headless testing without external network calls.
- `LocalTtsAdapter`: Stubs Kokoro ONNX model synthesis gracefully.

### Elimination of Duplicate Audio Playback
Previous versions suffered from a race defect:
- Rust `tts.rs` played audio via Rodio.
- Concurrently, `tts_speak` returned base64-encoded audio to `tauri.ts`, which played it through browser `new Audio(...)`.

**Resolution**:
1. `RodioAudioOutputDriver` is declared the **sole authoritative hardware output owner**.
2. In Tauri desktop mode, `src/services/tauri.ts` delegates hardware playback entirely to the host audio output driver and no longer instantiates an HTML5 `Audio` element.
3. Legacy `tts_speak` returns an empty string `""` so any un-migrated code cannot trigger browser playback.

---

## 7. VoiceSession & Authoritative Turn Lifecycle

A `VoiceSession` represents an ephemeral voice turn:
```rust
pub struct VoiceSession {
    pub id: VoiceSessionId,
    pub conversation_id: ConversationId,
    pub turn_id: Option<TurnId>, // Assigned only after ConversationCore creates the turn
    pub state: VoiceSessionState,
    pub cancellation_token: CancellationToken,
    pub started_at: Instant,
    pub capture_owner: CaptureOwner,
    pub stt_provider: String,
    pub tts_provider: String,
}
```

### Turn Lifecycle Progression:
1. `VoiceController::start_session(conversation_id, capture_owner)`:
   - Checks if previous session is `Speaking` or `Synthesizing`. If so, triggers **Barge-in**!
   - Initializes `VoiceSession` with `turn_id = None`.
   - Transitions state to `Listening`.
   - Emits `VoicePayload::SessionStarted` and `VoicePayload::StateChanged { state: "listening" }`.
2. Input submission (`submit_web_speech_transcript` or `submit_raw_audio`):
   - Releases capture driver.
   - Transitions state to `CoreExecution`.
   - Invokes `ConversationCore::submit_turn(...)`.
   - Extracts backend-generated **authoritative `TurnId`** and stores it in `session.turn_id = Some(turn_id)`.
   - Invokes `ConversationCore::execute_turn(&authoritative_turn_id, credentials)`.
3. Synthesis:
   - Transitions state to `Synthesizing`.
   - Invokes `TTSAdapter::synthesize(...)`.
4. Playback:
   - Transitions state to `Speaking`.
   - Dispatches audio buffer to `AudioOutputDriver::play(...)`.

---

## 8. Turn Interruption / Barge-in

When the user initiates recording or speaks while E.D.I.T.H. is in `Synthesizing` or `Speaking` state:
1. `VoiceController::start_session` triggers barge-in:
   - Active session's `CancellationToken` is cancelled.
   - `AudioOutputDriver::stop()` halts soundcard playback immediately.
   - Active capture driver is cancelled and released.
   - Emits `EdithPayload::Voice(VoicePayload::BargeInTriggered { interrupted_source: "tts_playback" })`.
   - Emits `VoicePayload::SessionEnded { reason: "interrupted_by_user_barge_in" }`.
2. New `VoiceSession` acquires capture and begins recording cleanly.

---

## 9. Cooperative Cancellation Model

Cancellation is propagated using the existing `crate::task::CancellationToken` hierarchy:
- Root `VoiceSession` token.
- Child operations observe the token:
  - STT transcription checks `cancellation.is_cancelled()`.
  - `ConversationCore::cancel_turn(turn_id)` is invoked if a turn ID exists.
  - TTS synthesis checks `cancellation.is_cancelled()`.
  - `AudioOutputDriver::stop()` flushes buffers immediately.

> **Cancellation Guarantee**: Cancellation is propagated cooperatively as promptly as the underlying adapter, network, or audio subsystem permits; tests verify that cancellation is observed and obsolete work does not continue.

---

## 10. Runtime-State Integration & Security

### Read Projection (`EdithRuntimeState`)
Voice state is exposed to self-inspection and telemetry strictly as a sanitized read projection:
```rust
pub struct VoiceStatusSummary {
    pub is_active: bool,
    pub state: String,
    pub session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub stt_provider: String,
    pub tts_provider: String,
    pub is_muted: bool,
    pub last_error: Option<String>,
}
```
- Raw PCM audio samples are **never** stored in runtime state.
- Transcripts are processed through `scrub_sensitive_text()` before logging or projection.
- STT/TTS credentials originate from `SettingsCredentialStore` and are never serialized into events.

---

## 11. Verification Matrix

| Test Identifier | Scope | Status |
|:---|:---|:---:|
| `test_audio_buffer_normalization` | Clamping, duration, mono conversion, i16 PCM round-trip | **PASS** |
| `test_audio_buffer_linear_resampling` | 16 kHz to 24 kHz linear interpolation resampling | **PASS** |
| `test_web_speech_bridge_normalization` | Validates clean transcripts and rejects empty speech | **PASS** |
| `test_stt_adapter_contract` | Mock STT transcription, options passing, and cancellation | **PASS** |
| `test_tts_adapter_contract` | Mock TTS audio synthesis, options parsing, and cancellation | **PASS** |
| `test_capture_ownership_exclusivity` | Enforces single active capture owner and rejects conflicts | **PASS** |
| `test_duplicate_playback_prevention_and_stop` | Single output sink verification and immediate stop behavior | **PASS** |
| `test_conversation_core_convergence_and_turn_id` | Full mock pipeline: Voice $\to$ STT $\to$ ConversationCore (authoritative TurnId) $\to$ TTS $\to$ Speaker | **PASS** |
| `test_barge_in_interruption` | Halts TTS playback and emits `BargeInTriggered` on new turn | **PASS** |
| `test_cooperative_cancellation` | Token cancellation propagation across active voice session | **PASS** |

---

## 12. Phase 10 Boundary (Non-Goals for Phase 9)

Phase 9 strictly implements the turn-based fallback voice pipeline. The following features belong exclusively to Phase 10:
- Full-duplex realtime speech-to-speech (S2S) streaming.
- WebRTC media transport and peer connections.
- WebSocket-based bidirectional audio streaming (e.g. Gemini Multimodal Live API, OpenAI Realtime API).
- Streaming server-side Voice Activity Detection (VAD).
- Mid-sentence live conversational barge-in without turn completion.
