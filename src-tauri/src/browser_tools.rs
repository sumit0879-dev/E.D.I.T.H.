use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use tauri::{AppHandle, Manager};
use crate::browser::{
    BrowserState,
    browser_observe_tab, browser_screenshot_tab, browser_click_element,
    browser_type_element, browser_scroll, browser_press_key, browser_focus_element,
    browser_wait, browser_create_tab, browser_switch_tab,
    browser_close_tab, browser_go_back_tab, browser_go_forward_tab, browser_reload_tab,
    browser_get_multi_state, browser_navigate_tab,
};
use crate::browser_risk::{BrowserRiskEngine, BrowserActionContext, BrowserRiskDecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: String,  // "observation", "navigation", "interaction"
    pub risk_level: String, // "OBSERVE", "LOW_RISK_ACTION", "BLOCKED_FOR_AI"
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolExecutionResult {
    pub success: bool,
    pub tool_name: String,
    pub tab_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: u64,
}

/// Returns the complete catalog of Browser Tool schemas for LLM discovery
pub fn get_browser_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // --- Observation Tools ---
        ToolDefinition {
            name: "browser_get_tabs".to_string(),
            description: "List all open browser tabs, their URLs, titles, and active status.".to_string(),
            category: "observation".to_string(),
            risk_level: "OBSERVE".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "browser_get_active_tab".to_string(),
            description: "Get the currently active browser tab information.".to_string(),
            category: "observation".to_string(),
            risk_level: "OBSERVE".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "browser_observe".to_string(),
            description: "Observe the live rendered DOM, semantic regions, headings, forms, links, and interactive elements of a browser tab.".to_string(),
            category: "observation".to_string(),
            risk_level: "OBSERVE".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Identifier of the tab to observe (e.g. 'tab_a')."
                    },
                    "scope": {
                        "type": "string",
                        "enum": ["full_page", "visible_viewport", "region", "element"],
                        "description": "Optional observation scope (default: 'full_page')."
                    }
                },
                "required": ["tab_id"]
            }),
        },
        ToolDefinition {
            name: "browser_screenshot".to_string(),
            description: "Capture a native viewport screenshot of a browser tab.".to_string(),
            category: "observation".to_string(),
            risk_level: "OBSERVE".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Identifier of the tab to screenshot."
                    }
                },
                "required": ["tab_id"]
            }),
        },

        // --- Navigation Tools ---
        ToolDefinition {
            name: "browser_open_url".to_string(),
            description: "Navigate a browser tab to an HTTPS URL or search query. Creates tab if not exists.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier (e.g. 'tab_a')."
                    },
                    "url": {
                        "type": "string",
                        "description": "Target URL (e.g. 'https://example.com') or search term."
                    }
                },
                "required": ["tab_id", "url"]
            }),
        },
        ToolDefinition {
            name: "browser_switch_tab".to_string(),
            description: "Switch active focus to a specific browser tab.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Tab identifier to switch to."
                    }
                },
                "required": ["tab_id"]
            }),
        },
        ToolDefinition {
            name: "browser_close_tab".to_string(),
            description: "Close a browser tab and release its resources.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Tab identifier to close."
                    }
                },
                "required": ["tab_id"]
            }),
        },
        ToolDefinition {
            name: "browser_back".to_string(),
            description: "Navigate back in browser tab history.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    }
                },
                "required": ["tab_id"]
            }),
        },
        ToolDefinition {
            name: "browser_forward".to_string(),
            description: "Navigate forward in browser tab history.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    }
                },
                "required": ["tab_id"]
            }),
        },
        ToolDefinition {
            name: "browser_reload".to_string(),
            description: "Reload the current page in a browser tab.".to_string(),
            category: "navigation".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    }
                },
                "required": ["tab_id"]
            }),
        },

        // --- Interaction Tools ---
        ToolDefinition {
            name: "browser_click".to_string(),
            description: "Click an interactive element identified by its deterministic element_id.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "element_id": {
                        "type": "string",
                        "description": "Target element identifier (from browser_observe, e.g. 'id_search' or 'el_button_...')."
                    }
                },
                "required": ["tab_id", "element_id"]
            }),
        },
        ToolDefinition {
            name: "browser_type".to_string(),
            description: "Type text into an input field or textarea. Strictly rejects password fields for security.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "element_id": {
                        "type": "string",
                        "description": "Target element identifier (from browser_observe)."
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to insert into the input field."
                    },
                    "clear_first": {
                        "type": "boolean",
                        "description": "Whether to clear existing text before typing (default: true)."
                    }
                },
                "required": ["tab_id", "element_id", "text"]
            }),
        },
        ToolDefinition {
            name: "browser_scroll".to_string(),
            description: "Scroll the browser viewport in a specified direction with bounded increment.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right", "top", "bottom"],
                        "description": "Direction to scroll."
                    },
                    "amount": {
                        "type": "number",
                        "description": "Scroll step in pixels (bounded between 50 and 1500, default: 350)."
                    }
                },
                "required": ["tab_id", "direction"]
            }),
        },
        ToolDefinition {
            name: "browser_press_key".to_string(),
            description: "Dispatch a key press event from the allowed key enum to the active element.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "key": {
                        "type": "string",
                        "enum": [
                            "Enter", "Escape", "Tab", "Backspace", "Delete",
                            "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
                            "Home", "End", "PageUp", "PageDown", "Space"
                        ],
                        "description": "Key to press."
                    }
                },
                "required": ["tab_id", "key"]
            }),
        },
        ToolDefinition {
            name: "browser_focus".to_string(),
            description: "Focus an element on the active page.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "element_id": {
                        "type": "string",
                        "description": "Target element identifier."
                    }
                },
                "required": ["tab_id", "element_id"]
            }),
        },
        ToolDefinition {
            name: "browser_wait".to_string(),
            description: "Wait for a page load, element, url change, or bounded timeout.".to_string(),
            category: "interaction".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target tab identifier."
                    },
                    "condition": {
                        "type": "string",
                        "enum": ["timeout", "url_changed", "element_present", "text_present", "page_load"],
                        "description": "Condition to wait for."
                    },
                    "target": {
                        "type": "string",
                        "description": "Target value (e.g. expected element_id, text substring, or initial URL)."
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "Maximum wait timeout in ms (max 10000ms, default 3000ms)."
                    }
                },
                "required": ["tab_id", "condition"]
            }),
        },

        // --- Phase 5.6A: Browser History & Bookmarks Tools (Part L) ---
        ToolDefinition {
            name: "browser_history_recent".to_string(),
            description: "Retrieve recent browser history entries (newest first).".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of history items to return (default 20, max 100)."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "browser_history_search".to_string(),
            description: "Search browsing history by URL or page title query.".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query matching against history URL or title."
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results (default 20, max 100)."
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "browser_history_delete".to_string(),
            description: "Delete a specific browser history record by its ID. Requires operator approval.".to_string(),
            category: "storage".to_string(),
            risk_level: "REQUIRE_APPROVAL".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Unique identifier of the history record to delete."
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "browser_history_clear".to_string(),
            description: "Permanently wipe all browser history. High consequence action requiring operator approval.".to_string(),
            category: "storage".to_string(),
            risk_level: "REQUIRE_APPROVAL".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "browser_bookmarks_list".to_string(),
            description: "Retrieve all saved browser bookmarks.".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "browser_bookmarks_search".to_string(),
            description: "Search saved bookmarks by title or URL.".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search term matching title or URL."
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "browser_bookmark_add".to_string(),
            description: "Save a new bookmark with title and URL (only http:// and https:// allowed).".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Bookmark display title."
                    },
                    "url": {
                        "type": "string",
                        "description": "Target web page URL."
                    },
                    "folder_id": {
                        "type": "string",
                        "description": "Optional folder ID to organize bookmark."
                    }
                },
                "required": ["title", "url"]
            }),
        },
        ToolDefinition {
            name: "browser_bookmark_remove".to_string(),
            description: "Remove a saved bookmark by ID. Requires operator confirmation.".to_string(),
            category: "storage".to_string(),
            risk_level: "REQUIRE_APPROVAL".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Bookmark ID to remove."
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "browser_bookmark_open".to_string(),
            description: "Open a saved bookmark in a browser tab.".to_string(),
            category: "storage".to_string(),
            risk_level: "LOW_RISK_ACTION".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "Target browser tab ID."
                    },
                    "url": {
                        "type": "string",
                        "description": "Bookmark URL to navigate to."
                    }
                },
                "required": ["tab_id", "url"]
            }),
        },
    ]
}

