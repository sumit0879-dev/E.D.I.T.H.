use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[tauri::command]
pub async fn minimize_all_windows() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = "(New-Object -ComObject Shell.Application).MinimizeAll()";
        Command::new("powershell")
            .args(["-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to minimize windows: {}", e))?;
        Ok("All windows minimized".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Window management is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub async fn restore_all_windows() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = "(New-Object -ComObject Shell.Application).UndoMinimizeALL()";
        Command::new("powershell")
            .args(["-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to restore windows: {}", e))?;
        Ok("Windows restored".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Window management is only supported on Windows".to_string())
    }
}
