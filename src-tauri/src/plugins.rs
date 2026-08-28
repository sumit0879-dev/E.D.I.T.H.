/// plugins.rs — E.D.I.T.H. Plugin System (Rust)

use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use tauri::command;
use crate::db::{self, DbState};
use tauri::State;

// ── Plugin definitions (compile-time) ─────────────────────────────────────────
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PluginDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

const PLUGIN_DEFS: &[PluginDef] = &[
    PluginDef {
        id: "web_search",
        name: "Web Search",
        description: "DuckDuckGo se web search karo. 'search [query]' bolke trigger hota hai.",
        category: "utility",
    },
    PluginDef {
        id: "media_player",
        name: "Media Player",
        description: "YouTube par videos search aur play karo. 'play [song/video]' bolke trigger hota hai.",
        category: "media",
    },
    PluginDef {
        id: "app_launcher",
        name: "App Launcher",
        description: "System apps aur custom apps ko launch karo. 'open [app name]' bolke trigger hota hai.",
        category: "system",
    },
    PluginDef {
        id: "system_control",
        name: "System Control",
        description: "Volume up/down, mute jaise system controls. 'volume up', 'mute' bolke trigger hota hai.",
        category: "system",
    },
    PluginDef {
        id: "terminal",
        name: "Terminal",
        description: "System terminal commands execute karo. 'cmd [command]' ya 'terminal [command]' bolke trigger hota hai.",
        category: "system",
    },
    PluginDef {
        id: "dev_agent",
        name: "Dev Agent (E.D.I.T.H.)",
        description: "AI coding assistant jo project files read/write/edit kar sakta hai. DevPanel se access karo.",
        category: "developer",
    },
    PluginDef {
        id: "whatsapp",
        name: "WhatsApp Integration",
        description: "Send WhatsApp messages via deep link. 'whatsapp [number] [message]' bolke trigger hota hai.",
        category: "social",
    },
    PluginDef {
        id: "gmail",
        name: "Gmail/Email Integration",
        description: "Send emails via default mail client. 'email [address] [message]' bolke trigger hota hai.",
        category: "social",
    },
];

// ── Built-in apps (compile-time) — Python _BUILTIN_APPS se convert ───────────
#[derive(serde::Serialize, Clone)]
pub struct BuiltinApp {
    pub id: &'static str,
    pub name: &'static str,
    pub path: &'static str,
    pub keywords: &'static str,
    pub builtin: bool,
}

pub const BUILTIN_APPS: &[BuiltinApp] = &[
    BuiltinApp { id: "bi_notepad",  name: "notepad",         path: "notepad.exe",  keywords: "notepad,text,editor",          builtin: true },
    BuiltinApp { id: "bi_calc",     name: "calculator",      path: "calc.exe",     keywords: "calculator,calc,math",         builtin: true },
    BuiltinApp { id: "bi_chrome",   name: "chrome",          path: "chrome",       keywords: "chrome,browser,google",        builtin: true },
    BuiltinApp { id: "bi_explorer", name: "file explorer",   path: "explorer.exe", keywords: "explorer,files,folder",        builtin: true },
    BuiltinApp { id: "bi_cmd",      name: "command prompt",  path: "cmd.exe",      keywords: "cmd,command,prompt,terminal",  builtin: true },
    BuiltinApp { id: "bi_taskmgr",  name: "task manager",    path: "taskmgr.exe",  keywords: "task,manager,process",         builtin: true },
    BuiltinApp { id: "bi_settings", name: "settings",        path: "ms-settings:", keywords: "settings,control",            builtin: true },
    BuiltinApp { id: "bi_paint",    name: "paint",           path: "mspaint.exe",  keywords: "paint,draw,art",              builtin: true },
    BuiltinApp { id: "bi_vscode",   name: "vs code",         path: "code",         keywords: "vscode,code,editor,ide",      builtin: true },
];

// ── Plugin state commands (SQLite persistent) ─────────────────────────────────

