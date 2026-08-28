use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FetchedModel {
    pub id: String,
    pub label: String,
}

#[tauri::command]
pub fn get_providers() -> Result<Value, String> {
    Ok(json!({
        "providers": [
            {
                "id": "groq",
                "name": "Groq Cloud",
                "models": [
                    {"id": "llama-3.3-70b-versatile", "label": "Meta LLaMA 3.3 70B Versatile"},
                    {"id": "llama-3.1-8b-instant", "label": "Meta LLaMA 3.1 8B Instant"},
                    {"id": "openai/gpt-oss-120b", "label": "OpenAI GPT-OSS 120B"},
                    {"id": "openai/gpt-oss-20b", "label": "OpenAI GPT-OSS 20B"},
                    {"id": "deepseek-r1-distill-llama-70b", "label": "DeepSeek R1 Distill LLaMA 70B"},
                    {"id": "qwen/qwen3.6-27b", "label": "Qwen 3.6 27B"},
                    {"id": "qwen-qwq-32b", "label": "Qwen QwQ 32B"},
                    {"id": "groq/compound", "label": "Groq Compound System"},
                    {"id": "groq/compound-mini", "label": "Groq Compound Mini"}
                ]
            },
            {
                "id": "gemini",
                "name": "Google Gemini API",
                "models": [
                    {"id": "gemini-2.5-flash", "label": "Gemini 2.5 Flash"},
                    {"id": "gemini-2.5-flash-lite", "label": "Gemini 2.5 Flash Lite"},
                    {"id": "gemini-2.5-pro", "label": "Gemini 2.5 Pro"},
                    {"id": "gemini-2.0-flash", "label": "Gemini 2.0 Flash"},
                    {"id": "gemini-2.0-flash-lite", "label": "Gemini 2.0 Flash Lite"},
                    {"id": "gemini-1.5-flash", "label": "Gemini 1.5 Flash"},
                    {"id": "gemini-1.5-pro", "label": "Gemini 1.5 Pro"},
                    {"id": "gemini-3.7-flash", "label": "Gemini 3.7 Flash"},
                    {"id": "gemini-3.6-flash", "label": "Gemini 3.6 Flash"},
                    {"id": "gemini-3.1-pro", "label": "Gemini 3.1 Pro"}
                ]
            }
        ]
    }))
}

#[tauri::command]
pub async fn fetch_custom_models(base_url: String, api_key: Option<String>) -> Result<Vec<FetchedModel>, String> {
    let raw_url = base_url.trim().trim_end_matches('/').to_string();
    let trimmed_key = api_key.as_deref().unwrap_or("").trim();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Check if this is a Google Gemini endpoint
    let is_gemini = raw_url.contains("generativelanguage.googleapis.com") || raw_url == "gemini";
    let endpoint = if is_gemini {
        if !trimmed_key.is_empty() {
            format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", trimmed_key)
        } else {
            "https://generativelanguage.googleapis.com/v1beta/models".to_string()
        }
    } else {
        let mut url = raw_url;
        if url.ends_with("/chat/completions") {
            url = url.trim_end_matches("/chat/completions").to_string();
        }
        if url.ends_with("/models") {
            url
        } else {
            format!("{}/models", url)
        }
    };

    let mut req = client.get(&endpoint);
    if !is_gemini && !trimmed_key.is_empty() {
        req = req.bearer_auth(trimmed_key);
    }

    let resp = req.send().await.map_err(|e| format!("Connection failed to {}: {}", endpoint, e))?;
    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("Server returned error ({}): {}", status, err_text));
    }

    let val: Value = resp.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
    let mut models = Vec::new();

    if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                let label = item.get("name").or_else(|| item.get("description")).and_then(|n| n.as_str()).unwrap_or(id);
                models.push(FetchedModel {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
    } else if let Some(data) = val.get("models").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(name) = item.get("name").or_else(|| item.get("id")).and_then(|i| i.as_str()) {
                // Strip "models/" prefix for clean Gemini IDs
                let clean_id = name.strip_prefix("models/").unwrap_or(name);
                
                // Only include generation models if supportedGenerationMethods is specified
                let is_gen_model = item.get("supportedGenerationMethods")
                    .and_then(|m| m.as_array())
                    .map(|methods| methods.iter().any(|m| m.as_str() == Some("generateContent")))
                    .unwrap_or(true);

                if is_gen_model {
                    let display_name = item.get("displayName")
                        .and_then(|d| d.as_str())
                        .unwrap_or(clean_id);
                    models.push(FetchedModel {
                        id: clean_id.to_string(),
                        label: display_name.to_string(),
                    });
                }
            }
        }
    }

    if models.is_empty() {
        return Err("No models found in the provider response. You can still add models manually.".to_string());
    }

    models.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(models)
}




