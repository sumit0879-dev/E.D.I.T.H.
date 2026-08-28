use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use std::sync::Mutex;
use std::process::{Child, Command, Stdio};
use lazy_static::lazy_static;
use std::io::{BufRead, BufReader};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;



#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub provider: String,
}

// Global state for the running llama-server process
lazy_static! {
    static ref LOCAL_SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
}

#[tauri::command]
pub async fn api_chat_cloud(
    app: AppHandle,
    api_key: String,
    url: String,
    request: ChatRequest,
    emit_event: Option<String>,
) -> Result<String, String> {
    let client = Client::builder().timeout(std::time::Duration::from_secs(60)).build().unwrap_or_else(|_| Client::new());
    let req_body = json!({
        "model": request.model,
        "messages": request.messages,
        "temperature": request.temperature,
        "stream": true
    });

    let mut request_builder = client.post(&url).json(&req_body);
    if !api_key.is_empty() {
        request_builder = request_builder.bearer_auth(&api_key);
    }

    let mut res = request_builder.send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let error_text = res.text().await.unwrap_or_default();
        return Err(format!("API Error: {}", error_text));
    }

    

    let mut full_text = String::new();

    while let Ok(Some(chunk)) = res.chunk().await {
        let chunk_str = String::from_utf8_lossy(&chunk);
        let lines: Vec<&str> = chunk_str.split('\n').collect();

        for line in lines {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        full_text.push_str(content);
                        if let Some(ref ev) = emit_event {
                            let _ = app.emit(ev, content.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(full_text)
}

/// Helper: Resolve a subfolder by checking multiple candidate directories
fn resolve_folder(folder_name: &str) -> std::path::PathBuf {
    let from_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(folder_name)));
    let from_project_root = std::env::current_dir()
        .ok()
        .map(|d| d.join("..").join(folder_name));
    let from_cwd = std::env::current_dir()
        .ok()
        .map(|d| d.join(folder_name));

    [from_exe, from_project_root, from_cwd]
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from(folder_name))
}

