use tauri::command;
use screenshots::Screen;
use base64::{Engine as _, engine::general_purpose::STANDARD as base64_standard};
use std::io::Cursor;

#[command]
pub async fn take_screenshot_cmd() -> Result<String, String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    
    if let Some(screen) = screens.first() {
        let capture = screen.capture().map_err(|e| e.to_string())?;
        
        // Convert the RGBA image buffer to PNG bytes using image crate
        let rgba_data = capture.into_raw();
        let (w, h) = (screen.display_info.width, screen.display_info.height);
        
        let img = image::RgbaImage::from_raw(w, h, rgba_data)
            .ok_or("Failed to create RgbaImage")?;
        
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let mut buf = Cursor::new(Vec::new());
        dyn_img.write_to(&mut buf, image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?;
        
        let base64_str = base64_standard.encode(buf.into_inner());
        Ok(format!("data:image/jpeg;base64,{}", base64_str))
    } else {
        Err("No screens found".to_string())
    }
}
