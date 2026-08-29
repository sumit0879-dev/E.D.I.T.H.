use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use rusqlite::Connection;
use tauri::State;
use tauri::Url;
use crate::db::DbState;

// ============================================================================
// Phase 5.6E: Browser Content Blocking & Web Request Privacy Policy Engine
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    Block { reason: String, category: String },
    Modify { headers: Vec<(String, String)> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyStatus {
    pub enabled: bool,
    pub block_ads: bool,
    pub block_trackers: bool,
    pub send_dnt: bool,
    pub send_gpc: bool,
    pub total_rules_loaded: usize,
    pub allowlisted_domains: Vec<String>,
    pub tab_stats: Option<TabPrivacyStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabPrivacyStats {
    pub tab_id: String,
    pub blocked_ads: u64,
    pub blocked_trackers: u64,
    pub blocked_total: u64,
    pub current_origin: String,
    pub is_site_allowlisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRule {
    pub id: String,
    pub pattern: String,
    pub rule_type: String,
    pub action: String,
    pub category: String,
    pub profile_id: String,
    pub enabled: bool,
    pub created_at: u64,
}

pub struct BrowserContentPolicyEngine {
    builtin_ad_domains: HashSet<&'static str>,
    builtin_tracker_domains: HashSet<&'static str>,
    builtin_path_patterns: Vec<&'static str>,
    tab_stats: Mutex<HashMap<String, TabPrivacyStats>>,
}

impl BrowserContentPolicyEngine {
    pub fn new() -> Self {
        let mut ad_domains = HashSet::new();
        ad_domains.insert("doubleclick.net");
        ad_domains.insert("googlesyndication.com");
        ad_domains.insert("adservice.google.com");
        ad_domains.insert("adnxs.com");
        ad_domains.insert("criteo.com");
        ad_domains.insert("amazon-adsystem.com");
        ad_domains.insert("taboola.com");
        ad_domains.insert("outbrain.com");
        ad_domains.insert("scorecardresearch.com");
        ad_domains.insert("zedo.com");
        ad_domains.insert("popads.net");
        ad_domains.insert("adcolony.com");
        ad_domains.insert("unityads.unity3d.com");
        ad_domains.insert("adbrite.com");
        ad_domains.insert("adskeeper.co.uk");
        ad_domains.insert("adsymptotic.com");
        ad_domains.insert("adroll.com");
        ad_domains.insert("rubiconproject.com");
        ad_domains.insert("casalemedia.com");
        ad_domains.insert("pubmatic.com");
        ad_domains.insert("openx.net");
        ad_domains.insert("applovin.com");
        ad_domains.insert("advertising.com");
        ad_domains.insert("adtechus.com");
        ad_domains.insert("yieldmo.com");

        let mut tracker_domains = HashSet::new();
        tracker_domains.insert("google-analytics.com");
        tracker_domains.insert("analytics.google.com");
        tracker_domains.insert("hotjar.com");
        tracker_domains.insert("segment.com");
        tracker_domains.insert("mixpanel.com");
        tracker_domains.insert("clarity.ms");
        tracker_domains.insert("quantserve.com");
        tracker_domains.insert("mc.yandex.ru");
        tracker_domains.insert("mouseflow.com");
        tracker_domains.insert("crazyegg.com");
        tracker_domains.insert("statcounter.com");
        tracker_domains.insert("fullstory.com");
        tracker_domains.insert("amplitude.com");
        tracker_domains.insert("chartbeat.com");
        tracker_domains.insert("optimizely.com");
        tracker_domains.insert("kissmetrics.com");
        tracker_domains.insert("branch.io");
        tracker_domains.insert("newrelic.com");
        tracker_domains.insert("nr-data.net");
        tracker_domains.insert("connect.facebook.net");
        tracker_domains.insert("analytics.tiktok.com");
        tracker_domains.insert("static.ads-twitter.com");

        let path_patterns = vec![
            "/pagead/",
            "/pagead/js/",
            "/pagead/gen_204",
            "/gtag/js",
            "/analytics.js",
            "/fbevents.js",
            "/beacon.js",
            "/telemetry/",
            "/track.js",
            "/pixel.gif",
            "/conversion.js",
        ];

        Self {
            builtin_ad_domains: ad_domains,
            builtin_tracker_domains: tracker_domains,
            builtin_path_patterns: path_patterns,
            tab_stats: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_domain_in_set(&self, domain: &str, set: &HashSet<&'static str>) -> bool {
        let clean = domain.trim().to_lowercase();
        if set.contains(clean.as_str()) {
            return true;
        }
        for item in set {
            if clean.ends_with(&format!(".{}", item)) {
                return true;
            }
        }
        false
    }

    pub fn evaluate_request(
        &self,
        target_url: &str,
        origin_url: Option<&str>,
        tab_id: Option<&str>,
        profile_id: Option<&str>,
        conn: &Connection,
    ) -> PolicyDecision {
        let pid = profile_id.unwrap_or("global");

        // 1. Check if privacy protection is enabled
        let settings = match crate::db::get_browser_privacy_settings(conn, pid) {
            Ok(s) => s,
            Err(_) => return PolicyDecision::Allow,
        };

        if !settings.enabled {
            return PolicyDecision::Allow;
        }

        let parsed_target = match Url::parse(target_url) {
            Ok(u) => u,
            Err(_) => return PolicyDecision::Allow,
        };

        let target_host = match parsed_target.host_str() {
            Some(h) => h.to_lowercase(),
            None => return PolicyDecision::Allow,
        };

        // 2. Check site allowlist (per-site exception: Step 7)
        if let Some(orig) = origin_url {
            if let Ok(orig_url) = Url::parse(orig) {
                if let Some(orig_host) = orig_url.host_str() {
                    let orig_clean = orig_host.to_lowercase();
                    if let Ok(allowlist) = crate::db::list_browser_privacy_allowlist(conn, Some(pid)) {
                        for item in allowlist {
                            let al_domain = item.domain.trim().to_lowercase();
                            if orig_clean == al_domain || orig_clean.ends_with(&format!(".{}", al_domain)) {
                                if let Some(tid) = tab_id {
                                    self.set_tab_allowlisted(tid, orig_clean.clone(), true);
                                }
                                return PolicyDecision::Allow;
                            }
                        }
                    }
                }
            }
        }

        if let Some(tid) = tab_id {
            let current_orig = origin_url.unwrap_or("").to_string();
            self.set_tab_allowlisted(tid, current_orig, false);
        }

        // 3. Check custom user rules from database (Step 8)
        if let Ok(rules) = crate::db::list_browser_privacy_rules(conn, Some(pid)) {
            for rule in rules {
                if !rule.enabled {
                    continue;
                }
                let pattern = rule.pattern.to_lowercase();
                let matches = match rule.rule_type.as_str() {
                    "DOMAIN" => target_host == pattern || target_host.ends_with(&format!(".{}", pattern)),
                    "WILDCARD" => target_url.to_lowercase().contains(&pattern),
                    "KEYWORD" => target_url.to_lowercase().contains(&pattern),
                    _ => target_host.contains(&pattern),
                };

                if matches {
                    if rule.action == "ALLOW" {
                        return PolicyDecision::Allow;
                    } else {
                        if let Some(tid) = tab_id {
                            self.record_blocked_request(tid, "CUSTOM");
                        }
                        return PolicyDecision::Block {
                            reason: format!("Blocked by user custom rule: '{}'", rule.pattern),
                            category: rule.category,
                        };
                    }
                }
            }
        }

        // 4. Check Built-in Ad blocking
        if settings.block_ads && self.is_domain_in_set(&target_host, &self.builtin_ad_domains) {
            if let Some(tid) = tab_id {
                self.record_blocked_request(tid, "AD");
            }
            return PolicyDecision::Block {
                reason: format!("Blocked advertising domain: '{}'", target_host),
                category: "AD".to_string(),
            };
        }

        // 5. Check Built-in Tracker & Analytics blocking
        if settings.block_trackers && self.is_domain_in_set(&target_host, &self.builtin_tracker_domains) {
            if let Some(tid) = tab_id {
                self.record_blocked_request(tid, "TRACKER");
            }
            return PolicyDecision::Block {
                reason: format!("Blocked tracking domain: '{}'", target_host),
                category: "TRACKER".to_string(),
            };
        }

        // 6. Check path patterns
        let path = parsed_target.path().to_lowercase();
        for pat in &self.builtin_path_patterns {
            if path.contains(pat) {
                if let Some(tid) = tab_id {
                    self.record_blocked_request(tid, "TRACKER");
                }
                return PolicyDecision::Block {
                    reason: format!("Blocked known tracker script path: '{}'", pat),
                    category: "TRACKER".to_string(),
                };
            }
        }

        PolicyDecision::Allow
    }

    pub fn record_blocked_request(&self, tab_id: &str, category: &str) {
        let mut stats = self.tab_stats.lock().unwrap();
        let entry = stats.entry(tab_id.to_string()).or_insert_with(|| TabPrivacyStats {
            tab_id: tab_id.to_string(),
            blocked_ads: 0,
            blocked_trackers: 0,
            blocked_total: 0,
            current_origin: String::new(),
            is_site_allowlisted: false,
        });

        entry.blocked_total += 1;
        if category == "AD" {
            entry.blocked_ads += 1;
        } else {
            entry.blocked_trackers += 1;
        }
    }

    pub fn set_tab_allowlisted(&self, tab_id: &str, origin: String, is_allowlisted: bool) {
        let mut stats = self.tab_stats.lock().unwrap();
        let entry = stats.entry(tab_id.to_string()).or_insert_with(|| TabPrivacyStats {
            tab_id: tab_id.to_string(),
            blocked_ads: 0,
            blocked_trackers: 0,
            blocked_total: 0,
            current_origin: origin.clone(),
            is_site_allowlisted: is_allowlisted,
        });
        entry.current_origin = origin;
        entry.is_site_allowlisted = is_allowlisted;
    }

    pub fn get_tab_stats(&self, tab_id: &str) -> TabPrivacyStats {
        let stats = self.tab_stats.lock().unwrap();
        stats.get(tab_id).cloned().unwrap_or_else(|| TabPrivacyStats {
            tab_id: tab_id.to_string(),
            blocked_ads: 0,
            blocked_trackers: 0,
            blocked_total: 0,
            current_origin: String::new(),
            is_site_allowlisted: false,
        })
    }

    pub fn reset_tab_stats(&self, tab_id: &str) {
        let mut stats = self.tab_stats.lock().unwrap();
        stats.remove(tab_id);
    }
}

lazy_static! {
    pub static ref GLOBAL_POLICY_ENGINE: Arc<BrowserContentPolicyEngine> = Arc::new(BrowserContentPolicyEngine::new());
}

// Pre-flight Client Interception & Privacy Initialization Script (Step 10)
pub const PRIVACY_PREFLIGHT_INIT_SCRIPT: &str = r#"
(function() {
  if (window.__EDITH_PRIVACY_INITIALIZED__) return;
  window.__EDITH_PRIVACY_INITIALIZED__ = true;

  // Step 10 Privacy Signals: Do-Not-Track & Global Privacy Control
  try {
    Object.defineProperty(navigator, 'doNotTrack', { value: '1', configurable: false, writable: false });
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: true, configurable: false, writable: false });
  } catch(e) {}

  // List of high-priority third-party ad/tracker patterns to abort immediately on client pre-flight
  var BLOCKED_PATTERNS = [
    'google-analytics.com', 'analytics.google.com', 'hotjar.com', 'doubleclick.net',
    'googlesyndication.com', 'adnxs.com', 'criteo.com', 'amazon-adsystem.com',
    'taboola.com', 'outbrain.com', 'scorecardresearch.com', 'clarity.ms',
    'segment.com', 'mixpanel.com', 'fbevents.js', 'facebook.net/en_US/fbevents.js'
  ];

  function shouldBlockUrl(urlStr) {
    if (!urlStr || typeof urlStr !== 'string') return false;
    var low = urlStr.toLowerCase();
    for (var i = 0; i < BLOCKED_PATTERNS.length; i++) {
      if (low.indexOf(BLOCKED_PATTERNS[i]) !== -1) {
        return true;
      }
    }
    return false;
  }

  // Pre-flight Interceptor: window.fetch
  if (window.fetch) {
    var _origFetch = window.fetch;
    window.fetch = function(input, init) {
      var url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
      if (shouldBlockUrl(url)) {
        return Promise.reject(new Error('E.D.I.T.H. Content Blocker: Request aborted by Host Privacy Engine (' + url + ')'));
      }
      return _origFetch.apply(this, arguments);
    };
  }

  // Pre-flight Interceptor: XMLHttpRequest
  if (window.XMLHttpRequest) {
    var _origOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url) {
      if (shouldBlockUrl(url)) {
        this.__edith_blocked__ = true;
      }
      return _origOpen.apply(this, arguments);
    };

    var _origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function() {
      if (this.__edith_blocked__) {
        try { this.abort(); } catch(e) {}
        return;
      }
      return _origSend.apply(this, arguments);
    };
  }
})();
"#;

// ============================================================================
// Tauri IPC Commands for Privacy Engine
// ============================================================================

#[tauri::command]
pub async fn browser_privacy_get_status(
    tab_id: Option<String>,
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<PrivacyStatus, String> {
    let pid = profile_id.unwrap_or_else(|| "global".to_string());
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let settings = crate::db::get_browser_privacy_settings(&conn, &pid)
        .map_err(|e| e.to_string())?;

    let allowlist = crate::db::list_browser_privacy_allowlist(&conn, Some(&pid))
        .map_err(|e| e.to_string())?;

    let rules = crate::db::list_browser_privacy_rules(&conn, Some(&pid))
        .map_err(|e| e.to_string())?;

    let tab_stats = tab_id.as_ref().map(|tid| GLOBAL_POLICY_ENGINE.get_tab_stats(tid));

    Ok(PrivacyStatus {
        enabled: settings.enabled,
        block_ads: settings.block_ads,
        block_trackers: settings.block_trackers,
        send_dnt: settings.send_dnt,
        send_gpc: settings.send_gpc,
        total_rules_loaded: rules.len() + GLOBAL_POLICY_ENGINE.builtin_ad_domains.len() + GLOBAL_POLICY_ENGINE.builtin_tracker_domains.len(),
        allowlisted_domains: allowlist.into_iter().map(|a| a.domain).collect(),
        tab_stats,
    })
}

#[tauri::command]
pub async fn browser_privacy_toggle_protection(
    enabled: bool,
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let pid = profile_id.unwrap_or_else(|| "global".to_string());
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let mut settings = crate::db::get_browser_privacy_settings(&conn, &pid)
        .unwrap_or_else(|_| crate::db::BrowserPrivacySettingsRecord {
            profile_id: pid.clone(),
            enabled: true,
            block_ads: true,
            block_trackers: true,
            send_dnt: true,
            send_gpc: true,
            created_at: chrono::Utc::now().timestamp_millis() as u64,
            updated_at: chrono::Utc::now().timestamp_millis() as u64,
        });

    settings.enabled = enabled;
    settings.updated_at = chrono::Utc::now().timestamp_millis() as u64;

    crate::db::upsert_browser_privacy_settings(&conn, &settings)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn browser_privacy_allowlist_domain(
    domain: String,
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let pid = profile_id.unwrap_or_else(|| "global".to_string());
    let clean = domain.trim().to_lowercase();
    if clean.is_empty() {
        return Err("Domain cannot be empty".to_string());
    }

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::add_browser_privacy_allowlist(&conn, &clean, &pid)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn browser_privacy_remove_allowlist(
    domain: String,
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let pid = profile_id.as_deref();
    let clean = domain.trim().to_lowercase();

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::remove_browser_privacy_allowlist(&conn, &clean, pid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_privacy_add_block_rule(
    pattern: String,
    rule_type: Option<String>,
    category: Option<String>,
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<String, String> {
    let pid = profile_id.unwrap_or_else(|| "global".to_string());
    let clean_pattern = pattern.trim().to_lowercase();
    if clean_pattern.is_empty() {
        return Err("Pattern cannot be empty".to_string());
    }

    let id = format!("rule_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp_millis() as u64;

    let rule = crate::db::BrowserPrivacyRuleRecord {
        id: id.clone(),
        pattern: clean_pattern,
        rule_type: rule_type.unwrap_or_else(|| "DOMAIN".to_string()),
        action: "BLOCK".to_string(),
        category: category.unwrap_or_else(|| "CUSTOM".to_string()),
        profile_id: pid,
        enabled: true,
        created_at: now,
    };

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::add_browser_privacy_rule(&conn, &rule)
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn browser_privacy_remove_block_rule(
    rule_id: String,
    db_state: State<'_, DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::delete_browser_privacy_rule(&conn, &rule_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_privacy_list_rules(
    profile_id: Option<String>,
    db_state: State<'_, DbState>,
) -> Result<Vec<PrivacyRule>, String> {
    let pid = profile_id.as_deref();
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let rules = crate::db::list_browser_privacy_rules(&conn, pid)
        .map_err(|e| e.to_string())?;

    Ok(rules.into_iter().map(|r| PrivacyRule {
        id: r.id,
        pattern: r.pattern,
        rule_type: r.rule_type,
        action: r.action,
        category: r.category,
        profile_id: r.profile_id,
        enabled: r.enabled,
        created_at: r.created_at,
    }).collect())
}

#[tauri::command]
pub async fn browser_privacy_get_tab_stats(tab_id: String) -> Result<TabPrivacyStats, String> {
    Ok(GLOBAL_POLICY_ENGINE.get_tab_stats(&tab_id))
}

#[tauri::command]
pub async fn browser_privacy_reset_stats(tab_id: String) -> Result<bool, String> {
    GLOBAL_POLICY_ENGINE.reset_tab_stats(&tab_id);
    Ok(true)
}
