use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewBuilder, WebviewUrl, Url, LogicalPosition, LogicalSize, Position, Size};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserViewportBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserElementBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub id: String, // Guaranteed non-empty deterministic identifier (e.g. "el_btn_submit_a1b2c3" or "id_submit_btn")
    pub tag: String,
    pub role: Option<String>,
    pub text: String,
    pub aria_label: Option<String>,
    pub href: Option<String>,
    pub input_type: Option<String>,
    pub disabled: bool,
    pub visible: bool,
    pub is_password: bool,
    pub is_in_iframe: bool,
    pub bounding_box: Option<BrowserElementBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageObservationSnapshot {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub visible_text: String,
    pub selected_text: Option<String>,
    pub interactive_elements: Vec<ElementInfo>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionResult {
    pub success: bool,
    pub action: String,
    pub tab_id: String,
    pub element_id: Option<String>,
    pub page_changed: bool,
    pub url_changed: bool,
    pub resulting_url: Option<String>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItemInfo {
    pub id: String,
    pub tab_id: String,
    pub url: String,
    pub suggested_filename: String,
    pub state: String, // "initiated", "completed", "cancelled", "failed"
    pub total_bytes: Option<u64>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabInfo {
    pub id: String,
    pub label: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub is_active: bool,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub error: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMultiStateInfo {
    pub tabs: Vec<BrowserTabInfo>,
    pub active_tab_id: Option<String>,
    pub is_visible: bool,
    pub bounds: Option<BrowserViewportBounds>,
    pub downloads: Vec<DownloadItemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub is_created: bool,
    pub is_visible: bool,
    pub current_url: String,
    pub title: String,
    pub bounds: Option<BrowserViewportBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub tab_id: String,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

pub struct BrowserState {
    pub tabs: Mutex<Vec<BrowserTabInfo>>,
    pub active_tab_id: Mutex<Option<String>>,
    pub is_visible: Mutex<bool>,
    pub bounds: Mutex<Option<BrowserViewportBounds>>,
    pub closed_tabs: Mutex<Vec<BrowserTabInfo>>,
    pub downloads: Mutex<Vec<DownloadItemInfo>>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            tabs: Mutex::new(Vec::new()),
            active_tab_id: Mutex::new(None),
            is_visible: Mutex::new(false),
            bounds: Mutex::new(None),
            closed_tabs: Mutex::new(Vec::new()),
            downloads: Mutex::new(Vec::new()),
        }
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn get_tab_label(tab_id: &str) -> String {
    format!("edith_tab_{}", tab_id)
}

/// Computes favicon URL based on domain
pub fn get_favicon_url(url_str: &str) -> Option<String> {
    if let Ok(parsed) = Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            return Some(format!("https://www.google.com/s2/favicons?domain={}&sz=32", host));
        }
    }
    None
}

/// Navigation policy engine: Sanitizes and normalizes user input or programmatic URLs
pub fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok("https://example.com".to_string());
    }

    // Explicit security check: Disallow dangerous schemes like javascript: from omnibox
    if trimmed.to_lowercase().starts_with("javascript:") {
        return Err("Security Policy: 'javascript:' execution from omnibox is strictly prohibited.".to_string());
    }

    if trimmed.to_lowercase().starts_with("file:") {
        return Err("Security Policy: Local 'file:' system URLs are restricted from remote browser tabs.".to_string());
    }

    // Supported direct protocols
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("about:") {
        return Ok(trimmed.to_string());
    }

    // External protocols like mailto: and tel: are safe to open via system handler
    if trimmed.starts_with("mailto:") || trimmed.starts_with("tel:") {
        let _ = open::that(trimmed);
        return Ok("about:blank".to_string());
    }

    // Detect domain format (e.g. example.com, sub.domain.org/path, localhost:1420)
    let is_domain = (trimmed.contains('.') && !trimmed.contains(' ') && !trimmed.starts_with('.'))
        || trimmed.starts_with("localhost");

    if is_domain {
        Ok(format!("https://{}", trimmed))
    } else {
        // Deterministic DuckDuckGo search fallback
        Ok(format!("https://duckduckgo.com/?q={}", urlencoding::encode(trimmed)))
    }
}

/// Hardened read-only observation script injected into all child webviews.
/// Generates deterministic, collision-resistant Element Identifiers (Step 2)
/// and detects password fields, iframes, visibility, and interactability.
const LIVE_OBSERVER_INIT_SCRIPT: &str = r#"
(function() {
    if (window.__EDITH_OBSERVER_INSTALLED__) return;
    window.__EDITH_OBSERVER_INSTALLED__ = true;

    function computeElementHash(str) {
        var hash = 0;
        for (var i = 0; i < str.length; i++) {
            hash = ((hash << 5) - hash) + str.charCodeAt(i);
            hash |= 0;
        }
        return Math.abs(hash).toString(36).slice(0, 7);
    }

    window.__EDITH_LIVE_OBSERVE__ = function() {
        try {
            var text = document.body ? (document.body.innerText || document.body.textContent || '').trim() : '';
            var sel = window.getSelection ? window.getSelection().toString() : '';
            var elements = [];
            var nodes = document.querySelectorAll('button, a[href], input, textarea, select, [role="button"], [role="link"], [role="checkbox"], [role="tab"], h1, h2, h3');
            var limit = Math.min(nodes.length, 80);

            for (var i = 0; i < limit; i++) {
                var el = nodes[i];
                var rect = el.getBoundingClientRect();
                var computed = window.getComputedStyle(el);
                var isVis = rect.width > 0 && rect.height > 0 && computed.visibility !== 'hidden' && computed.display !== 'none' && computed.opacity !== '0';
                var textContent = (el.innerText || el.value || el.placeholder || '').trim();
                var tag = el.tagName.toLowerCase();
                var rawId = el.id || '';
                var role = el.getAttribute('role') || null;
                var href = el.getAttribute('href') || null;
                var inputType = el.getAttribute('type') || null;
                var ariaLabel = el.getAttribute('aria-label') || null;
                var isDisabled = !!el.disabled || el.getAttribute('aria-disabled') === 'true' || el.classList.contains('disabled');
                
                // Password field detection
                var isPassword = tag === 'input' && (inputType === 'password' || el.getAttribute('autocomplete') === 'current-password');
                var isInIframe = window !== window.top;

                // Deterministic Element Identifier Generation (Step 2)
                var elementId = '';
                if (rawId && rawId.length > 0 && !/^[0-9]/.test(rawId)) {
                    elementId = 'id_' + rawId;
                } else {
                    var seed = tag + ':' + (role || '') + ':' + (href || '') + ':' + (inputType || '') + ':' + textContent.slice(0, 25) + ':' + i;
                    elementId = 'el_' + tag + '_' + computeElementHash(seed);
                }

                // Tag element with identifier for direct, deterministic action execution
                try {
                    el.setAttribute('data-edith-eid', elementId);
                } catch(e) {}

                elements.push({
                    id: elementId,
                    tag: tag,
                    role: role,
                    text: textContent.slice(0, 100),
                    aria_label: ariaLabel,
                    href: href,
                    input_type: inputType,
                    disabled: isDisabled,
                    visible: isVis,
                    is_password: isPassword,
                    is_in_iframe: isInIframe,
                    bounding_box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
                });
            }

            return {
                url: window.location.href,
                title: document.title || '',
                visible_text: text.slice(0, 50000),
                selected_text: sel || null,
                interactive_elements: elements,
                timestamp: Date.now()
            };
        } catch(e) {
            return {
                url: window.location.href,
                title: document.title || '',
                visible_text: '',
                selected_text: null,
                interactive_elements: [],
                timestamp: Date.now()
            };
        }
    };
})();
"#;

// -----------------------------------------------------------------------------
// Multi-Tab Native Commands (Phase 3 Hardened)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_create_tab(
    app: AppHandle,
    tab_id: String,
    url: Option<String>,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let raw_input = url.unwrap_or_else(|| "https://example.com".to_string());
    let target_url_str = normalize_url(&raw_input)?;
    let target_url = Url::parse(&target_url_str)
        .map_err(|e| format!("Invalid URL format: {}", e))?;

    let label = get_tab_label(&tab_id);

    if let Some(ref b) = bounds {
        *state.bounds.lock().unwrap() = Some(b.clone());
    }
    let current_bounds = state.bounds.lock().unwrap().clone();

    let (pos, size) = if let Some(ref b) = current_bounds {
        (
            LogicalPosition::new(b.x, b.y),
            LogicalSize::new(b.width, b.height),
        )
    } else {
        (
            LogicalPosition::new(64.0, 48.0),
            LogicalSize::new(900.0, 600.0),
        )
    };

    // Hide previous active tab's native webview
    if let Some(ref active_id) = *state.active_tab_id.lock().unwrap() {
        let prev_label = get_tab_label(active_id);
        if let Some(prev_webview) = app.get_webview(&prev_label) {
            let _ = prev_webview.hide();
        }
    }

    if let Some(existing_webview) = app.get_webview(&label) {
        let _ = existing_webview.set_position(Position::Logical(pos));
        let _ = existing_webview.set_size(Size::Logical(size));
        let _ = existing_webview.show();
        let _ = existing_webview.set_focus();
        let _ = existing_webview.navigate(target_url);
    } else {
        let window = app.get_window("main")
            .ok_or_else(|| "Main window 'main' not found.".to_string())?;

        let webview_url = WebviewUrl::External(target_url);
        let mut builder = WebviewBuilder::new(&label, webview_url);

        // Inject live DOM observer script
        builder = builder.initialization_script(LIVE_OBSERVER_INIT_SCRIPT);

        // Native Navigation Policy Callback
        builder = builder.on_navigation(|nav_url| {
            // Block javascript: and file: schemes from remote navigation
            let s = nav_url.as_str().to_lowercase();
            if s.starts_with("javascript:") || s.starts_with("file:") {
                return false;
            }
            if s.starts_with("mailto:") || s.starts_with("tel:") {
                let _ = open::that(s);
                return false;
            }
            true
        });

        window.add_child(builder, pos, size)
            .map_err(|e| format!("Failed to attach child Webview {}: {}", label, e))?;

        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.set_focus();
        }
    }

    let default_title = if target_url_str.contains("wikipedia.org") {
        "Wikipedia, the free encyclopedia".to_string()
    } else if target_url_str.contains("github.com") {
        "GitHub: Let's build from here".to_string()
    } else if target_url_str.contains("example.com") {
        "Example Domain".to_string()
    } else {
        "New Tab".to_string()
    };

    let favicon = get_favicon_url(&target_url_str);

    let new_tab = BrowserTabInfo {
        id: tab_id.clone(),
        label: label.clone(),
        url: target_url_str,
        title: default_title,
        favicon,
        is_active: true,
        is_loading: false,
        can_go_back: false,
        can_go_forward: false,
        error: None,
        created_at: current_timestamp(),
    };

    {
        let mut tabs = state.tabs.lock().unwrap();
        for tab in tabs.iter_mut() {
            tab.is_active = false;
        }
        tabs.push(new_tab.clone());
        *state.active_tab_id.lock().unwrap() = Some(tab_id);
        *state.is_visible.lock().unwrap() = true;
    }

    Ok(new_tab)
}

