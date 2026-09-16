use crate::policy::context::PolicyContext;
use crate::policy::types::{ActionRequest, ActionTarget, PolicyConstraints, PolicyOutcome, RiskLevel};

/// Evaluates browser automation actions and DOM interaction proposals.
pub struct BrowserAdapter;

impl BrowserAdapter {
    pub fn evaluate(
        req: &ActionRequest,
        _ctx: &PolicyContext,
        constraints: &PolicyConstraints,
    ) -> (RiskLevel, PolicyOutcome, String) {
        let op = req.operation.trim().to_lowercase();

        // 1. Evaluate URL / Navigation Actions
        if op == "navigate" || op == "open_url" || op == "browser_open_url" || op == "new_tab" || op == "browser_new_tab" {
            let url_str = match &req.target {
                ActionTarget::Url(u) => Some(u.clone()),
                _ => req.arguments.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
            };

            if let Some(target_url) = url_str {
                let lower_url = target_url.trim().to_lowercase();

                // Prohibit dangerous URI schemes
                if lower_url.starts_with("javascript:") {
                    return (
                        RiskLevel::Critical,
                        PolicyOutcome::Blocked,
                        "Arbitrary JavaScript execution via javascript: URL scheme is strictly prohibited.".to_string(),
                    );
                }

                if lower_url.starts_with("file:") {
                    return (
                        RiskLevel::Critical,
                        PolicyOutcome::Blocked,
                        "Local filesystem navigation via file: URL scheme is strictly prohibited.".to_string(),
                    );
                }

                if lower_url.starts_with("data:text/html") || lower_url.starts_with("vbscript:") {
                    return (
                        RiskLevel::Critical,
                        PolicyOutcome::Blocked,
                        "Unsafe data: or script execution URI scheme is prohibited.".to_string(),
                    );
                }

                // Check external native protocol handlers
                if !lower_url.starts_with("http://")
                    && !lower_url.starts_with("https://")
                    && !lower_url.starts_with("about:")
                    && lower_url.contains(':')
                {
                    return (
                        RiskLevel::High,
                        PolicyOutcome::Blocked,
                        format!(
                            "External native protocol '{}' is blocked from autonomous execution.",
                            lower_url.split(':').next().unwrap_or("")
                        ),
                    );
                }

                // Network constraint check
                if !constraints.allow_network_access {
                    return (
                        RiskLevel::High,
                        PolicyOutcome::Blocked,
                        "Network access is disabled in current policy constraints.".to_string(),
                    );
                }

                return (
                    RiskLevel::Low,
                    PolicyOutcome::Allow,
                    "Standard HTTP/HTTPS navigation approved.".to_string(),
                );
            } else if op == "new_tab" || op == "browser_new_tab" {
                return (
                    RiskLevel::Low,
                    PolicyOutcome::Allow,
                    "Opening blank new tab approved.".to_string(),
                );
            }
        }

        // 2. Read-only, Passive Observation & Low-Risk Interaction Tools
        if matches!(
            op.as_str(),
            "get_tabs"
                | "get_active_tab"
                | "observe"
                | "screenshot"
                | "switch_tab"
                | "close_tab"
                | "back"
                | "forward"
                | "reload"
                | "scroll"
                | "press_key"
                | "focus"
                | "wait"
                | "select_option"
                | "history_recent"
                | "history_search"
                | "bookmarks_list"
                | "bookmarks_search"
                | "downloads_recent"
                | "download_get"
                | "extract"
        ) {
            return (
                RiskLevel::Low,
                PolicyOutcome::Allow,
                "Passive observation, storage query, or standard navigation action permitted.".to_string(),
            );
        }

        // 2b. Destructive or Sensitive Storage & Download Operations
        if matches!(
            op.as_str(),
            "history_delete"
                | "history_clear"
                | "bookmark_remove"
                | "download_cancel"
                | "download_start"
                | "save_page"
                | "print"
        ) {
            return (
                RiskLevel::High,
                PolicyOutcome::ConfirmationRequired,
                format!(
                    "High consequence browser operation '{}' requires explicit operator confirmation.",
                    op
                ),
            );
        }

        // 3. Typing Security & Sensitive Input Analysis
        if op == "type" || op == "browser_type" {
            let is_password = req
                .arguments
                .get("is_password")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let text_to_type = req
                .arguments
                .get("text")
                .or_else(|| req.arguments.get("text_to_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut selector_or_name = match &req.target {
                ActionTarget::BrowserElement { selector, element_id, .. } => {
                    selector.clone().or_else(|| element_id.clone()).unwrap_or_default()
                }
                _ => String::new(),
            };
            if selector_or_name.is_empty() {
                selector_or_name = req
                    .arguments
                    .get("selector")
                    .or_else(|| req.arguments.get("element_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }

            let lower_selector = selector_or_name.to_lowercase();
            let is_password_field = is_password
                || lower_selector.contains("password")
                || lower_selector.contains("passwd")
                || lower_selector.contains("pwd");

            if is_password_field {
                return (
                    RiskLevel::Critical,
                    PolicyOutcome::ConfirmationRequired,
                    "Target is a sensitive credential / password field. Explicit operator confirmation required."
                        .to_string(),
                );
            }

            // Financial & Card credentials
            let lower_text = text_to_type.to_lowercase();
            if lower_selector.contains("cvv")
                || lower_selector.contains("cvc")
                || lower_selector.contains("creditcard")
                || lower_selector.contains("cardnumber")
                || lower_text.contains("card")
            {
                return (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    "Target involves payment or credit card details. Explicit operator confirmation required."
                        .to_string(),
                );
            }

            return (
                RiskLevel::Medium,
                PolicyOutcome::Allow,
                "Standard text input action permitted.".to_string(),
            );
        }

        // 4. Click Actions & High-Risk Buttons
        if op == "click" || op == "browser_click" {
            let mut element_text = match &req.target {
                ActionTarget::BrowserElement { text, .. } => text.clone().unwrap_or_default(),
                _ => String::new(),
            };
            if element_text.is_empty() {
                element_text = req
                    .arguments
                    .get("element_text")
                    .or_else(|| req.arguments.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }

            let lower_text = element_text.to_lowercase();

            // Financial transaction triggers
            if lower_text.contains("buy now")
                || lower_text.contains("place order")
                || lower_text.contains("pay")
                || lower_text.contains("checkout")
                || lower_text.contains("purchase")
                || lower_text.contains("confirm payment")
            {
                return (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    format!(
                        "Clicking financial transaction button '{}' requires explicit operator confirmation.",
                        element_text
                    ),
                );
            }

            // Destructive actions
            if lower_text.contains("delete account")
                || lower_text.contains("cancel subscription")
                || lower_text.contains("wipe data")
                || lower_text.contains("destroy")
            {
                return (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    format!(
                        "Clicking destructive action button '{}' requires explicit operator confirmation.",
                        element_text
                    ),
                );
            }

            return (
                RiskLevel::Low,
                PolicyOutcome::Allow,
                "Standard UI interaction click permitted.".to_string(),
            );
        }

        // Default browser action fallback
        (
            RiskLevel::Medium,
            PolicyOutcome::Allow,
            "Browser operation evaluated under standard policies.".to_string(),
        )
    }
}
