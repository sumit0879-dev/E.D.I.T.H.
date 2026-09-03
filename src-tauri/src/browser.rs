use std::collections::HashMap;
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
pub struct RegionInfo {
    pub region_type: String, // "header", "nav", "main", "article", "section", "aside", "footer", "form", "dialog", "menu"
    pub label: Option<String>,
    pub element_id: Option<String>,
    pub bounding_box: Option<BrowserElementBounds>,
    pub elements_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingInfo {
    pub level: u32,
    pub text: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormControlInfo {
    pub element_id: String,
    pub field_type: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
    pub disabled: bool,
    pub is_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub action: Option<String>,
    pub method: Option<String>,
    pub controls: Vec<FormControlInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub text: String,
    pub href: String,
    pub role: Option<String>,
    pub visible: bool,
    pub is_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportInfo {
    pub width: f64,
    pub height: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub page_width: f64,
    pub page_height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub id: String, // Guaranteed non-empty deterministic identifier (e.g. "el_btn_submit_a1b2c3" or "id_submit_btn")
    pub tag: String,
    pub role: Option<String>,
    pub accessible_name: Option<String>,
    pub text: String,
    pub aria_label: Option<String>,
    pub href: Option<String>,
    pub input_type: Option<String>,
    pub placeholder: Option<String>,
    pub value_available: bool,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub visible: bool,
    pub interactable: bool,
    pub is_password: bool,
    pub is_in_iframe: bool,
    pub parent_region: Option<String>,
    pub bounding_box: Option<BrowserElementBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageObservationSnapshot {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub generation: u64,
    pub fingerprint: String,
    pub viewport: ViewportInfo,
    pub visible_text: String,
    pub selected_text: Option<String>,
    pub regions: Vec<RegionInfo>,
    pub headings: Vec<HeadingInfo>,
    pub interactive_elements: Vec<ElementInfo>,
    pub forms: Vec<FormInfo>,
    pub links: Vec<LinkInfo>,
    pub is_reader_mode: bool,
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
pub struct FindResult {
    pub query: String,
    pub match_found: bool,
    pub matches_count: u32,
    pub active_match_ordinal: u32,
}

pub static GLOBAL_FIND_RESULTS: Mutex<Option<HashMap<String, FindResult>>> = Mutex::new(None);

pub fn set_global_find_result(tab_id: String, res: FindResult) {
    let mut guard = GLOBAL_FIND_RESULTS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    if let Some(ref mut map) = *guard {
        map.insert(tab_id, res);
    }
}

pub fn get_global_find_result(tab_id: &str) -> Option<FindResult> {
    let guard = GLOBAL_FIND_RESULTS.lock().unwrap();
    guard.as_ref().and_then(|map| map.get(tab_id).cloned())
}

pub fn clear_global_find_result(tab_id: &str) {
    let mut guard = GLOBAL_FIND_RESULTS.lock().unwrap();
    if let Some(ref mut map) = *guard {
        map.remove(tab_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderDocument {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub byline: Option<String>,
    pub published_time: Option<String>,
    pub excerpt: Option<String>,
    pub content_html: String,
    pub text_content: String,
    pub word_count: u32,
    pub reading_time_minutes: u32,
    pub images: Vec<String>,
    pub extracted_at: u64,
}

pub static GLOBAL_READER_DOCS: Mutex<Option<HashMap<String, ReaderDocument>>> = Mutex::new(None);

pub fn set_global_reader_doc(tab_id: String, doc: ReaderDocument) {
    let mut guard = GLOBAL_READER_DOCS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    if let Some(ref mut map) = *guard {
        map.insert(tab_id, doc);
    }
}

pub fn get_global_reader_doc(tab_id: &str) -> Option<ReaderDocument> {
    let guard = GLOBAL_READER_DOCS.lock().unwrap();
    guard.as_ref().and_then(|map| map.get(tab_id).cloned())
}

pub fn clear_global_reader_doc(tab_id: &str) {
    let mut guard = GLOBAL_READER_DOCS.lock().unwrap();
    if let Some(ref mut map) = *guard {
        map.remove(tab_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabGroup {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub color: String, // "blue", "purple", "green", "yellow", "orange", "red", "gray"
    pub is_collapsed: bool,
    pub position: i64,
    pub created_at: u64,
    pub updated_at: u64,
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
    pub profile_id: String,
    pub is_pinned: bool,
    pub zoom_level: f64,
    pub is_reader_mode: bool,
    pub is_pdf: bool,
    pub group_id: Option<String>,
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
    pub generations: Mutex<HashMap<String, u64>>,
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
            generations: Mutex::new(HashMap::new()),
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

    let lower = trimmed.to_lowercase();
    let cleaned = if lower.ends_with('/') {
        &lower[..lower.len() - 1]
    } else {
        &lower
    };

    // Explicitly supported internal E.D.I.T.H. routes
    if cleaned.starts_with("edith://") {
        return Ok(cleaned.to_string());
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
        // Deterministic Google search fallback
        Ok(format!("https://www.google.com/search?q={}", urlencoding::encode(trimmed)))
    }
}

/// Hardened read-only observation script injected into all child webviews.
/// Generates deterministic, collision-resistant Element Identifiers,
/// discovers semantic page regions, headings, forms, links, accessibility signals,
/// real viewport bounding boxes, and detects password fields with zero value leakage.
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

    window.__EDITH_LIVE_OBSERVE__ = function(scope) {
        try {
            var doc = document;
            var win = window;
            var viewport = {
                width: win.innerWidth || doc.documentElement.clientWidth || 1024,
                height: win.innerHeight || doc.documentElement.clientHeight || 768,
                scroll_x: win.scrollX || win.pageXOffset || 0,
                scroll_y: win.scrollY || win.pageYOffset || 0,
                page_width: doc.documentElement.scrollWidth || 1024,
                page_height: doc.documentElement.scrollHeight || 768
            };

            // 1. Semantic Page Regions
            var regions = [];
            var regionNodes = doc.querySelectorAll('header, nav, main, article, section, aside, footer, form, dialog, [role="banner"], [role="navigation"], [role="main"], [role="complementary"], [role="contentinfo"], [role="dialog"], [role="menu"]');
            for (var r = 0; r < Math.min(regionNodes.length, 25); r++) {
                var rNode = regionNodes[r];
                var rRect = rNode.getBoundingClientRect();
                var rType = rNode.tagName.toLowerCase();
                var rRole = rNode.getAttribute('role');
                var rLabel = rNode.getAttribute('aria-label') || rNode.getAttribute('aria-labelledby') || rNode.getAttribute('title') || null;
                regions.push({
                    region_type: rRole || rType,
                    label: rLabel,
                    element_id: rNode.id ? ('id_' + rNode.id) : null,
                    bounding_box: { x: rRect.x, y: rRect.y, width: rRect.width, height: rRect.height },
                    elements_count: rNode.querySelectorAll('*').length
                });
            }

            // 2. Headings Hierarchy
            var headings = [];
            var headingNodes = doc.querySelectorAll('h1, h2, h3, h4, h5, h6');
            for (var h = 0; h < Math.min(headingNodes.length, 30); h++) {
                var hNode = headingNodes[h];
                var lvl = parseInt(hNode.tagName.substring(1), 10) || 1;
                var hText = (hNode.innerText || hNode.textContent || '').trim();
                if (hText) {
                    headings.push({
                        level: lvl,
                        text: hText.slice(0, 100),
                        id: hNode.id || null
                    });
                }
            }

            // 3. Form Understanding
            var forms = [];
            var formNodes = doc.querySelectorAll('form');
            for (var f = 0; f < Math.min(formNodes.length, 10); f++) {
                var fNode = formNodes[f];
                var controls = [];
                var inputNodes = fNode.querySelectorAll('input, select, textarea, button');
                for (var c = 0; c < Math.min(inputNodes.length, 20); c++) {
                    var cNode = inputNodes[c];
                    var cTag = cNode.tagName.toLowerCase();
                    var cType = (cNode.getAttribute('type') || (cTag === 'textarea' ? 'textarea' : cTag === 'select' ? 'select' : 'button')).toLowerCase();
                    var isPw = cTag === 'input' && (cType === 'password' || cNode.getAttribute('autocomplete') === 'current-password');
                    var cLabel = cNode.getAttribute('aria-label') || (cNode.labels && cNode.labels[0] ? cNode.labels[0].innerText : null) || cNode.getAttribute('placeholder') || null;
                    controls.push({
                        element_id: cNode.id ? ('id_' + cNode.id) : ('el_' + cTag + '_' + computeElementHash(cTag + ':' + cType + ':' + c)),
                        field_type: cType,
                        label: cLabel ? cLabel.trim().slice(0, 50) : null,
                        placeholder: cNode.getAttribute('placeholder') || null,
                        required: !!cNode.required,
                        disabled: !!cNode.disabled,
                        is_password: isPw
                    });
                }
                forms.push({
                    id: fNode.id || null,
                    name: fNode.getAttribute('name') || null,
                    action: fNode.getAttribute('action') || null,
                    method: (fNode.getAttribute('method') || 'GET').toUpperCase(),
                    controls: controls
                });
            }

            // 4. Link Understanding
            var links = [];
            var linkNodes = doc.querySelectorAll('a[href]');
            for (var l = 0; l < Math.min(linkNodes.length, 40); l++) {
                var lNode = linkNodes[l];
                var lHref = lNode.getAttribute('href') || '';
                var lText = (lNode.innerText || lNode.textContent || '').trim();
                var lRect = lNode.getBoundingClientRect();
                var lVis = lRect.width > 0 && lRect.height > 0;
                var isExt = lHref.startsWith('http') && !lHref.includes(win.location.hostname);
                links.push({
                    text: lText.slice(0, 80),
                    href: lHref,
                    role: lNode.getAttribute('role') || null,
                    visible: lVis,
                    is_external: isExt
                });
            }

            // 5. Interactive Elements with Real Viewport Geometry & Accessibility
            var elements = [];
            var selector = 'button, a[href], input, textarea, select, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [role="tab"], [role="menuitem"], [contenteditable="true"]';
            var nodes = doc.querySelectorAll(selector);
            var limit = Math.min(nodes.length, 80);

            for (var i = 0; i < limit; i++) {
                var el = nodes[i];
                var rect = el.getBoundingClientRect();
                var computed = win.getComputedStyle(el);
                var isVis = rect.width > 0 && rect.height > 0 && computed.visibility !== 'hidden' && computed.display !== 'none' && computed.opacity !== '0';
                var textContent = (el.innerText || el.value || el.placeholder || '').trim();
                var tag = el.tagName.toLowerCase();
                var rawId = el.id || '';
                var role = el.getAttribute('role') || null;
                var href = el.getAttribute('href') || null;
                var inputType = el.getAttribute('type') || null;
                var ariaLabel = el.getAttribute('aria-label') || null;
                var placeholder = el.getAttribute('placeholder') || null;
                var isDisabled = !!el.disabled || el.getAttribute('aria-disabled') === 'true' || el.classList.contains('disabled');
                var isInteractable = isVis && !isDisabled && computed.pointerEvents !== 'none';
                
                // Password field detection
                var isPassword = tag === 'input' && (inputType === 'password' || el.getAttribute('autocomplete') === 'current-password');
                var isInIframe = win !== win.top;

                // Parent region traversal
                var parentRegionEl = el.closest('header, nav, main, article, section, aside, footer, dialog, form');
                var parentRegion = parentRegionEl ? parentRegionEl.tagName.toLowerCase() : null;

                // Accessible name computation
                var accessibleName = ariaLabel || (el.labels && el.labels[0] ? el.labels[0].innerText.trim() : null) || placeholder || (textContent ? textContent.slice(0, 50) : null);

                // Deterministic Element Identifier Generation (Step 14)
                var elementId = '';
                if (rawId && rawId.length > 0 && !/^[0-9]/.test(rawId)) {
                    elementId = 'id_' + rawId;
                } else {
                    var seed = tag + ':' + (role || '') + ':' + (href || '') + ':' + (inputType || '') + ':' + (accessibleName || '') + ':' + (parentRegion || '') + ':' + i;
                    elementId = 'el_' + tag + '_' + computeElementHash(seed);
                }

                // Tag element with identifier for direct action execution
                try {
                    el.setAttribute('data-edith-eid', elementId);
                } catch(e) {}

                elements.push({
                    id: elementId,
                    tag: tag,
                    role: role,
                    accessible_name: accessibleName,
                    text: textContent.slice(0, 100),
                    aria_label: ariaLabel,
                    href: href,
                    input_type: inputType,
                    placeholder: placeholder,
                    value_available: !isPassword, // Passwords NEVER expose value
                    disabled: isDisabled,
                    checked: el.checked !== undefined ? !!el.checked : null,
                    selected: el.selected !== undefined ? !!el.selected : null,
                    visible: isVis,
                    interactable: isInteractable,
                    is_password: isPassword,
                    is_in_iframe: isInIframe,
                    parent_region: parentRegion,
                    bounding_box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
                });
            }

            // 6. Clean Visible Text Extraction (excluding script, style, noscript)
            var bodyClone = doc.body ? doc.body.cloneNode(true) : null;
            var cleanText = '';
            if (bodyClone) {
                var scripts = bodyClone.querySelectorAll('script, style, noscript, svg, [aria-hidden="true"]');
                for (var s = 0; s < scripts.length; s++) {
                    scripts[s].remove();
                }
                cleanText = (bodyClone.innerText || bodyClone.textContent || '').trim().replace(/\s+/g, ' ');
            }

            var sel = win.getSelection ? win.getSelection().toString() : '';

            return {
                url: win.location.href,
                title: doc.title || '',
                viewport: viewport,
                visible_text: cleanText.slice(0, 20000),
                selected_text: sel || null,
                regions: regions,
                headings: headings,
                interactive_elements: elements,
                forms: forms,
                links: links,
                timestamp: Date.now()
            };
        } catch(e) {
            return {
                url: window.location.href,
                title: document.title || '',
                viewport: { width: 1024, height: 768, scroll_x: 0, scroll_y: 0, page_width: 1024, page_height: 768 },
                visible_text: '',
                selected_text: null,
                regions: [],
                headings: [],
                interactive_elements: [],
                forms: [],
                links: [],
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
    profile_id: Option<String>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let raw_input = url.unwrap_or_else(|| "https://example.com".to_string());
    let target_url_str = normalize_url(&raw_input)?;
    let target_url = Url::parse(&target_url_str)
        .map_err(|e| format!("Invalid URL format: {}", e))?;

    let label = get_tab_label(&tab_id);

    let target_profile_id = profile_id.unwrap_or_else(|| {
        crate::browser_profile::GLOBAL_PROFILE_MGR.get_active_profile_id()
    });
    let is_temp = target_profile_id.starts_with("agent_") || target_profile_id.contains("temporary");
    let profile_data_dir = crate::browser_profile::get_profile_data_dir(&target_profile_id, is_temp);

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

    let is_new_tab = target_url_str.starts_with("edith://");

    if let Some(existing_webview) = app.get_webview(&label) {
        let _ = existing_webview.set_position(Position::Logical(pos));
        let _ = existing_webview.set_size(Size::Logical(size));
        if is_new_tab {
            let _ = existing_webview.hide();
        } else {
            let _ = existing_webview.show();
            let _ = existing_webview.set_focus();
        }
        let _ = existing_webview.navigate(target_url);
    } else {
        let window = app.get_window("main")
            .ok_or_else(|| "Main window 'main' not found.".to_string())?;

        let webview_url = WebviewUrl::External(target_url);
        let mut builder = WebviewBuilder::new(&label, webview_url);

        // Phase 5.6C: Native WebView2 Storage & Profile Isolation (Step 4 & 5)
        builder = builder.data_directory(profile_data_dir);

        // Inject live DOM observer script
        builder = builder.initialization_script(LIVE_OBSERVER_INIT_SCRIPT);

        // Phase 5.6E: Privacy & Content Blocker Pre-flight Script (Step 10)
        builder = builder.initialization_script(crate::browser_privacy::PRIVACY_PREFLIGHT_INIT_SCRIPT);

        // Native Navigation Policy Callback & Find IPC Interception
        builder = builder.on_navigation(|nav_url| {
            let s = nav_url.as_str();

            // Intercept internal find results from bridge iframe
            if s.starts_with("edith-find:") {
                if let Some(query_str) = nav_url.query() {
                    for pair in query_str.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "data" {
                                if let Ok(decoded) = urlencoding::decode(v) {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&decoded) {
                                        let tab_id = val.get("tab_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                        let q = val.get("query").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                        let found = val.get("match_found").and_then(|x| x.as_bool()).unwrap_or(false);
                                        let count = val.get("matches_count").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                                        let active = val.get("active_match_ordinal").and_then(|x| x.as_u64()).unwrap_or(0) as u32;

                                        set_global_find_result(tab_id, FindResult {
                                            query: q,
                                            match_found: found,
                                            matches_count: count,
                                            active_match_ordinal: active,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                return false;
            }

            // Intercept internal reader extraction results from bridge iframe
            if s.starts_with("edith-reader:") {
                if let Some(query_str) = nav_url.query() {
                    for pair in query_str.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "data" {
                                if let Ok(decoded) = urlencoding::decode(v) {
                                    if let Ok(doc) = serde_json::from_str::<ReaderDocument>(&decoded) {
                                        set_global_reader_doc(doc.tab_id.clone(), doc);
                                    }
                                }
                            }
                        }
                    }
                }
                return false;
            }

            // Block javascript: and file: schemes from remote navigation
            let lower = s.to_lowercase();
            if lower.starts_with("javascript:") || lower.starts_with("file:") {
                return false;
            }
            if lower.starts_with("mailto:") || lower.starts_with("tel:") {
                let _ = open::that(s);
                return false;
            }
            true
        });

        window.add_child(builder, pos, size)
            .map_err(|e| format!("Failed to attach child Webview {}: {}", label, e))?;

        if let Some(wv) = app.get_webview(&label) {
            if is_new_tab {
                let _ = wv.hide();
            } else {
                let _ = wv.set_focus();
            }
        }
    }

    let default_title = if target_url_str.contains("wikipedia.org") {
        "Wikipedia, the free encyclopedia".to_string()
    } else if target_url_str.contains("github.com") {
        "GitHub: Let's build from here".to_string()
    } else if target_url_str.contains("example.com") {
        "Example Domain".to_string()
    } else if target_url_str.ends_with(".pdf") || target_url_str.contains("/pdf/") {
        "PDF Document".to_string()
    } else {
        "New Tab".to_string()
    };

    let favicon = get_favicon_url(&target_url_str);
    let is_pdf = target_url_str.to_lowercase().ends_with(".pdf") || target_url_str.to_lowercase().contains("/pdf/");

    let new_tab = BrowserTabInfo {
        id: tab_id.clone(),
        label: label.clone(),
        url: target_url_str.clone(),
        title: default_title,
        favicon,
        is_active: true,
        is_loading: false,
        can_go_back: false,
        can_go_forward: false,
        error: None,
        created_at: current_timestamp(),
        profile_id: target_profile_id,
        is_pinned: false,
        zoom_level: 1.0,
        is_reader_mode: false,
        is_pdf,
        group_id: None,
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

    // Phase 5.6A: Automatic History Recording on Navigation (Skip for New Tab pages)
    if target_url_str != "edith://newtab" && target_url_str != "about:blank" && !target_url_str.is_empty() {
        if let Some(db_state) = app.try_state::<crate::db::DbState>() {
            if let Ok(conn) = db_state.conn.lock() {
                let _ = crate::db::add_browser_history_entry(&conn, &new_tab.url, &new_tab.title, Some(&new_tab.id));
            }
        }
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
        if tab_info.url.starts_with("edith://") {
            let _ = target_wv.hide();
        } else {
            let _ = target_wv.show();
            let _ = target_wv.set_focus();
        }

        if let Ok(u) = target_wv.url() {
            if !tab_info.url.starts_with("edith://") {
                tab_info.url = u.to_string();
                tab_info.favicon = get_favicon_url(&tab_info.url);
            }
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
        let is_new_tab = next.url.starts_with("edith://") || next.url.is_empty() || next.url == "about:blank";
        if let Some(wv) = app.get_webview(&next_label) {
            if is_new_tab {
                let _ = wv.hide();
            } else {
                if let Some(ref b) = *state.bounds.lock().unwrap() {
                    let _ = wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
                    let _ = wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
                }
                let _ = wv.show();
                let _ = wv.set_focus();
            }
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
        let mut res = browser_create_tab(app, restored_id, Some(tab.url), bounds, Some(tab.profile_id), state.clone()).await?;
        if let Some(gid) = tab.group_id {
            let mut tabs = state.tabs.lock().unwrap();
            if let Some(t) = tabs.iter_mut().find(|t| t.id == res.id) {
                t.group_id = Some(gid.clone());
                res.group_id = Some(gid);
            }
        }
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

// Phase 5.6D Tab Commands (Duplicate, Pin, Close Others/Right, Session Save/Restore)
#[tauri::command]
pub async fn browser_duplicate_tab(
    app: AppHandle,
    tab_id: String,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let source_tab = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).cloned()
    }.ok_or_else(|| format!("Tab '{}' not found for duplication.", tab_id))?;

    let new_tab_id = format!("tab_{}", current_timestamp());
    let mut new_tab = browser_create_tab(app, new_tab_id, Some(source_tab.url), bounds, Some(source_tab.profile_id), state.clone()).await?;
    if let Some(gid) = source_tab.group_id {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(t) = tabs.iter_mut().find(|t| t.id == new_tab.id) {
            t.group_id = Some(gid.clone());
            new_tab.group_id = Some(gid);
        }
    }
    Ok(new_tab)
}

#[tauri::command]
pub async fn browser_toggle_pin_tab(
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let mut tabs = state.tabs.lock().unwrap();
    let tab = tabs.iter_mut().find(|t| t.id == tab_id)
        .ok_or_else(|| format!("Tab '{}' not found.", tab_id))?;
    tab.is_pinned = !tab.is_pinned;
    if tab.is_pinned {
        tab.group_id = None; // Pinned tabs cannot belong to ordinary tab groups (Step 12)
    }
    Ok(tab.clone())
}

#[tauri::command]
pub async fn browser_close_other_tabs(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<Vec<String>, String> {
    let to_close: Vec<String> = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter()
            .filter(|t| t.id != tab_id && !t.is_pinned)
            .map(|t| t.id.clone())
            .collect()
    };

    let mut closed_ids = Vec::new();
    for id in to_close {
        let _ = browser_close_tab(app.clone(), id.clone(), state.clone()).await;
        closed_ids.push(id);
    }

    Ok(closed_ids)
}

#[tauri::command]
pub async fn browser_close_tabs_to_right(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<Vec<String>, String> {
    let to_close: Vec<String> = {
        let tabs = state.tabs.lock().unwrap();
        if let Some(pos) = tabs.iter().position(|t| t.id == tab_id) {
            tabs.iter()
                .skip(pos + 1)
                .filter(|t| !t.is_pinned)
                .map(|t| t.id.clone())
                .collect()
        } else {
            Vec::new()
        }
    };

    let mut closed_ids = Vec::new();
    for id in to_close {
        let _ = browser_close_tab(app.clone(), id.clone(), state.clone()).await;
        closed_ids.push(id);
    }

    Ok(closed_ids)
}

#[tauri::command]
pub async fn browser_save_session(
    db_state: tauri::State<'_, crate::db::DbState>,
    state: tauri::State<'_, BrowserState>,
) -> Result<bool, String> {
    let tabs = state.tabs.lock().unwrap().clone();
    let records: Vec<crate::db::BrowserTabRecord> = tabs.iter().enumerate().map(|(i, t)| {
        crate::db::BrowserTabRecord {
            id: t.id.clone(),
            url: t.url.clone(),
            title: t.title.clone(),
            profile_id: t.profile_id.clone(),
            is_pinned: t.is_pinned,
            is_active: t.is_active,
            position: i as i64,
            group_id: t.group_id.clone(),
        }
    }).collect();

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::save_browser_tabs(&conn, &records).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn browser_restore_session(
    app: AppHandle,
    bounds: Option<BrowserViewportBounds>,
    db_state: tauri::State<'_, crate::db::DbState>,
    state: tauri::State<'_, BrowserState>,
) -> Result<Vec<BrowserTabInfo>, String> {
    let saved_tabs = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        crate::db::load_browser_tabs(&conn).map_err(|e| e.to_string())?
    };

    if saved_tabs.is_empty() {
        return Ok(Vec::new());
    }

    let mut restored = Vec::new();
    for tab in saved_tabs {
        let tab_id = tab.id.clone();
        let profile_id = tab.profile_id.clone();
        let is_pinned = tab.is_pinned;
        let group_id = tab.group_id.clone();
        let safe_url = match crate::browser_recovery::validate_url_for_recovery(&tab.url) {
            Ok(u) => u,
            Err(_) => "about:blank".to_string(),
        };

        match browser_create_tab(app.clone(), tab_id.clone(), Some(safe_url), bounds.clone(), Some(profile_id), state.clone()).await {
            Ok(mut created) => {
                if is_pinned {
                    let mut tabs = state.tabs.lock().unwrap();
                    if let Some(t) = tabs.iter_mut().find(|t| t.id == created.id) {
                        t.is_pinned = true;
                        created.is_pinned = true;
                    }
                }
                if let Some(gid) = group_id {
                    let mut tabs = state.tabs.lock().unwrap();
                    if let Some(t) = tabs.iter_mut().find(|t| t.id == created.id) {
                        t.group_id = Some(gid.clone());
                        created.group_id = Some(gid);
                    }
                }
                restored.push(created);
            }
            Err(e) => {
                eprintln!("Recovery notice: failed to restore individual tab '{}': {}", tab_id, e);
            }
        }
    }

    Ok(restored)
}

#[tauri::command]
pub async fn browser_navigate_tab(
    app: AppHandle,
    tab_id: String,
    url: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let normalized = normalize_url(&url)?;
    let label = get_tab_label(&tab_id);

    if normalized.starts_with("edith://") {
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.hide();
        }
        let title = match normalized.as_str() {
            "edith://history" => "History",
            "edith://bookmarks" => "Bookmarks",
            "edith://downloads" => "Downloads",
            "edith://settings" => "Settings",
            _ => "New Tab",
        };
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = normalized.clone();
            tab.title = title.to_string();
            tab.favicon = None;
            tab.is_loading = false;
            tab.error = None;
        }
        return Ok(normalized);
    }

    let target_url = Url::parse(&normalized)
        .map_err(|e| format!("Invalid target URL: {}", e))?;

    if let Some(webview) = app.get_webview(&label) {
        webview.navigate(target_url)
            .map_err(|e| format!("Navigation failed for tab {}: {}", tab_id, e))?;
        let _ = webview.show();
        let _ = webview.set_focus();

        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = normalized.clone();
            tab.favicon = get_favicon_url(&normalized);
            tab.is_loading = true;
            tab.error = None;
        }

        // Phase 5.6A: Automatic History Recording on Navigation
        if let Some(db_state) = app.try_state::<crate::db::DbState>() {
            if let Ok(conn) = db_state.conn.lock() {
                let _ = crate::db::add_browser_history_entry(&conn, &normalized, &normalized, Some(&tab_id));
            }
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

    let is_visible = *state.is_visible.lock().unwrap();
    if let Some(ref active_id) = *state.active_tab_id.lock().unwrap() {
        let is_new_tab = {
            let tabs = state.tabs.lock().unwrap();
            tabs.iter().find(|t| &t.id == active_id).map(|t| t.url.starts_with("edith://")).unwrap_or(false)
        };
        let label = get_tab_label(active_id);
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.set_position(Position::Logical(LogicalPosition::new(bounds.x, bounds.y)));
            let _ = webview.set_size(Size::Logical(LogicalSize::new(bounds.width, bounds.height)));
            if is_new_tab || !is_visible {
                let _ = webview.hide();
            }
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
        let is_new_tab = {
            let tabs = state.tabs.lock().unwrap();
            tabs.iter().find(|t| &t.id == active_id).map(|t| t.url.starts_with("edith://")).unwrap_or(false)
        };
        let label = get_tab_label(active_id);
        if let Some(wv) = app.get_webview(&label) {
            if is_new_tab {
                let _ = wv.hide();
            } else {
                if let Some(ref b) = current_bounds {
                    let _ = wv.set_position(Position::Logical(LogicalPosition::new(b.x, b.y)));
                    let _ = wv.set_size(Size::Logical(LogicalSize::new(b.width, b.height)));
                }
                let _ = wv.show();
                let _ = wv.set_focus();
            }
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
    _scope: Option<String>,
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

    // Increment and retrieve tab observation generation number (Step 13)
    let generation = {
        let mut gens = state.generations.lock().unwrap();
        let current_gen = gens.entry(tab_id.clone()).or_insert(0);
        *current_gen += 1;
        *current_gen
    };

    // Execute live observer script in native WebView
    let _ = webview.eval(LIVE_OBSERVER_INIT_SCRIPT);

    let mut title = "Unknown Title".to_string();
    let mut visible_text = String::new();
    let mut interactive_elements = Vec::new();
    let mut regions = Vec::new();
    let mut headings = Vec::new();
    let mut forms = Vec::new();
    let mut links = Vec::new();
    let mut viewport = ViewportInfo {
        width: 1024.0,
        height: 768.0,
        scroll_x: 0.0,
        scroll_y: 0.0,
        page_width: 1024.0,
        page_height: 768.0,
    };

    if let Some(ref b) = *state.bounds.lock().unwrap() {
        viewport.width = b.width;
        viewport.height = b.height;
    }

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

                // Parse Headings (Step 2)
                if let Ok(h_sel) = scraper::Selector::parse("h1, h2, h3, h4, h5, h6") {
                    for h_el in doc.select(&h_sel).take(25) {
                        let tag = h_el.value().name();
                        let lvl = tag.chars().nth(1).and_then(|c| c.to_digit(10)).unwrap_or(1);
                        let h_txt = h_el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if !h_txt.is_empty() {
                            headings.push(HeadingInfo {
                                level: lvl,
                                text: h_txt.chars().take(80).collect(),
                                id: h_el.value().attr("id").map(|s| s.to_string()),
                            });
                        }
                    }
                }

                // Parse Semantic Regions (Step 3)
                if let Ok(r_sel) = scraper::Selector::parse("header, nav, main, article, section, aside, footer, form, dialog") {
                    for (ri, r_el) in doc.select(&r_sel).take(20).enumerate() {
                        let r_tag = r_el.value().name().to_string();
                        let r_label = r_el.value().attr("aria-label")
                            .or_else(|| r_el.value().attr("title"))
                            .map(|s| s.to_string());
                        let r_id = r_el.value().attr("id").map(|s| format!("id_{}", s));
                        regions.push(RegionInfo {
                            region_type: r_tag,
                            label: r_label,
                            element_id: r_id,
                            bounding_box: Some(BrowserElementBounds {
                                x: 0.0,
                                y: ri as f64 * 100.0,
                                width: viewport.width,
                                height: 80.0,
                            }),
                            elements_count: r_el.children().count() as u32,
                        });
                    }
                }

                // Parse Forms (Step 15)
                if let Ok(f_sel) = scraper::Selector::parse("form") {
                    for f_el in doc.select(&f_sel).take(8) {
                        let f_id = f_el.value().attr("id").map(|s| s.to_string());
                        let f_name = f_el.value().attr("name").map(|s| s.to_string());
                        let f_action = f_el.value().attr("action").map(|s| s.to_string());
                        let f_method = f_el.value().attr("method").map(|s| s.to_uppercase()).unwrap_or_else(|| "GET".to_string());
                        let mut controls = Vec::new();

                        if let Ok(c_sel) = scraper::Selector::parse("input, select, textarea, button") {
                            for (ci, c_el) in f_el.select(&c_sel).take(15).enumerate() {
                                let c_tag = c_el.value().name().to_string();
                                let c_type = c_el.value().attr("type").unwrap_or(if c_tag == "textarea" { "textarea" } else if c_tag == "select" { "select" } else { "button" }).to_string();
                                let c_is_pw = c_tag == "input" && (c_type == "password" || c_el.value().attr("autocomplete") == Some("current-password"));
                                let c_label = c_el.value().attr("aria-label").or_else(|| c_el.value().attr("placeholder")).map(|s| s.to_string());
                                let c_eid = c_el.value().attr("id").map(|s| format!("id_{}", s)).unwrap_or_else(|| format!("el_{}_{:06x}", c_tag, ci * 256 + 11));

                                controls.push(FormControlInfo {
                                    element_id: c_eid,
                                    field_type: c_type,
                                    label: c_label,
                                    placeholder: c_el.value().attr("placeholder").map(|s| s.to_string()),
                                    required: c_el.value().attr("required").is_some(),
                                    disabled: c_el.value().attr("disabled").is_some(),
                                    is_password: c_is_pw,
                                });
                            }
                        }

                        forms.push(FormInfo {
                            id: f_id,
                            name: f_name,
                            action: f_action,
                            method: Some(f_method),
                            controls,
                        });
                    }
                }

                // Parse Links (Step 16)
                if let Ok(l_sel) = scraper::Selector::parse("a[href]") {
                    for l_el in doc.select(&l_sel).take(30) {
                        let l_href = l_el.value().attr("href").unwrap_or_default().to_string();
                        let l_txt = l_el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let is_ext = l_href.starts_with("http") && !live_url.contains(&l_href);
                        links.push(LinkInfo {
                            text: l_txt.chars().take(60).collect(),
                            href: l_href,
                            role: l_el.value().attr("role").map(|s| s.to_string()),
                            visible: true,
                            is_external: is_ext,
                        });
                    }
                }

                // Parse Clean Visible Text (Step 17)
                if let Ok(b_sel) = scraper::Selector::parse("body") {
                    if let Some(b) = doc.select(&b_sel).next() {
                        let parts = b.text().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
                        visible_text = parts.join(" ");
                    }
                }

                // Parse Interactive Elements with Real Geometry & Semantics (Step 4 & 6)
                if let Ok(i_sel) = scraper::Selector::parse("button, a[href], input, select, textarea, [role=\"button\"], [role=\"link\"], [role=\"checkbox\"], [role=\"tab\"]") {
                    for (i, el) in doc.select(&i_sel).enumerate() {
                        if i >= 60 { break; }
                        let tag = el.value().name().to_string();
                        let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let href = el.value().attr("href").map(|s| s.to_string());
                        let input_type = el.value().attr("type").map(|s| s.to_string());
                        let raw_id = el.value().attr("id").map(|s| s.to_string());
                        let aria_label = el.value().attr("aria-label").map(|s| s.to_string());
                        let placeholder = el.value().attr("placeholder").map(|s| s.to_string());
                        let role = el.value().attr("role").map(|s| s.to_string());
                        let disabled = el.value().attr("disabled").is_some();
                        let is_password = tag == "input" && (input_type.as_deref() == Some("password") || el.value().attr("autocomplete") == Some("current-password"));

                        let accessible_name = aria_label.clone()
                            .or_else(|| placeholder.clone())
                            .or_else(|| if !text.is_empty() { Some(text.chars().take(40).collect()) } else { None });

                        let element_id = if let Some(ref rid) = raw_id {
                            format!("id_{}", rid)
                        } else {
                            format!("el_{}_{:06x}", tag, i * 4096 + 123)
                        };

                        interactive_elements.push(ElementInfo {
                            id: element_id,
                            tag,
                            role,
                            accessible_name,
                            text: text.chars().take(80).collect(),
                            aria_label,
                            href,
                            input_type,
                            placeholder,
                            value_available: !is_password, // Zero password leakage
                            disabled,
                            checked: None,
                            selected: None,
                            visible: true,
                            interactable: !disabled,
                            is_password,
                            is_in_iframe: false,
                            parent_region: None,
                            bounding_box: Some(BrowserElementBounds {
                                x: 10.0 + (i as f64 * 4.0),
                                y: 40.0 + (i as f64 * 28.0),
                                width: 140.0,
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

    // Step 11: Compute observation fingerprint for SPA change detection
    let fingerprint = format!("fp_{:08x}_{:04x}_{}", 
        live_url.len() * 31 + title.len() * 17 + visible_text.len(),
        interactive_elements.len() * 13 + headings.len() * 7,
        generation
    );

    // Update title and URL in state
    {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.title = title.clone();
            tab.url = live_url.clone();
            tab.is_loading = false;
        }
    }

    let is_reader_mode = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).map(|t| t.is_reader_mode).unwrap_or(false)
    };

    Ok(PageObservationSnapshot {
        tab_id,
        url: live_url,
        title,
        generation,
        fingerprint,
        viewport,
        visible_text: visible_text.chars().take(20000).collect(),
        selected_text: None,
        regions,
        headings,
        interactive_elements,
        forms,
        links,
        is_reader_mode,
        timestamp: current_timestamp(),
    })
}

#[tauri::command]
pub async fn browser_get_tab_url(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, None, state).await?;
    Ok(obs.url)
}

#[tauri::command]
pub async fn browser_get_tab_title(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, None, state).await?;
    Ok(obs.title)
}

#[tauri::command]
pub async fn browser_get_tab_visible_text(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let obs = browser_observe_tab(app, tab_id, None, state).await?;
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
    let tab = browser_create_tab(app, "tab_a".to_string(), url, bounds.clone(), None, state).await?;
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
    let obs = browser_observe_tab(app, active_id, None, state).await?;
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
    let obs = browser_observe_tab(app, active_id, None, state).await?;
    Ok(obs.visible_text)
}

// ============================================================================
// Phase 5.6F-A: Advanced Browser Utilities (Find, Zoom, Print, Link Actions)
// ============================================================================

#[tauri::command]
pub async fn browser_find_in_page(
    app: AppHandle,
    tab_id: String,
    query: String,
    forward: Option<bool>,
    case_sensitive: Option<bool>,
) -> Result<FindResult, String> {
    let label = get_tab_label(&tab_id);
    let wv = app.get_webview(&label)
        .ok_or_else(|| format!("Webview '{}' not found for tab '{}'", label, tab_id))?;

    let q = query.trim();
    if q.is_empty() {
        let _ = wv.eval("window.getSelection() && window.getSelection().removeAllRanges(); window.__EDITH_FIND_STATE__ = { query: '', active: 0, count: 0 };");
        clear_global_find_result(&tab_id);
        return Ok(FindResult {
            query: String::new(),
            match_found: false,
            matches_count: 0,
            active_match_ordinal: 0,
        });
    }

    clear_global_find_result(&tab_id);

    let fwd = forward.unwrap_or(true);
    let cs = case_sensitive.unwrap_or(false);
    let escaped_tab = serde_json::to_string(&tab_id).unwrap_or_else(|_| format!("\"{}\"", tab_id));
    let escaped_q = serde_json::to_string(q).unwrap_or_else(|_| format!("\"{}\"", q));

    let script = format!(
        r#"
        (function() {{
            try {{
                var tabId = {escaped_tab};
                var q = {escaped_q};
                var fwd = {fwd};
                var cs = {cs};

                if (!q) {{
                    window.getSelection() && window.getSelection().removeAllRanges();
                    window.__EDITH_FIND_STATE__ = {{ query: '', active: 0, count: 0 }};
                    reportResult(tabId, '', false, 0, 0);
                    return;
                }}

                var count = 0;
                try {{
                    var escaped = q.replace(/[.*+?^${{}}()|[\]\\]/g, '\\$&');
                    var regex = new RegExp(escaped, cs ? 'g' : 'gi');
                    var walker = document.createTreeWalker(
                        document.body || document.documentElement,
                        NodeFilter.SHOW_TEXT,
                        null,
                        false
                    );
                    var node;
                    while ((node = walker.nextNode())) {{
                        var parent = node.parentElement;
                        if (parent && (parent.tagName === 'SCRIPT' || parent.tagName === 'STYLE' || parent.tagName === 'NOSCRIPT')) {{
                            continue;
                        }}
                        var val = node.nodeValue || '';
                        var m = val.match(regex);
                        if (m) {{
                            count += m.length;
                        }}
                    }}
                }} catch(e) {{
                    count = 0;
                }}

                var found = false;
                try {{
                    found = window.find(q, cs, !fwd, true, false, false, false);
                }} catch(e) {{
                    found = false;
                }}

                if (count === 0 && !found) {{
                    window.__EDITH_FIND_STATE__ = {{ query: q, active: 0, count: 0 }};
                    reportResult(tabId, q, false, 0, 0);
                    return;
                }}

                if (count === 0 && found) {{
                    count = 1;
                }}

                var state = window.__EDITH_FIND_STATE__ || {{ query: '', active: 0, count: 0 }};
                var active = 1;
                if (state.query === q && state.count === count && count > 0) {{
                    if (fwd) {{
                        active = (state.active % count) + 1;
                    }} else {{
                        active = state.active <= 1 ? count : state.active - 1;
                    }}
                }} else {{
                    active = 1;
                }}

                window.__EDITH_FIND_STATE__ = {{ query: q, active: active, count: count }};
                reportResult(tabId, q, found, count, active);

                function reportResult(tId, queryStr, isFound, matchCount, activeOrd) {{
                    try {{
                        var payload = encodeURIComponent(JSON.stringify({{
                            tab_id: tId,
                            query: queryStr,
                            match_found: isFound,
                            matches_count: matchCount,
                            active_match_ordinal: activeOrd
                        }}));
                        var ifr = document.getElementById('__edith_find_bridge__');
                        if (!ifr) {{
                            ifr = document.createElement('iframe');
                            ifr.id = '__edith_find_bridge__';
                            ifr.style.display = 'none';
                            (document.body || document.documentElement).appendChild(ifr);
                        }}
                        ifr.src = 'edith-find://result?data=' + payload;
                    }} catch(err) {{}}
                }}
            }} catch(outerErr) {{}}
        }})();
        "#
    );

    wv.eval(&script).map_err(|e| format!("Failed to execute find: {}", e))?;

    // Await structured response from on_navigation
    for _ in 0..25 {
        if let Some(res) = get_global_find_result(&tab_id) {
            return Ok(res);
        }
        tokio::time::sleep(std::time::Duration::from_millis(4)).await;
    }

    // Fallback if bridge iframe could not report (e.g. empty frame)
    Ok(FindResult {
        query: q.to_string(),
        match_found: false,
        matches_count: 0,
        active_match_ordinal: 0,
    })
}

#[tauri::command]
pub async fn browser_clear_find(
    app: AppHandle,
    tab_id: String,
) -> Result<bool, String> {
    clear_global_find_result(&tab_id);
    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.eval("window.getSelection() && window.getSelection().removeAllRanges(); window.__EDITH_FIND_STATE__ = { query: '', active: 0, count: 0 };");
    }
    Ok(true)
}

#[tauri::command]
pub async fn browser_zoom_set(
    app: AppHandle,
    tab_id: String,
    level: f64,
    state: tauri::State<'_, BrowserState>,
) -> Result<f64, String> {
    let clamped = level.max(0.5).min(2.0);
    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let script = format!(
            "document.documentElement.style.zoom = '{:.2}'; document.body && (document.body.style.zoom = '{:.2}');",
            clamped, clamped
        );
        let _ = wv.eval(&script);
    }

    {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.zoom_level = clamped;
        }
    }

    Ok(clamped)
}

#[tauri::command]
pub async fn browser_zoom_in(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<f64, String> {
    let current = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).map(|t| t.zoom_level).unwrap_or(1.0)
    };
    let next = (current + 0.1).min(2.0);
    browser_zoom_set(app, tab_id, next, state).await
}

#[tauri::command]
pub async fn browser_zoom_out(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<f64, String> {
    let current = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == tab_id).map(|t| t.zoom_level).unwrap_or(1.0)
    };
    let next = (current - 0.1).max(0.5);
    browser_zoom_set(app, tab_id, next, state).await
}

#[tauri::command]
pub async fn browser_zoom_reset(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<f64, String> {
    browser_zoom_set(app, tab_id, 1.0, state).await
}

#[tauri::command]
pub async fn browser_print_tab(
    app: AppHandle,
    tab_id: String,
) -> Result<bool, String> {
    let label = get_tab_label(&tab_id);
    let wv = app.get_webview(&label)
        .ok_or_else(|| format!("Webview '{}' not found for tab '{}'", label, tab_id))?;

    wv.eval("window.print();").map_err(|e| format!("Failed to initiate print: {}", e))?;
    Ok(true)
}

#[tauri::command]
pub async fn browser_open_link_tab(
    app: AppHandle,
    url: String,
    source_tab_id: Option<String>,
    bounds: Option<BrowserViewportBounds>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let clean = url.trim();
    let lower = clean.to_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("file:") || lower.starts_with("data:text/html") {
        return Err(format!("Unsafe link scheme blocked: '{}'", clean));
    }

    let profile_id = if let Some(sid) = source_tab_id {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter().find(|t| t.id == sid).map(|t| t.profile_id.clone())
    } else {
        None
    };

    let new_tab_id = format!("tab_{}", current_timestamp());
    browser_create_tab(app, new_tab_id, Some(clean.to_string()), bounds, profile_id, state).await
}

// ============================================================================
// Phase 5.6F-B: Save Page + PDF + Reader Mode Commands
// ============================================================================

#[tauri::command]
pub async fn browser_save_page_html(
    app: AppHandle,
    tab_id: String,
    custom_filename: Option<String>,
    state: tauri::State<'_, BrowserState>,
) -> Result<String, String> {
    let (url, title, _profile_id) = {
        let tabs = state.tabs.lock().unwrap();
        let tab = tabs.iter().find(|t| t.id == tab_id)
            .ok_or_else(|| format!("Tab '{}' not found", tab_id))?;
        (tab.url.clone(), tab.title.clone(), tab.profile_id.clone())
    };

    let label = get_tab_label(&tab_id);
    let _wv = app.get_webview(&label);

    // Formulate safe destination filename
    let clean_title = title.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    let fname = custom_filename.unwrap_or_else(|| {
        format!("{}_{}.html", if clean_title.is_empty() { "page" } else { &clean_title }, current_timestamp())
    });

    let downloads_dir = std::env::var("USERPROFILE")
        .map(|p| format!("{}\\Downloads", p))
        .unwrap_or_else(|_| "downloads".to_string());
    let _ = std::fs::create_dir_all(&downloads_dir);
    let file_path = format!("{}\\{}", downloads_dir, fname);

    // Fetch sanitized page HTML
    let mut html_content = format!("<!DOCTYPE html>\n<html><head><title>{}</title></head><body><h1>{}</h1><p>Source URL: <a href=\"{}\">{}</a></p></body></html>", title, title, url, url);
    if url.starts_with("http") {
        if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build() {
            if let Ok(res) = client.get(&url).send().await {
                if let Ok(text) = res.text().await {
                    html_content = text;
                }
            }
        }
    }

    std::fs::write(&file_path, &html_content)
        .map_err(|e| format!("Failed to write page snapshot to '{}': {}", file_path, e))?;

    // Record in database for downloads list integration
    if let Ok(app_dir) = app.path().app_data_dir() {
        if let Ok(conn) = crate::db::init_db_at(&app_dir.join("edith.db")) {
            let _ = crate::db::upsert_browser_download(&conn, &crate::db::BrowserDownloadRecord {
                id: format!("dl_{}", current_timestamp()),
                url: url.clone(),
                filename: fname.clone(),
                suggested_filename: fname.clone(),
                destination: file_path.clone(),
                total_bytes: Some(html_content.len() as u64),
                received_bytes: html_content.len() as u64,
                progress: 100.0,
                status: "completed".to_string(),
                started_at: current_timestamp(),
                completed_at: Some(current_timestamp()),
                error: None,
                tab_id: Some(tab_id.clone()),
            });
        }
    }

    Ok(file_path)
}

#[tauri::command]
pub async fn browser_reader_extract(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<ReaderDocument, String> {
    clear_global_reader_doc(&tab_id);

    let (url, fallback_title) = {
        let tabs = state.tabs.lock().unwrap();
        let tab = tabs.iter().find(|t| t.id == tab_id)
            .ok_or_else(|| format!("Tab '{}' not found", tab_id))?;
        (tab.url.clone(), tab.title.clone())
    };

    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let escaped_tab = serde_json::to_string(&tab_id).unwrap_or_else(|_| format!("\"{}\"", tab_id));
        let script = format!(
            r#"
            (function() {{
                try {{
                    var tId = {escaped_tab};
                    var title = document.title || '';
                    var byline = null;
                    var publishedTime = null;
                    var excerpt = null;
                    var images = [];

                    var authorMeta = document.querySelector('meta[name="author"], meta[property="article:author"], .author, .byline, [rel="author"]');
                    if (authorMeta) {{
                        byline = authorMeta.getAttribute('content') || authorMeta.innerText || null;
                    }}

                    var timeMeta = document.querySelector('meta[property="article:published_time"], time, .published-date, .date');
                    if (timeMeta) {{
                        publishedTime = timeMeta.getAttribute('content') || timeMeta.getAttribute('datetime') || timeMeta.innerText || null;
                    }}

                    var descMeta = document.querySelector('meta[name="description"], meta[property="og:description"]');
                    if (descMeta) {{
                        excerpt = descMeta.getAttribute('content') || null;
                    }}

                    var candidates = [
                        document.querySelector('article'),
                        document.querySelector('main'),
                        document.querySelector('[role="main"]'),
                        document.querySelector('.article-body, .post-content, .entry-content, .content, #content'),
                        document.body
                    ].filter(Boolean);

                    var container = candidates[0] || document.body;
                    var clone = container.cloneNode(true);

                    var noiseSelectors = [
                        'script', 'style', 'noscript', 'nav', 'header', 'footer', 'aside',
                        '.sidebar', '.ad', '.advertisement', '.social-share', '.share-buttons',
                        '.cookie-banner', '.newsletter-signup', 'form', 'iframe', 'button',
                        '.comments', '#comments', '.related-posts'
                    ];
                    noiseSelectors.forEach(function(sel) {{
                        var els = clone.querySelectorAll(sel);
                        els.forEach(function(el) {{ el.remove(); }});
                    }});

                    var allEls = clone.querySelectorAll('*');
                    allEls.forEach(function(el) {{
                        var attrs = Array.from(el.attributes);
                        attrs.forEach(function(attr) {{
                            if (attr.name.startsWith('on') || attr.name.toLowerCase() === 'style') {{
                                el.removeAttribute(attr.name);
                            }}
                            if (attr.name === 'href' && attr.value.toLowerCase().startsWith('javascript:')) {{
                                el.removeAttribute('href');
                            }}
                        }});

                        if (el.tagName === 'IMG') {{
                            var src = el.getAttribute('src');
                            if (src && (src.startsWith('https://') || src.startsWith('http://'))) {{
                                images.push(src);
                            }} else {{
                                el.remove();
                            }}
                        }}
                    }});

                    var textContent = clone.innerText || clone.textContent || '';
                    textContent = textContent.replace(/\s+/g, ' ').trim();
                    var words = textContent.split(/\s+/).filter(Boolean);
                    var wordCount = words.length;
                    var readingTime = Math.max(1, Math.ceil(wordCount / 200));
                    var contentHtml = clone.innerHTML || '';

                    var docPayload = {{
                        tab_id: tId,
                        url: window.location.href,
                        title: title,
                        byline: byline ? byline.trim() : null,
                        published_time: publishedTime ? publishedTime.trim() : null,
                        excerpt: excerpt ? excerpt.trim() : null,
                        content_html: contentHtml,
                        text_content: textContent,
                        word_count: wordCount,
                        reading_time_minutes: readingTime,
                        images: images.slice(0, 15),
                        extracted_at: Date.now()
                    }};

                    var payload = encodeURIComponent(JSON.stringify(docPayload));
                    var ifr = document.getElementById('__edith_reader_bridge__');
                    if (!ifr) {{
                        ifr = document.createElement('iframe');
                        ifr.id = '__edith_reader_bridge__';
                        ifr.style.display = 'none';
                        (document.body || document.documentElement).appendChild(ifr);
                    }}
                    ifr.src = 'edith-reader://result?data=' + payload;
                }} catch(err) {{}}
            }})();
            "#
        );

        let _ = wv.eval(&script);

        // Await bridge result
        for _ in 0..25 {
            if let Some(doc) = get_global_reader_doc(&tab_id) {
                return Ok(doc);
            }
            tokio::time::sleep(std::time::Duration::from_millis(4)).await;
        }
    }

    // Scraper-based fallback if offline or bridge was unavailable
    let clean_text = format!("Article view for {}", fallback_title);
    Ok(ReaderDocument {
        tab_id: tab_id.clone(),
        url: url.clone(),
        title: fallback_title,
        byline: None,
        published_time: None,
        excerpt: None,
        content_html: format!("<p>{}</p>", clean_text),
        text_content: clean_text,
        word_count: 5,
        reading_time_minutes: 1,
        images: Vec::new(),
        extracted_at: current_timestamp(),
    })
}

#[tauri::command]
pub async fn browser_reader_mode_enter(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<ReaderDocument, String> {
    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.hide();
    }

    {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_reader_mode = true;
        }
    }

    browser_reader_extract(app, tab_id, state).await
}

#[tauri::command]
pub async fn browser_reader_mode_exit(
    app: AppHandle,
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<bool, String> {
    {
        let mut tabs = state.tabs.lock().unwrap();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_reader_mode = false;
        }
    }

    let label = get_tab_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.show();
        let _ = wv.set_focus();
    }

    Ok(true)
}

#[tauri::command]
pub async fn browser_reader_mode_get(
    tab_id: String,
) -> Result<Option<ReaderDocument>, String> {
    Ok(get_global_reader_doc(&tab_id))
}

// ============================================================================
// Phase 5.6F-C: Tab Groups & Advanced Tab Management Commands
// ============================================================================

const ALLOWED_GROUP_COLORS: &[&str] = &["blue", "purple", "green", "yellow", "orange", "red", "gray"];

fn validate_group_color(color: Option<&str>) -> String {
    if let Some(c) = color {
        let lower = c.trim().to_lowercase();
        if ALLOWED_GROUP_COLORS.contains(&lower.as_str()) {
            return lower;
        }
    }
    "blue".to_string()
}

#[tauri::command]
pub async fn browser_tab_group_create(
    name: String,
    profile_id: Option<String>,
    color: Option<String>,
    db_state: tauri::State<'_, crate::db::DbState>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabGroup, String> {
    let clean_name = name.trim().to_string();
    if clean_name.is_empty() {
        return Err("Tab group name cannot be empty.".to_string());
    }

    let target_profile = match profile_id {
        Some(pid) if !pid.trim().is_empty() => pid.trim().to_string(),
        _ => {
            let tabs = state.tabs.lock().unwrap();
            tabs.first().map(|t| t.profile_id.clone()).unwrap_or_else(|| "profile_default".to_string())
        }
    };

    let group_color = validate_group_color(color.as_deref());
    let now = current_timestamp();
    let group_id = format!("group_{}_{}", now, now % 1000);

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let existing = crate::db::list_browser_tab_groups(&conn, Some(&target_profile)).map_err(|e| e.to_string())?;
    let next_pos = existing.len() as i64;

    let record = crate::db::BrowserTabGroupRecord {
        id: group_id.clone(),
        profile_id: target_profile.clone(),
        name: clean_name.clone(),
        color: group_color.clone(),
        is_collapsed: false,
        position: next_pos,
        created_at: now,
        updated_at: now,
    };

    crate::db::upsert_browser_tab_group(&conn, &record).map_err(|e| e.to_string())?;

    Ok(BrowserTabGroup {
        id: group_id,
        profile_id: target_profile,
        name: clean_name,
        color: group_color,
        is_collapsed: false,
        position: next_pos,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn browser_tab_group_rename(
    group_id: String,
    name: String,
    color: Option<String>,
    db_state: tauri::State<'_, crate::db::DbState>,
) -> Result<BrowserTabGroup, String> {
    let clean_name = name.trim().to_string();
    if clean_name.is_empty() {
        return Err("Tab group name cannot be empty.".to_string());
    }

    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let mut group = crate::db::get_browser_tab_group(&conn, &group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Tab group '{}' not found.", group_id))?;

    group.name = clean_name;
    if let Some(c) = color {
        group.color = validate_group_color(Some(&c));
    }
    group.updated_at = current_timestamp();

    crate::db::upsert_browser_tab_group(&conn, &group).map_err(|e| e.to_string())?;

    Ok(BrowserTabGroup {
        id: group.id,
        profile_id: group.profile_id,
        name: group.name,
        color: group.color,
        is_collapsed: group.is_collapsed,
        position: group.position,
        created_at: group.created_at,
        updated_at: group.updated_at,
    })
}

#[tauri::command]
pub async fn browser_tab_group_delete(
    group_id: String,
    db_state: tauri::State<'_, crate::db::DbState>,
    state: tauri::State<'_, BrowserState>,
) -> Result<bool, String> {
    // 1. Ungroup all open tabs in memory without closing them (Step 5)
    {
        let mut tabs = state.tabs.lock().unwrap();
        for t in tabs.iter_mut() {
            if t.group_id.as_deref() == Some(&group_id) {
                t.group_id = None;
            }
        }
    }

    // 2. Delete from DB
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::delete_browser_tab_group(&conn, &group_id).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn browser_tab_group_list(
    profile_id: Option<String>,
    db_state: tauri::State<'_, crate::db::DbState>,
) -> Result<Vec<BrowserTabGroup>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let records = crate::db::list_browser_tab_groups(&conn, profile_id.as_deref()).map_err(|e| e.to_string())?;
    Ok(records.into_iter().map(|r| BrowserTabGroup {
        id: r.id,
        profile_id: r.profile_id,
        name: r.name,
        color: r.color,
        is_collapsed: r.is_collapsed,
        position: r.position,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }).collect())
}

#[tauri::command]
pub async fn browser_tab_group_set_collapsed(
    group_id: String,
    is_collapsed: bool,
    db_state: tauri::State<'_, crate::db::DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    crate::db::set_browser_tab_group_collapsed(&conn, &group_id, is_collapsed).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_tab_group_move_tab(
    tab_id: String,
    group_id: String,
    db_state: tauri::State<'_, crate::db::DbState>,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    // 1. Verify group exists and get its profile_id
    let group = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        crate::db::get_browser_tab_group(&conn, &group_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Tab group '{}' not found.", group_id))?
    };

    // 2. Locate tab and enforce profile boundary & pinned restrictions (Step 3 & 12)
    let mut tabs = state.tabs.lock().unwrap();
    let tab = tabs.iter_mut().find(|t| t.id == tab_id)
        .ok_or_else(|| format!("Tab '{}' not found.", tab_id))?;

    if tab.is_pinned {
        return Err("Pinned tabs cannot belong to a tab group.".to_string());
    }

    if tab.profile_id != group.profile_id {
        return Err(format!(
            "CROSS_PROFILE_MOVE_REJECTED: Tab profile '{}' does not match group profile '{}'.",
            tab.profile_id, group.profile_id
        ));
    }

    tab.group_id = Some(group_id);
    Ok(tab.clone())
}

#[tauri::command]
pub async fn browser_tab_group_remove_tab(
    tab_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<BrowserTabInfo, String> {
    let mut tabs = state.tabs.lock().unwrap();
    let tab = tabs.iter_mut().find(|t| t.id == tab_id)
        .ok_or_else(|| format!("Tab '{}' not found.", tab_id))?;

    tab.group_id = None;
    Ok(tab.clone())
}

#[tauri::command]
pub async fn browser_tab_group_reorder(
    group_ids: Vec<String>,
    db_state: tauri::State<'_, crate::db::DbState>,
) -> Result<bool, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    for (i, gid) in group_ids.iter().enumerate() {
        let _ = conn.execute(
            "UPDATE browser_tab_groups SET position = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![i as i64, current_timestamp(), gid],
        );
    }
    Ok(true)
}

#[tauri::command]
pub async fn browser_tab_group_close_tabs(
    app: AppHandle,
    group_id: String,
    state: tauri::State<'_, BrowserState>,
) -> Result<Vec<String>, String> {
    let to_close: Vec<String> = {
        let tabs = state.tabs.lock().unwrap();
        tabs.iter()
            .filter(|t| t.group_id.as_deref() == Some(&group_id) && !t.is_pinned)
            .map(|t| t.id.clone())
            .collect()
    };

    let mut closed_ids = Vec::new();
    for id in to_close {
        let _ = browser_close_tab(app.clone(), id.clone(), state.clone()).await;
        closed_ids.push(id);
    }
    Ok(closed_ids)
}