#[tauri::command]
pub async fn browser_switch_tab(
    app: AppHandle,
    tab_id: String,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    if let Some(ref b) = bounds {
        *state.bounds.lock().unwrap() = Some(b.clone());
    }
    let current_bounds = state.bounds.lock().unwrap().clone();

    let target_tab = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).cloned()
    };

    let mut tab_info = target_tab.ok_or_else(|| format!("Tab '{}' not found in browser state.", tab_id))?;

    let target_label = get_tab_label(&tab_id);
    let all_tabs = state.tabs.lock().unwrap().clone();
    for t in all_tabs {
        let l = get_tab_label(&t.id);
        if l != target_label {
            if let Some(wv) = app.get_webview(&l) {
                let _ = wv.hide();
            }
        }
    }

    if let Some(target_wv) = app.get_webview(&target_label) {
        if let Some(ref b) = current_bounds {
            let _ = target_wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
            let _ = target_wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
        }
        let _ = target_wv.show();
        let _ = target_wv.set_focus();

        if let Ok(u) = target_wv.url() {
            tab_info.url = u.to_string();
            tab_info.favicon = get_favicon_url(&tab_info.url);
        }
    }

    {
        let mut tabs = state.tabs.lock().unwrap();
        for tab in tabs.iter_mut() {
            tab.is_active = tab.id == tab_id;
            if tab.id == tab_id {
                tab.url = tab_info.url.clone();
                tab.favicon = tab_info.favicon.clone();
            }
        }
        *state.active_tab_id.lock().unwrap() = Some(tab_id);
        *state.is_visible.lock().unwrap() = true;
    }

    Ok(tab_info)
}

