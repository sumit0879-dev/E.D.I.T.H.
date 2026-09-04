use super::errors::ProviderError;
use std::collections::HashMap;

/// An abstract credential store interface for resolving provider API keys/tokens.
/// Provider adapters MUST NOT know where credentials physically reside.
pub trait CredentialStore: Send + Sync {
    /// Resolves the credential (API key or bearer token) for the specified provider ID.
    fn get_credential(&self, provider_id: &str) -> Result<Option<String>, ProviderError>;
}

/// A credential store backed by E.D.I.T.H.'s settings key-value map and custom provider definitions.
/// For Phase 1, this bridges existing SQLite settings storage behind the credential abstraction
/// without exposing secrets to frontend layers or coupling adapters to storage formats.
#[derive(Debug, Clone, Default)]
pub struct SettingsCredentialStore {
    settings: HashMap<String, String>,
}

impl SettingsCredentialStore {
    pub fn new(settings: HashMap<String, String>) -> Self {
        Self { settings }
    }

    pub fn from_json_value(value: &serde_json::Value) -> Self {
        let mut map = HashMap::new();
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                } else {
                    map.insert(k.clone(), v.to_string());
                }
            }
        }
        Self { settings: map }
    }
}

impl CredentialStore for SettingsCredentialStore {
    fn get_credential(&self, provider_id: &str) -> Result<Option<String>, ProviderError> {
        // 1. Direct setting key variations: apiKey_<id>, api_key_<id>, apiKey
        let direct_candidates = [
            format!("apiKey_{}", provider_id),
            format!("api_key_{}", provider_id),
        ];

        for key in direct_candidates {
            if let Some(val) = self.settings.get(&key) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Ok(Some(trimmed.to_string()));
                }
            }
        }

        // 2. Check custom providers JSON array if present
        if let Some(custom_providers_raw) = self.settings.get("customProviders") {
            if let Ok(providers_list) = serde_json::from_str::<Vec<serde_json::Value>>(custom_providers_raw) {
                for cp in providers_list {
                    if cp.get("id").and_then(|i| i.as_str()) == Some(provider_id) {
                        if let Some(key) = cp.get("apiKey").and_then(|k| k.as_str()) {
                            let trimmed = key.trim();
                            if !trimmed.is_empty() {
                                return Ok(Some(trimmed.to_string()));
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback to generic "apiKey" if provider is default or matches
        if let Some(val) = self.settings.get("apiKey") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Ok(Some(trimmed.to_string()));
            }
        }

        // 4. Environment variable fallback (optional for local/server setups)
        let env_var = match provider_id {
            "groq" => "GROQ_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "openai" => "OPENAI_API_KEY",
            _ => "",
        };

        if !env_var.is_empty() {
            if let Ok(env_key) = std::env::var(env_var) {
                let trimmed = env_key.trim();
                if !trimmed.is_empty() {
                    return Ok(Some(trimmed.to_string()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_direct_key() {
        let mut settings = HashMap::new();
        settings.insert("apiKey_groq".to_string(), "gsk_test123".to_string());
        let store = SettingsCredentialStore::new(settings);

        let key = store.get_credential("groq").unwrap();
        assert_eq!(key, Some("gsk_test123".to_string()));

        let missing = store.get_credential("gemini").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_resolve_custom_provider_key() {
        let mut settings = HashMap::new();
        let custom_json = r#"[
            {"id": "my_ollama", "baseUrl": "http://localhost:11434/v1", "apiKey": "custom_secret_key"}
        ]"#;
        settings.insert("customProviders".to_string(), custom_json.to_string());
        let store = SettingsCredentialStore::new(settings);

        let key = store.get_credential("my_ollama").unwrap();
        assert_eq!(key, Some("custom_secret_key".to_string()));
    }
}
