use crate::ai::CredentialStore;
use crate::db::{self, DbState};
use crate::llm::ChatMessage;
use crate::plugins;
use serde::{Deserialize, Serialize};
use tauri::command;
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct ChatHistoryItem {
    pub role: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatResponse {
    pub response: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
}

impl ChatResponse {
    pub fn new(response: String, r#type: String) -> Self {
        Self {
            response,
            r#type,
            stream_id: None,
            turn_id: None,
        }
    }

    pub fn with_stream(mut self, stream_id: impl Into<String>, turn_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_turn(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }
}

fn plugin_enabled(state: &DbState, plugin_id: &str) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let saved = db::get_plugin_states(&conn).map_err(|e| e.to_string())?;
    Ok(*saved.get(plugin_id).unwrap_or(&true))
}

fn plugin_disabled_response(plugin_id: &str) -> ChatResponse {
    ChatResponse::new(
        format!(
            "Plugin '{}' is disabled in Settings. Enable it under PlugIn to use this command.",
            plugin_id
        ),
        "error".to_string(),
    )
}



#[command]
pub async fn chat_command(
    app: tauri::AppHandle,
    message: String,
    session_id: String,
    history: Vec<ChatHistoryItem>,
    app_settings: serde_json::Value,
    db_state: State<'_, DbState>,
    turn_id: Option<String>,
) -> Result<ChatResponse, String> {
    println!("RUST: chat_command invoked! message: {}, session_id: {}", message, session_id);
    let effective_turn_id = turn_id.unwrap_or_else(|| crate::events::TurnId::new().to_string());
    let stream_id = crate::events::StreamId::new().to_string();
    let emitter = crate::events::EventEmitter::from_app(&app);
    let correlation = crate::events::EventCorrelation::for_stream(
        Some(session_id.clone()),
        Some(effective_turn_id.clone()),
        Some(stream_id.clone()),
    );

    let msg_lower = message.to_lowercase();

    if msg_lower.starts_with("open ") || msg_lower.starts_with("launch ") {
        if !plugin_enabled(&db_state, "app_launcher")? {
            return Ok(plugin_disabled_response("app_launcher").with_turn(&effective_turn_id));
        }
        let offset = if msg_lower.starts_with("open ") { 5 } else { 7 };
        let app_name = message[offset..].trim();
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let launch_path = plugins::resolve_launch_path(&conn, app_name);
        drop(conn);
        return Ok(ChatResponse::new(plugins::plugin_app_launcher(&launch_path), "plugin".to_string()).with_turn(&effective_turn_id));
    }

    if msg_lower.starts_with("play ") {
        if !plugin_enabled(&db_state, "media_player")? {
            return Ok(plugin_disabled_response("media_player").with_turn(&effective_turn_id));
        }
        return Ok(ChatResponse::new(plugins::plugin_media_player(message[5..].trim().to_string()).await, "plugin".to_string()).with_turn(&effective_turn_id));
    }

    if msg_lower.starts_with("whatsapp ") {
        if !plugin_enabled(&db_state, "whatsapp")? {
            return Ok(plugin_disabled_response("whatsapp").with_turn(&effective_turn_id));
        }
        let payload = message[9..].trim();
        // naive split: first word is number, rest is message
        let parts: Vec<&str> = payload.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return Ok(ChatResponse::new(plugins::plugin_whatsapp(parts[0], parts[1]), "plugin".to_string()).with_turn(&effective_turn_id));
        } else {
             return Ok(ChatResponse::new("Please provide both number and message. Example: whatsapp 1234567890 hello".to_string(), "error".to_string()).with_turn(&effective_turn_id));
        }
    }
    if msg_lower.starts_with("email ") {
        if !plugin_enabled(&db_state, "gmail")? {
            return Ok(plugin_disabled_response("gmail").with_turn(&effective_turn_id));
        }
        let payload = message[6..].trim();
        let parts: Vec<&str> = payload.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return Ok(ChatResponse::new(plugins::plugin_gmail(parts[0], parts[1]), "plugin".to_string()).with_turn(&effective_turn_id));
        } else {
             return Ok(ChatResponse::new("Please provide both email and message. Example: email test@test.com hello".to_string(), "error".to_string()).with_turn(&effective_turn_id));
        }
    }

    if msg_lower.starts_with("cmd ") || msg_lower.starts_with("terminal ") {
        if !plugin_enabled(&db_state, "terminal")? {
            return Ok(plugin_disabled_response("terminal").with_turn(&effective_turn_id));
        }
        let q = if msg_lower.starts_with("cmd ") { message[4..].trim() } else { message[9..].trim() };
        return Ok(ChatResponse::new(plugins::plugin_system_terminal(q), "plugin".to_string()).with_turn(&effective_turn_id));
    }


    if msg_lower.contains("volume up") || msg_lower.contains("volume down") || msg_lower.contains("mute") {
        if !plugin_enabled(&db_state, "system_control")? {
            return Ok(plugin_disabled_response("system_control").with_turn(&effective_turn_id));
        }
        let action = if msg_lower.contains("volume up") { "volume_up" }
                     else if msg_lower.contains("volume down") { "volume_down" }
                     else { "mute" };
        return Ok(ChatResponse::new(plugins::plugin_system_control(action), "plugin".to_string()).with_turn(&effective_turn_id));
    }

    let custom_instr   = app_settings.get("customInstructions").and_then(|v| v.as_str()).unwrap_or("");
    let nickname       = app_settings.get("nickname").and_then(|v| v.as_str()).unwrap_or("");
    let occupation     = app_settings.get("occupation").and_then(|v| v.as_str()).unwrap_or("");
    let more_about_you = app_settings.get("moreAboutYou").and_then(|v| v.as_str()).unwrap_or("");

    let mut sys = if custom_instr.is_empty() {
        "You are E.D.I.T.H. (Even Dead, I'm The Hero), an advanced Stark-grade AI PC assistant. Keep responses clear, helpful, intelligent, and friendly. Always wrap code in Markdown triple backticks.".to_string()
    } else {
        custom_instr.to_string()
    };

    if !nickname.is_empty() || !occupation.is_empty() || !more_about_you.is_empty() {
        sys.push_str("\n\nUser Information:\n");
        if !nickname.is_empty()       { sys.push_str(&format!("- Name: {}\n", nickname)); }
        if !occupation.is_empty()     { sys.push_str(&format!("- Occupation: {}\n", occupation)); }
        if !more_about_you.is_empty() { sys.push_str(&format!("- About user: {}\n", more_about_you)); }
    }

    let is_search = msg_lower.starts_with("search ") || msg_lower.starts_with("research ");

    if is_search {
        if !plugin_enabled(&db_state, "web_search")? {
            return Ok(plugin_disabled_response("web_search"));
        }
        let q = if msg_lower.starts_with("search ") { message[7..].trim() } else { message[9..].trim() };
        let tavily_key = app_settings.get("tavilyApiKey").and_then(|v| v.as_str()).map(|s| s.to_string());
        let context = plugins::plugin_web_search(q.to_string(), tavily_key).await;

        let app2 = app.clone();
        let ctx = context.clone();
        let src = format!("Web: {}", q);
        tokio::spawn(async move {
            let _ = crate::memory::save_to_memory_cmd(app2, ctx, src).await;
        });

        sys.push_str(&format!(
            "\n\n[Deep Web Search - Tavily API]\nUser searched: '{}'.\nResults:\n{}\n\nSynthesize into a clear, helpful answer with sources.",
            q, context
        ));
    } else {
        if let Ok(chunks) = crate::memory::search_memory_cmd(app.clone(), message.clone()).await {
            if !chunks.is_empty() {
                sys.push_str("\n\n[Stored Knowledge / Memory Context]:\n");
                for ch in &chunks {
                    sys.push_str(&format!("- (Source: {}) {}\n", ch.source, ch.text));
                }
                sys.push_str("\nUse the above context if relevant to answer the user.");
            }
        }
    }

    let temp: f64 = app_settings.get("temperature")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.7);

    let mut messages = vec![ChatMessage { role: "system".to_string(), content: sys }];
    for h in history {
        messages.push(ChatMessage {
            role: if h.role == "user" { "user".to_string() } else { "assistant".to_string() },
            content: h.text,
        });
    }
    messages.push(ChatMessage { role: "user".to_string(), content: message.clone() });

    let ai_mode = app_settings.get("aiMode").and_then(|v| v.as_str()).unwrap_or("api");
    let provider_id = if ai_mode == "local" && !is_search {
        "local".to_string()
    } else {
        app_settings
            .get("selectedProvider")
            .and_then(|v| v.as_str())
            .unwrap_or("groq")
            .to_string()
    };

    let model = if ai_mode == "local" && !is_search {
        "local-model".to_string()
    } else {
        app_settings
            .get("selectedModel")
            .and_then(|v| v.as_str())
            .unwrap_or("llama-3.3-70b-versatile")
            .to_string()
    };

    let mut registry = crate::ai::ProviderRegistry::standard_builtins();
    if let Some(custom_providers_raw) = app_settings.get("customProviders").and_then(|v| v.as_str()) {
        registry.load_custom_providers(custom_providers_raw);
    }

    let cred_store = crate::ai::SettingsCredentialStore::from_json_value(&app_settings);
    let creds = cred_store.get_credential(&provider_id).ok().flatten();

    let adapter = match registry.resolve_provider(&provider_id) {
        Ok(a) => a,
        Err(e) => return Ok(ChatResponse::new(e.to_string(), "error".to_string()).with_turn(&effective_turn_id)),
    };

    let ai_messages: Vec<crate::ai::ChatMessage> = messages
        .into_iter()
        .map(|m| crate::ai::ChatMessage { role: m.role, content: m.content })
        .collect();

    let req = crate::ai::GenerateRequest {
        model: model.clone(),
        messages: ai_messages,
        temperature: temp,
        max_tokens: None,
        stream: true,
    };

    let stream_cap = adapter.as_streaming_text();
    let reply_result = if let Some(streamer) = stream_cap {
        let _ = emitter.emit_stream_started(&correlation, &model);
        let emitter_clone = emitter.clone();
        let correlation_clone = correlation.clone();
        let seq = std::sync::atomic::AtomicU64::new(0);

        let res = streamer.stream(&req, &creds, Box::new(move |chunk| {
            if !chunk.text.is_empty() {
                let n = seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                let _ = emitter_clone.emit_stream_chunk(&correlation_clone, chunk.text, n, false);
            }
        })).await;

        match &res {
            Ok(_) => {
                let _ = emitter.emit_stream_finished(&correlation, None, Some("stop".to_string()));
            }
            Err(e) => {
                let _ = emitter.emit_stream_failed(&correlation, &e.to_string(), None);
            }
        }
        res
    } else if let Some(gen) = adapter.as_text_generation() {
        let _ = emitter.emit_stream_started(&correlation, &model);
        let res = gen.generate(&req, &creds).await;
        match &res {
            Ok(reply) => {
                let _ = emitter.emit_stream_chunk(&correlation, reply.text.clone(), 1, true);
                let _ = emitter.emit_stream_finished(&correlation, None, Some("stop".to_string()));
            }
            Err(e) => {
                let _ = emitter.emit_stream_failed(&correlation, &e.to_string(), None);
            }
        }
        res
    } else {
        return Ok(ChatResponse::new(
            format!("Provider '{}' does not support text generation", provider_id),
            "error".to_string(),
        ).with_turn(&effective_turn_id));
    };

    match reply_result {
        Ok(reply) => {
            let app3 = app.clone();
            let r = reply.text.clone();
            let sid = session_id.clone();
            let msg = message.clone();
            tokio::spawn(async move {
                let combined = format!("User: {}\nAssistant: {}", msg, r);
                let _ = crate::memory::save_to_memory_cmd(app3, combined, format!("chat:{}", sid)).await;
            });
            Ok(ChatResponse::new(reply.text, "ai".to_string()).with_stream(&stream_id, &effective_turn_id))
        }
        Err(e) => Ok(ChatResponse::new(e.to_string(), "error".to_string()).with_stream(&stream_id, &effective_turn_id)),
    }
}