#[derive(serde::Serialize)]
pub struct PluginWithState {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub enabled: bool,
}

#[command]
pub fn get_plugins(state: State<'_, DbState>) -> Result<Vec<PluginWithState>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let saved = db::get_plugin_states(&conn).unwrap_or_default();

    let plugins = PLUGIN_DEFS
        .iter()
        .map(|p| PluginWithState {
            id: p.id.to_string(),
            name: p.name.to_string(),
            description: p.description.to_string(),
            category: p.category.to_string(),
            // Default true; saved state override karo
            enabled: *saved.get(p.id).unwrap_or(&true),
        })
        .collect();

    Ok(plugins)
}

#[command]
pub fn toggle_plugin(plugin_id: String, state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let saved = db::get_plugin_states(&conn).unwrap_or_default();

    let current = *saved.get(plugin_id.as_str()).unwrap_or(&true);
    let new_state = !current;

    db::set_plugin_state(&conn, &plugin_id, new_state).map_err(|e| e.to_string())?;
    Ok(new_state)
}

#[command]
pub fn get_builtin_apps() -> Vec<BuiltinApp> {
    BUILTIN_APPS.to_vec()
}

// ── Plugin action commands ────────────────────────────────────────────────────

#[command]
pub fn plugin_system_terminal(cmd: &str) -> String {
    match crate::security::CommandPolicy::parse_and_validate(cmd) {
        Ok((prog, args)) => {
            match crate::security::CommandPolicy::evaluate_risk(&prog, &args) {
                Ok((_risk, requires_approval)) => {
                    if requires_approval {
                        format!("Security Policy Notice: Command '{0} {1}' is a high-risk operation and requires human approval via the Dev Agent proposal workflow.", prog, args.join(" "))
                    } else {
                        let work_dir = std::env::current_dir().ok();
                        match crate::security::CommandPolicy::execute(&prog, &args, work_dir.as_deref()) {
                            Ok(res) => res.output,
                            Err(e) => format!("Execution Error: {}", e),
                        }
                    }
                }
                Err(policy_err) => format!("Security Policy Violation: {}", policy_err),
            }
        }
        Err(validation_err) => validation_err,
    }
}

#[command]
pub fn plugin_system_control(action: &str) -> String {
    let script = match action {
        "volume_up"   => "$obj = new-object -com wscript.shell; $obj.SendKeys([char]175)",
        "volume_down" => "$obj = new-object -com wscript.shell; $obj.SendKeys([char]174)",
        "mute"        => "$obj = new-object -com wscript.shell; $obj.SendKeys([char]173)",
        _ => return format!("Unknown action: {}", action),
    };

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("powershell").args(["-Command", script]).creation_flags(CREATE_NO_WINDOW).spawn();
        format!("System control '{}' executed.", action)
    }
    #[cfg(not(target_os = "windows"))]
    {
        "System control is only supported on Windows.".to_string()
    }
}

/// Resolve user-facing app name to executable/path via built-in list and custom_apps DB.
pub fn resolve_launch_path(conn: &rusqlite::Connection, app_name: &str) -> String {
    let query = app_name.trim().to_lowercase();
    if query.is_empty() {
        return app_name.to_string();
    }

    for app in BUILTIN_APPS {
        if app.name == query || keywords_match(app.keywords, &query) {
            return app.path.to_string();
        }
    }

    if let Ok(custom) = db::get_custom_apps(conn) {
        for app in custom {
            if app.name == query || keywords_match(&app.keywords, &query) {
                return app.path;
            }
        }
    }

    app_name.trim().to_string()
}

fn keywords_match(keywords: &str, query: &str) -> bool {
    keywords
        .split(',')
        .any(|k| k.trim() == query || query.contains(k.trim()))
}

#[command]
pub fn plugin_app_launcher(app_path: &str) -> String {
    match crate::security::AppLauncherPolicy::validate_and_launch(app_path, None) {
        Ok(msg) => msg,
        Err(e) => format!("Execution Error: {}", e),
    }
}

