use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherResult {
    pub temperature: f64,
    pub weather_code: i32,
    pub condition: String,
}

// Map WMO Weather codes to simple conditions
fn get_condition(code: i32) -> String {
    match code {
        0 => "Clear sky".to_string(),
        1 | 2 | 3 => "Partly cloudy".to_string(),
        45 | 48 => "Fog".to_string(),
        51 | 53 | 55 => "Drizzle".to_string(),
        61 | 63 | 65 => "Rain".to_string(),
        71 | 73 | 75 => "Snow".to_string(),
        80 | 81 | 82 => "Rain showers".to_string(),
        95 | 96 | 99 => "Thunderstorm".to_string(),
        _ => "Unknown".to_string(),
    }
}

#[tauri::command]
pub async fn get_weather(lat: f64, lon: f64) -> Result<WeatherResult, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code&timezone=auto",
        lat, lon
    );
    
    let resp = reqwest::get(&url).await.map_err(|e| format!("Failed to fetch weather: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let current = json.get("current").ok_or("No current data found")?;
    let temp = current.get("temperature_2m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let code = current.get("weather_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    
    Ok(WeatherResult {
        temperature: temp,
        weather_code: code,
        condition: get_condition(code),
    })
}
