# P0 Agentic Runtime Stabilization — Implementation Report

**Branch:** `feature/p0-agentic-runtime-stabilization`  
**Date:** October 2, 2026  
**Status:** Completed & Verified  

---

## 1. Executive Summary

This report documents the resolution and verification of all P0 runtime defects identified during the E2E investigation of the E.D.I.T.H. installed package. The agentic execution pipeline from initial user input through LLM generation, tool execution, operator approval, and terminal response streaming is now completely stabilized, validated by unit and regression test suites across Rust and TypeScript.

---

## 2. Issues Addressed & Root Causes Resolved

| ID | Issue | Verified Root Cause | Implementation Fix |
|---|---|---|---|
| **P0-A** | `messages.X.tool_calls.0.type is missing` | Domain `ToolCall` serialized as a flat `{id, name, arguments}` object. Outbound serialization directly passed `req.messages` to OpenAI/Groq REST APIs without converting assistant tool calls to standard wire schema `{"type": "function", "function": {"name": ..., "arguments": ...}}`. | Added `format_messages_for_openai_wire` in [`src-tauri/src/ai/provider.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/provider.rs) and `format_messages_for_gemini_wire` in [`src-tauri/src/ai/adapters/gemini.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/gemini.rs). Integrated into `groq.rs`, `openai_compatible.rs`, and `gemini.rs` for both text generation and streaming. |
| **P0-B** | `no such table: plugin_states` & DB Mismatch | Commit `94258e0` accidentally omitted table creation for `plugin_states` and `custom_apps` in `init_db_at()`. Fresh databases failed on startup during `get_plugin_states()`. Furthermore, [`browser.rs:2835`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/browser.rs) referenced `edith.db` instead of canonical `edith_memory.db`. | Restored `CREATE TABLE IF NOT EXISTS custom_apps` and `plugin_states` in `init_db_at()` in [`src-tauri/src/db.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs). Unified database path in `browser.rs` to `edith_memory.db`. Hardened `plugin_enabled()` in [`src-tauri/src/chat.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs) with `.unwrap_or_default()` defensive fallback. |
| **P0-C** | Indefinite `"Synthesizing response..."` Hang | In `groq.rs`, `openai_compatible.rs`, and `gemini.rs`, an unlabeled `break;` inside `for line in chunk_str.lines()` on `data: [DONE]` only exited the line loop, leaving the outer `while let Ok(Some(chunk)) = res.chunk().await` suspended indefinitely on HTTP keep-alive connections. Also, turns lacked generation timeouts. | Labeled the outer while loop `'stream_loop: while let Ok(Some(chunk)) = res.chunk().await` and invoked `break 'stream_loop;` on `[DONE]`. Added `emitted_done` tracking to guarantee single terminal chunk delivery. Wrapped streaming and generation futures in [`core.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs) with a 60-second `tokio::time::timeout`. |
| **P0-C / Agentic Loop** | Approval Modal Bypass / Indefinite Spin | When `PolicyEngine` returned `ConfirmationRequired`, `core.rs` formatted `{"status": "approval_required"}` as a tool result and immediately called `continue;`, re-prompting the LLM without human authorization. | Updated [`core.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs) to detect `ToolStatus::ApprovalRequired`, immediately halt agentic model iteration, emit terminal stream event, format human-in-the-loop notice to the user, and transition cleanly to `Completed` without re-prompting the LLM. |
| **TurnId Disconnect** | Stream Events Dropped by Frontend | `ChatView.tsx` subscribed to client timestamp `assistantMsgId` (`msg-172...`), while the backend authored and emitted with UUID `turn_id` (`turn-9e8a...`). `streamRouter.ts` dropped all chunks due to ID mismatch. | Refactored [`src/views/ChatView.tsx`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx) to switch its stream subscription to the authoritative `turnResult.turn_id` returned by `submitConversationTurn`. |
| **Settings Mapping** | Selected Provider/Model Ignored | `ChatView.tsx` accessed `settings.aiProvider` and `settings.aiModel` (which were `undefined`), defaulting all turns to `groq` / `llama-3.3-70b-versatile` regardless of user configuration in Settings. | Updated `ChatView.tsx` to read `settings.selectedProvider || settings.aiProvider` and `settings.selectedModel || settings.aiModel`. |
| **Identity Grounding** | Model Self-Identity Hallucination | Models like `openai/gpt-oss-120b` claimed to be `GPT-4-Turbo` because `ContextAssembler` provided no runtime grounding metadata in system instructions. | Added `build_system_prompt_with_identity` and `assemble_messages_with_model` in [`src-tauri/src/conversation/context.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/context.rs), grounding active `provider_id` and `model_id` in system prompts. |

---

## 3. Modified Files Summary

1. **[`src-tauri/src/db.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/db.rs)**
   - Restored `custom_apps` and `plugin_states` table creation inside `init_db_at()`.
   - Added unit test module `db::tests` testing clean creation, CRUD, and non-destructive self-healing of legacy databases.
2. **[`src-tauri/src/browser.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/browser.rs)**
   - Changed download DB connection target from `edith.db` to canonical `edith_memory.db`.
3. **[`src-tauri/src/chat.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/chat.rs)**
   - Hardened `plugin_enabled()` with `.unwrap_or_default()` to prevent database glitches from crashing the chat command routing.
