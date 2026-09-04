use serde::{Deserialize, Serialize};
use std::fmt;

/// Normalized errors produced by AI providers.
/// Higher architectural layers only consume these normalized errors
/// rather than raw HTTP or provider-specific wire schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum ProviderError {
    AuthFailure { message: String },
    InvalidRequest { message: String },
    ModelUnavailable { model: String, reason: String },
    CapabilityUnsupported { capability: String },
    RateLimited { retry_after_secs: Option<u64>, message: String },
    NetworkFailure { message: String },
    Timeout { message: String },
    ServerError { status_code: u16, message: String },
    MalformedResponse { message: String },
    Unknown { message: String },
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthFailure { message } => write!(f, "Authentication Failed: {}", message),
            Self::InvalidRequest { message } => write!(f, "Invalid Request: {}", message),
            Self::ModelUnavailable { model, reason } => {
                write!(f, "Model '{}' is unavailable: {}", model, reason)
            }
            Self::CapabilityUnsupported { capability } => {
                write!(f, "Capability '{}' is not supported by this provider/model", capability)
            }
            Self::RateLimited { retry_after_secs, message } => {
                if let Some(secs) = retry_after_secs {
                    write!(f, "Rate limited (retry in {}s): {}", secs, message)
                } else {
                    write!(f, "Rate limited: {}", message)
                }
            }
            Self::NetworkFailure { message } => write!(f, "Network Failure: {}", message),
            Self::Timeout { message } => write!(f, "Request Timeout: {}", message),
            Self::ServerError { status_code, message } => {
                write!(f, "Provider Server Error (HTTP {}): {}", status_code, message)
            }
            Self::MalformedResponse { message } => write!(f, "Malformed Response: {}", message),
            Self::Unknown { message } => write!(f, "Provider Error: {}", message),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Redacts potential secrets, keys, or bearer tokens from error strings
/// to prevent leaking credentials to logs or higher UI layers.
pub fn sanitize_error_message(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("bearer") || lower.contains("gsk_") || lower.contains("sk-") || lower.contains("aiza") {
        return "Authentication failed: Invalid or expired credentials. Please verify your provider settings.".to_string();
    }

    // Strip key=... query parameter patterns
    if let Some(pos) = raw.find("key=") {
        let prefix = &raw[..pos];
        return format!("{}key=[REDACTED]", prefix);
    }

    raw.to_string()
}

/// Normalizes an HTTP status code and response body from any provider into a typed `ProviderError`.
pub fn normalize_http_error(status: reqwest::StatusCode, raw_body: &str) -> ProviderError {
    let code = status.as_u16();

    // Try extracting JSON error message
    let parsed_message = if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_body) {
        val.get("error")
            .and_then(|e| {
                if let Some(msg) = e.get("message").and_then(|m| m.as_str()) {
                    Some(msg.to_string())
                } else if let Some(msg) = e.as_str() {
                    Some(msg.to_string())
                } else {
                    None
                }
            })
            .or_else(|| val.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
    } else {
        None
    };

    let detail = parsed_message
        .unwrap_or_else(|| {
            if !raw_body.trim().is_empty() && !raw_body.contains('{') {
                raw_body.trim().to_string()
            } else {
                format!("Request failed with HTTP {}", code)
            }
        });

    let sanitized = sanitize_error_message(&detail);

    match code {
        401 | 403 => ProviderError::AuthFailure { message: sanitized },
        404 => ProviderError::ModelUnavailable {
            model: "unknown".to_string(),
            reason: sanitized,
        },
        429 => ProviderError::RateLimited {
            retry_after_secs: None,
            message: sanitized,
        },
        400 | 422 => ProviderError::InvalidRequest { message: sanitized },
        500..=599 => ProviderError::ServerError {
            status_code: code,
            message: sanitized,
        },
        408 => ProviderError::Timeout { message: sanitized },
        _ => ProviderError::Unknown { message: sanitized },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitization_redacts_tokens() {
        let msg = "Invalid API key gsk_abcdef123456789 supplied in header";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("gsk_"));
        assert!(sanitized.contains("Authentication failed"));

        let msg2 = "Unauthorized: Bearer secret_token_xyz was rejected";
        let sanitized2 = sanitize_error_message(msg2);
        assert!(!sanitized2.contains("secret_token_xyz"));
        assert!(sanitized2.contains("Authentication failed"));
    }

    #[test]
    fn test_normalize_http_error_json() {
        let body = r#"{"error": {"message": "Invalid model specified", "type": "invalid_request_error"}}"#;
        let err = normalize_http_error(reqwest::StatusCode::BAD_REQUEST, body);
        match err {
            ProviderError::InvalidRequest { message } => {
                assert_eq!(message, "Invalid model specified");
            }
            _ => panic!("Expected InvalidRequest, got {:?}", err),
        }
    }

    #[test]
    fn test_normalize_http_error_auth() {
        let body = r#"{"error": {"message": "Incorrect API key provided: gsk_12345"}}"#;
        let err = normalize_http_error(reqwest::StatusCode::UNAUTHORIZED, body);
        match err {
            ProviderError::AuthFailure { message } => {
                assert!(!message.contains("gsk_"));
            }
            _ => panic!("Expected AuthFailure, got {:?}", err),
        }
    }

    #[test]
    fn test_normalize_http_error_rate_limit() {
        let body = r#"{"error": {"message": "Rate limit reached for requests"}}"#;
        let err = normalize_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS, body);
        match err {
            ProviderError::RateLimited { message, .. } => {
                assert!(message.contains("Rate limit reached"));
            }
            _ => panic!("Expected RateLimited, got {:?}", err),
        }
    }
}
