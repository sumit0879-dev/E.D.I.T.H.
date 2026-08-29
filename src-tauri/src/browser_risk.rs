use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

// ============================================================================
// BROWSER RISK & SAFETY ENGINE TYPES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrowserRiskLevel {
    Low,
    Medium,
    High,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrowserRiskDecision {
    Allow,
    RequireApproval,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionContext {
    pub tool_name: String,
    pub tab_id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub element_id: Option<String>,
    #[serde(default)]
    pub element_tag: Option<String>,
    #[serde(default)]
    pub element_role: Option<String>,
    #[serde(default)]
    pub element_text: Option<String>,
    #[serde(default)]
    pub element_aria_label: Option<String>,
    #[serde(default)]
    pub element_href: Option<String>,
    #[serde(default)]
    pub input_type: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub text_to_type: Option<String>,
    #[serde(default)]
    pub is_password: bool,
    #[serde(default)]
    pub form_action: Option<String>,
    #[serde(default)]
    pub form_method: Option<String>,
    #[serde(default)]
    pub parent_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRiskAssessment {
    pub risk_level: BrowserRiskLevel,
    pub decision: BrowserRiskDecision,
    pub policy_code: String,
    pub reason: String,
    pub user_explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRiskAuditEntry {
    pub id: String,
    pub timestamp: u64,
    pub task_id: Option<String>,
    pub tool_name: String,
    pub tab_id: String,
    pub risk_level: BrowserRiskLevel,
    pub decision: BrowserRiskDecision,
    pub policy_code: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingBrowserActionApproval {
    pub approval_id: String,
    pub task_id: Option<String>,
    pub context: BrowserActionContext,
    pub assessment: BrowserRiskAssessment,
    pub created_at: u64,
    pub status: String, // "pending", "approved", "rejected", "expired"
}

// Global In-Memory Audit Log & Pending Approvals
lazy_static! {
    static ref BROWSER_RISK_AUDIT_LOG: Mutex<Vec<BrowserRiskAuditEntry>> = Mutex::new(Vec::new());
    static ref PENDING_APPROVALS: Mutex<HashMap<String, PendingBrowserActionApproval>> = Mutex::new(HashMap::new());
}

// ============================================================================
// BROWSER RISK & SAFETY EVALUATION ENGINE
// ============================================================================

pub struct BrowserRiskEngine;

impl BrowserRiskEngine {
    /// Evaluates the risk of an AI-requested browser action before execution.
    /// Combines action type baseline, target element semantics, form context,
    /// and destination URL into a strict, host-enforced decision.
    pub fn assess_risk(ctx: &BrowserActionContext) -> BrowserRiskAssessment {
        let tool = ctx.tool_name.as_str();

        // 1. Navigation Security Assessment (Step 8)
        if tool == "browser_open_url" {
            if let Some(ref target_url) = ctx.url {
                let lower_url = target_url.trim().to_lowercase();
                
                if lower_url.starts_with("javascript:") {
                    return BrowserRiskAssessment {
                        risk_level: BrowserRiskLevel::Blocked,
                        decision: BrowserRiskDecision::Block,
                        policy_code: "UNSAFE_SCHEME_JAVASCRIPT".to_string(),
                        reason: "Arbitrary JavaScript execution via javascript: URL scheme is strictly prohibited.".to_string(),
                        user_explanation: "Blocked: Navigation to javascript: URI schemes is prohibited for security isolation.".to_string(),
                    };
                }
                
                if lower_url.starts_with("file:") {
                    return BrowserRiskAssessment {
                        risk_level: BrowserRiskLevel::Blocked,
                        decision: BrowserRiskDecision::Block,
                        policy_code: "UNSAFE_SCHEME_FILE".to_string(),
                        reason: "Local file system navigation via file: URL scheme is strictly prohibited.".to_string(),
                        user_explanation: "Blocked: Access to local file:// paths through browser navigation is prohibited.".to_string(),
                    };
                }

                if lower_url.starts_with("data:text/html") || lower_url.starts_with("vbscript:") {
                    return BrowserRiskAssessment {
                        risk_level: BrowserRiskLevel::Blocked,
                        decision: BrowserRiskDecision::Block,
                        policy_code: "UNSAFE_DATA_SCHEME".to_string(),
                        reason: "Unsafe data: or script execution URI scheme is prohibited.".to_string(),
                        user_explanation: "Blocked: Unsafe URI scheme rejected by browser security policy.".to_string(),
                    };
                }

                // Check for unsupported native protocols (e.g. mailto, tel, steam, discord)
                if !lower_url.starts_with("http://") 
                    && !lower_url.starts_with("https://") 
                    && !lower_url.starts_with("about:")
                    && lower_url.contains(':') 
                {
                    return BrowserRiskAssessment {
                        risk_level: BrowserRiskLevel::Blocked,
                        decision: BrowserRiskDecision::Block,
                        policy_code: "UNSUPPORTED_NATIVE_PROTOCOL".to_string(),
                        reason: format!("External native protocol '{}' is blocked from autonomous execution.", lower_url.split(':').next().unwrap_or("")),
                        user_explanation: "Blocked: Launching external protocol handlers is not allowed autonomously.".to_string(),
                    };
                }
            }

            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_NAVIGATION".to_string(),
                reason: "Standard HTTP/HTTPS navigation approved.".to_string(),
                user_explanation: "Navigation permitted.".to_string(),
            };
        }

        // 2. Read-Only & Passive Observation Tools (Step 3)
        if matches!(
            tool,
            "browser_get_tabs"
                | "browser_get_active_tab"
                | "browser_observe"
                | "browser_screenshot"
                | "browser_switch_tab"
                | "browser_back"
                | "browser_forward"
                | "browser_reload"
                | "browser_scroll"
                | "browser_focus"
                | "browser_wait"
        ) {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_OBSERVATION".to_string(),
                reason: "Read-only or passive viewport interaction approved.".to_string(),
                user_explanation: "Observation action permitted.".to_string(),
            };
        }

        // 3. Typing Security & Sensitive Input Analysis (Step 6)
        if tool == "browser_type" {
            // Password fields -> STRICTLY BLOCKED
            if ctx.is_password || ctx.input_type.as_deref() == Some("password") {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::Blocked,
                    decision: BrowserRiskDecision::Block,
                    policy_code: "SENSITIVE_INPUT_PASSWORD".to_string(),
                    reason: "Automated typing into password fields is strictly prohibited.".to_string(),
                    user_explanation: "Blocked: Automated typing into password or credential fields is prohibited.".to_string(),
                };
            }

            let combined_meta = format!(
                "{} {} {} {}",
                ctx.element_aria_label.as_deref().unwrap_or(""),
                ctx.placeholder.as_deref().unwrap_or(""),
                ctx.element_text.as_deref().unwrap_or(""),
                ctx.element_id.as_deref().unwrap_or("")
            ).to_lowercase();

            // Credit Card & Payment Inputs -> HIGH / REQUIRE_APPROVAL
            if combined_meta.contains("credit card")
                || combined_meta.contains("card number")
                || combined_meta.contains("cvv")
                || combined_meta.contains("cvc")
                || combined_meta.contains("cc-number")
                || combined_meta.contains("cc-exp")
                || combined_meta.contains("cardholder")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "SENSITIVE_INPUT_PAYMENT".to_string(),
                    reason: "Target input appears to be a financial or credit card field.".to_string(),
                    user_explanation: "Approval required: this action attempts to enter payment or credit card information.".to_string(),
                };
            }

            // OTP / 2FA / Verification Code Inputs -> REQUIRE_APPROVAL
            if combined_meta.contains("one-time code")
                || combined_meta.contains("verification code")
                || combined_meta.contains("2fa")
                || combined_meta.contains("otp")
                || combined_meta.contains("security code")
                || combined_meta.contains("passcode")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "SENSITIVE_INPUT_OTP_2FA".to_string(),
                    reason: "Target input appears to be a two-factor authentication or one-time verification code.".to_string(),
                    user_explanation: "Approval required: this action enters an identity verification or 2FA passcode.".to_string(),
                };
            }

            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_INTERACTION".to_string(),
                reason: "Standard text typing approved.".to_string(),
                user_explanation: "Typing permitted.".to_string(),
            };
        }

        // 4. Click & Key Press Semantic Target Analysis (Step 4, 7)
        if tool == "browser_click" || tool == "browser_press_key" || tool == "browser_select_option" {
            let combined_target = format!(
                "{} {} {} {} {}",
                ctx.element_text.as_deref().unwrap_or(""),
                ctx.element_aria_label.as_deref().unwrap_or(""),
                ctx.element_href.as_deref().unwrap_or(""),
                ctx.form_action.as_deref().unwrap_or(""),
                ctx.placeholder.as_deref().unwrap_or("")
            ).to_lowercase();

            // A. Destructive Actions (Account Deletion, Data Erase, Repository Drop)
            if combined_target.contains("delete account")
                || combined_target.contains("delete my account")
                || combined_target.contains("close account")
                || combined_target.contains("erase data")
                || combined_target.contains("factory reset")
                || combined_target.contains("delete repository")
                || combined_target.contains("drop database")
                || combined_target.contains("wipe all")
                || combined_target.contains("cancel subscription")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "DESTRUCTIVE_ACTION".to_string(),
                    reason: "Target action appears to perform irreversible account deletion, data erasure, or cancellation.".to_string(),
                    user_explanation: "Approval required: this action appears to perform a permanent account or data deletion.".to_string(),
                };
            }

            // B. Purchase & Financial Checkout Actions
            if combined_target.contains("buy now")
                || combined_target.contains("place order")
                || combined_target.contains("pay now")
                || combined_target.contains("complete purchase")
                || combined_target.contains("confirm payment")
                || combined_target.contains("checkout now")
                || combined_target.contains("subscribe now")
                || combined_target.contains("authorize payment")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "PURCHASE_PAYMENT_ACTION".to_string(),
                    reason: "Target action appears to submit a purchase, transaction, or financial payment.".to_string(),
                    user_explanation: "Approval required: this action appears to initiate a financial transaction or purchase.".to_string(),
                };
            }

            // C. Irreversible Public Actions (Fund Transfer, Message Dispatch, Social Publishing)
            if combined_target.contains("transfer funds")
                || combined_target.contains("send money")
                || combined_target.contains("confirm wire")
                || combined_target.contains("post tweet")
                || combined_target.contains("publish post")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "SEND_MESSAGE_ACTION".to_string(),
                    reason: "Target action appears to transfer funds or broadcast a public message.".to_string(),
                    user_explanation: "Approval required: this action initiates an irreversible message or fund dispatch.".to_string(),
                };
            }

            // D. Account Security Modifications (2FA, Password Reset, API Key Revocation)
            if combined_target.contains("change password")
                || combined_target.contains("disable 2fa")
                || combined_target.contains("reset two-factor")
                || combined_target.contains("delete api key")
                || combined_target.contains("revoke token")
            {
                return BrowserRiskAssessment {
                    risk_level: BrowserRiskLevel::High,
                    decision: BrowserRiskDecision::RequireApproval,
                    policy_code: "ACCOUNT_SECURITY_ACTION".to_string(),
                    reason: "Target action modifies critical account security or credential configurations.".to_string(),
                    user_explanation: "Approval required: this action modifies account security credentials.".to_string(),
                };
            }

            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_INTERACTION".to_string(),
                reason: "Standard UI click/interaction approved.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        // Phase 5.6A: Browser History & Bookmarks Risk Policies (Part M)
        if tool == "browser_history_recent" || tool == "browser_history_search" || tool == "browser_bookmarks_list" || tool == "browser_bookmarks_search" || tool == "browser_bookmark_add" || tool == "browser_bookmark_open" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_STORAGE_READ_WRITE".to_string(),
                reason: "Standard history/bookmark query or safe bookmark addition permitted.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_history_delete" || tool == "browser_bookmark_remove" || tool == "browser_bookmark_delete" || tool == "browser_bookmarks_delete" || tool == "browser_bookmark_folder_delete" || tool == "browser_bookmarks_delete_folder" || (tool.contains("bookmark") && tool.contains("delete")) || (tool.contains("bookmark") && tool.contains("remove")) {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "STORAGE_DELETE_APPROVAL".to_string(),
                reason: "Deleting history items or bookmarks requires human operator approval.".to_string(),
                user_explanation: "Approval required: this action removes a saved browser history or bookmark record.".to_string(),
            };
        }

        if tool == "browser_history_clear" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::High,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "STORAGE_CLEAR_APPROVAL".to_string(),
                reason: "Wiping all browsing history is a high-consequence action requiring operator approval.".to_string(),
                user_explanation: "Approval required: this action will permanently clear all browsing history.".to_string(),
            };
        }

        // Phase 5.6B: Download Manager Risk Policies (Step 8, 9, 17)
        if tool == "browser_downloads_recent" || tool == "browser_download_get" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_DOWNLOAD_QUERY".to_string(),
                reason: "Querying download history permitted.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_download_cancel" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "DOWNLOAD_CANCEL_APPROVAL".to_string(),
                reason: "Cancelling an active download requires operator confirmation.".to_string(),
                user_explanation: "Approval required: this action cancels an in-progress file download.".to_string(),
            };
        }

        if tool.contains("execute") || tool.contains("run_download") {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Blocked,
                decision: BrowserRiskDecision::Block,
                policy_code: "BLOCKED_BINARY_EXECUTION".to_string(),
                reason: "Automatic execution of downloaded binaries is strictly blocked for security.".to_string(),
                user_explanation: "Blocked: autonomous execution of downloaded files is prohibited.".to_string(),
            };
        }

        if tool == "browser_download_start" || tool.contains("download") {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "FILE_DOWNLOAD_ACTION".to_string(),
                reason: "File download requires operator confirmation to prevent untrusted payload retrieval.".to_string(),
                user_explanation: "Approval required: the agent requests to download an external file to disk.".to_string(),
            };
        }

        // Phase 5.6C: Browser Profile Isolation Risk Policies (Step 13)
        if tool == "browser_profiles_list" || tool == "browser_profile_get" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_PROFILE_QUERY".to_string(),
                reason: "Listing and inspecting browser profile metadata is permitted.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_profile_create" || tool == "browser_profile_rename" || tool == "browser_profile_switch" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "PROFILE_SWITCH_APPROVAL".to_string(),
                reason: "Creating, renaming, or switching browser profiles requires operator authorization.".to_string(),
                user_explanation: "Approval required: this action creates or switches the browser storage profile.".to_string(),
            };
        }

        if tool == "browser_profile_delete" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::High,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "PROFILE_DELETE_APPROVAL".to_string(),
                reason: "Deleting a browser profile and its storage directory requires explicit human approval.".to_string(),
                user_explanation: "Approval required: this action will permanently delete a browser profile and its storage data.".to_string(),
            };
        }

        // Phase 5.6E: Content Blocking & Privacy Policy Risk Rules (Step 16)
        if tool == "browser_protection_status" || tool == "browser_site_protection_status" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_PRIVACY_QUERY".to_string(),
                reason: "Inspecting content blocking and privacy status is permitted.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_site_allow" || tool == "browser_site_disallow" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "PRIVACY_SITE_ALLOWLIST_APPROVAL".to_string(),
                reason: "Modifying site privacy exceptions and allowlists requires operator confirmation.".to_string(),
                user_explanation: "Approval required: the agent requests to modify privacy protection settings for this site.".to_string(),
            };
        }

        // Phase 5.6F-A: Advanced Browser Utilities Risk Rules
        if tool == "browser_find" || tool == "browser_find_next" || tool == "browser_find_previous" || tool == "browser_zoom" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_BROWSER_UTILITY".to_string(),
                reason: "Finding text and adjusting zoom in page are safe non-mutating utilities.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_print" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "PRINT_ACTION_APPROVAL".to_string(),
                reason: "Initiating a print job triggers an external physical or PDF dialog requiring confirmation.".to_string(),
                user_explanation: "Approval required: the agent requests to print the active web page.".to_string(),
            };
        }

        // Phase 5.6F-B: Save Page + Reader Mode Risk Rules
        if tool == "browser_reader_mode_enter" || tool == "browser_reader_mode_exit" || tool == "browser_reader_mode_get" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_READER_MODE".to_string(),
                reason: "Reader mode content extraction and viewing is a safe read-only operation.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_save_page" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "SAVE_PAGE_APPROVAL".to_string(),
                reason: "Saving a webpage snapshot writes files to the local disk, requiring operator approval.".to_string(),
                user_explanation: "Approval required: the agent requests to save the current web page to disk.".to_string(),
            };
        }

        // Phase 5.6F-C: Tab Groups & Advanced Tab Management Risk Rules
        if tool == "browser_tab_groups_list" || tool == "browser_tab_group_create" || tool == "browser_tab_group_rename" || tool == "browser_tab_group_move_tab" || tool == "browser_tab_group_remove_tab" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Low,
                decision: BrowserRiskDecision::Allow,
                policy_code: "SAFE_TAB_GROUP_ACTION".to_string(),
                reason: "Creating, renaming, moving, and organizing tab groups is a safe metadata operation.".to_string(),
                user_explanation: "Action permitted.".to_string(),
            };
        }

        if tool == "browser_tab_group_delete" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "TAB_GROUP_DELETE_APPROVAL".to_string(),
                reason: "Deleting a user tab group requires confirmation (tabs will be ungrouped).".to_string(),
                user_explanation: "Approval required: the agent requests to delete a tab group.".to_string(),
            };
        }

        if tool == "browser_tab_group_close_tabs" {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::High,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "CLOSE_GROUP_TABS_APPROVAL".to_string(),
                reason: "Bulk closing all tabs in a group is a high-consequence action requiring operator approval.".to_string(),
                user_explanation: "Approval required: this action will close all tabs inside the specified group.".to_string(),
            };
        }

        if tool.contains("cookie") || tool.contains("credential") || tool.contains("password") || tool.contains("export_storage") {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Blocked,
                decision: BrowserRiskDecision::Block,
                policy_code: "BLOCKED_CREDENTIAL_EXTRACTION".to_string(),
                reason: "Extracting cookies, credentials, or session databases is strictly prohibited.".to_string(),
                user_explanation: "Blocked: credential and cookie extraction is forbidden.".to_string(),
            };
        }

        if tool.contains("upload") {
            return BrowserRiskAssessment {
                risk_level: BrowserRiskLevel::Medium,
                decision: BrowserRiskDecision::RequireApproval,
                policy_code: "FILE_UPLOAD_ACTION".to_string(),
                reason: "Local file upload requires explicit operator authorization to protect local data.".to_string(),
                user_explanation: "Approval required: the page requests to upload a local file.".to_string(),
            };
        }

        // Fallback for unclassified / unknown browser tools
        BrowserRiskAssessment {
            risk_level: BrowserRiskLevel::Blocked,
            decision: BrowserRiskDecision::Block,
            policy_code: "UNKNOWN_TOOL".to_string(),
            reason: format!("Tool '{}' is not registered in the Browser Safety Policy.", tool),
            user_explanation: "Blocked: Unrecognized browser action.".to_string(),
        }
    }

    /// Logs an assessed browser action to the security audit trail.
    /// Strictly filters out passwords, tokens, or sensitive values.
    pub fn record_audit_log(
        task_id: Option<String>,
        tool_name: String,
        tab_id: String,
        assessment: &BrowserRiskAssessment,
    ) {
        let now = chrono::Utc::now().timestamp() as u64;
        let entry = BrowserRiskAuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now,
            task_id,
            tool_name,
            tab_id,
            risk_level: assessment.risk_level,
            decision: assessment.decision,
            policy_code: assessment.policy_code.clone(),
            reason: assessment.reason.clone(),
        };

        if let Ok(mut logs) = BROWSER_RISK_AUDIT_LOG.lock() {
            if logs.len() >= 200 {
                logs.remove(0);
            }
            logs.push(entry);
        }
    }

    /// Retrieves recent risk audit log entries.
    pub fn get_audit_logs() -> Vec<BrowserRiskAuditEntry> {
        BROWSER_RISK_AUDIT_LOG.lock().map(|logs| logs.clone()).unwrap_or_default()
    }

    /// Creates a pending approval record for human operator authorization.
    pub fn create_pending_approval(
        task_id: Option<String>,
        ctx: BrowserActionContext,
        assessment: BrowserRiskAssessment,
    ) -> String {
        let approval_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp() as u64;
        let record = PendingBrowserActionApproval {
            approval_id: approval_id.clone(),
            task_id,
            context: ctx,
            assessment,
            created_at: now,
            status: "pending".to_string(),
        };

        if let Ok(mut store) = PENDING_APPROVALS.lock() {
            store.insert(approval_id.clone(), record);
        }
        approval_id
    }

    /// Resolves a pending approval (Approved / Rejected).
    pub fn resolve_approval(approval_id: &str, decision: &str) -> Result<PendingBrowserActionApproval, String> {
        let mut store = PENDING_APPROVALS.lock().map_err(|e| e.to_string())?;
        if let Some(record) = store.get_mut(approval_id) {
            if record.status != "pending" {
                return Err(format!("Approval '{}' is already resolved as '{}'.", approval_id, record.status));
            }
            record.status = decision.to_lowercase();
            Ok(record.clone())
        } else {
            Err(format!("Pending approval '{}' not found.", approval_id))
        }
    }
}

// ============================================================================
// TAURI COMMANDS FOR RISK ENGINE & HITL
// ============================================================================

#[tauri::command]
pub fn browser_assess_action_risk(context: BrowserActionContext) -> Result<BrowserRiskAssessment, String> {
    let assessment = BrowserRiskEngine::assess_risk(&context);
    BrowserRiskEngine::record_audit_log(None, context.tool_name.clone(), context.tab_id.clone(), &assessment);
    Ok(assessment)
}

#[tauri::command]
pub fn browser_get_risk_audit_log() -> Result<Vec<BrowserRiskAuditEntry>, String> {
    Ok(BrowserRiskEngine::get_audit_logs())
}

#[tauri::command]
pub fn browser_resolve_action_approval(approval_id: String, decision: String) -> Result<PendingBrowserActionApproval, String> {
    BrowserRiskEngine::resolve_approval(&approval_id, &decision)
}