/// Start llama-server.exe as a background subprocess
#[tauri::command]
pub async fn load_local_llm(_app: AppHandle, path: String, load_mode: String) -> Result<(), String> {
    let _ = _app.emit("model-progress", "Process starting... (0%)");
    println!("RUST: Starting local server... Model: {}, Mode: {}", path, load_mode);

    // Stop any existing server first
    let _ = stop_local_llm().await;

    // --- Step 1: Resolve llama-server.exe path ---
    let llama_dir = resolve_folder("Llama");
    let exe_path = llama_dir.join("llama-server.exe");

    if !exe_path.exists() {
        return Err(format!(
            "llama-server.exe nahi mila! Expected path: {}. Kripya 'Llama' folder mein llama-server.exe rakhein.",
            exe_path.display()
        ));
    }
    println!("RUST: Resolved llama-server path: {:?}", exe_path);

    // --- Step 2: Resolve model file path ---
    // If user gave an absolute path (e.g. D:\Models\xyz.gguf), use it directly.
    // Otherwise, look inside the resolved "Models" folder.
    let model_path = std::path::PathBuf::from(&path);
    let full_model_path = if model_path.is_absolute() && model_path.exists() {
        model_path
    } else {
        let models_dir = resolve_folder("Models");
        let candidate = models_dir.join(&path);
        candidate
    };

    // --- Step 3: Validate model file exists ---
    if !full_model_path.exists() {
        return Err(format!(
            "Model file nahi mila: '{}'. Kripya sahi .gguf file ka path daalo ya Models folder mein rakhein.",
            full_model_path.display()
        ));
    }
    let full_model_str = full_model_path.to_string_lossy().to_string();
    println!("RUST: Using full model path: {}", full_model_str);

    // --- Step 4: Build args ---
    let mut args = vec![
        "-m".to_string(), full_model_str,
        "--port".to_string(), "11434".to_string(),
        "-c".to_string(), "2048".to_string(),
    ];

    if load_mode == "ram" {
        args.push("--mlock".to_string());
    }

    // --- Step 5: Spawn process with captured stderr for debugging ---
    #[cfg(target_os = "windows")]
    let mut child = Command::new(&exe_path)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to start llama-server.exe: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(&exe_path)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start llama-server.exe: {}", e))?;

    // Take stderr handle before moving child into the global mutex
    let stderr_handle = child.stderr.take();

    // --- Step 6: Read stderr in a background thread for logging ---
    if let Some(stderr) = stderr_handle {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => println!("[llama-server] {}", l),
                    Err(_) => break,
                }
            }
        });
    }

    // Store process handle
    {
        let mut guard = LOCAL_SERVER_PROCESS.lock().unwrap();
        *guard = Some(child);
    }

    // --- Step 7: Active health check - poll until server is ready ---
    // Instead of blind sleep, we ping the server every 500ms for up to 120 seconds.
    // This means fast models load fast, slow models get enough time.
    let client = Client::new();
    let health_url = "http://127.0.0.1:11434/health";
    let max_attempts = 240; // 240 * 500ms = 120 seconds max wait
    let mut server_ready = false;

    for attempt in 1..=max_attempts {
        // Check if process died
        {
            let mut guard = LOCAL_SERVER_PROCESS.lock().unwrap();
            if let Some(ref mut proc) = *guard {
                match proc.try_wait() {
                    Ok(Some(status)) => {
                        *guard = None;
                        return Err(format!(
                            "llama-server crash ho gaya (exit code: {}). Terminal mein [llama-server] logs dekhein.",
                            status
                        ));
                    }
                    Ok(None) => { /* still running, good */ }
                    Err(e) => {
                        *guard = None;
                        return Err(format!("Process check error: {}", e));
                    }
                }
            } else {
                return Err("llama-server process unexpectedly stopped.".to_string());
            }
        }

        // Try hitting the health endpoint
        if let Ok(resp) = client.get(health_url).send().await {
            if resp.status().is_success() {
                server_ready = true;
                let _ = _app.emit("model-progress", "Server Ready! (100%)");
                println!("RUST: Server is READY after {} attempts (~{}ms)", attempt, attempt * 500);
                break;
            }
        }

        if attempt % 20 == 0 {
            let msg = format!("Loading weights into RAM (Attempt {}/{})...", attempt, max_attempts);
            let _ = _app.emit("model-progress", msg);
            println!("RUST: Waiting for server... attempt {}/{}", attempt, max_attempts);
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    if !server_ready {
        // Server never became ready - kill it
        let _ = stop_local_llm().await;
        return Err("llama-server 120 seconds mein ready nahi hua. Model bahut bada ho sakta hai ya RAM insufficient hai. Terminal logs check karein.".to_string());
    }

    println!("RUST: Local server started successfully!");
    Ok(())
}

/// Kill the llama-server.exe subprocess
#[tauri::command]
pub async fn stop_local_llm() -> Result<(), String> {
    let mut guard = LOCAL_SERVER_PROCESS.lock().unwrap();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
        println!("RUST: Local server killed.");
    }
    Ok(())
}

/// Local chat via HTTP requests to the spawned llama-server
#[tauri::command]
pub async fn local_chat(
    app: AppHandle,
    prompt: String,
    emit_event: Option<String>,
) -> Result<String, String> {
    // Determine if server is running
    {
        let guard = LOCAL_SERVER_PROCESS.lock().unwrap();
        if guard.is_none() {
            return Err("No local model loaded. Please load a model first from Settings.".to_string());
        }
    }

    // Wrap the plain string prompt into OpenAI format for the server
    let req = ChatRequest {
        model: "local-model".to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "You are E.D.I.T.H. (Even Dead, I'm The Hero), an advanced Stark-grade AI PC assistant. You ALWAYS reply ONLY in Hinglish (Hindi written in English script). NEVER write in pure English or pure Hindi. Keep responses short, smart, and friendly. CRITICAL RULE: Whenever you write ANY code, you MUST wrap it inside Markdown triple backticks (```language) so it renders correctly.".to_string() },
            ChatMessage { role: "user".to_string(), content: prompt }
        ],
        temperature: 0.4,
        provider: "local".to_string(),
    };

    api_chat_cloud(
        app,
        "".to_string(), // no api key needed for local server
        "http://127.0.0.1:11434/v1/chat/completions".to_string(),
        req,
        emit_event
    ).await
}



