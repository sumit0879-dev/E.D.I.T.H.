//! computer.rs — Universal Tool Runtime Computer Control Domain
//!
//! Provides normalized tool definitions, structured argument validation schemas,
//! and the `ComputerDomainExecutor` implementation dispatching to platform adapters.

use crate::computer_control::{ComputerControlState, GLOBAL_COMPUTER_CONTROL_MGR};
use crate::tools::cancellation::ScopedCancellationToken;
use crate::tools::domains::computer_platform::ComputerPlatform;
#[cfg(not(target_os = "windows"))]
use crate::tools::domains::computer_platform::MockPlatformAdapter;
#[cfg(target_os = "windows")]
use crate::tools::domains::computer_platform::WindowsPlatformAdapter;
use crate::tools::executor::DomainExecutor;
use crate::tools::types::{ToolDefinition, ToolDomain, ToolExecutionError, ToolRequest};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Returns the normalized tool definitions for the 14 stable Computer domain tools.
pub fn get_computer_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::new(
            "computer.observe_screen",
            ToolDomain::Computer,
            "Inspect desktop screen dimensions, display count, cursor coordinates, and active foreground window summary.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        ToolDefinition::new(
            "computer.screenshot",
            ToolDomain::Computer,
            "Capture a visual screenshot of the primary display or active window as a base64 JPEG data URL.",
            json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "enum": ["screen", "active_window"],
                        "description": "Visual capture region: entire screen or active window bounds"
                    }
                },
                "additionalProperties": false
            }),
            true,
            10000,
        ),
        ToolDefinition::new(
            "computer.get_active_window",
            ToolDomain::Computer,
            "Query the currently focused top-level window including title, process name, PID, and bounding coordinates.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        ToolDefinition::new(
            "computer.list_windows",
            ToolDomain::Computer,
            "List all open and visible desktop application windows with their titles, process names, and bounds.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
            5000,
        ),
        ToolDefinition::new(
            "computer.focus_window",
            ToolDomain::Computer,
            "Activate and bring an application window into foreground focus by title or process name substring.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Title substring of the target window to focus" },
                    "process_name": { "type": "string", "description": "Optional process name filter (e.g. 'notepad.exe')" }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.launch_app",
            ToolDomain::Computer,
            "Launch an approved application from the registered built-in or custom application catalog.",
            json!({
                "type": "object",
                "properties": {
                    "app_name": { "type": "string", "description": "Name or executable identifier of the approved application" }
                },
                "required": ["app_name"],
                "additionalProperties": false
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "computer.close_window",
            ToolDomain::Computer,
            "Gracefully request an open application window to close via WM_CLOSE.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Title substring of the target window to close" },
                    "process_name": { "type": "string", "description": "Optional process name filter" }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.move_cursor",
            ToolDomain::Computer,
            "Move the mouse cursor to absolute desktop coordinates (x, y).",
            json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate in pixels from top-left" },
                    "y": { "type": "integer", "description": "Y coordinate in pixels from top-left" }
                },
                "required": ["x", "y"],
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.click",
            ToolDomain::Computer,
            "Perform a mouse button click (left, right, middle) at current or specified desktop coordinates.",
            json!({
                "type": "object",
                "properties": {
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "description": "Mouse button to click (default: 'left')"
                    },
                    "x": { "type": "integer", "description": "Optional X coordinate to move cursor before clicking" },
                    "y": { "type": "integer", "description": "Optional Y coordinate to move cursor before clicking" }
                },
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.double_click",
            ToolDomain::Computer,
            "Perform a double left-click at current or specified desktop coordinates.",
            json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "Optional X coordinate to move cursor before clicking" },
                    "y": { "type": "integer", "description": "Optional Y coordinate to move cursor before clicking" }
                },
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.right_click",
            ToolDomain::Computer,
            "Perform a right-click at current or specified desktop coordinates to open context menus.",
            json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "Optional X coordinate to move cursor before clicking" },
                    "y": { "type": "integer", "description": "Optional Y coordinate to move cursor before clicking" }
                },
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.type",
            ToolDomain::Computer,
            "Type text into the currently focused desktop input field or window.",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text characters to type into the focused element" },
                    "is_sensitive": { "type": "boolean", "description": "Flag indicating if the text contains sensitive credentials" }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
            false,
            10000,
        ),
        ToolDefinition::new(
            "computer.press_key",
            ToolDomain::Computer,
            "Press and release a keyboard key (e.g. enter, tab, escape, backspace, arrows, function keys).",
            json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "enum": [
                            "enter", "tab", "escape", "backspace", "space",
                            "up", "down", "left", "right", "home", "end",
                            "pageup", "pagedown", "delete", "f1", "f2",
                            "f3", "f4", "f5", "f6", "f7", "f8", "f9",
                            "f10", "f11", "f12"
                        ],
                        "description": "Standard keyboard key name"
                    }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
            false,
            5000,
        ),
        ToolDefinition::new(
            "computer.hotkey",
            ToolDomain::Computer,
            "Execute a keyboard shortcut combination in sequence (e.g. ['ctrl', 'c'], ['alt', 'tab']).",
            json!({
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of key names to press together in order"
                    }
                },
                "required": ["keys"],
                "additionalProperties": false
            }),
            false,
            5000,
        ),
    ]
}

