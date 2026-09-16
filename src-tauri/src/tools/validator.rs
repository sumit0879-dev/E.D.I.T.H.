use super::types::ToolExecutionError;
use serde_json::Value;

/// Evaluates and validates incoming tool argument payloads against strict JSON Schema contracts.
pub struct ArgumentValidator;

impl ArgumentValidator {
    pub fn validate(args: &Value, schema: &Value) -> Result<(), ToolExecutionError> {
        // 1. Arguments payload must be a JSON Object
        let obj = args.as_object().ok_or_else(|| {
            ToolExecutionError::InvalidArguments("Arguments payload must be a JSON object.".to_string())
        })?;

        // 2. Validate required properties
        if let Some(required_arr) = schema.get("required").and_then(|v| v.as_array()) {
            for req_val in required_arr {
                if let Some(req_key) = req_val.as_str() {
                    if !obj.contains_key(req_key) {
                        return Err(ToolExecutionError::InvalidArguments(format!(
                            "Missing required argument: '{}'.",
                            req_key
                        )));
                    }
                }
            }
        }

        // 3. Validate property types and bounds if declared
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            // Check additionalProperties if explicitly set to false
            if let Some(false) = schema.get("additionalProperties").and_then(|v| v.as_bool()) {
                for key in obj.keys() {
                    if !props.contains_key(key) {
                        return Err(ToolExecutionError::InvalidArguments(format!(
                            "Unrecognized argument '{}' rejected by strict schema.",
                            key
                        )));
                    }
                }
            }

            for (key, val) in obj {
                if let Some(prop_spec) = props.get(key) {
                    Self::validate_value(key, val, prop_spec)?;
                }
            }
        }

        Ok(())
    }

    fn validate_value(key: &str, val: &Value, spec: &Value) -> Result<(), ToolExecutionError> {
        // Type checking
        if let Some(expected_type) = spec.get("type").and_then(|v| v.as_str()) {
            match expected_type {
                "string" => {
                    let s = val.as_str().ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be a string.",
                            key
                        ))
                    })?;

                    // String length bounds
                    if let Some(min_len) = spec.get("minLength").and_then(|v| v.as_u64()) {
                        if (s.len() as u64) < min_len {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' length must be at least {} characters.",
                                key, min_len
                            )));
                        }
                    }

                    if let Some(max_len) = spec.get("maxLength").and_then(|v| v.as_u64()) {
                        if (s.len() as u64) > max_len {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' length must not exceed {} characters.",
                                key, max_len
                            )));
                        }
                    }

                    // Enum membership check
                    if let Some(enum_vals) = spec.get("enum").and_then(|v| v.as_array()) {
                        let allowed_strings: Vec<&str> =
                            enum_vals.iter().filter_map(|e| e.as_str()).collect();
                        if !allowed_strings.contains(&s) {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' value '{}' is not one of allowed enum values: {:?}",
                                key, s, allowed_strings
                            )));
                        }
                    }
                }
                "number" => {
                    let n = val.as_f64().ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be a number.",
                            key
                        ))
                    })?;

                    if let Some(min) = spec.get("minimum").and_then(|v| v.as_f64()) {
                        if n < min {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' value {} is below minimum bound {}.",
                                key, n, min
                            )));
                        }
                    }

                    if let Some(max) = spec.get("maximum").and_then(|v| v.as_f64()) {
                        if n > max {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' value {} exceeds maximum bound {}.",
                                key, n, max
                            )));
                        }
                    }
                }
                "integer" => {
                    let i = val.as_i64().ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be an integer.",
                            key
                        ))
                    })?;

                    if let Some(min) = spec.get("minimum").and_then(|v| v.as_i64()) {
                        if i < min {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' value {} is below minimum bound {}.",
                                key, i, min
                            )));
                        }
                    }

                    if let Some(max) = spec.get("maximum").and_then(|v| v.as_i64()) {
                        if i > max {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' value {} exceeds maximum bound {}.",
                                key, i, max
                            )));
                        }
                    }
                }
                "boolean" => {
                    if !val.is_boolean() {
                        return Err(ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be a boolean.",
                            key
                        )));
                    }
                }
                "array" => {
                    let arr = val.as_array().ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be an array.",
                            key
                        ))
                    })?;

                    if let Some(min_items) = spec.get("minItems").and_then(|v| v.as_u64()) {
                        if (arr.len() as u64) < min_items {
                            return Err(ToolExecutionError::InvalidArguments(format!(
                                "Argument '{}' array must contain at least {} items.",
                                key, min_items
                            )));
                        }
                    }
                }
                "object" => {
                    if !val.is_object() {
                        return Err(ToolExecutionError::InvalidArguments(format!(
                            "Argument '{}' must be a JSON object.",
                            key
                        )));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
