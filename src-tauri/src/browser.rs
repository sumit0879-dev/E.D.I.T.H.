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
pub struct BrowserInfo {
    pub is_created: bool,
    pub is_visible: bool,
    pub current_url: String,
    pub title: String,
    pub bounds: Option<BrowserViewportBounds>,
}

pub struct BrowserState {
    pub is_created: Mutex<bool>,
    pub is_visible: Mutex<bool>,
    pub current_url: Mutex<String>,
    pub current_title: Mutex<String>,
    pub bounds: Mutex<Option<BrowserViewportBounds>>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            is_created: Mutex::new(false),
            is_visible: Mutex::new(false),
            current_url: Mutex::new("https://example.com".to_string()),
            current_title: Mutex::new("Example Domain".to_string()),
            bounds: Mutex::new(None),
        }
    }
}

const BROWSER_WEBVIEW_LABEL: &str = "edith_browser_webview";

/// Normalizes user input into a valid HTTPS URL or search query URL
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "https://example.com".to_string();
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    // Check if it looks like a domain name (e.g., example.com, localhost:1420, sub.domain.org/path)
    let is_domain = (trimmed.contains('.') && !trimmed.contains(' ') && !trimmed.starts_with('.'))
        || trimmed.starts_with("localhost");

    if is_domain {
        format!("https://{}", trimmed)
    } else {
        // Fallback to DuckDuckGo search query
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(trimmed))
    }
}

#[tauri::command]
pub async fn browser_create(
    app: AppHandle,
    url: Option<String>,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserInfo, String> {
    let target_url_str = normalize_url(&url.unwrap_or_else(|| "https://example.com".to_string()));
    let target_url = Url::parse(&target_url_str)
        .map_err(|e| format!("Invalid URL format: {}", e))?;

    // Check if already created
    if let Some(existing_webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        if let Some(ref b) = bounds {
            let _ = existing_webview.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
            let _ = existing_webview.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
        }
        let _ = existing_webview.show();
        let _ = existing_webview.set_focus();
        let _ = existing_webview.navigate(target_url);

        *state.is_created.lock().unwrap() = true;
        *state.is_visible.lock().unwrap() = true;
        *state.current_url.lock().unwrap() = target_url_str.clone();
        if let Some(b) = bounds.clone() {
            *state.bounds.lock().unwrap() = Some(b);
        }

        return Ok(BrowserInfo {
            is_created: true,
            is_visible: true,
            current_url: target_url_str,
            title: state.current_title.lock().unwrap().clone(),
            bounds,
        });
    }

    // Retrieve main window to attach child Webview surface inside the E.D.I.T.H. window
    let window = app.get_window("main")
        .ok_or_else(|| "Main window 'main' not found.".to_string())?;

    let webview_url = WebviewUrl::External(target_url);
    let builder = WebviewBuilder::new(BROWSER_WEBVIEW_LABEL, webview_url);

    let (pos, size) = if let Some(ref b) = bounds {
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

    window.add_child(builder, pos, size)
        .map_err(|e| format!("Failed to create and attach native child Webview: {}", e))?;

    *state.is_created.lock().unwrap() = true;
    *state.is_visible.lock().unwrap() = true;
    *state.current_url.lock().unwrap() = target_url_str.clone();
    *state.bounds.lock().unwrap() = bounds.clone();

    Ok(BrowserInfo {
        is_created: true,
        is_visible: true,
        current_url: target_url_str,
        title: "Loading...".to_string(),
        bounds,
    })
}

#[tauri::command]
pub async fn browser_destroy(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        let _ = webview.close();
    }
    *state.is_created.lock().unwrap() = false;
    *state.is_visible.lock().unwrap() = false;
    Ok(())
}

#[tauri::command]
pub async fn browser_show(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        let _ = webview.show();
        let _ = webview.set_focus();
        *state.is_visible.lock().unwrap() = true;
        Ok(())
    } else {
        Err("Native browser webview does not exist.".to_string())
    }
}

#[tauri::command]
pub async fn browser_hide(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        let _ = webview.hide();
        *state.is_visible.lock().unwrap() = false;
        Ok(())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn browser_navigate(
    app: AppHandle,
    url: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let normalized = normalize_url(&url);
    let target_url = Url::parse(&normalized)
        .map_err(|e| format!("Invalid target URL: {}", e))?;

    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.navigate(target_url)
            .map_err(|e| format!("Navigation failed: {}", e))?;
        *state.current_url.lock().unwrap() = normalized.clone();
        Ok(normalized)
    } else {
        Err("Native browser webview not initialized.".to_string())
    }
}

#[tauri::command]
pub async fn browser_go_back(app: AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.eval("window.history.back();")
            .map_err(|e| format!("Go back failed: {}", e))?;
        Ok(())
    } else {
        Err("Native browser webview not initialized.".to_string())
    }
}

#[tauri::command]
pub async fn browser_go_forward(app: AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.eval("window.history.forward();")
            .map_err(|e| format!("Go forward failed: {}", e))?;
        Ok(())
    } else {
        Err("Native browser webview not initialized.".to_string())
    }
}

#[tauri::command]
pub async fn browser_reload(app: AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.eval("window.location.reload();")
            .map_err(|e| format!("Reload failed: {}", e))?;
        Ok(())
    } else {
        Err("Native browser webview not initialized.".to_string())
    }
}

#[tauri::command]
pub async fn browser_set_bounds(
    app: AppHandle,
    bounds: BrowserViewportBounds,
    state: tauri::State<'_, BrowserState>,
) -> Result<(), String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        let _ = webview.set_position(Position::Logical(LogicalPosition::new(bounds.x, bounds.y)));
        let _ = webview.set_size(Size::Logical(LogicalSize::new(bounds.width, bounds.height)));
        *state.bounds.lock().unwrap() = Some(bounds);
        Ok(())
    } else {
        *state.bounds.lock().unwrap() = Some(bounds);
        Ok(())
    }
}

#[tauri::command]
pub async fn browser_get_url(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        match webview.url() {
            Ok(u) => {
                let url_str = u.to_string();
                *state.current_url.lock().unwrap() = url_str.clone();
                Ok(url_str)
            }
            Err(_) => Ok(state.current_url.lock().unwrap().clone()),
        }
    } else {
        Ok(state.current_url.lock().unwrap().clone())
    }
}

#[tauri::command]
pub async fn browser_get_title(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let current_url = if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.url().map(|u| u.to_string()).unwrap_or_else(|_| state.current_url.lock().unwrap().clone())
    } else {
        state.current_url.lock().unwrap().clone()
    };

    if current_url.is_empty() {
        return Ok("No Page Loaded".to_string());
    }

    // Scoped observation: Fetch and parse title from current URL securely
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
                            *state.current_title.lock().unwrap() = title.clone();
                            return Ok(title);
                        }
                    }
                }
            }
            Ok(state.current_title.lock().unwrap().clone())
        }
        Err(_) => Ok(state.current_title.lock().unwrap().clone()),
    }
}

#[tauri::command]
pub async fn browser_get_visible_text(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let current_url = if let Some(webview) = app.get_webview(BROWSER_WEBVIEW_LABEL) {
        webview.url().map(|u| u.to_string()).unwrap_or_else(|_| state.current_url.lock().unwrap().clone())
    } else {
        state.current_url.lock().unwrap().clone()
    };

    if current_url.is_empty() {
        return Ok("No page loaded to extract text.".to_string());
    }

    // Scoped observation: Safely extract visible body text, limited to 50,000 characters
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
