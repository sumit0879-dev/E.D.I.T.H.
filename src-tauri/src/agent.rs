use serde::Serialize;
use tauri::{AppHandle, State, Emitter};
use std::sync::Mutex;
use crate::db::DbState;
use crate::llm::{api_chat_cloud, ChatMessage, ChatRequest};

#[derive(Serialize)]
pub struct AgentStatus {
    pub is_ready: bool,
    pub project_path: String,
}

pub struct AgentState {
    pub path: Mutex<String>,
}

#[tauri::command]
pub async fn agent_status(state: State<'_, AgentState>) -> Result<AgentStatus, String> {
    let path = state.path.lock().unwrap().clone();
    let is_ready = !path.is_empty();
    Ok(AgentStatus {
        is_ready,
        project_path: path,
    })
}

fn get_provider_url(provider: &str) -> String {
    match provider {
        "groq" => "https://api.groq.com/openai/v1/chat/completions".to_string(),
        "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
        "together" => "https://api.together.xyz/v1/chat/completions".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        "deepseek" => "https://api.deepseek.com/chat/completions".to_string(),
        "cerebras" => "https://api.cerebras.ai/v1/chat/completions".to_string(),
        "sambanova" => "https://api.sambanova.ai/v1/chat/completions".to_string(),
        "mistral" => "https://api.mistral.ai/v1/chat/completions".to_string(),
        "huggingface" => "https://api-inference.huggingface.co/v1/chat/completions".to_string(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string(),
        _ => "https://api.groq.com/openai/v1/chat/completions".to_string(),
    }
}

#[tauri::command]
pub async fn agent_chat(
    app: AppHandle, 
    message: String, 
    session_id: Option<String>,
    state: State<'_, AgentState>,
    db_state: State<'_, DbState>
) -> Result<String, String> {
    let project_path = state.path.lock().unwrap().clone();
    
    let settings = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        crate::db::get_all_settings(&conn).unwrap_or_default()
    };
    
    let ai_mode = settings.get("aiMode").cloned().unwrap_or_else(|| "api".to_string());
    let provider = settings.get("selectedProvider").cloned().unwrap_or_else(|| "groq".to_string());
    let model = settings.get("selectedModel").cloned().unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());
    // Try multiple key format options for backward compatibility
    let api_key_key = format!("api_key_{}", provider);
    let api_key = settings.get(&api_key_key)
        .or_else(|| settings.get(&format!("apiKey_{}", provider)))
        .or_else(|| settings.get("apiKey")).cloned().unwrap_or_default();
    
    if ai_mode != "local" && api_key.is_empty() {
        return Err("API Key is missing for selected provider.".to_string());
    }
    
    let custom_instr = settings.get("customInstructions").cloned().unwrap_or_default();
    let nickname = settings.get("nickname").cloned().unwrap_or_default();
    let occupation = settings.get("occupation").cloned().unwrap_or_default();
    let more_about_you = settings.get("moreAboutYou").cloned().unwrap_or_default();
    
    let mut sys_content = custom_instr;
    if sys_content.is_empty() {
        sys_content = "You are an expert developer agent.".to_string();
    }
    
    sys_content.push_str(&format!("
Context Path: {}
", project_path));
    
    if !nickname.is_empty() || !occupation.is_empty() || !more_about_you.is_empty() {
        sys_content.push_str("
User Information:
");
        if !nickname.is_empty() { sys_content.push_str(&format!("- Name/Nickname: {}
", nickname)); }
        if !occupation.is_empty() { sys_content.push_str(&format!("- Occupation: {}
", occupation)); }
        if !more_about_you.is_empty() { sys_content.push_str(&format!("- More about the user: {}
", more_about_you)); }
    }
    
    sys_content.push_str("
You have the following tools available. To use them, output the EXACT text format:
1. Run a terminal command: [RUN_CMD: <command>]
2. Read a file: [READ_FILE: <absolute_path>]
You can only use one tool at a time. Do not write anything after the tool block. Wait for the user to provide the result.");
    
    let system_prompt = sys_content;
    
    let temp_str = settings.get("temperature").cloned().unwrap_or_else(|| "0.7".to_string());
    let temperature = temp_str.parse::<f64>().unwrap_or(0.7);
    
    let mut messages = vec![
        ChatMessage { role: "system".to_string(), content: system_prompt },
        ChatMessage { role: "user".to_string(), content: message }
    ];
    
    let max_iterations = 5;
    let mut final_response = String::new();
    
    for _ in 0..max_iterations {
        let req = ChatRequest {
            model: if ai_mode == "local" { "local-model".to_string() } else { model.clone() },
            messages: messages.clone(),
            temperature,
            provider: if ai_mode == "local" { "local".to_string() } else { provider.clone() },
        };
        
        let ai_reply = if ai_mode == "local" {
            api_chat_cloud(
                app.clone(),
                "".to_string(),
                "http://127.0.0.1:11434/v1/chat/completions".to_string(),
                req,
                Some("chat-chunk".to_string())
            ).await?
        } else {
            api_chat_cloud(
                app.clone(), 
                api_key.clone(), 
                get_provider_url(&provider), 
                req, 
                Some("chat-chunk".to_string())
            ).await?
        };
        
        final_response.push_str(&ai_reply);
        messages.push(ChatMessage { role: "assistant".to_string(), content: ai_reply.clone() });
        
        if ai_reply.contains("[RUN_CMD:") {
            if let Some(start) = ai_reply.find("[RUN_CMD:") {
                if let Some(end) = ai_reply[start..].find("]") {
                    let raw_cmd = ai_reply[start + 9 .. start + end].trim();
                    
                    // SEC-02 & SEC-01 Hardening: Validate and create immutable backend proposal
                    let (prog, args) = match crate::security::CommandPolicy::parse_and_validate(raw_cmd) {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = app.emit("chat-chunk", format!("> 🛡️ **COMMAND BLOCKED**: {}\n\n", e));
                            messages.push(ChatMessage { role: "user".to_string(), content: format!("Command Blocked: {}", e) });
                            continue;
                        }
                    };

                    let (risk, requires_approval) = match crate::security::CommandPolicy::evaluate_risk(&prog, &args) {
                        Ok(eval) => eval,
                        Err(e) => {
                            let _ = app.emit("chat-chunk", format!("> 🛡️ **POLICY REJECTION**: {}\n\n", e));
                            messages.push(ChatMessage { role: "user".to_string(), content: format!("Policy Rejection: {}", e) });
                            continue;
                        }
                    };

                    let work_path = if !project_path.is_empty() {
                        std::path::PathBuf::from(&project_path)
                    } else {
                        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                    };

                    if requires_approval {
                        let proposal = crate::security::ProposalEngine::create_proposal(
                            &session_id.clone().unwrap_or_default(),
                            &prog,
                            &args,
                            &work_path,
                            risk,
                        );

                        let _ = app.emit("tool-proposal", serde_json::json!({
                            "proposal_id": proposal.proposal_id,
                            "session_id": proposal.session_id,
                            "command": proposal.command_display,
                            "working_dir": proposal.working_dir,
                            "risk_level": proposal.risk_level,
                            "expires_at": proposal.expires_at,
                        }));

                        let _ = app.emit("chat-chunk", format!(
                            "\n\n> 📋 **Tool Execution Proposal** (`{}`)\n> • Target Directory: `{}`\n> • Risk Level: `{}`\n> • Proposal ID: `{}`\n> • Status: *Awaiting operator approval in UI*\n\n",
                            proposal.command_display,
                            proposal.working_dir,
                            proposal.risk_level,
                            proposal.proposal_id
                        ));
                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!("Tool proposal created (ID: {}). Awaiting human operator approval.", proposal.proposal_id),
                        });
                        continue;
                    } else {
                        // Low-risk diagnostic command executed directly with sandboxing
                        let _ = app.emit("chat-chunk", format!("\n\n> ⚡ Executing Diagnostic: `{}`\n\n", raw_cmd));
                        match crate::security::CommandPolicy::execute(&prog, &args, Some(&work_path)) {
                            Ok(res) => {
                                let _ = app.emit("chat-chunk", format!("> 📋 Result ({} bytes, {}ms):\n```text\n{}\n```\n\n", res.output.len(), res.execution_time_ms, res.output.chars().take(400).collect::<String>()));
                                messages.push(ChatMessage { role: "user".to_string(), content: format!("Command Result:\n{}", res.output) });
                                continue;
                            }
                            Err(e) => {
                                let _ = app.emit("chat-chunk", format!("> ❌ Execution Error: {}\n\n", e));
                                messages.push(ChatMessage { role: "user".to_string(), content: format!("Execution Error: {}", e) });
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        if ai_reply.contains("[READ_FILE:") {
            if let Some(start) = ai_reply.find("[READ_FILE:") {
                if let Some(end) = ai_reply[start..].find("]") {
                    let raw_path = ai_reply[start + 11 .. start + end].trim();
                    let _ = app.emit("chat-chunk", format!("\n\n> 🔍 Reading: `{}`\n\n", raw_path));
                    
                    // Resolve within project path context
                    let target_path = if std::path::Path::new(raw_path).is_absolute() {
                        std::path::PathBuf::from(raw_path)
                    } else if !project_path.is_empty() {
                        std::path::Path::new(&project_path).join(raw_path)
                    } else {
                        std::path::PathBuf::from(raw_path)
                    };

                    // SEC-03 Hardening: Enforce exact PathSandbox containment validation
                    let allowed_roots = if !project_path.is_empty() {
                        vec![std::path::PathBuf::from(&project_path)]
                    } else {
                        vec![std::env::current_dir().unwrap_or_default()]
                    };
                    let res_str = match crate::security::PathSandbox::verify_containment(&target_path.to_string_lossy(), &allowed_roots) {
                        Ok(safe_path) => {
                            match std::fs::read_to_string(&safe_path) {
                                Ok(content) => content,
                                Err(e) => format!("Error reading file: {}", e),
                            }
                        }
                        Err(policy_err) => {
                            format!("Security Sandbox Violation: {}", policy_err)
                        }
                    };

                    messages.push(ChatMessage { role: "user".to_string(), content: format!("File Content:\n{}", res_str) });
                    continue;
                }
            }
        }
        
        break;
    }
    
    Ok(final_response)
}

#[tauri::command]
pub async fn agent_set_path(path: String, state: State<'_, AgentState>) -> Result<(), String> {
    *state.path.lock().unwrap() = path;
    Ok(())
}

#[tauri::command]
pub async fn agent_reset(state: State<'_, AgentState>) -> Result<(), String> {
    *state.path.lock().map_err(|e| e.to_string())? = String::new();
    Ok(())
}

