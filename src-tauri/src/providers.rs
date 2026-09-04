use crate::ai::adapters::{GeminiAdapter, OpenAICompatibleAdapter};
use crate::ai::provider::ModelDiscoveryCapability;
use crate::ai::registry::ProviderRegistry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FetchedModel {
    pub id: String,
    pub label: String,
}

/// Returns the supported built-in providers and default model catalogs.
/// Backed by the authoritative ProviderRegistry.
#[tauri::command]
pub fn get_providers() -> Result<Value, String> {
    let registry = ProviderRegistry::standard_builtins();

    let mut provider_list = Vec::new();

    // Groq models from adapter
    if let Some(groq) = registry.get("groq") {
        let models: Vec<Value> = groq
            .models()
            .into_iter()
            .map(|m| json!({ "id": m.model_id, "label": m.display_name }))
            .collect();
        provider_list.push(json!({
            "id": groq.id(),
            "name": groq.name(),
            "models": models,
        }));
    }

    // Gemini models from adapter
    if let Some(gemini) = registry.get("gemini") {
        let models: Vec<Value> = gemini
            .models()
            .into_iter()
            .map(|m| json!({ "id": m.model_id, "label": m.display_name }))
            .collect();
        provider_list.push(json!({
            "id": gemini.id(),
            "name": gemini.name(),
            "models": models,
        }));
    }

    Ok(json!({ "providers": provider_list }))
}

/// Dynamically discovers models from any OpenAI-compatible or Gemini endpoint.
/// Delegated entirely to the provider capability layer.
#[tauri::command]
pub async fn fetch_custom_models(
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<FetchedModel>, String> {
    let raw_url = base_url.trim().trim_end_matches('/').to_string();
    let creds = api_key.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);

    let is_gemini = raw_url.contains("generativelanguage.googleapis.com") || raw_url == "gemini";

    let discovered = if is_gemini {
        let adapter = GeminiAdapter::new();
        adapter
            .discover_models(&creds)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let adapter = OpenAICompatibleAdapter::new("custom", "Custom Provider", raw_url, vec![], None);
        adapter
            .discover_models(&creds)
            .await
            .map_err(|e| e.to_string())?
    };

    if discovered.is_empty() {
        return Err("No models found in the provider response. You can still add models manually.".to_string());
    }

    let mut result: Vec<FetchedModel> = discovered
        .into_iter()
        .map(|m| FetchedModel {
            id: m.model_id,
            label: m.display_name,
        })
        .collect();

    result.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(result)
}