4. **[`src-tauri/src/ai/provider.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/provider.rs)**
   - Added `format_messages_for_openai_wire(&[ChatMessage]) -> Vec<serde_json::Value>` ensuring assistant tool calls emit `"type": "function"` and nested `"function"` blocks, and tool results emit `"role": "tool"`.
   - Added unit tests `test_format_messages_for_openai_wire_assistant_tool_calls` and `test_format_messages_for_openai_wire_tool_result`.
5. **[`src-tauri/src/ai/adapters/groq.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/groq.rs)**
   - Used `format_messages_for_openai_wire` in `generate` and `stream`.
   - Labeled outer stream loop `'stream_loop` and broke cleanly on `[DONE]` with terminal chunk emission.
6. **[`src-tauri/src/ai/adapters/openai_compatible.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/openai_compatible.rs)**
   - Used `format_messages_for_openai_wire` in `generate` and `stream`.
   - Labeled outer stream loop `'stream_loop` and broke cleanly on `[DONE]` with terminal chunk emission.
7. **[`src-tauri/src/ai/adapters/gemini.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/ai/adapters/gemini.rs)**
   - Added `format_messages_for_gemini_wire` mapping function names via `to_gemini_name`.
   - Labeled outer stream loop `'stream_loop` and broke cleanly on `[DONE]`.
8. **[`src-tauri/src/conversation/context.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/context.rs)**
   - Added `build_system_prompt_with_identity` and `assemble_messages_with_model` injecting runtime provider and model metadata.
9. **[`src-tauri/src/conversation/core.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/core.rs)**
   - Connected `assemble_messages_with_model` with active `model_selection`.
   - Added 60-second timeouts around `stream` and `generate` operations.
   - Halted agentic loop on `ToolStatus::ApprovalRequired` to await operator authorization.
10. **[`src-tauri/src/conversation/tests.rs`](file:///E:/Projects/E.D.I.T.H/src-tauri/src/conversation/tests.rs)**
    - Updated `test_agentic_computer_and_approval_flow` to verify that the LLM is never called a second time after an action requiring approval is withheld.
11. **[`src/views/ChatView.tsx`](file:///E:/Projects/E.D.I.T.H/src/views/ChatView.tsx)**
    - Switched stream subscription to authoritative `turnResult.turn_id`.
    - Mapped settings references to `settings.selectedProvider` and `settings.selectedModel`.

---

## 4. Verification & Test Execution Results

### A. Provider Wire Schema Tests
```text
running 5 tests
test ai::provider::tests::test_chat_message_tool_result_format ... ok
test ai::provider::tests::test_format_messages_for_openai_wire_tool_result ... ok
test ai::provider::tests::test_format_messages_for_openai_wire_assistant_tool_calls ... ok
test ai::provider::tests::test_generate_request_with_tools_json ... ok
test ai::provider::tests::test_tool_call_serialization ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s
```

### B. Database Initialization & Self-Healing Tests
```text
running 4 tests
test db::tests::test_custom_apps_crud ... ok
test db::tests::test_plugin_states_crud ... ok
test db::tests::test_fresh_db_initialization_contains_plugin_states_and_custom_apps ... ok
test db::tests::test_database_self_healing_missing_tables ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; finished in 1.83s
```

### C. Conversation Core & Agentic Flow Tests
```text
running 13 tests
test conversation::tests::tests::test_agentic_tool_calling_loop ... ok
test conversation::tests::tests::test_authoritative_stream_id_belongs_to_turn ... ok
test conversation::tests::tests::test_agentic_computer_and_approval_flow ... ok
test conversation::tests::tests::test_cancellation_uses_correct_stream_id_and_single_terminal_event ... ok
test conversation::tests::tests::test_client_id_cannot_become_authoritative_turn_id ... ok
test conversation::tests::tests::test_concurrent_turn_isolation ... ok
test conversation::tests::tests::test_context_assembly_with_memory_and_profile ... ok
test conversation::tests::tests::test_normalized_error_propagation ... ok
test conversation::tests::tests::test_provider_routing_and_streaming_correlation ... ok
test conversation::tests::tests::test_turn_lifecycle_invalid_transitions_rejected ... ok
test conversation::tests::tests::test_turn_creation_authoritative_turn_id ... ok
test conversation::tests::tests::test_turn_lifecycle_valid_transitions ... ok
test conversation::tests::tests::test_turn_scoped_cancellation ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; finished in 0.02s
```

### D. Full-Stack Builds
- **Rust Backend:** `cargo check` completed with code `0`.
- **Frontend Typecheck & Bundle:** `npx tsc --noEmit && vite build` built 1877 modules into production assets with code `0`.

---

## 5. Architectural Invariants Maintained

- **Security & Policy:** `PolicyEngine` evaluation was not bypassed or weakened; human-in-the-loop confirmation was strengthened by halting model execution when an approval is pending.
- **Authority:** Backend `TurnId` and `StreamId` authority was strictly preserved; the client now adheres to the backend's authoritative IDs.
- **DPAPI & Secrets:** Windows DPAPI protection for API keys in the SQLite database remains completely intact.
- **Browser Profile Isolation:** Unifying `browser.rs` to `edith_memory.db` preserves all download tracking alongside application data without compromising profile isolation.
- **No Production Mocks:** All production code utilizes real implementations and typed protocol encoders.