/// Executes a Browser Tool call deterministically through the Browser Core
pub async fn execute_browser_tool(
    app: AppHandle,
    tool_name: &str,
    args: &serde_json::Value,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserToolExecutionResult, String> {
    let start = Instant::now();

    // Phase 5.3: Centralized Host-Enforced Risk & Safety Assessment
    let target_tab_id = args.get("tab_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Phase 5.5: Human <-> AI Control Ownership & Takeover Verification (Step 9)
    if !target_tab_id.is_empty() {
        if let Err(ctrl_err) = crate::browser_control::GLOBAL_CONTROL_MGR.verify_ai_action_permitted(&target_tab_id, tool_name) {
            return Ok(BrowserToolExecutionResult {
                success: false,
                tool_name: tool_name.to_string(),
                tab_id: Some(target_tab_id),
                data: None,
                error: Some(ctrl_err),
                error_code: Some("CONTROL_TAKEOVER_BLOCKED".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    }

    // Phase 5.3: Central Host-Enforced Risk & Safety Assessment Before Execution
    let action_url = args.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let action_element_id = args.get("element_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let action_text = args.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());

    let risk_ctx = BrowserActionContext {
        tool_name: tool_name.to_string(),
        tab_id: target_tab_id.clone(),
        url: action_url,
        title: None,
        element_id: action_element_id.clone(),
        element_tag: None,
        element_role: None,
        element_text: action_text.clone(),
        element_aria_label: None,
        element_href: None,
        input_type: None,
        placeholder: None,
        text_to_type: action_text,
        is_password: false,
        form_action: None,
        form_method: None,
        parent_region: None,
    };

    let assessment = BrowserRiskEngine::assess_risk(&risk_ctx);
    BrowserRiskEngine::record_audit_log(None, tool_name.to_string(), target_tab_id.clone(), &assessment);

    if assessment.decision == BrowserRiskDecision::Block {
        return Ok(BrowserToolExecutionResult {
            success: false,
            tool_name: tool_name.to_string(),
            tab_id: if target_tab_id.is_empty() { None } else { Some(target_tab_id) },
            data: None,
            error: Some(assessment.user_explanation),
            error_code: Some(assessment.policy_code),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    if assessment.decision == BrowserRiskDecision::RequireApproval {
        let approval_id = BrowserRiskEngine::create_pending_approval(None, risk_ctx, assessment.clone());
        return Ok(BrowserToolExecutionResult {
            success: false,
            tool_name: tool_name.to_string(),
            tab_id: if target_tab_id.is_empty() { None } else { Some(target_tab_id) },
            data: Some(json!({
                "approval_required": true,
                "approval_id": approval_id,
                "policy_code": assessment.policy_code
            })),
            error: Some(format!("REQUIRE_APPROVAL: {}", assessment.user_explanation)),
            error_code: Some("REQUIRE_APPROVAL".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    match tool_name {
        "browser_get_tabs" => {
            let multi = browser_get_multi_state(app, state).await?;
            let tabs_json = json!({
                "tabs": multi.tabs,
                "active_tab_id": multi.active_tab_id,
                "total_tabs": multi.tabs.len()
            });
            Ok(BrowserToolExecutionResult {
                success: true,
                tool_name: tool_name.to_string(),
                tab_id: multi.active_tab_id,
                data: Some(tabs_json),
                error: None,
                error_code: None,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        }

        "browser_get_active_tab" => {
            let multi = browser_get_multi_state(app, state).await?;
            let active = multi.tabs.iter().find(|t| Some(&t.id) == multi.active_tab_id.as_ref());
            if let Some(tab) = active {
                Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab.id.clone()),
                    data: Some(json!(tab)),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            } else {
                Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some("No active browser tab found.".to_string()),
                    error_code: Some("NO_ACTIVE_TAB".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
        }

        "browser_observe" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let scope = args.get("scope").and_then(|v| v.as_str()).map(|s| s.to_string());

            match browser_observe_tab(app, tab_id.to_string(), scope, state).await {
                Ok(obs) => {
                    let compact_data = json!({
                        "tab_id": obs.tab_id,
                        "url": obs.url,
                        "title": obs.title,
                        "generation": obs.generation,
                        "fingerprint": obs.fingerprint,
                        "viewport": obs.viewport,
                        "visible_text": obs.visible_text.chars().take(20000).collect::<String>(),
                        "selected_text": obs.selected_text,
                        "regions": obs.regions,
                        "headings": obs.headings,
                        "interactive_elements_count": obs.interactive_elements.len(),
                        "interactive_elements": obs.interactive_elements.iter().take(60).collect::<Vec<_>>(),
                        "forms": obs.forms,
                        "links": obs.links.iter().take(30).collect::<Vec<_>>(),
                        "timestamp": obs.timestamp,
                    });
                    Ok(BrowserToolExecutionResult {
                        success: true,
                        tool_name: tool_name.to_string(),
                        tab_id: Some(tab_id.to_string()),
                        data: Some(compact_data),
                        error: None,
                        error_code: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("OBSERVATION_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_screenshot" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_screenshot_tab(tab_id.to_string(), None, state).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({
                        "tab_id": res.tab_id,
                        "width": res.width,
                        "height": res.height,
                        "data_url_length": res.data_url.len(),
                    })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("SCREENSHOT_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_open_url" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let url = args.get("url").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'url'.".to_string())?;

            match browser_create_tab(app, tab_id.to_string(), Some(url.to_string()), None, state).await {
                Ok(tab) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab.id),
                    data: Some(json!({
                        "tab_id": tab_id,
                        "url": tab.url,
                        "title": tab.title,
                    })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("NAVIGATION_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_switch_tab" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_switch_tab(app, tab_id.to_string(), None, state).await {
                Ok(tab) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab.id),
                    data: Some(json!({ "tab_id": tab_id, "url": tab.url })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("SWITCH_TAB_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_close_tab" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_close_tab(app, tab_id.to_string(), state).await {
                Ok(next_tab) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({
                        "closed_tab_id": tab_id,
                        "next_active_tab": next_tab.map(|t| t.id),
                    })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("CLOSE_TAB_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_back" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_go_back_tab(app, tab_id.to_string()).await {
                Ok(_) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "tab_id": tab_id, "action": "back" })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("BACK_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_forward" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_go_forward_tab(app, tab_id.to_string()).await {
                Ok(_) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "tab_id": tab_id, "action": "forward" })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("FORWARD_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_reload" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;

            match browser_reload_tab(app, tab_id.to_string()).await {
                Ok(_) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "tab_id": tab_id, "action": "reload" })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("RELOAD_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_click" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let element_id = args.get("element_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'element_id'.".to_string())?;

            match browser_click_element(app, tab_id.to_string(), element_id.to_string(), state).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({
                        "element_id": res.element_id,
                        "page_changed": res.page_changed,
                        "url_changed": res.url_changed,
                        "resulting_url": res.resulting_url,
                    })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("CLICK_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_type" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let element_id = args.get("element_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'element_id'.".to_string())?;
            let text = args.get("text").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'text'.".to_string())?;
            let clear_first = args.get("clear_first").and_then(|v| v.as_bool()).unwrap_or(true);

            // Bounded input validation
            if text.len() > 5000 {
                return Err("INPUT_TOO_LARGE: Type text exceeds maximum bounded length (5000 characters).".to_string());
            }

            match browser_type_element(app, tab_id.to_string(), element_id.to_string(), text.to_string(), Some(clear_first)).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({
                        "element_id": res.element_id,
                        "characters_typed": text.len(),
                        "page_changed": res.page_changed,
                    })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("TYPE_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_scroll" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let direction = args.get("direction").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'direction'.".to_string())?;
            let amount = args.get("amount").and_then(|v| v.as_i64()).map(|n| n as i32);

            match browser_scroll(app, tab_id.to_string(), direction.to_string(), amount).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "direction": direction, "page_changed": res.page_changed })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("SCROLL_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_press_key" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let key = args.get("key").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'key'.".to_string())?;

            match browser_press_key(app, tab_id.to_string(), key.to_string()).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "key": key, "page_changed": res.page_changed })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("KEY_PRESS_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_focus" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let element_id = args.get("element_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'element_id'.".to_string())?;

            match browser_focus_element(app, tab_id.to_string(), element_id.to_string()).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "element_id": res.element_id })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("FOCUS_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_wait" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let condition = args.get("condition").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'condition'.".to_string())?;
            let target = args.get("target").and_then(|v| v.as_str()).map(|s| s.to_string());
            let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64());

            match browser_wait(app, tab_id.to_string(), condition.to_string(), target, timeout_ms, state).await {
                Ok(res) => Ok(BrowserToolExecutionResult {
                    success: res.success,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "condition": condition, "resulting_url": res.resulting_url })),
                    error: res.error,
                    error_code: res.error_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("WAIT_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        // --- Phase 5.6A: Browser History & Bookmarks Tool Execution ---
        "browser_history_recent" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|u| u as u32);
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::get_recent_browser_history(&conn, limit) {
                Ok(entries) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "count": entries.len(), "entries": entries })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_history_search" => {
            let query = args.get("query").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'query'.".to_string())?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|u| u as u32);
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::search_browser_history(&conn, query, limit) {
                Ok(entries) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "query": query, "count": entries.len(), "entries": entries })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_history_delete" => {
            let id = args.get("id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'id'.".to_string())?;
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::delete_browser_history_entry(&conn, id) {
                Ok(deleted) => Ok(BrowserToolExecutionResult {
                    success: deleted,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "id": id, "deleted": deleted })),
                    error: if deleted { None } else { Some("Record not found".to_string()) },
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_history_clear" => {
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::clear_browser_history(&conn) {
                Ok(count) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "cleared_count": count })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_bookmarks_list" => {
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::get_all_browser_bookmarks(&conn) {
                Ok(bookmarks) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "count": bookmarks.len(), "bookmarks": bookmarks })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_bookmarks_search" => {
            let query = args.get("query").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'query'.".to_string())?;
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::search_browser_bookmarks(&conn, query) {
                Ok(bookmarks) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "query": query, "count": bookmarks.len(), "bookmarks": bookmarks })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_bookmark_add" => {
            let title = args.get("title").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'title'.".to_string())?;
            let url = args.get("url").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'url'.".to_string())?;
            let folder_id = args.get("folder_id").and_then(|v| v.as_str());

            let url_trimmed = url.trim();
            if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
                return Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some("INVALID_URL: Only standard http:// and https:// URLs can be bookmarked.".to_string()),
                    error_code: Some("INVALID_URL_SCHEME".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }

            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::add_browser_bookmark(&conn, title, url_trimmed, folder_id, None) {
                Ok(bm) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!(bm)),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_bookmark_remove" => {
            let id = args.get("id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'id'.".to_string())?;
            let db_state = app.try_state::<crate::db::DbState>()
                .ok_or_else(|| "DB_UNAVAILABLE: Database state is not loaded.".to_string())?;
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            match crate::db::delete_browser_bookmark(&conn, id) {
                Ok(deleted) => Ok(BrowserToolExecutionResult {
                    success: deleted,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: Some(json!({ "id": id, "deleted": deleted })),
                    error: if deleted { None } else { Some("Bookmark not found".to_string()) },
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: None,
                    data: None,
                    error: Some(e.to_string()),
                    error_code: Some("DB_ERROR".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        "browser_bookmark_open" => {
            let tab_id = args.get("tab_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'tab_id'.".to_string())?;
            let url = args.get("url").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'url'.".to_string())?;

            match browser_navigate_tab(app, tab_id.to_string(), url.to_string(), state).await {
                Ok(loaded_url) => Ok(BrowserToolExecutionResult {
                    success: true,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: Some(json!({ "url": loaded_url })),
                    error: None,
                    error_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Ok(BrowserToolExecutionResult {
                    success: false,
                    tool_name: tool_name.to_string(),
                    tab_id: Some(tab_id.to_string()),
                    data: None,
                    error: Some(e),
                    error_code: Some("NAVIGATION_FAILED".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }

        _ => Err(format!("UNKNOWN_BROWSER_TOOL: Tool '{}' is not registered in the Browser Tool Layer.", tool_name)),
    }
}

// -----------------------------------------------------------------------------
// Tauri Command Exposing Browser Tool Layer
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_get_tool_definitions_cmd() -> Result<Vec<ToolDefinition>, String> {
    Ok(get_browser_tool_definitions())
}

#[tauri::command]
pub async fn browser_execute_tool_cmd(
    app: AppHandle,
    tool_name: String,
    arguments: serde_json::Value,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserToolExecutionResult, String> {
    execute_browser_tool(app, &tool_name, &arguments, state).await
}
