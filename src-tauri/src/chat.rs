use crate::db::{self, DbState};
use crate::llm::{api_chat_cloud, ChatMessage, ChatRequest};
use crate::plugins;
use serde::{Deserialize, Serialize};
use tauri::command;
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct ChatHistoryItem {
    pub role: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
    pub r#type: String,
}

fn plugin_enabled(state: &DbState, plugin_id: &str) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let saved = db::get_plugin_states(&conn).map_err(|e| e.to_string())?;
    Ok(*saved.get(plugin_id).unwrap_or(&true))
}

fn plugin_disabled_response(plugin_id: &str) -> ChatResponse {
    ChatResponse {
        response: format!(
            "Plugin '{}' is disabled in Settings. Enable it under PlugIn to use this command.",
            plugin_id
        ),
        r#type: "error".to_string(),
    }
}

fn resolve_provider_config(
    provider_id: &str,
    app_settings: &serde_json::Value,
) -> (String, String) {
    // Returns (api_url, api_key)
    if provider_id == "gemini" {
        let key = app_settings
            .get("apiKey_gemini")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return (
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string(),
            key,
        );
    }

    if provider_id == "groq" {
        let key = app_settings
            .get("apiKey_groq")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return (
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            key,
        );
    }

    // Check custom providers from settings JSON
    if let Some(custom_providers_raw) = app_settings.get("customProviders").and_then(|v| v.as_str()) {
        if let Ok(providers_list) = serde_json::from_str::<Vec<serde_json::Value>>(custom_providers_raw) {
            for cp in providers_list {
                if cp.get("id").and_then(|i| i.as_str()) == Some(provider_id) {
                    let base_url = cp.get("baseUrl").and_then(|u| u.as_str()).unwrap_or("").trim().trim_end_matches('/').to_string();
                    let url = if base_url.ends_with("/chat/completions") {
                        base_url
                    } else {
                        format!("{}/chat/completions", base_url)
                    };
                    let key = cp.get("apiKey").and_then(|k| k.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            app_settings.get(&format!("apiKey_{}", provider_id))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_default();
                    return (url, key);
                }
            }
        }
    }

    // Fallback: Groq
    let key = app_settings
        .get(&format!("apiKey_{}", provider_id))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    ("https://api.groq.com/openai/v1/chat/completions".to_string(), key)
}

#[command]
pub async fn chat_command(
    app: tauri::AppHandle,
    message: String,
    session_id: String,
    history: Vec<ChatHistoryItem>,
    app_settings: serde_json::Value,
    db_state: State<'_, DbState>,
) -> Result<ChatResponse, String> {
    println!("RUST: chat_command invoked! message: {}, session_id: {}", message, session_id);
    let msg_lower = message.to_lowercase();

    if msg_lower.starts_with("open ") || msg_lower.starts_with("launch ") {
        if !plugin_enabled(&db_state, "app_launcher")? {
            return Ok(plugin_disabled_response("app_launcher"));
        }
        let offset = if msg_lower.starts_with("open ") { 5 } else { 7 };
        let app_name = message[offset..].trim();
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let launch_path = plugins::resolve_launch_path(&conn, app_name);
        drop(conn);
        return Ok(ChatResponse {
            response: plugins::plugin_app_launcher(&launch_path),
            r#type: "plugin".to_string(),
        });
    }

    if msg_lower.starts_with("play ") {
        if !plugin_enabled(&db_state, "media_player")? {
            return Ok(plugin_disabled_response("media_player"));
        }
        return Ok(ChatResponse {
            response: plugins::plugin_media_player(message[5..].trim().to_string()).await,
            r#type: "plugin".to_string(),
        });
    }

    if msg_lower.starts_with("whatsapp ") {
        if !plugin_enabled(&db_state, "whatsapp")? {
            return Ok(plugin_disabled_response("whatsapp"));
        }
        let payload = message[9..].trim();
        // naive split: first word is number, rest is message
        let parts: Vec<&str> = payload.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return Ok(ChatResponse {
                response: plugins::plugin_whatsapp(parts[0], parts[1]),
                r#type: "plugin".to_string(),
            });
        } else {
             return Ok(ChatResponse {
                response: "Please provide both number and message. Example: whatsapp 1234567890 hello".to_string(),
                r#type: "error".to_string(),
            });
        }
    }
    if msg_lower.starts_with("email ") {
        if !plugin_enabled(&db_state, "gmail")? {
            return Ok(plugin_disabled_response("gmail"));
        }
        let payload = message[6..].trim();
        let parts: Vec<&str> = payload.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return Ok(ChatResponse {
                response: plugins::plugin_gmail(parts[0], parts[1]),
                r#type: "plugin".to_string(),
            });
        } else {
             return Ok(ChatResponse {
                response: "Please provide both email and message. Example: email test@test.com hello".to_string(),
                r#type: "error".to_string(),
            });
        }
    }

    if msg_lower.starts_with("cmd ") || msg_lower.starts_with("terminal ") {
        if !plugin_enabled(&db_state, "terminal")? {
            return Ok(plugin_disabled_response("terminal"));
        }
        let q = if msg_lower.starts_with("cmd ") { message[4..].trim() } else { message[9..].trim() };
        return Ok(ChatResponse {
            response: plugins::plugin_system_terminal(q),
            r#type: "plugin".to_string(),
        });
    }


    if msg_lower.contains("volume up") || msg_lower.contains("volume down") || msg_lower.contains("mute") {
        if !plugin_enabled(&db_state, "system_control")? {
            return Ok(plugin_disabled_response("system_control"));
        }
        let action = if msg_lower.contains("volume up") { "volume_up" }
                     else if msg_lower.contains("volume down") { "volume_down" }
                     else { "mute" };
        return Ok(ChatResponse {
            response: plugins::plugin_system_control(action),
            r#type: "plugin".to_string(),
        });
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
    if ai_mode == "local" && !is_search {
        let req = ChatRequest { model: "local-model".to_string(), messages, temperature: temp, provider: "local".to_string() };
        return match api_chat_cloud(app, "".to_string(), "http://127.0.0.1:11434/v1/chat/completions".to_string(), req, Some("chat-chunk".to_string())).await {
            Ok(reply) => Ok(ChatResponse { response: reply, r#type: "ai".to_string() }),
            Err(e)    => Ok(ChatResponse { response: format!("Local Model Error: {}", e), r#type: "error".to_string() }),
        };
    }

    let provider = app_settings
        .get("selectedProvider")
        .and_then(|v| v.as_str())
        .unwrap_or("groq")
        .to_string();

    let model = app_settings
        .get("selectedModel")
        .and_then(|v| v.as_str())
        .unwrap_or("llama-3.3-70b-versatile")
        .to_string();

    let (url, api_key) = resolve_provider_config(&provider, &app_settings);
    let req = ChatRequest { model, messages, temperature: temp, provider: provider.clone() };

    match api_chat_cloud(app.clone(), api_key, url, req, Some("chat-chunk".to_string())).await {
        Ok(reply) => {
            let app3 = app.clone();
            let r = reply.clone();
            let sid = session_id.clone();
            let msg = message.clone();
            tokio::spawn(async move {
                let combined = format!("User: {}\nAssistant: {}", msg, r);
                let _ = crate::memory::save_to_memory_cmd(app3, combined, format!("chat:{}", sid)).await;
            });
            Ok(ChatResponse { response: reply, r#type: "ai".to_string() })
        },
        Err(e) => Ok(ChatResponse { response: e, r#type: "error".to_string() }),
    }
}
