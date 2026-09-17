//! computer_platform.rs — Platform Abstraction & Windows Win32 Desktop Adapter
//!
//! Encapsulates low-level platform APIs for screen observation, window enumeration,
//! cursor movement, mouse clicks, and keyboard inputs behind the `ComputerPlatform` trait.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformWindowInfo {
    pub handle: u64,
    pub title: String,
    pub process_name: String,
    pub pid: u32,
    pub bounds: WindowBounds,
    pub is_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformScreenMetrics {
    pub primary_width: i32,
    pub primary_height: i32,
    pub screen_count: usize,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub active_window: Option<PlatformWindowInfo>,
}

/// Abstract contract for platform-specific desktop automation capabilities.
pub trait ComputerPlatform: Send + Sync {
    fn observe_screen(&self) -> Result<PlatformScreenMetrics, String>;
    fn screenshot(&self, target: &str) -> Result<String, String>;
    fn get_active_window(&self) -> Result<Option<PlatformWindowInfo>, String>;
    fn list_windows(&self) -> Result<Vec<PlatformWindowInfo>, String>;
    fn focus_window(&self, title: &str, process_name: Option<&str>) -> Result<bool, String>;
    fn close_window(&self, title: &str) -> Result<bool, String>;
    fn move_cursor(&self, x: i32, y: i32) -> Result<(), String>;
    fn click(&self, button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String>;
    fn double_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String>;
    fn right_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String>;
    fn type_text(&self, text: &str) -> Result<(), String>;
    fn press_key(&self, key: &str) -> Result<(), String>;
    fn hotkey(&self, keys: &[String]) -> Result<(), String>;
}

// ============================================================================
// WINDOWS NATIVE PLATFORM ADAPTER
// ============================================================================

#[cfg(target_os = "windows")]
pub struct WindowsPlatformAdapter;

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct POINT {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowTextW(hwnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
    fn GetWindowThreadProcessId(hwnd: isize, lpdwProcessId: *mut u32) -> u32;
    fn EnumWindows(lpEnumFunc: unsafe extern "system" fn(isize, isize) -> i32, lParam: isize) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn SetForegroundWindow(hwnd: isize) -> i32;
    fn ShowWindow(hwnd: isize, nCmdShow: i32) -> i32;
    fn PostMessageW(hwnd: isize, msg: u32, wParam: usize, lParam: isize) -> i32;
    fn GetCursorPos(lpPoint: *mut POINT) -> i32;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn mouse_event(dwFlags: u32, dx: u32, dy: u32, dwData: u32, dwExtraInfo: usize);
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn GetWindowRect(hwnd: isize, lpRect: *mut RECT) -> i32;
    fn GetSystemMetrics(nIndex: i32) -> i32;
    fn IsIconic(hwnd: isize) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> i32;
    fn QueryFullProcessImageNameW(hProcess: isize, dwFlags: u32, lpExeName: *mut u16, lpdwSize: *mut u32) -> i32;
}

#[cfg(target_os = "windows")]
const SM_CXSCREEN: i32 = 0;
#[cfg(target_os = "windows")]
const SM_CYSCREEN: i32 = 1;
#[cfg(target_os = "windows")]
const SW_RESTORE: i32 = 9;
#[cfg(target_os = "windows")]
const WM_CLOSE: u32 = 0x0010;

#[cfg(target_os = "windows")]
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
#[cfg(target_os = "windows")]
const KEYEVENTF_KEYUP: u32 = 0x0002;

#[cfg(target_os = "windows")]
impl WindowsPlatformAdapter {
    pub fn new() -> Self {
        Self
    }

    fn get_process_name_for_pid(pid: u32) -> String {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle != 0 {
                let mut buf = [0u16; 1024];
                let mut size = buf.len() as u32;
                let res = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
                CloseHandle(handle);
                if res != 0 {
                    let path = String::from_utf16_lossy(&buf[..size as usize]);
                    if let Some(name) = std::path::Path::new(&path).file_name() {
                        return name.to_string_lossy().to_string();
                    }
                    return path;
                }
            }
        }
        format!("pid_{}", pid)
    }

    fn get_window_info_from_hwnd(hwnd: isize) -> Option<PlatformWindowInfo> {
        if hwnd == 0 {
            return None;
        }
        unsafe {
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            let title = if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            };

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let process_name = Self::get_process_name_for_pid(pid);

            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect);
            let bounds = WindowBounds {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(0),
                height: (rect.bottom - rect.top).max(0),
            };

            let is_minimized = IsIconic(hwnd) != 0;

            Some(PlatformWindowInfo {
                handle: hwnd as u64,
                title,
                process_name,
                pid,
                bounds,
                is_minimized,
            })
        }
    }

    fn key_to_vk(key: &str) -> Option<u8> {
        match key.to_lowercase().as_str() {
            "enter" | "return" => Some(0x0D),
            "tab" => Some(0x09),
            "escape" | "esc" => Some(0x1B),
            "backspace" | "back" => Some(0x08),
            "space" => Some(0x20),
            "up" => Some(0x26),
            "down" => Some(0x28),
            "left" => Some(0x25),
            "right" => Some(0x27),
            "home" => Some(0x24),
            "end" => Some(0x23),
            "pageup" => Some(0x21),
            "pagedown" => Some(0x22),
            "delete" | "del" => Some(0x2E),
            "ctrl" | "control" => Some(0x11),
            "alt" => Some(0x12),
            "shift" => Some(0x10),
            "win" | "windows" | "super" => Some(0x5B),
            "f1" => Some(0x70),
            "f2" => Some(0x71),
            "f3" => Some(0x72),
            "f4" => Some(0x73),
            "f5" => Some(0x74),
            "f6" => Some(0x75),
            "f7" => Some(0x76),
            "f8" => Some(0x77),
            "f9" => Some(0x78),
            "f10" => Some(0x79),
            "f11" => Some(0x7A),
            "f12" => Some(0x7B),
            "a" => Some(0x41),
            "b" => Some(0x42),
            "c" => Some(0x43),
            "d" => Some(0x44),
            "e" => Some(0x45),
            "f" => Some(0x46),
            "g" => Some(0x47),
            "h" => Some(0x48),
            "i" => Some(0x49),
            "j" => Some(0x4A),
            "k" => Some(0x4B),
            "l" => Some(0x4C),
            "m" => Some(0x4D),
            "n" => Some(0x4E),
            "o" => Some(0x4F),
            "p" => Some(0x50),
            "q" => Some(0x51),
            "r" => Some(0x52),
            "s" => Some(0x53),
            "t" => Some(0x54),
            "u" => Some(0x55),
            "v" => Some(0x56),
            "w" => Some(0x57),
            "x" => Some(0x58),
            "y" => Some(0x59),
            "z" => Some(0x5A),
            _ => None,
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: isize, lparam: isize) -> i32 {
    let list = &mut *(lparam as *mut Vec<PlatformWindowInfo>);
    if IsWindowVisible(hwnd) != 0 {
        if let Some(info) = WindowsPlatformAdapter::get_window_info_from_hwnd(hwnd) {
            if !info.title.trim().is_empty() && info.bounds.width > 0 && info.bounds.height > 0 {
                list.push(info);
            }
        }
    }
    1
}

#[cfg(target_os = "windows")]
impl ComputerPlatform for WindowsPlatformAdapter {
    fn observe_screen(&self) -> Result<PlatformScreenMetrics, String> {
        let (w, h) = unsafe {
            (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };
        let mut pt = POINT::default();
        unsafe {
            GetCursorPos(&mut pt);
        }
        let active_window = self.get_active_window().unwrap_or(None);
        let screen_count = screenshots::Screen::all().map(|s| s.len()).unwrap_or(1);

        Ok(PlatformScreenMetrics {
            primary_width: w,
            primary_height: h,
            screen_count,
            cursor_x: pt.x,
            cursor_y: pt.y,
            active_window,
        })
    }

    fn screenshot(&self, target: &str) -> Result<String, String> {
        let screens = screenshots::Screen::all().map_err(|e| format!("Failed to query screens: {}", e))?;
        let screen = screens.first().ok_or("No screen display detected.")?;
        let capture = screen.capture().map_err(|e| format!("Failed screen capture: {}", e))?;

        let mut img = image::RgbaImage::from_raw(capture.width(), capture.height(), capture.into_raw())
            .ok_or("Failed to construct image buffer.")?;

        if target.eq_ignore_ascii_case("active_window") {
            if let Ok(Some(win)) = self.get_active_window() {
                let x = win.bounds.x.max(0) as u32;
                let y = win.bounds.y.max(0) as u32;
                let w = (win.bounds.width as u32).min(img.width().saturating_sub(x));
                let h = (win.bounds.height as u32).min(img.height().saturating_sub(y));
                if w > 0 && h > 0 {
                    let cropped = image::imageops::crop_imm(&img, x, y, w, h).to_image();
                    img = cropped;
                }
            }
        }

        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let mut buf = Cursor::new(Vec::new());
        dyn_img.write_to(&mut buf, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Image JPEG encoding failed: {}", e))?;
        let b64 = BASE64_STANDARD.encode(buf.into_inner());
        Ok(format!("data:image/jpeg;base64,{}", b64))
    }

    fn get_active_window(&self) -> Result<Option<PlatformWindowInfo>, String> {
        let hwnd = unsafe { GetForegroundWindow() };
        Ok(Self::get_window_info_from_hwnd(hwnd))
    }

    fn list_windows(&self) -> Result<Vec<PlatformWindowInfo>, String> {
        let mut list: Vec<PlatformWindowInfo> = Vec::new();
        let ptr = &mut list as *mut Vec<PlatformWindowInfo> as isize;
        unsafe {
            EnumWindows(enum_windows_callback, ptr);
        }
        Ok(list)
    }

    fn focus_window(&self, title: &str, process_name: Option<&str>) -> Result<bool, String> {
        let windows = self.list_windows()?;
        let target = windows.into_iter().find(|w| {
            let title_match = w.title.to_lowercase().contains(&title.to_lowercase());
            if let Some(p) = process_name {
                title_match && w.process_name.to_lowercase().contains(&p.to_lowercase())
            } else {
                title_match
            }
        });

        if let Some(win) = target {
            unsafe {
                ShowWindow(win.handle as isize, SW_RESTORE);
                SetForegroundWindow(win.handle as isize);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn close_window(&self, title: &str) -> Result<bool, String> {
        let windows = self.list_windows()?;
        let target = windows.into_iter().find(|w| {
            w.title.to_lowercase().contains(&title.to_lowercase())
        });

        if let Some(win) = target {
            // Safety guard: prevent closing explorer.exe or edith-v2
            let proc_lower = win.process_name.to_lowercase();
            if proc_lower.contains("explorer") || proc_lower.contains("edith") {
                return Err(format!("Closing protected core system window '{}' is strictly prohibited.", win.process_name));
            }
            unsafe {
                PostMessageW(win.handle as isize, WM_CLOSE, 0, 0);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn move_cursor(&self, x: i32, y: i32) -> Result<(), String> {
        let (max_w, max_h) = unsafe {
            (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };
        let clamped_x = x.clamp(0, max_w);
        let clamped_y = y.clamp(0, max_h);
        unsafe {
            SetCursorPos(clamped_x, clamped_y);
        }
        Ok(())
    }

    fn click(&self, button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        if let (Some(cx), Some(cy)) = (x, y) {
            self.move_cursor(cx, cy)?;
        }
        let (down, up) = match button.to_lowercase().as_str() {
            "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        };
        unsafe {
            mouse_event(down, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(15));
            mouse_event(up, 0, 0, 0, 0);
        }
        Ok(())
    }

    fn double_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        self.click("left", x, y)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.click("left", None, None)
    }

    fn right_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        self.click("right", x, y)
    }

    fn type_text(&self, text: &str) -> Result<(), String> {
        for ch in text.chars() {
            if ch == '\n' {
                self.press_key("enter")?;
            } else if ch == '\t' {
                self.press_key("tab")?;
            } else {
                let s = ch.to_string();
                if let Some(vk) = Self::key_to_vk(&s) {
                    let is_upper = ch.is_ascii_uppercase();
                    unsafe {
                        if is_upper {
                            keybd_event(0x10, 0, 0, 0); // Shift down
                        }
                        keybd_event(vk, 0, 0, 0);
                        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
                        if is_upper {
                            keybd_event(0x10, 0, KEYEVENTF_KEYUP, 0); // Shift up
                        }
                    }
                } else {
                    // Fallback for space or punctuation
                    let vk = match ch {
                        ' ' => Some(0x20),
                        '.' => Some(0xBE),
                        ',' => Some(0xBC),
                        '-' => Some(0xBD),
                        _ => None,
                    };
                    if let Some(v) = vk {
                        unsafe {
                            keybd_event(v, 0, 0, 0);
                            keybd_event(v, 0, KEYEVENTF_KEYUP, 0);
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }

    fn press_key(&self, key: &str) -> Result<(), String> {
        let vk = Self::key_to_vk(key)
            .ok_or_else(|| format!("Unsupported keyboard key: '{}'", key))?;
        unsafe {
            keybd_event(vk, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(15));
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
        }
        Ok(())
    }

    fn hotkey(&self, keys: &[String]) -> Result<(), String> {
        let mut vks = Vec::new();
        for k in keys {
            let vk = Self::key_to_vk(k)
                .ok_or_else(|| format!("Unsupported hotkey component: '{}'", k))?;
            vks.push(vk);
        }

        unsafe {
            // Press in order
            for &vk in &vks {
                keybd_event(vk, 0, 0, 0);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            // Release in reverse order
            for &vk in vks.iter().rev() {
                keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Ok(())
    }
}

// ============================================================================
// MOCK PLATFORM ADAPTER (For Testing & Cross-Platform Builds)
// ============================================================================

pub struct MockPlatformAdapter {
    pub windows: Mutex<Vec<PlatformWindowInfo>>,
    pub cursor: Mutex<(i32, i32)>,
    pub typed_log: Mutex<Vec<String>>,
    pub key_log: Mutex<Vec<String>>,
}

impl Default for MockPlatformAdapter {
    fn default() -> Self {
        Self {
            windows: Mutex::new(vec![
                PlatformWindowInfo {
                    handle: 1001,
                    title: "Notepad".to_string(),
                    process_name: "notepad.exe".to_string(),
                    pid: 1234,
                    bounds: WindowBounds { x: 100, y: 100, width: 800, height: 600 },
                    is_minimized: false,
                },
                PlatformWindowInfo {
                    handle: 1002,
                    title: "Calculator".to_string(),
                    process_name: "calc.exe".to_string(),
                    pid: 5678,
                    bounds: WindowBounds { x: 200, y: 200, width: 400, height: 500 },
                    is_minimized: false,
                },
            ]),
            cursor: Mutex::new((100, 100)),
            typed_log: Mutex::new(Vec::new()),
            key_log: Mutex::new(Vec::new()),
        }
    }
}

impl MockPlatformAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ComputerPlatform for MockPlatformAdapter {
    fn observe_screen(&self) -> Result<PlatformScreenMetrics, String> {
        let (cx, cy) = *self.cursor.lock().unwrap();
        let win = self.windows.lock().unwrap().first().cloned();
        Ok(PlatformScreenMetrics {
            primary_width: 1920,
            primary_height: 1080,
            screen_count: 1,
            cursor_x: cx,
            cursor_y: cy,
            active_window: win,
        })
    }

    fn screenshot(&self, target: &str) -> Result<String, String> {
        Ok(format!("data:image/jpeg;base64,MOCK_SCREENSHOT_DATA_{}", target))
    }

    fn get_active_window(&self) -> Result<Option<PlatformWindowInfo>, String> {
        Ok(self.windows.lock().unwrap().first().cloned())
    }

    fn list_windows(&self) -> Result<Vec<PlatformWindowInfo>, String> {
        Ok(self.windows.lock().unwrap().clone())
    }

    fn focus_window(&self, title: &str, _process_name: Option<&str>) -> Result<bool, String> {
        let mut list = self.windows.lock().unwrap();
        if let Some(pos) = list.iter().position(|w| w.title.to_lowercase().contains(&title.to_lowercase())) {
            let win = list.remove(pos);
            list.insert(0, win);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn close_window(&self, title: &str) -> Result<bool, String> {
        let mut list = self.windows.lock().unwrap();
        if let Some(pos) = list.iter().position(|w| w.title.to_lowercase().contains(&title.to_lowercase())) {
            list.remove(pos);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn move_cursor(&self, x: i32, y: i32) -> Result<(), String> {
        let mut c = self.cursor.lock().unwrap();
        *c = (x, y);
        Ok(())
    }

    fn click(&self, button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        if let (Some(cx), Some(cy)) = (x, y) {
            self.move_cursor(cx, cy)?;
        }
        self.key_log.lock().unwrap().push(format!("click:{}", button));
        Ok(())
    }

    fn double_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        self.click("left", x, y)?;
        self.click("left", None, None)
    }

    fn right_click(&self, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
        self.click("right", x, y)
    }

    fn type_text(&self, text: &str) -> Result<(), String> {
        self.typed_log.lock().unwrap().push(text.to_string());
        Ok(())
    }

    fn press_key(&self, key: &str) -> Result<(), String> {
        self.key_log.lock().unwrap().push(key.to_string());
        Ok(())
    }

    fn hotkey(&self, keys: &[String]) -> Result<(), String> {
        self.key_log.lock().unwrap().push(keys.join("+"));
        Ok(())
    }
}
