use tauri::command;
use std::process::Command;

#[command]
pub async fn arrange_windows_cmd(action: String) -> Result<String, String> {
    let script = match action.as_str() {
        "minimize_all" => {
            // PowerShell command to minimize all windows
            "(New-Object -ComObject Shell.Application).MinimizeAll()"
        },
        "restore_all" => {
            "(New-Object -ComObject Shell.Application).UndoMinimizeAll()"
        },
        _ => return Err("Unknown window action".to_string())
    };

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to execute powershell: {}", e))?;

    if output.status.success() {
        Ok(format!("Successfully executed window action: {}", action))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
