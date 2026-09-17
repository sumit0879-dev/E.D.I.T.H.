//! computer.rs — Policy Engine Computer Domain Security Adapter
//!
//! Enforces host-level security constraints, risk classification, and human confirmation (HITL)
//! requirements before any desktop or system control action is dispatched.

use crate::computer_control::{ComputerControlState, GLOBAL_COMPUTER_CONTROL_MGR};
use crate::policy::context::PolicyContext;
use crate::policy::types::{ActionRequest, ActionTarget, PolicyConstraints, PolicyOutcome, RiskLevel};
use crate::plugins::BUILTIN_APPS;

/// Evaluates desktop automation and system interaction proposals.
pub struct ComputerAdapter;

impl ComputerAdapter {
    pub fn evaluate(
        req: &ActionRequest,
        _ctx: &PolicyContext,
        constraints: &PolicyConstraints,
    ) -> (RiskLevel, PolicyOutcome, String) {
        // 1. Human Takeover State Check
        if GLOBAL_COMPUTER_CONTROL_MGR.get_control_state() == ComputerControlState::AiPaused {
            return (
                RiskLevel::High,
                PolicyOutcome::Blocked,
                "Autonomous computer execution is currently paused by human operator takeover.".to_string(),
            );
        }

        let op = req.operation.trim().to_lowercase();

        // 2. Read-Only Observation & Passive Desktop Inspection
        if matches!(
            op.as_str(),
            "observe_screen" | "screenshot" | "get_active_window" | "list_windows" | "wait"
        ) {
            return (
                RiskLevel::Low,
                PolicyOutcome::Allow,
                "Passive desktop observation or read-only query permitted.".to_string(),
            );
        }

        // 3. Mouse Movement and Scrolling
        if matches!(op.as_str(), "move_cursor" | "scroll") {
            return (
                RiskLevel::Low,
                PolicyOutcome::Allow,
                "Non-destructive pointer movement or scrolling permitted.".to_string(),
            );
        }

        // 4. Mouse Clicks (click, double_click, right_click)
        if matches!(op.as_str(), "click" | "double_click" | "right_click") {
            // Inspect target window/context if specified
            let target_str = match &req.target {
                ActionTarget::SystemTarget(t) => t.to_lowercase(),
                _ => req.arguments.get("target").and_then(|v| v.as_str()).unwrap_or("").to_lowercase(),
            };

            // Disallow clicking into secure desktop or UAC prompts
            if target_str.contains("uac")
                || target_str.contains("user account control")
                || target_str.contains("windows security")
            {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::Blocked,
                    "Automated clicking into privileged Windows Security or UAC dialogs is strictly prohibited.".to_string(),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                "Standard mouse click interaction permitted.".to_string(),
            );
        }

        // 5. Window Focus
        if op == "focus_window" {
            let title = match &req.target {
                ActionTarget::SystemTarget(t) => t.to_lowercase(),
                _ => req.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("").to_lowercase(),
            };

            if title.contains("uac") || title.contains("windows security") {
                return (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    "Focusing security dialog requires explicit operator confirmation.".to_string(),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                "Focusing application window permitted.".to_string(),
            );
        }

        // 6. Application Launching (launch_app)
        if op == "launch_app" {
            if !constraints.allow_command_execution {
                return (
                    RiskLevel::High,
                    PolicyOutcome::Blocked,
                    "Application launching is disabled under active policy constraints.".to_string(),
                );
            }

            let app_name = match &req.target {
                ActionTarget::SystemTarget(a) => a.clone(),
                _ => req.arguments.get("app_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            };

            let trimmed = app_name.trim().to_lowercase();
            if trimmed.is_empty() {
                return (
                    RiskLevel::Low,
                    PolicyOutcome::Blocked,
                    "Target application name cannot be empty.".to_string(),
                );
            }

            // Verify if it's a known built-in app
            let is_builtin = BUILTIN_APPS.iter().any(|b| {
                b.name.eq_ignore_ascii_case(&trimmed) || b.path.eq_ignore_ascii_case(&trimmed)
            });

            // If not registered or whitelisted, block arbitrary execution
            if !is_builtin {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::Blocked,
                    format!("Application '{}' is not registered in the approved application catalog.", app_name),
                );
            }

            // Launching approved application requires human confirmation to prevent background surprises
            return (
                RiskLevel::High,
                PolicyOutcome::ConfirmationRequired,
                format!("Launching application '{}' requires explicit operator confirmation.", app_name),
            );
        }

        // 7. Window Closure (close_window)
        if op == "close_window" {
            let title = match &req.target {
                ActionTarget::SystemTarget(t) => t.to_lowercase(),
                _ => req.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("").to_lowercase(),
            };

            if title.contains("explorer") || title.contains("edith") || title.contains("task manager") || title.contains("taskmgr") {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::Blocked,
                    "Closing core system components, task manager, or E.D.I.T.H. is strictly prohibited.".to_string(),
                );
            }

            return (
                RiskLevel::High,
                PolicyOutcome::ConfirmationRequired,
                format!("Closing application window '{}' may cause unsaved data loss and requires explicit confirmation.", title),
            );
        }

        // 8. Text Typing (type)
        if op == "type" {
            let text = req.arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let is_sensitive = req.arguments.get("is_sensitive").and_then(|v| v.as_bool()).unwrap_or(false);

            let lower_text = text.to_lowercase();
            let has_sensitive_tokens = is_sensitive
                || lower_text.contains("password")
                || lower_text.contains("passwd")
                || lower_text.contains("api_key")
                || lower_text.contains("secret")
                || lower_text.contains("bearer ");

            if has_sensitive_tokens {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::ConfirmationRequired,
                    "Typing sensitive credential or secret data requires explicit operator confirmation.".to_string(),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                "Standard keyboard text entry permitted.".to_string(),
            );
        }

        // 9. Single Key Press (press_key)
        if op == "press_key" {
            let key = req.arguments.get("key").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            
            // Critical keys that can alter system power or security
            if key == "power" || key == "sleep" {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::Blocked,
                    "System power/sleep keys are prohibited from automated execution.".to_string(),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                format!("Single keypress '{}' permitted.", key),
            );
        }

        // 10. Key Combinations / Hotkeys (hotkey)
        if op == "hotkey" {
            let keys: Vec<String> = req
                .arguments
                .get("keys")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|k| k.as_str().map(|s| s.to_lowercase()))
                        .collect()
                })
                .unwrap_or_default();

            let joined = keys.join("+");

            // Strictly prohibited system escalation hotkeys
            if joined.contains("ctrl+alt+del") || joined.contains("win+r") || joined.contains("win+l") {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::Blocked,
                    format!("Privileged system hotkey '{}' is strictly prohibited from autonomous execution.", joined),
                );
            }

            // High-risk application termination or management hotkeys
            if joined.contains("alt+f4") || joined.contains("ctrl+shift+esc") {
                return (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    format!("High-consequence desktop hotkey '{}' requires explicit operator confirmation.", joined),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                format!("Standard application hotkey '{}' permitted.", joined),
            );
        }

        // Default: Unknown computer operation
        (
            RiskLevel::High,
            PolicyOutcome::Blocked,
            format!("Unknown or unhandled computer domain operation '{}'.", op),
        )
    }
}
