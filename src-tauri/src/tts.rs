use edge_tts_rust::{EdgeTtsClient, SpeakOptions};
use lazy_static::lazy_static;
use regex::Regex;
use rodio::Decoder;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex as StdMutex;
use std::thread;
use std::num::{NonZeroU16, NonZeroU32};

lazy_static! {
    static ref AUDIO_SENDER: StdMutex<Option<Sender<AudioCommand>>> = StdMutex::new(None);
    // static ref KOKORO_ENGINE: TokioMutex<Option<(String, String, TtsEngine)>> = TokioMutex::new(None);
}

#[allow(dead_code)]
enum AudioCommand {
    PlayRawPcm {
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    },
    PlayEncodedBytes(Vec<u8>),
    Stop,
}

fn ensure_audio_thread() {
    let mut sender = AUDIO_SENDER.lock().unwrap();
    if sender.is_none() {
        println!("[AUDIO THREAD] Initializing audio thread...");
        let (tx, rx) = channel::<AudioCommand>();
        *sender = Some(tx);
        thread::spawn(move || {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(handle) => {
                    println!("[AUDIO THREAD] Audio device opened successfully");
                    let mut current_player: Option<rodio::Player> = None;
                    for cmd in rx {
                        match cmd {
                            AudioCommand::PlayRawPcm { samples, channels, sample_rate } => {
                                println!("[AUDIO THREAD] PlayRawPcm command received ({} samples, {}Hz)", samples.len(), sample_rate);
                                let player = rodio::Player::connect_new(&handle.mixer());
                                player.set_volume(1.0);
                                let ch = NonZeroU16::new(channels).unwrap_or(NonZeroU16::new(1).unwrap());
                                let sr = NonZeroU32::new(sample_rate).unwrap_or(NonZeroU32::new(24000).unwrap());
                                let source = rodio::buffer::SamplesBuffer::new(ch, sr, samples);
                                player.append(source);
                                player.play();
                                current_player = Some(player);
                                println!("[AUDIO THREAD] Audio appended to player, starting playback...");
                            }
                            AudioCommand::PlayEncodedBytes(bytes) => {
                                println!("[AUDIO THREAD] PlayEncodedBytes command received ({} bytes)", bytes.len());
                                let cursor = std::io::Cursor::new(bytes);
                                match Decoder::new(cursor) {
                                    Ok(source) => {
                                        println!("[AUDIO THREAD] Decoder ready");
                                        let player = rodio::Player::connect_new(&handle.mixer());
                                        player.set_volume(1.0);
                                        player.append(source);
                                        player.play();
                                        current_player = Some(player);
                                        println!("[AUDIO THREAD] Audio appended to player, starting playback...");
                                    }
                                    Err(e) => {
                                        println!("[AUDIO THREAD] Decoder error: {}", e);
                                    }
                                }
                            }
                            AudioCommand::Stop => {
                                println!("[AUDIO THREAD] Stop command received");
                                if let Some(player) = current_player.take() {
                                    player.stop();
                                    println!("[AUDIO THREAD] Player stopped");
                                }
                            }
                        }
                    }
                    println!("[AUDIO THREAD] Channel closed, audio thread exiting");
                }
                Err(e) => {
                    println!("[AUDIO THREAD] CRITICAL: Failed to open audio device: {}", e);
                    println!("[AUDIO THREAD] No audio output will work!");
                }
            }
        });
        println!("[AUDIO THREAD] Audio thread spawned");
    } else {
        println!("[AUDIO THREAD] Audio thread already exists");
    }
}

#[tauri::command]
pub async fn tts_speak(text: String, voice: Option<String>) -> Result<String, String> {
    println!("[CLOUD TTS] Starting Azure EdgeTTS synthesis...");
    println!("  Text: \"{}\"", text);
    println!("  Voice: {:?}", voice);
    
    ensure_audio_thread();
    
    let client = EdgeTtsClient::new().map_err(|e| {
        let err_msg = format!("EdgeTTS client creation failed: {}", e);
        println!("{}", err_msg);
        err_msg
    })?;
    
    let voice_name = voice.unwrap_or_else(|| "hi-IN-SwaraNeural".to_string());
    let voice_name = if voice_name.is_empty() { "hi-IN-SwaraNeural".to_string() } else { voice_name };
    println!("[CLOUD TTS] Using voice: {}", voice_name);

    let options = SpeakOptions {
        voice: voice_name,
        ..Default::default()
    };
    
    let re = Regex::new(r"[*`_~#]").unwrap();
    let clean_text = re.replace_all(&text, "").to_string();
    println!("[CLOUD TTS] Cleaned text: \"{}\"", clean_text);
    
    let res = client.synthesize(clean_text, options).await.map_err(|e| {
        let err_msg = format!("EdgeTTS synthesis failed: {}", e);
        println!("{}", err_msg);
        err_msg
    })?;
    
    println!("[CLOUD TTS] Synthesis OK ({} bytes)", res.audio.len());
    
    if let Some(sender) = AUDIO_SENDER.lock().unwrap().as_ref() {
        let _ = sender.send(AudioCommand::PlayEncodedBytes(res.audio.clone()));
    }
    
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &res.audio);
    println!("[CLOUD TTS] Done!");
    Ok(b64)
}