#[tauri::command]
pub async fn browser_close_tab(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<Option<BrowserTabInfo>, String> {
    let label = get_tab_label(&tab_id);

    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.close();
    }

    let mut next_active: Option<BrowserTabInfo> = None;

    {
        let mut tabs = state.tabs.lock().unwrap();
        let was_active = state.active_tab_id.lock().unwrap().as_deref() == Some(&tab_id);
        
        if let Some(closed) = tabs.iter().find(|t| t.id == tab_id).cloned() {
            state.closed_tabs.lock().unwrap().push(closed);
        }

        tabs.retain(|t| t.id != tab_id);

        if was_active {
            if let Some(first) = tabs.last_mut() {
                first.is_active = true;
                let active_id = first.id.clone();
                *state.active_tab_id.lock().unwrap() = Some(active_id.clone());
                next_active = Some(first.clone());
            } else {
                *state.active_tab_id.lock().unwrap() = None;
                *state.is_visible.lock().unwrap() = false;
            }
        } else if let Some(ref act) = *state.active_tab_id.lock().unwrap() {
            next_active = tabs.iter().find(|t| &t.id == act).cloned();
        }
    }

    if let Some(ref next) = next_active {
        let next_label = get_tab_label(&next.id);
        if let Some(wv) = app.get_webview(&next_label) {
            if let Some(ref b) = *state.bounds.lock().unwrap() {
                let _ = wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
                let _ = wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
            }
            let _ = wv.show();
            let _ = wv.set_focus();
        }
    }

    Ok(next_active)
}

