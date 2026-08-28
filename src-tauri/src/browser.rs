use std::sync::Mutex;
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
pub struct BrowserTabInfo {
    pub id: String,
    pub label: String,
    pub url: String,
    pub title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMultiStateInfo {
    pub tabs: Vec<BrowserTabInfo>,
    pub active_tab_id: Option<String>,
    pub is_visible: bool,
    pub bounds: Option<BrowserViewportBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub is_created: bool,
    pub is_visible: bool,
    pub current_url: String,
    pub title: String,
    pub bounds: Option<BrowserViewportBounds>,
}

pub struct BrowserState {
    pub tabs: Mutex<Vec<BrowserTabInfo>>,
    pub active_tab_id: Mutex<Option<String>>,
    pub is_visible: Mutex<bool>,
    pub bounds: Mutex<Option<BrowserViewportBounds>>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            tabs: Mutex::new(Vec::new()),
            active_tab_id: Mutex::new(None),
            is_visible: Mutex::new(false),
            bounds: Mutex::new(None),
        }
    }
}

pub fn get_tab_label(tab_id: &str) -> String {
    format!("edith_tab_{}", tab_id)
}

/// Normalizes user input into a valid HTTPS URL or search query URL
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "https://example.com".to_string();
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    let is_domain = (trimmed.contains('.') && !trimmed.contains(' ') && !trimmed.starts_with('.'))
        || trimmed.starts_with("localhost");

    if is_domain {
        format!("https://{}", trimmed)
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(trimmed))
    }
}