#[tauri::command]
pub async fn local_tts_speak(text: String, voice: String, model_name: String) -> Result<(), String> {
    ensure_audio_thread();
    
    println!("[KOKORO TTS] Starting synthesis...");
    println!("  Text: \"{}\"", text);
    println!("  Voice: {}", voice);
    println!("  Model: {}", model_name);
    
    let base_dir = get_base_dir();
    let m_name = if model_name.is_empty() { "kokoro-v1.0.int8.onnx".to_string() } else { model_name };
    let _v_name = if voice.is_empty() { "af_sky".to_string() } else { voice };
    
    let models_dir = base_dir.join("AI Engines").join("Kokoro").join("models");
    let voices_dir = base_dir.join("AI Engines").join("Kokoro").join("voices");
    
    std::fs::create_dir_all(&models_dir).unwrap_or_default();
    std::fs::create_dir_all(&voices_dir).unwrap_or_default();
    
    let model_path = models_dir.join(&m_name);
    let mut voices_bin_path = voices_dir.join("voices-v1.0.bin");
    if let Ok(entries) = std::fs::read_dir(&voices_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().ends_with(".bin") {
                voices_bin_path = entry.path();
                break;
            }
        }
    }
    
    println!("[KOKORO TTS] Checking files...");
    println!("  Model path: {}", model_path.to_string_lossy());
    println!("  Voice path: {}", voices_bin_path.to_string_lossy());
    
    if !model_path.exists() {
        let err_msg = format!("Model file NOT found: {}", model_path.to_string_lossy());
        println!("{}", err_msg);
        return Err(err_msg);
    }
    if !voices_bin_path.exists() {
        let err_msg = format!("Voices .bin file NOT found: {}", voices_bin_path.to_string_lossy());
        println!("{}", err_msg);
        return Err(err_msg);
    }
    
    println!("Files found!");

    /*
    let mut engine_guard = KOKORO_ENGINE.lock().await;
    let reload_needed = match &*engine_guard {
        Some((curr_model, _, _)) => curr_model != &m_name,
        None => true,
    };
    
    if reload_needed {
        println!("Loading Kokoro engine...");
        let new_engine = TtsEngine::with_paths(&model_path.to_string_lossy(), &voices_bin_path.to_string_lossy())
            .await
            .map_err(|e| {
                let err_msg = format!("Engine load error: {}", e);
                println!("{}", err_msg);
                err_msg
            })?;
        println!("Engine loaded!");
        *engine_guard = Some((m_name.clone(), voices_bin_path.to_string_lossy().to_string(), new_engine));
    } else {
        println!("Reusing existing engine");
    }
    
    let re = Regex::new(r"[*`_~#]").unwrap();
    let clean_text = re.replace_all(&text, "").to_string();
    
    if let Some((_, _, engine)) = engine_guard.as_mut() {
        let split_regex = Regex::new(r"[.!?,\n]+").unwrap();
        let chunks: Vec<&str> = split_regex.split(&clean_text)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        let chunks = if chunks.is_empty() && !clean_text.trim().is_empty() {
            vec![clean_text.as_str()]
        } else {
            chunks
        };
        
        println!("Split into {} chunks", chunks.len());

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.trim().is_empty() { continue; }
            
            println!("  [{}/{}] Synthesizing: \"{}\"", i+1, chunks.len(), chunk);
            
            match engine.synthesize_with_options(chunk, Some(&v_name), 1.0, 1.0, Some("en")) {
                Ok(audio_data) => {
                    println!("    Synthesis OK ({} samples)", audio_data.len());
                    
                    match AUDIO_SENDER.lock().unwrap().as_ref() {
                        Some(sender) => {
                            match sender.send(AudioCommand::PlayRawPcm {
                                samples: audio_data,
                                channels: 1,
                                sample_rate: 24000,
                            }) {
                                Ok(_) => println!("    Sent to audio player"),
                                Err(e) => println!("    Failed to send to player: {}", e),
                            }
                        },
                        None => println!("    No audio sender available!"),
                    }
                },
                Err(e) => println!("    Synthesis failed: {}", e),
            }
        }
    } else {
        println!("Engine guard is empty!");
        return Err("Engine not available".to_string());
    }
    
    println!("[KOKORO TTS] Done!");
    */
    println!("[KOKORO TTS] Local TTS is currently disabled to speed up app build.");
    Err("Local Kokoro TTS is currently disabled. Enable kokoro-micro in Cargo.toml and uncomment code in tts.rs to use local TTS.".to_string())
}

#[tauri::command]
fn get_base_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    if dir.ends_with("src-tauri") {
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        }
    }
    dir
}

#[tauri::command]
pub fn get_kokoro_models() -> Result<Vec<String>, String> {
    println!("📦 [KOKORO] Scanning directory for models (not loading into RAM)...");
    let base_dir = get_base_dir();
    let models_dir = base_dir.join("AI Engines").join("Kokoro").join("models");
    println!("📦 [KOKORO] Models directory: {}", models_dir.to_string_lossy());
    let mut models = Vec::new();
    std::fs::create_dir_all(&models_dir).unwrap_or_default();
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".onnx") {
                        println!("📦 [KOKORO] Found model: {}", file_name);
                        models.push(file_name);
                    }
                }
            }
        }
    }
    println!("📦 [KOKORO] Total models found: {}", models.len());
    Ok(models)
}

#[tauri::command]
pub async fn tts_stop() -> Result<(), String> {
    println!("⏹️ [AUDIO] Stop command received");
    if let Some(sender) = AUDIO_SENDER.lock().unwrap().as_ref() {
        match sender.send(AudioCommand::Stop) {
            Ok(_) => println!("⏹️ [AUDIO] ✅ Stop command sent"),
            Err(e) => println!("⏹️ [AUDIO] ❌ Failed to send stop command: {}", e),
        }
    } else {
        println!("⏹️ [AUDIO] ❌ No audio sender available!");
    }
    Ok(())
}

#[tauri::command]
pub async fn tts_set_voice(_voice: String) -> Result<(), String> {
    Ok(())
}
