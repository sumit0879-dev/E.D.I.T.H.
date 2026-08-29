use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

// ============================================================================
// PHASE 5.5 CONTROL OWNERSHIP MODEL & DATA STRUCTURES (Steps 1, 2)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrowserControlState {
    UserControlled,
    AiControlled,
    AiPaused,
    WaitingForApproval,
    Transitioning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabControlInfo {
    pub tab_id: String,
    pub control_state: BrowserControlState,
    pub last_transition: u64,
    pub ai_task_id: Option<String>,
    pub reason: Option<String>,
}

pub struct BrowserControlManager {
    pub tab_controls: Mutex<HashMap<String, TabControlInfo>>,
}

impl Default for BrowserControlManager {
    fn default() -> Self {
        Self {
            tab_controls: Mutex::new(HashMap::new()),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_CONTROL_MGR: Arc<BrowserControlManager> = Arc::new(BrowserControlManager::default());
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

// ============================================================================
// CONTROL STATE MACHINE IMPLEMENTATION (Steps 3, 4, 5, 7, 8, 9, 20, 22)
// ============================================================================

impl BrowserControlManager {
    /// Returns control state of a tab, defaulting to UserControlled (Step 20 & 22 Fail-Safe)
    pub fn get_tab_control_state(&self, tab_id: &str) -> BrowserControlState {
        let controls = self.tab_controls.lock().unwrap();
        controls.get(tab_id)
            .map(|c| c.control_state)
            .unwrap_or(BrowserControlState::UserControlled)
    }

    /// Returns full TabControlInfo for a tab
    pub fn get_tab_control_info(&self, tab_id: &str) -> TabControlInfo {
        let controls = self.tab_controls.lock().unwrap();
        controls.get(tab_id).cloned().unwrap_or_else(|| TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::UserControlled,
            last_transition: current_timestamp_ms(),
            ai_task_id: None,
            reason: Some("Default user control".to_string()),
        })
    }

    /// Returns all tab control records
    pub fn get_all_tab_controls(&self) -> Vec<TabControlInfo> {
        let controls = self.tab_controls.lock().unwrap();
        controls.values().cloned().collect()
    }

    /// Step 3: User -> AI Handoff
    pub fn request_ai_control(&self, app: &AppHandle, tab_id: &str, task_id: Option<String>) -> Result<TabControlInfo, String> {
        let mut controls = self.tab_controls.lock().unwrap();
        let now = current_timestamp_ms();

        let info = TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::AiControlled,
            last_transition: now,
            ai_task_id: task_id,
            reason: Some("Control granted to AI".to_string()),
        };

        controls.insert(tab_id.to_string(), info.clone());

        let _ = app.emit("browser-control-changed", json!({
            "tab_id": tab_id,
            "control_state": "AI_CONTROLLED",
            "timestamp": now
        }));

        Ok(info)
    }

    /// Step 4: AI -> User Immediate Takeover (Host-Enforced Priority)
    pub fn takeover_tab(&self, app: &AppHandle, tab_id: &str, reason: Option<String>) -> Result<TabControlInfo, String> {
        let mut controls = self.tab_controls.lock().unwrap();
        let now = current_timestamp_ms();

        let info = TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::UserControlled,
            last_transition: now,
            ai_task_id: None,
            reason: reason.or_else(|| Some("Immediate human takeover".to_string())),
        };

        controls.insert(tab_id.to_string(), info.clone());

        let _ = app.emit("browser-control-changed", json!({
            "tab_id": tab_id,
            "control_state": "USER_CONTROLLED",
            "timestamp": now,
            "event": "HUMAN_TAKEOVER"
        }));

        Ok(info)
    }

    /// Step 5: Explicit AI Release
    pub fn release_ai_control(&self, app: &AppHandle, tab_id: &str) -> Result<TabControlInfo, String> {
        let mut controls = self.tab_controls.lock().unwrap();
        let now = current_timestamp_ms();

        let info = TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::UserControlled,
            last_transition: now,
            ai_task_id: None,
            reason: Some("AI task finished. Control returned to human.".to_string()),
        };

        controls.insert(tab_id.to_string(), info.clone());

        let _ = app.emit("browser-control-changed", json!({
            "tab_id": tab_id,
            "control_state": "USER_CONTROLLED",
            "timestamp": now,
            "event": "AI_RELEASED"
        }));

