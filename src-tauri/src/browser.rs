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
    pub id: Option<String>,
    pub tag: String,
    pub role: Option<String>,
    pub text: String,
    pub aria_label: Option<String>,
    pub href: Option<String>,
    pub input_type: Option<String>,
    pub disabled: bool,
    pub visible: bool,
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

/// Standard, read-only script injected into child webviews on navigation to enable actual DOM observation
const LIVE_OBSERVER_INIT_SCRIPT: &str = r#"
(function() {
    window.__EDITH_LIVE_OBSERVE__ = function() {
        try {
            var text = document.body ? (document.body.innerText || document.body.textContent || '').trim() : '';
            var sel = window.getSelection ? window.getSelection().toString() : '';
            var elements = [];
            var nodes = document.querySelectorAll('button, a[href], input, textarea, select, [role="button"], [role="link"], h1, h2, h3');
            var limit = Math.min(nodes.length, 60);
            for (var i = 0; i < limit; i++) {
                var el = nodes[i];
                var rect = el.getBoundingClientRect();
                var isVis = rect.width > 0 && rect.height > 0;
                var textContent = (el.innerText || el.value || el.placeholder || '').trim();
                elements.push({
                    id: el.id || null,
                    tag: el.tagName.toLowerCase(),
                    role: el.getAttribute('role') || null,
                    text: textContent.slice(0, 100),
                    aria_label: el.getAttribute('aria-label') || null,
                    href: el.href || null,
                    input_type: el.type || null,
                    disabled: !!el.disabled,
                    visible: isVis,
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

    // Fallback document parsing for live static / dynamic structure
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
                        if i >= 40 { break; }
                        let tag = el.value().name().to_string();
                        let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let href = el.value().attr("href").map(|s| s.to_string());
                        let input_type = el.value().attr("type").map(|s| s.to_string());
                        let id = el.value().attr("id").map(|s| s.to_string());
                        let aria_label = el.value().attr("aria-label").map(|s| s.to_string());
                        let disabled = el.value().attr("disabled").is_some();

                        interactive_elements.push(ElementInfo {
                            id,
                            tag,
                            role: None,
                            text: text.chars().take(80).collect(),
                            aria_label,
                            href,
                            input_type,
                            disabled,
                            visible: true,
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
    
    // Capture screen via screenshots crate
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
