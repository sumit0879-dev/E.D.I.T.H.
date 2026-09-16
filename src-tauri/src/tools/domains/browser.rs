use crate::browser::BrowserState;
use crate::browser_tools::execute_browser_tool;
use crate::tools::cancellation::ScopedCancellationToken;
use crate::tools::executor::DomainExecutor;
use crate::tools::types::{ToolDefinition, ToolDomain, ToolExecutionError, ToolRequest};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use tauri::{AppHandle, Manager};

/// Returns the normalized tool definitions for the 15 stable Browser domain tools.
pub fn get_browser_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::new(
            "browser.observe",
            ToolDomain::Browser,
            "Observe the live rendered DOM, interactive elements, forms, and headings of a browser tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "scope": {
                        "type": "string",
                        "enum": ["full_page", "visible_viewport", "region", "element"],
                        "description": "Observation scope filter"
                    }
                },
                "required": ["tab_id"]
            }),
            true,
            10000,
        ),
        ToolDefinition::new(
            "browser.screenshot",
            ToolDomain::Browser,
            "Capture a viewport screenshot of a browser tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            true,
            10000,
        ),
        ToolDefinition::new(
            "browser.get_tabs",
            ToolDomain::Browser,
            "List all open browser tabs with their titles, URLs, and active focus status.",
            json!({
                "type": "object",
                "properties": {}
            }),
            true,
            5000,
        ),
        ToolDefinition::new(
            "browser.get_active_tab",
            ToolDomain::Browser,
            "Retrieve state and URL of the currently focused browser tab.",
            json!({
                "type": "object",
                "properties": {}
            }),
            true,
            5000,
        ),
        ToolDefinition::new(
            "browser.navigate",
            ToolDomain::Browser,
            "Navigate a browser tab to an HTTP or HTTPS destination URL.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "url": { "type": "string", "description": "Target HTTP/HTTPS URL" }
                },
                "required": ["tab_id", "url"]
            }),
            false,
            30000,
        ),
        ToolDefinition::new(
            "browser.click",
            ToolDomain::Browser,
            "Click an interactive element identified by its deterministic element_id.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "element_id": { "type": "string", "description": "Target element ID from observation" }
                },
                "required": ["tab_id", "element_id"]
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "browser.type",
            ToolDomain::Browser,
            "Type text into an input field or textarea. Rejects password fields for security.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "element_id": { "type": "string", "description": "Target input element ID" },
                    "text": { "type": "string", "description": "Text to insert" },
                    "clear_first": { "type": "boolean", "description": "Whether to clear input before typing" }
                },
                "required": ["tab_id", "element_id", "text"]
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "browser.scroll",
            ToolDomain::Browser,
            "Scroll the browser viewport in a specified direction with bounded increment.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right", "top", "bottom"],
                        "description": "Direction to scroll"
                    },
                    "amount": { "type": "number", "minimum": 50, "maximum": 1500, "description": "Pixels to scroll" }
                },
                "required": ["tab_id", "direction"]
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "browser.press_key",
            ToolDomain::Browser,
            "Dispatch a keyboard keypress event to the focused element in the active tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" },
                    "key": {
                        "type": "string",
                        "enum": [
                            "Enter", "Escape", "Tab", "Backspace", "Delete",
                            "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
                            "Home", "End", "PageUp", "PageDown", "Space"
                        ],
                        "description": "Key to press"
                    }
                },
                "required": ["tab_id", "key"]
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "browser.new_tab",
            ToolDomain::Browser,
            "Open a new browser tab with an optional initial URL.",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Optional initial URL" }
                }
            }),
            false,
            15000,
        ),
        ToolDefinition::new(
            "browser.close_tab",
            ToolDomain::Browser,
            "Close a specified browser tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "browser.switch_tab",
            ToolDomain::Browser,
            "Switch the active user-facing focus to another browser tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "browser.back",
            ToolDomain::Browser,
            "Navigate backward in browser tab history.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "browser.forward",
            ToolDomain::Browser,
            "Navigate forward in browser tab history.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "browser.reload",
            ToolDomain::Browser,
            "Reload the active page in a browser tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "Target browser tab identifier" }
                },
                "required": ["tab_id"]
            }),
            false,
            15000,
        ),
    ]
}

/// Domain executor bridging Universal Tool Runtime requests to the existing WebView2 browser engine.
#[derive(Clone)]
pub struct BrowserDomainExecutor {
    app: Option<AppHandle>,
}

impl BrowserDomainExecutor {
    pub fn new(app: Option<AppHandle>) -> Self {
        Self { app }
    }

    /// Translates namespaced "browser.*" name to legacy "browser_*" name expected by browser_tools.rs
    fn map_tool_name(name: &str) -> &str {
        match name {
            "browser.observe" => "browser_observe",
            "browser.screenshot" => "browser_screenshot",
            "browser.get_tabs" => "browser_get_tabs",
            "browser.get_active_tab" => "browser_get_active_tab",
            "browser.navigate" => "browser_open_url",
            "browser.click" => "browser_click",
            "browser.type" => "browser_type",
            "browser.scroll" => "browser_scroll",
            "browser.press_key" => "browser_press_key",
            "browser.new_tab" => "browser_new_tab",
            "browser.close_tab" => "browser_close_tab",
            "browser.switch_tab" => "browser_switch_tab",
            "browser.back" => "browser_back",
            "browser.forward" => "browser_forward",
            "browser.reload" => "browser_reload",
            other => other,
        }
    }
}

impl DomainExecutor for BrowserDomainExecutor {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Browser
    }

    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        _definition: &'a ToolDefinition,
        cancel_token: ScopedCancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ToolExecutionError>> + Send + 'a>> {
        Box::pin(async move {
            // Check cancellation before dispatching
            if cancel_token.is_cancelled() {
                return Err(ToolExecutionError::Cancelled(
                    "Execution cancelled prior to dispatch.".to_string(),
                ));
            }

            // Headless / Test mode when Tauri AppHandle is absent
            let app = match &self.app {
                Some(a) => a,
                None => {
                    // Safe mock return for headless tests
                    return Ok(json!({
                        "mock": true,
                        "tool": request.tool_name,
                        "arguments": request.arguments,
                        "status": "success"
                    }));
                }
            };

            let state = app.state::<BrowserState>();
            let legacy_name = Self::map_tool_name(&request.tool_name);

            // Execute through existing browser_tools engine with cancellation race
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    Err(ToolExecutionError::Cancelled(
                        "Execution was cancelled during browser operation.".to_string(),
                    ))
                }
                res = execute_browser_tool(app.clone(), legacy_name, &request.arguments, state) => {
                    match res {
                        Ok(exec_res) => {
                            if exec_res.success {
                                Ok(exec_res.data.unwrap_or(json!({ "success": true })))
                            } else {
                                let err_msg = exec_res.error.unwrap_or_else(|| "Browser action failed.".to_string());
                                Err(ToolExecutionError::DomainError(err_msg))
                            }
                        }
                        Err(err) => Err(ToolExecutionError::DomainError(err)),
                    }
                }
            }
        })
    }
}