        Ok(info)
    }

    /// Step 7: Pause AI Control
    pub fn pause_ai_control(&self, app: &AppHandle, tab_id: &str) -> Result<TabControlInfo, String> {
        let mut controls = self.tab_controls.lock().unwrap();
        let now = current_timestamp_ms();

        let current_task = controls.get(tab_id).and_then(|c| c.ai_task_id.clone());
        let info = TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::AiPaused,
            last_transition: now,
            ai_task_id: current_task,
            reason: Some("AI control paused by operator".to_string()),
        };

        controls.insert(tab_id.to_string(), info.clone());

        let _ = app.emit("browser-control-changed", json!({
            "tab_id": tab_id,
            "control_state": "AI_PAUSED",
            "timestamp": now
        }));

        Ok(info)
    }

    /// Step 7: Resume AI Control (Forces re-observation)
    pub fn resume_ai_control(&self, app: &AppHandle, tab_id: &str) -> Result<TabControlInfo, String> {
        let mut controls = self.tab_controls.lock().unwrap();
        let now = current_timestamp_ms();

        let current_task = controls.get(tab_id).and_then(|c| c.ai_task_id.clone());
        let info = TabControlInfo {
            tab_id: tab_id.to_string(),
            control_state: BrowserControlState::AiControlled,
            last_transition: now,
            ai_task_id: current_task,
            reason: Some("AI control resumed. Fresh observation required.".to_string()),
        };

        controls.insert(tab_id.to_string(), info.clone());

        let _ = app.emit("browser-control-changed", json!({
            "tab_id": tab_id,
            "control_state": "AI_CONTROLLED",
            "timestamp": now,
            "event": "RESUMED_FRESH_OBSERVE"
        }));

        Ok(info)
    }

    /// Step 9: Host-side check before any AI action execution
    pub fn verify_ai_action_permitted(&self, tab_id: &str, tool_name: &str) -> Result<(), String> {
        // Read-only observation tools are permitted for background inspection
        if tool_name == "browser_observe" || tool_name == "browser_screenshot" {
            return Ok(());
        }

        let state = self.get_tab_control_state(tab_id);
        match state {
            BrowserControlState::AiControlled => Ok(()),
            BrowserControlState::UserControlled => Err(format!(
                "CONTROL_REJECTED: Tab '{}' is currently USER_CONTROLLED. AI actions are blocked after human takeover.",
                tab_id
            )),
            BrowserControlState::AiPaused => Err(format!(
                "CONTROL_REJECTED: Tab '{}' is currently AI_PAUSED. Resume AI control before executing actions.",
                tab_id
            )),
            BrowserControlState::WaitingForApproval => Err(format!(
                "CONTROL_REJECTED: Tab '{}' is WAITING_FOR_APPROVAL. Human approval is required before execution.",
                tab_id
            )),
            BrowserControlState::Transitioning => Err(format!(
                "CONTROL_REJECTED: Tab '{}' is in TRANSITIONING state.",
                tab_id
            )),
        }
    }
}

// ============================================================================
// TAURI COMMANDS FOR HUMAN <-> AI CONTROL
// ============================================================================

#[tauri::command]
pub fn browser_request_ai_control(app: AppHandle, tab_id: String, task_id: Option<String>) -> Result<TabControlInfo, String> {
    GLOBAL_CONTROL_MGR.request_ai_control(&app, &tab_id, task_id)
}

#[tauri::command]
pub fn browser_takeover_tab(app: AppHandle, tab_id: String, reason: Option<String>) -> Result<TabControlInfo, String> {
    GLOBAL_CONTROL_MGR.takeover_tab(&app, &tab_id, reason)
}

#[tauri::command]
pub fn browser_release_ai_control(app: AppHandle, tab_id: String) -> Result<TabControlInfo, String> {
    GLOBAL_CONTROL_MGR.release_ai_control(&app, &tab_id)
}

#[tauri::command]
pub fn browser_pause_ai_control(app: AppHandle, tab_id: String) -> Result<TabControlInfo, String> {
    GLOBAL_CONTROL_MGR.pause_ai_control(&app, &tab_id)
}

#[tauri::command]
pub fn browser_resume_ai_control(app: AppHandle, tab_id: String) -> Result<TabControlInfo, String> {
    GLOBAL_CONTROL_MGR.resume_ai_control(&app, &tab_id)
}

#[tauri::command]
pub fn browser_get_tab_control_info(tab_id: String) -> Result<TabControlInfo, String> {
    Ok(GLOBAL_CONTROL_MGR.get_tab_control_info(&tab_id))
}

#[tauri::command]
pub fn browser_get_all_tab_controls() -> Result<Vec<TabControlInfo>, String> {
    Ok(GLOBAL_CONTROL_MGR.get_all_tab_controls())
}