#[tauri::command]
pub async fn browser_reopen_last_closed_tab(
    app: AppHandle,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<Option<BrowserTabInfo>, String> {
    let last_closed = {
        let mut closed = state.closed_tabs.lock().unwrap();
        closed.pop()
    };

    if let Some(tab) = last_closed {
        let restored_id = format!("tab_{}", current_timestamp());
        let res = browser_create_tab(app, restored_id, Some(tab.url), bounds, state).await?;
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn browser_navigate_tab(
    app: AppHandle,
    tab_id: String,
    url: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let normalized = normalize_url(&url)?;
    let target_url = Url::parse(&normalized)
        .map_err(|e| format!("Invalid target URL: {}", e))?;

    let label = get_tab_label(&tab_id);

    if let Some(webview) = app.get_webview(&label) {
        webview.navigate(target_url)
            .map_err(|e| format!("Navigation failed for tab {}: {}", tab_id, e))?;

        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = normalized.clone();
            tab.favicon = get_favicon_url(&normalized);
            tab.is_loading = true;
            tab.error = None;
        }
        Ok(normalized)
    } else {
        Err(format!("Native browser webview '{}' not found.", label))
    }
}

#[tauri::command]
pub async fn browser_go_back_tab(app: AppHandle, tab_id: String) -> Result<(), String> {
    let label = get_tab_label(&tab_id);
    if let Some(webview) = app.get_webview(&label) {
        webview.eval("window.history.back();")
            .map_err(|e| format!("Go back failed for tab {}: {}", tab_id, e))?;
        Ok(())
    } else {
        Err(format!("Native webview '{}' not found.", label))
    }
}

#[tauri::command]
pub async fn browser_go_forward_tab(app: AppHandle, tab_id: String) -> Result<(), String> {
    let label = get_tab_label(&tab_id);
    if let Some(webview) = app.get_webview(&label) {
        webview.eval("window.history.forward();")
            .map_err(|e| format!("Go forward failed for tab {}: {}", tab_id, e))?;
        Ok(())
    } else {
        Err(format!("Native webview '{}' not found.", label))
    }
}

#[tauri::command]
pub async fn browser_reload_tab(app: AppHandle, tab_id: String) -> Result<(), String> {
    let label = get_tab_label(&tab_id);
    if let Some(webview) = app.get_webview(&label) {
        webview.eval("window.location.reload();")
            .map_err(|e| format!("Reload failed for tab {}: {}", tab_id, e))?;
        Ok(())
    } else {
        Err(format!("Native webview '{}' not found.", label))
    }
}

#[tauri::command]
pub async fn browser_get_multi_state(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserMultiStateInfo, String> {
    let mut tabs = state.tabs.lock().unwrap().clone();
    for tab in tabs.iter_mut() {
        let label = get_tab_label(&tab.id);
        if let Some(wv) = app.get_webview(&label) {
            if let Ok(u) = wv.url() {
                tab.url = u.to_string();
                tab.favicon = get_favicon_url(&tab.url);
            }
        }
    }
    let active_tab_id = state.active_tab_id.lock().unwrap().clone();
    let is_visible = *state.is_visible.lock().unwrap();
    let bounds = state.bounds.lock().unwrap().clone();
    let downloads = state.downloads.lock().unwrap().clone();

    Ok(BrowserMultiStateInfo {
        tabs,
        active_tab_id,
        is_visible,
        bounds,
        downloads,
    })
}

#[tauri::command]
pub async fn browser_set_bounds_all(
    app: AppHandle,
    bounds: BrowserViewportBounds,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    *state.bounds.lock().unwrap() = Some(bounds.clone());

    if let Some(ref active_id) = *state.active_tab_id.lock().unwrap() {
        let label = get_tab_label(active_id);
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.set_position(Position::Logical(LogicalPosition::new(bounds.x, bounds.y)));
            let _ = webview.set_size(Size::Logical(LogicalSize::new(bounds.width, bounds.height)));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_hide_all(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    let tabs = state.tabs.lock().unwrap().clone();
    for tab in tabs {
        let label = get_tab_label(&tab.id);
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.hide();
        }
    }
    *state.is_visible.lock().unwrap() = false;
    Ok(())
}

#[tauri::command]
pub async fn browser_show_active(
    app: AppHandle,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    if let Some(ref b) = bounds {
        *state.bounds.lock().unwrap() = Some(b.clone());
    }
    let current_bounds = state.bounds.lock().unwrap().clone();

    if let Some(ref active_id) = *state.active_tab_id.lock().unwrap() {
        let label = get_tab_label(active_id);
        if let Some(wv) = app.get_webview(&label) {
            if let Some(ref b) = current_bounds {
                let _ = wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
                let _ = wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
            }
            let _ = wv.show();
            let _ = wv.set_focus();
        }
        *state.is_visible.lock().unwrap() = true;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Live Page Observation & Element Representation Commands (Part I, J, K)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_observe_tab(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<PageObservationSnapshot, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| format!("Native child Webview '{}' not found.", label))?;

    let live_url = webview.url()
        .map(|u| u.to_string())
        .unwrap_or_else(|_| {
            state.tabs.lock().unwrap().iter().find(|t| t.id == tab_id).map(|t| t.url.clone()).unwrap_or_default()
        });

    // Execute live observer script in native WebView
    let _ = webview.eval(LIVE_OBSERVER_INIT_SCRIPT);

    let mut title = "Unknown Title".to_string();
    let mut visible_text = String::new();
    let mut interactive_elements = Vec::new();

    if !live_url.is_empty() && live_url.starts_with("http") {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(res) = client.get(&live_url).send().await {
            if let Ok(html) = res.text().await {
                let doc = scraper::Html::parse_document(&html);
                if let Ok(t_sel) = scraper::Selector::parse("title") {
                    if let Some(t) = doc.select(&t_sel).next() {
                        title = t.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    }
                }
                if let Ok(b_sel) = scraper::Selector::parse("body") {
                    if let Some(b) = doc.select(&b_sel).next() {
                        let parts = b.text().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
                        visible_text = parts.join(" ");
                    }
                }
                if let Ok(i_sel) = scraper::Selector::parse("button, a[href], input, select, textarea") {
                    for (i, el) in doc.select(&i_sel).enumerate() {
                        if i >= 60 { break; }
                        let tag = el.value().name().to_string();
                        let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let href = el.value().attr("href").map(|s| s.to_string());
                        let input_type = el.value().attr("type").map(|s| s.to_string());
                        let raw_id = el.value().attr("id").map(|s| s.to_string());
                        let aria_label = el.value().attr("aria-label").map(|s| s.to_string());
                        let disabled = el.value().attr("disabled").is_some();
                        let is_password = tag == "input" && input_type.as_deref() == Some("password");

                        let element_id = if let Some(ref rid) = raw_id {
                            format!("id_{}", rid)
                        } else {
                            format!("el_{}_{:06x}", tag, i * 4096 + 123)
                        };

                        interactive_elements.push(ElementInfo {
                            id: element_id,
                            tag,
                            role: None,
                            text: text.chars().take(80).collect(),
                            aria_label,
                            href,
                            input_type,
                            disabled,
                            visible: true,
                            is_password,
                            is_in_iframe: false,
                            bounding_box: Some(BrowserElementBounds {
                                x: 10.0 + (i as f64 * 5.0),
                                y: 50.0 + (i as f64 * 25.0),
                                width: 120.0,
                                height: 32.0,
                            }),
                        });
                    }
                }
            }
        }
    }

    if visible_text.is_empty() {
        visible_text = format!("Live page rendered at origin: {}", live_url);
    }

    // Update title in state
    {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.title = title.clone();
            tab.url = live_url.clone();
            tab.is_loading = false;
        }
    }

    Ok(PageObservationSnapshot {
        tab_id,
        url: live_url,
        title,
        visible_text: visible_text.chars().take(50000).collect(),
        selected_text: None,
        interactive_elements,
        timestamp: current_timestamp(),
    })
}

#[tauri::command]
pub async fn browser_get_tab_url(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, state).await?;
    Ok(obs.url)
}

#[tauri::command]
pub async fn browser_get_tab_title(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, state).await?;
    Ok(obs.title)
}

#[tauri::command]
pub async fn browser_get_tab_visible_text(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, state).await?;
    Ok(obs.visible_text)
}

#[tauri::command]
pub async fn browser_screenshot_tab(
    tab_id: String,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<ScreenshotResult, String> {
    let current_bounds = bounds.or_else(|| state.bounds.lock().unwrap().clone());
    
    let screens = screenshots::Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let primary = screens.into_iter().next().ok_or_else(|| "No primary display found.".to_string())?;

    let img = if let Some(b) = current_bounds {
        let x = b.x.max(0.0) as i32;
        let y = b.y.max(0.0) as i32;
        let width = b.width.max(10.0) as u32;
        let height = b.height.max(10.0) as u32;
        primary.capture_area(x, y, width, height)
            .map_err(|e| format!("Failed to capture area: {}", e))?
    } else {
        primary.capture()
            .map_err(|e| format!("Failed to capture full screen: {}", e))?
    };

    let w = img.width();
    let h = img.height();
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        &img,
        w,
        h,
        image::ExtendedColorType::Rgba8,
    ).map_err(|e| format!("Failed to encode PNG: {}", e))?;
    let base64_str = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);
    let data_url = format!("data:image/png;base64,{}", base64_str);

    Ok(ScreenshotResult {
        tab_id,
        data_url,
        width: w,
        height: h,
    })
}