/// Web search — scraper crate se proper HTML parsing (brittle string search replace kiya)
#[command]
pub async fn plugin_web_search(query: String, api_key: Option<String>) -> String {
    let key = match api_key {
        Some(k) if !k.trim().is_empty() => k,
        _ => return "Error: Tavily API key is missing. Please set it in Settings.".to_string(),
    };

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "api_key": key,
        "query": query,
        "search_depth": "advanced",
        "include_answer": true,
        "max_results": 5
    });

    match client.post("https://api.tavily.com/search")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(answer) = json.get("answer").and_then(|a| a.as_str()) {
                    let mut results = format!("Tavily Answer: {}\n\nSources:\n", answer);
                    if let Some(results_arr) = json.get("results").and_then(|r| r.as_array()) {
                        for res in results_arr {
                            if let (Some(title), Some(url)) = (res.get("title").and_then(|t| t.as_str()), res.get("url").and_then(|u| u.as_str())) {
                                results.push_str(&format!("- [{}]({}): {}\n", title, url, res.get("content").and_then(|c| c.as_str()).unwrap_or("")));
                            }
                        }
                    }
                    return results;
                }
                return "No answer from Tavily.".to_string();
            }
            "Failed to parse Tavily response".to_string()
        },
        Err(e) => format!("Tavily API request failed: {}", e)
    }
}

#[command]
pub async fn plugin_media_player(query: String) -> String {
    let url = format!(
        "https://www.youtube.com/results?search_query={}",
        urlencoding::encode(&query)
    );

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap_or_else(|_| reqwest::Client::new());
    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(html) = resp.text().await {
            let re = regex::Regex::new(r"watch\?v=([a-zA-Z0-9_-]{11})").unwrap();
            if let Some(caps) = re.captures(&html) {
                if let Some(id) = caps.get(1) {
                    let video_url = format!("https://www.youtube.com/watch?v={}", id.as_str());
                    let _ = open::that(&video_url);
                    return format!("Playing on YouTube: {}", query);
                }
            }
        }
    }

    match open::that(&url) {
        Ok(_)  => format!("YouTube opened for: {}", query),
        Err(e) => format!("Failed to open YouTube: {}", e),
    }
}




use screenshots::Screen;
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose};

#[tauri::command]
pub async fn take_screenshot() -> Result<String, String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    let screen = screens.first().ok_or("No screen found")?;
    
    let image = screen.capture().map_err(|e| e.to_string())?;
    let mut buffer = Cursor::new(Vec::new());
    
    let img = image::RgbaImage::from_raw(image.width(), image.height(), image.into_raw())
        .ok_or("Failed to create image")?;
    
    let dyn_img = image::DynamicImage::ImageRgba8(img);
    dyn_img.write_to(&mut buffer, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    
    let b64 = general_purpose::STANDARD.encode(buffer.into_inner());
    
    Ok(format!("data:image/jpeg;base64,{}", b64))
}

#[command]
pub fn plugin_whatsapp(number: &str, message: &str) -> String {
    let mut num = number.replace(" ", "").replace("+", "").replace("-", "");
    if num.len() == 10 {
        // Assume India by default if 10 digits
        num = format!("91{}", num);
    }
    
    let url = format!("https://wa.me/{}?text={}", num, urlencoding::encode(message));
    
    if let Err(e) = open::that(&url) {
        return format!("Failed to open WhatsApp: {}", e);
    }
    
    format!("Opening WhatsApp to send message to {}", num)
}

#[command]
pub fn plugin_gmail(email: &str, message: &str) -> String {
    let url = format!("mailto:{}?subject=Message from E.D.I.T.H. AI&body={}", email, urlencoding::encode(message));
    
    if let Err(e) = open::that(&url) {
        return format!("Failed to open Mail client: {}", e);
    }
    
    format!("Opening default Mail client to email {}", email)
}