/// Authoritative execution adapter for the Computer Control domain.
pub struct ComputerDomainExecutor {
    app: Option<AppHandle>,
    platform: Arc<dyn ComputerPlatform>,
}

impl ComputerDomainExecutor {
    /// Creates a production executor using native Windows platform APIs.
    pub fn new(app: Option<AppHandle>) -> Self {
        #[cfg(target_os = "windows")]
        let platform: Arc<dyn ComputerPlatform> = Arc::new(WindowsPlatformAdapter::new());
        #[cfg(not(target_os = "windows"))]
        let platform: Arc<dyn ComputerPlatform> = Arc::new(MockPlatformAdapter::new());

        Self { app, platform }
    }

    /// Creates an executor with an explicit platform adapter (useful for testing and mocks).
    pub fn with_platform(app: Option<AppHandle>, platform: Arc<dyn ComputerPlatform>) -> Self {
        Self { app, platform }
    }
}

impl DomainExecutor for ComputerDomainExecutor {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Computer
    }

    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        _definition: &'a ToolDefinition,
        cancel_token: ScopedCancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ToolExecutionError>> + Send + 'a>>
    {
        Box::pin(async move {
            // 1. Cooperative Cancellation Check
            if cancel_token.is_cancelled() {
                return Err(ToolExecutionError::Cancelled(
                    "Computer action cancelled prior to dispatch.".to_string(),
                ));
            }

            // 2. Human Takeover & Control State Preemption
            if GLOBAL_COMPUTER_CONTROL_MGR.get_control_state() == ComputerControlState::AiPaused {
                return Err(ToolExecutionError::DomainError(
                    "Computer execution preempted: Human operator is currently in control."
                        .to_string(),
                ));
            }

            // Headless / Test mode when Tauri AppHandle is absent and mock platform active
            if self.app.is_none() && cfg!(test) {
                // Return structured mock outputs for tests that verify routing
            }

            let op = request
                .tool_name
                .strip_prefix("computer.")
                .unwrap_or(&request.tool_name);
            let args = &request.arguments;

            match op {
                "observe_screen" => {
                    let metrics = self.platform.observe_screen().map_err(|e| {
                        ToolExecutionError::DomainError(format!("Failed to observe screen: {}", e))
                    })?;
                    Ok(json!({
                        "success": true,
                        "metrics": metrics,
                        "control_state": GLOBAL_COMPUTER_CONTROL_MGR.get_control_state()
                    }))
                }

                "screenshot" => {
                    let target = args
                        .get("target")
                        .and_then(|v| v.as_str())
                        .unwrap_or("screen");
                    let data_url = self.platform.screenshot(target).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Screenshot capture failed: {}", e))
                    })?;
                    Ok(json!({
                        "success": true,
                        "target": target,
                        "image_data_url": data_url
                    }))
                }

                "get_active_window" => {
                    let window = self.platform.get_active_window().map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to query active window: {}",
                            e
                        ))
                    })?;
                    Ok(json!({
                        "success": true,
                        "active_window": window
                    }))
                }

                "list_windows" => {
                    let windows = self.platform.list_windows().map_err(|e| {
                        ToolExecutionError::DomainError(format!(
                            "Failed to list open windows: {}",
                            e
                        ))
                    })?;
                    Ok(json!({
                        "success": true,
                        "count": windows.len(),
                        "windows": windows
                    }))
                }

                "focus_window" => {
                    let title = args.get("title").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required field 'title'.".to_string(),
                        )
                    })?;
                    let process_name = args.get("process_name").and_then(|v| v.as_str());

                    let focused = self
                        .platform
                        .focus_window(title, process_name)
                        .map_err(|e| {
                            ToolExecutionError::DomainError(format!(
                                "Failed to focus window: {}",
                                e
                            ))
                        })?;

                    if !focused {
                        return Err(ToolExecutionError::DomainError(format!(
                            "No matching window found with title '{}'",
                            title
                        )));
                    }

                    Ok(json!({
                        "success": true,
                        "focused_window": title,
                        "verified": true
                    }))
                }

                "launch_app" => {
                    let app_name =
                        args.get("app_name")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ToolExecutionError::InvalidArguments(
                                    "Missing required field 'app_name'.".to_string(),
                                )
                            })?;

                    // Execute through central AppLauncherPolicy with database connection if available
                    let conn_guard = if let Some(ref app) = self.app {
                        if let Some(db_state) = app.try_state::<crate::db::DbState>() {
                            Some(db_state)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let res = if let Some(state) = conn_guard {
                        let conn = state
                            .conn
                            .lock()
                            .map_err(|e| ToolExecutionError::DomainError(e.to_string()))?;
                        crate::security::AppLauncherPolicy::validate_and_launch(
                            app_name,
                            Some(&conn),
                        )
                    } else {
                        crate::security::AppLauncherPolicy::validate_and_launch(app_name, None)
                    };

                    match res {
                        Ok(msg) => Ok(json!({
                            "success": true,
                            "app_name": app_name,
                            "message": msg
                        })),
                        Err(err) => Err(ToolExecutionError::DomainError(err)),
                    }
                }

                "close_window" => {
                    let title = args.get("title").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required field 'title'.".to_string(),
                        )
                    })?;

                    let closed = self.platform.close_window(title).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Failed to close window: {}", e))
                    })?;

                    if !closed {
                        return Err(ToolExecutionError::DomainError(format!(
                            "No matching window found to close with title '{}'",
                            title
                        )));
                    }

                    Ok(json!({
                        "success": true,
                        "closed_window": title
                    }))
                }

                "move_cursor" => {
                    let x = args.get("x").and_then(|v| v.as_i64()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required integer 'x'.".to_string(),
                        )
                    })? as i32;
                    let y = args.get("y").and_then(|v| v.as_i64()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required integer 'y'.".to_string(),
                        )
                    })? as i32;

                    self.platform.move_cursor(x, y).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Failed to move cursor: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "cursor": { "x": x, "y": y }
                    }))
                }

                "click" => {
                    let button = args
                        .get("button")
                        .and_then(|v| v.as_str())
                        .unwrap_or("left");
                    let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);

                    self.platform.click(button, x, y).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Mouse click failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "clicked_button": button,
                        "coordinates": { "x": x, "y": y }
                    }))
                }

                "double_click" => {
                    let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);

                    self.platform.double_click(x, y).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Double click failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "coordinates": { "x": x, "y": y }
                    }))
                }

                "right_click" => {
                    let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);

                    self.platform.right_click(x, y).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Right click failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "coordinates": { "x": x, "y": y }
                    }))
                }

                "type" => {
                    let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required field 'text'.".to_string(),
                        )
                    })?;

                    self.platform.type_text(text).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Keyboard typing failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "character_count": text.chars().count()
                    }))
                }

                "press_key" => {
                    let key = args.get("key").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolExecutionError::InvalidArguments(
                            "Missing required field 'key'.".to_string(),
                        )
                    })?;

                    self.platform.press_key(key).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Key press failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "pressed_key": key
                    }))
                }

                "hotkey" => {
                    let keys_arr =
                        args.get("keys").and_then(|v| v.as_array()).ok_or_else(|| {
                            ToolExecutionError::InvalidArguments(
                                "Missing required array 'keys'.".to_string(),
                            )
                        })?;

                    let keys: Vec<String> = keys_arr
                        .iter()
                        .filter_map(|k| k.as_str().map(|s| s.to_string()))
                        .collect();

                    if keys.is_empty() {
                        return Err(ToolExecutionError::InvalidArguments(
                            "Hotkey array cannot be empty.".to_string(),
                        ));
                    }

                    self.platform.hotkey(&keys).map_err(|e| {
                        ToolExecutionError::DomainError(format!("Hotkey execution failed: {}", e))
                    })?;

                    Ok(json!({
                        "success": true,
                        "hotkey": keys.join("+")
                    }))
                }

                other => Err(ToolExecutionError::DomainError(format!(
                    "Unsupported computer domain operation '{}'",
                    other
                ))),
            }
        })
    }
}