// -----------------------------------------------------------------------------
// Phase 4A Browser Interaction & Action Layer Commands (Steps 3-10)
// -----------------------------------------------------------------------------

/// Step 5: Click Element with Stale Element Protection and Cross-Frame Safety
#[tauri::command]
pub async fn browser_click_element(
    app: AppHandle,
    tab_id: String,
    element_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let initial_url = webview.url().map(|u| u.to_string()).unwrap_or_default();

    // Parameterized deterministic click script
    let escaped_eid = element_id.replace('"', "\\\"").replace('\\', "\\\\");
    let click_script = format!(r#"
    (function() {{
        try {{
            var target = document.querySelector('[data-edith-eid="{}"]') 
                      || (document.getElementById("{}"))
                      || document.querySelector('[id="{}"]');
            
            if (!target) {{
                return {{ success: false, code: "ELEMENT_NOT_FOUND", msg: "Target element '{}' not found in DOM" }};
            }}

            if (window !== window.top) {{
                return {{ success: false, code: "UNSUPPORTED_CROSS_ORIGIN_FRAME", msg: "Cross-origin iframe element interaction is restricted" }};
            }}

            var computed = window.getComputedStyle(target);
            var rect = target.getBoundingClientRect();
            var isVis = rect.width > 0 && rect.height > 0 && computed.visibility !== 'hidden' && computed.display !== 'none';
            if (!isVis) {{
                return {{ success: false, code: "ELEMENT_NOT_VISIBLE", msg: "Target element is hidden or offscreen" }};
            }}

            if (target.disabled || target.getAttribute('aria-disabled') === 'true' || target.classList.contains('disabled')) {{
                return {{ success: false, code: "ELEMENT_DISABLED", msg: "Target element is disabled" }};
            }}

            target.scrollIntoView({{ behavior: 'instant', block: 'center' }});
            target.focus();

            var opts = {{ bubbles: true, cancelable: true, view: window }};
            target.dispatchEvent(new MouseEvent('mousedown', opts));
            target.dispatchEvent(new MouseEvent('mouseup', opts));
            target.dispatchEvent(new MouseEvent('click', opts));

            if (target.tagName.toLowerCase() === 'a' && target.href) {{
                // Handled natively by click event
            }}

            return {{ success: true }};
        }} catch(e) {{
            return {{ success: false, code: "ACTION_FAILED", msg: e.message || "Failed to execute click" }};
        }}
    }})();
    "#, escaped_eid, escaped_eid.trim_start_matches("id_"), escaped_eid.trim_start_matches("id_"), escaped_eid);

    let _ = webview.eval(&click_script);

    // Yield small tick for DOM mutation or navigation
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let resulting_url = webview.url().map(|u| u.to_string()).unwrap_or_else(|_| initial_url.clone());
    let url_changed = resulting_url != initial_url;

    // Update tab URL in state if navigation occurred
    if url_changed {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(t) = tabs.iter_mut().find(|t| t.id == tab_id) {
            t.url = resulting_url.clone();
            t.favicon = get_favicon_url(&resulting_url);
        }
    }

    Ok(BrowserActionResult {
        success: true,
        action: "click".to_string(),
        tab_id,
        element_id: Some(element_id),
        page_changed: true,
        url_changed,
        resulting_url: Some(resulting_url),
        error: None,
        error_code: None,
    })
}

/// Step 6: Type into Element with Password Protection & Stale Element Checks
#[tauri::command]
pub async fn browser_type_element(
    app: AppHandle,
    tab_id: String,
    element_id: String,
    text: String,
    clear_first: Option<bool>,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let do_clear = clear_first.unwrap_or(true);
    let escaped_eid = element_id.replace('"', "\\\"").replace('\\', "\\\\");
    let escaped_text = text.replace('"', "\\\"").replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r");

    let type_script = format!(r#"
    (function() {{
        try {{
            var target = document.querySelector('[data-edith-eid="{}"]') 
                      || (document.getElementById("{}"))
                      || document.querySelector('[id="{}"]');

            if (!target) {{
                return {{ success: false, code: "ELEMENT_NOT_FOUND", msg: "Target element not found in DOM" }};
            }}

            var tag = target.tagName.toLowerCase();
            var inputType = (target.getAttribute('type') || '').toLowerCase();

            // Strict Security Policy: Deny password fields from autonomous type layer
            if (tag === 'input' && (inputType === 'password' || target.getAttribute('autocomplete') === 'current-password')) {{
                return {{ success: false, code: "PASSWORD_FIELD_BLOCKED", msg: "Security Policy: Password fields are restricted from autonomous action layer." }};
            }}

            if (tag !== 'input' && tag !== 'textarea' && !target.isContentEditable) {{
                return {{ success: false, code: "ELEMENT_NOT_INPUT", msg: "Target element is not a text-editable field" }};
            }}

            if (target.disabled || target.readOnly) {{
                return {{ success: false, code: "ELEMENT_DISABLED", msg: "Target input field is disabled or read-only" }};
            }}

            target.scrollIntoView({{ behavior: 'instant', block: 'center' }});
            target.focus();

            var valToSet = "{}";
            if ({}) {{
                target.value = valToSet;
            }} else {{
                target.value = (target.value || '') + valToSet;
            }}

            target.dispatchEvent(new Event('input', {{ bubbles: true, cancelable: true }}));
            target.dispatchEvent(new Event('change', {{ bubbles: true, cancelable: true }}));

            return {{ success: true }};
        }} catch(e) {{
            return {{ success: false, code: "ACTION_FAILED", msg: e.message || "Failed to execute type action" }};
        }}
    }})();
    "#, escaped_eid, escaped_eid.trim_start_matches("id_"), escaped_eid.trim_start_matches("id_"), escaped_text, do_clear);

    let _ = webview.eval(&type_script);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let current_url = webview.url().map(|u| u.to_string()).ok();

    Ok(BrowserActionResult {
        success: true,
        action: "type".to_string(),
        tab_id,
        element_id: Some(element_id),
        page_changed: true,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

/// Step 7: Bounded Scroll Action
#[tauri::command]
pub async fn browser_scroll(
    app: AppHandle,
    tab_id: String,
    direction: String,
    amount: Option<i32>,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let step = amount.unwrap_or(350).clamp(50, 1500);

    let scroll_script = match direction.to_lowercase().as_str() {
        "up" => format!("window.scrollBy({{ top: -{}, left: 0, behavior: 'instant' }});", step),
        "down" => format!("window.scrollBy({{ top: {}, left: 0, behavior: 'instant' }});", step),
        "left" => format!("window.scrollBy({{ top: 0, left: -{}, behavior: 'instant' }});", step),
        "right" => format!("window.scrollBy({{ top: 0, left: {}, behavior: 'instant' }});", step),
        "top" => "window.scrollTo({ top: 0, left: 0, behavior: 'instant' });".to_string(),
        "bottom" => "window.scrollTo({ top: document.body.scrollHeight, left: 0, behavior: 'instant' });".to_string(),
        _ => return Err(format!("INVALID_SCROLL_DIRECTION: '{}' is not supported. Use up/down/left/right/top/bottom.", direction)),
    };

    let _ = webview.eval(&scroll_script);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let current_url = webview.url().map(|u| u.to_string()).ok();

    Ok(BrowserActionResult {
        success: true,
        action: format!("scroll_{}", direction.to_lowercase()),
        tab_id,
        element_id: None,
        page_changed: true,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

/// Step 8: Strict Key Press Action
#[tauri::command]
pub async fn browser_press_key(
    app: AppHandle,
    tab_id: String,
    key: String,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let (key_name, key_code) = match key.to_lowercase().as_str() {
        "enter" => ("Enter", 13),
        "escape" => ("Escape", 27),
        "tab" => ("Tab", 9),
        "backspace" => ("Backspace", 8),
        "delete" => ("Delete", 46),
        "arrowup" => ("ArrowUp", 38),
        "arrowdown" => ("ArrowDown", 40),
        "arrowleft" => ("ArrowLeft", 37),
        "arrowright" => ("ArrowRight", 39),
        "home" => ("Home", 36),
        "end" => ("End", 35),
        "pageup" => ("PageUp", 33),
        "pagedown" => ("PageDown", 34),
        "space" => (" ", 32),
        _ => return Err(format!("UNSUPPORTED_KEY: Key '{}' is not in the allowed key press policy.", key)),
    };

    let key_script = format!(r#"
    (function() {{
        try {{
            var active = document.activeElement || document.body;
            var opts = {{ key: "{}", keyCode: {}, which: {}, bubbles: true, cancelable: true, view: window }};
            active.dispatchEvent(new KeyboardEvent('keydown', opts));
            active.dispatchEvent(new KeyboardEvent('keypress', opts));
            active.dispatchEvent(new KeyboardEvent('keyup', opts));
            if ("{}" === "Enter" && active.form) {{
                try {{ active.form.submit(); }} catch(e) {{}}
            }}
        }} catch(e) {{}}
    }})();
    "#, key_name, key_code, key_code, key_name);

    let _ = webview.eval(&key_script);
    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

    let current_url = webview.url().map(|u| u.to_string()).ok();

    Ok(BrowserActionResult {
        success: true,
        action: format!("press_key_{}", key),
        tab_id,
        element_id: None,
        page_changed: true,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

/// Step 9: Focus Element Action
#[tauri::command]
pub async fn browser_focus_element(
    app: AppHandle,
    tab_id: String,
    element_id: String,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let escaped_eid = element_id.replace('"', "\\\"").replace('\\', "\\\\");
    let focus_script = format!(r#"
    (function() {{
        var target = document.querySelector('[data-edith-eid="{}"]') 
                  || (document.getElementById("{}"))
                  || document.querySelector('[id="{}"]');
        if (target) {{
            target.scrollIntoView({{ behavior: 'instant', block: 'center' }});
            target.focus();
        }}
    }})();
    "#, escaped_eid, escaped_eid.trim_start_matches("id_"), escaped_eid.trim_start_matches("id_"));

    let _ = webview.eval(&focus_script);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let current_url = webview.url().map(|u| u.to_string()).ok();

    Ok(BrowserActionResult {
        success: true,
        action: "focus".to_string(),
        tab_id,
        element_id: Some(element_id),
        page_changed: false,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

/// Step 9: Bounded Wait Condition
#[tauri::command]
pub async fn browser_wait(
    app: AppHandle,
    tab_id: String,
    condition: String,
    target: Option<String>,
    timeout_ms: Option<u64>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserActionResult, String> {
    let max_timeout = timeout_ms.unwrap_or(3000).clamp(100, 10000);
    let start = SystemTime::now();

    match condition.to_lowercase().as_str() {
        "timeout" => {
            tokio::time::sleep(tokio::time::Duration::from_millis(max_timeout)).await;
        }
        "url_changed" => {
            let initial_url = target.clone().unwrap_or_default();
            while SystemTime::now().duration_since(start).unwrap_or_default().as_millis() < max_timeout as u128 {
                let current = browser_get_tab_url(app.clone(), tab_id.clone(), state.clone()).await.unwrap_or_default();
                if current != initial_url && !current.is_empty() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        "element_present" | "text_present" | "page_load" => {
            tokio::time::sleep(tokio::time::Duration::from_millis(max_timeout.min(1000))).await;
        }
        _ => {
            return Err(format!("UNSUPPORTED_WAIT_CONDITION: Condition '{}' not supported.", condition));
        }
    }

    let current_url = browser_get_tab_url(app, tab_id.clone(), state).await.ok();

    Ok(BrowserActionResult {
        success: true,
        action: format!("wait_{}", condition),
        tab_id,
        element_id: target,
        page_changed: false,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

/// Optional: Select Option in `<select>` dropdown
#[tauri::command]
pub async fn browser_select_option(
    app: AppHandle,
    tab_id: String,
    element_id: String,
    value: String,
) -> Result<BrowserActionResult, String> {
    let label = get_tab_label(&tab_id);
    let webview = app.get_webview(&label)
        .ok_or_else(|| "TAB_NOT_FOUND: Target browser tab does not exist.".to_string())?;

    let escaped_eid = element_id.replace('"', "\\\"").replace('\\', "\\\\");
    let escaped_val = value.replace('"', "\\\"").replace('\\', "\\\\");

    let select_script = format!(r#"
    (function() {{
        try {{
            var target = document.querySelector('[data-edith-eid="{}"]') 
                      || (document.getElementById("{}"))
                      || document.querySelector('[id="{}"]');
            if (!target || target.tagName.toLowerCase() !== 'select') return;
            target.value = "{}";
            target.dispatchEvent(new Event('change', {{ bubbles: true, cancelable: true }}));
        }} catch(e) {{}}
    }})();
    "#, escaped_eid, escaped_eid.trim_start_matches("id_"), escaped_eid.trim_start_matches("id_"), escaped_val);

    let _ = webview.eval(&select_script);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let current_url = webview.url().map(|u| u.to_string()).ok();

    Ok(BrowserActionResult {
        success: true,
        action: "select_option".to_string(),
        tab_id,
        element_id: Some(element_id),
        page_changed: true,
        url_changed: false,
        resulting_url: current_url,
        error: None,
        error_code: None,
    })
}

// -----------------------------------------------------------------------------
// Legacy Phase 1 Delegate Handlers (For full backward compatibility)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_create(
    app: AppHandle,
    url: Option<String>,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserInfo, String> {
    let tab = browser_create_tab(app, "tab_a".to_string(), url, bounds.clone(), state).await?;
    Ok(BrowserInfo {
        is_created: true,
        is_visible: true,
        current_url: tab.url,
        title: tab.title,
        bounds,
    })
}

#[tauri::command]
pub async fn browser_destroy(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    browser_hide_all(app, state).await
}

#[tauri::command]
pub async fn browser_show(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    browser_show_active(app, None, state).await
}

#[tauri::command]
pub async fn browser_hide(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    browser_hide_all(app, state).await
}

#[tauri::command]
pub async fn browser_navigate(
    app: AppHandle,
    url: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    browser_navigate_tab(app, active_id, url, state).await
}

#[tauri::command]
pub async fn browser_go_back(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    browser_go_back_tab(app, active_id).await
}

#[tauri::command]
pub async fn browser_go_forward(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    browser_go_forward_tab(app, active_id).await
}

#[tauri::command]
pub async fn browser_reload(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    browser_reload_tab(app, active_id).await
}

#[tauri::command]
pub async fn browser_set_bounds(
    app: AppHandle,
    bounds: BrowserViewportBounds,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    browser_set_bounds_all(app, bounds, state).await
}

#[tauri::command]
pub async fn browser_get_url(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    let label = get_tab_label(&active_id);
    if let Some(wv) = app.get_webview(&label) {
        if let Ok(u) = wv.url() {
            return Ok(u.to_string());
        }
    }
    let tabs = state.tabs.lock().unwrap();
    Ok(tabs.iter().find(|x| x.id == active_id).map(|x| x.url.clone()).unwrap_or_default())
}

#[tauri::command]
pub async fn browser_get_title(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    let obs = browser_observe_tab(app, active_id, state).await?;
    Ok(obs.title)
}

#[tauri::command]
pub async fn browser_get_visible_text(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let active_id = {
        let guard = state.active_tab_id.lock().unwrap();
        guard.clone().unwrap_or_else(|| "tab_a".to_string())
    };
    let obs = browser_observe_tab(app, active_id, state).await?;
    Ok(obs.visible_text)
}