// -----------------------------------------------------------------------------
// Multi-Tab Native Commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn browser_create_tab(
    app: AppHandle,
    tab_id: String,
    url: Option<String>,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let target_url_str = normalize_url(&url.unwrap_or_else(|| "https://example.com".to_string()));
    let target_url = Url::parse(&target_url_str)
        .map_err(|e| format!("Invalid URL format: {}", e))?;

    let label = get_tab_label(&tab_id);

    // If bounds provided, store in state
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

    // Hide any currently active tab's Webview
    if let Some(ref active_id) = *state.active_tab_id.lock().unwrap() {
        let prev_label = get_tab_label(active_id);
        if let Some(prev_webview) = app.get_webview(&prev_label) {
            let _ = prev_webview.hide();
        }
    }

    // Check if webview already exists
    if let Some(existing_webview) = app.get_webview(&label) {
        let _ = existing_webview.set_position(Position::Logical(pos));
        let _ = existing_webview.set_size(Size::Logical(size));
        let _ = existing_webview.show();
        let _ = existing_webview.set_focus();
        let _ = existing_webview.navigate(target_url);
    } else {
        // Retrieve parent window to attach child Webview surface inside the E.D.I.T.H. main window
        let window = app.get_window("main")
            .ok_or_else(|| "Main window 'main' not found.".to_string())?;

        let webview_url = WebviewUrl::External(target_url);
        let builder = WebviewBuilder::new(&label, webview_url);

        window.add_child(builder, pos, size)
            .map_err(|e| format!("Failed to create and attach native child Webview {}: {}", label, e))?;

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

    let new_tab = BrowserTabInfo {
        id: tab_id.clone(),
        label: label.clone(),
        url: target_url_str,
        title: default_title,
        is_active: true,
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

    // Check if tab exists
    let target_tab = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).cloned()
    };

    let mut tab_info = target_tab.ok_or_else(|| format!("Tab '{}' not found in browser state.", tab_id))?;

    // Hide all existing tab webviews except the target
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

    // Show and reposition the target tab webview
    if let Some(target_wv) = app.get_webview(&target_label) {
        if let Some(ref b) = current_bounds {
            let _ = target_wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
            let _ = target_wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
        }
        let _ = target_wv.show();
        let _ = target_wv.set_focus();

        // Update live URL
        if let Ok(u) = target_wv.url() {
            tab_info.url = u.to_string();
        }
    }

    // Update active tab in state
    {
        let mut tabs = state.tabs.lock().unwrap();
        for tab in tabs.iter_mut() {
            tab.is_active = tab.id == tab_id;
            if tab.id == tab_id {
                tab.url = tab_info.url.clone();
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

    // Destroy native Webview
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.close();
    }

    let mut next_active: Option<BrowserTabInfo> = None;

    {
        let mut tabs = state.tabs.lock().unwrap();
        let was_active = state.active_tab_id.lock().unwrap().as_deref() == Some(&tab_id);
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

    // If a new tab became active, show it
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
pub async fn browser_navigate_tab(
    app: AppHandle,
    tab_id: String,
    url: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let normalized = normalize_url(&url);
    let target_url = Url::parse(&normalized)
        .map_err(|e| format!("Invalid target URL: {}", e))?;

    let label = get_tab_label(&tab_id);

    if let Some(webview) = app.get_webview(&label) {
        webview.navigate(target_url)
            .map_err(|e| format!("Navigation failed for tab {}: {}", tab_id, e))?;

        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = normalized.clone();
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
            }
        }
    }
    let active_tab_id = state.active_tab_id.lock().unwrap().clone();
    let is_visible = *state.is_visible.lock().unwrap();
    let bounds = state.bounds.lock().unwrap().clone();

    Ok(BrowserMultiStateInfo {
        tabs,
        active_tab_id,
        is_visible,
        bounds,
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

#[tauri::command]
pub async fn browser_get_tab_url(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        match wv.url() {
            Ok(u) => {
                let url_str = u.to_string();
                let mut tabs = state.tabs.lock().unwrap();
                if let Some(t) = tabs.iter_mut().find(|x| x.id == tab_id) {
                    t.url = url_str.clone();
                }
                Ok(url_str)
            }
            Err(_) => {
                let tabs = state.tabs.lock().unwrap();
                Ok(tabs.iter().find(|x| x.id == tab_id).map(|x| x.url.clone()).unwrap_or_default())
            }
        }
    } else {
        let tabs = state.tabs.lock().unwrap();
        Ok(tabs.iter().find(|x| x.id == tab_id).map(|x| x.url.clone()).unwrap_or_default())
    }
}

#[tauri::command]
pub async fn browser_get_tab_title(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let label = get_tab_label(&tab_id);
    let current_url = if let Some(wv) = app.get_webview(&label) {
        wv.url().map(|u| u.to_string()).unwrap_or_default()
    } else {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|x| x.id == tab_id).map(|x| x.url.clone()).unwrap_or_default()
    };

    if current_url.is_empty() {
        return Ok("No Page Loaded".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    match client.get(&current_url).send().await {
        Ok(res) => {
            if let Ok(html) = res.text().await {
                let document = scraper::Html::parse_document(&html);
                if let Ok(title_sel) = scraper::Selector::parse("title") {
                    if let Some(title_el) = document.select(&title_sel).next() {
                        let title = title_el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if !title.is_empty() {
                            let mut tabs = state.tabs.lock().unwrap();
                            if let Some(t) = tabs.iter_mut().find(|x| x.id == tab_id) {
                                t.title = title.clone();
                            }
                            return Ok(title);
                        }
                    }
                }
            }
            let tabs = state.tabs.lock().unwrap();
            Ok(tabs.iter().find(|x| x.id == tab_id).map(|x| x.title.clone()).unwrap_or_else(|| "Unknown Title".to_string()))
        }
        Err(_) => {
            let tabs = state.tabs.lock().unwrap();
            Ok(tabs.iter().find(|x| x.id == tab_id).map(|x| x.title.clone()).unwrap_or_else(|| "Unknown Title".to_string()))
        }
    }
}

#[tauri::command]
pub async fn browser_get_tab_visible_text(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let label = get_tab_label(&tab_id);
    let current_url = if let Some(wv) = app.get_webview(&label) {
        wv.url().map(|u| u.to_string()).unwrap_or_default()
    } else {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|x| x.id == tab_id).map(|x| x.url.clone()).unwrap_or_default()
    };

    if current_url.is_empty() {
        return Ok("No page loaded to extract text.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let res = client.get(&current_url).send().await
        .map_err(|e| format!("Failed to read page: {}", e))?;

    let html = res.text().await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let document = scraper::Html::parse_document(&html);
    let body_sel = scraper::Selector::parse("body").map_err(|e| format!("Selector error: {:?}", e))?;

    let mut text_parts = Vec::new();
    if let Some(body) = document.select(&body_sel).next() {
        for text in body.text() {
            let t = text.trim();
            if !t.is_empty() {
                text_parts.push(t);
            }
        }
    }

    let joined = text_parts.join(" ");
    let bounded = if joined.len() > 50_000 {
        format!("{}... [Truncated at 50,000 characters]", &joined[..50_000])
    } else {
        joined
    };

    Ok(bounded)
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
    browser_get_tab_url(app, active_id, state).await
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
    browser_get_tab_title(app, active_id, state).await
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
    browser_get_tab_visible_text(app, active_id, state).await
}
