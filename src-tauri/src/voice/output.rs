//! output.rs — Single authoritative hardware audio playback driver.
//!
//! Enforces the single-authoritative-hardware-playback-path guarantee:
//! - Hardware soundcard output is exclusively owned by this driver.
//! - Dual audio output (simultaneous Rodio and browser HTML5 Audio) is completely eliminated.
//! - Interruptions halt current playback immediately without queue bleeding.

#![allow(deprecated)]

use super::audio::AudioBuffer;
use super::errors::VoiceError;
use crate::events::VoiceSessionId;
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, Player};
use std::io::Cursor;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Hardware abstraction for playing synthesized audio.
pub trait AudioOutputDriver: Send + Sync {
    /// Dispatches an `AudioBuffer` to the hardware soundcard for playback.
    fn play(&self, buffer: AudioBuffer, session_id: VoiceSessionId) -> Result<(), VoiceError>;

    /// Dispatches raw encoded audio bytes (e.g. MP3) directly to the soundcard.
    fn play_encoded_bytes(&self, bytes: Vec<u8>, session_id: VoiceSessionId) -> Result<(), VoiceError>;

    /// Halts active playback immediately, flushes buffers, and resets player state.
    fn stop(&self) -> Result<(), VoiceError>;

    /// Temporarily pauses active playback.
    fn pause(&self) -> Result<(), VoiceError>;

    /// Resumes paused playback.
    fn resume(&self) -> Result<(), VoiceError>;

    /// Adjusts playback volume scaling (0.0 to 1.0).
    fn set_volume(&self, volume: f32) -> Result<(), VoiceError>;

    /// Returns whether audio is currently actively rendering to speakers.
    fn is_playing(&self) -> bool;

    /// Dynamically switches hardware output sink without application restart.
    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError>;

    /// Opaque identifier of the currently active output device.
    fn current_device_id(&self) -> Option<String>;

    /// Human-readable name of the currently active output device (UI-only).
    fn current_device_name(&self) -> Option<String>;
}

#[allow(dead_code)]
enum OutputCommand {
    PlayBuffer {
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
        session_id: VoiceSessionId,
    },
    PlayEncoded {
        bytes: Vec<u8>,
        session_id: VoiceSessionId,
    },
    Stop,
    Pause,
    Resume,
    SetVolume(f32),
    SwitchDevice {
        device_name: Option<String>,
        device_id: Option<String>,
    },
}

/// Production Rodio audio output driver (single authoritative sink).
pub struct RodioAudioOutputDriver {
    sender: Arc<Mutex<Option<Sender<OutputCommand>>>>,
    is_playing_flag: Arc<AtomicBool>,
    current_device_id: Arc<std::sync::RwLock<Option<String>>>,
    current_device_name: Arc<std::sync::RwLock<Option<String>>>,
}

impl RodioAudioOutputDriver {
    pub fn new() -> Self {
        let driver = Self {
            sender: Arc::new(Mutex::new(None)),
            is_playing_flag: Arc::new(AtomicBool::new(false)),
            current_device_id: Arc::new(std::sync::RwLock::new(None)),
            current_device_name: Arc::new(std::sync::RwLock::new(None)),
        };
        driver.ensure_started();
        driver
    }

    fn ensure_started(&self) {
        let mut lock = self.sender.lock().unwrap();
        if lock.is_some() {
            return;
        }

        let (tx, rx) = channel::<OutputCommand>();
        *lock = Some(tx);

        let playing_flag = Arc::clone(&self.is_playing_flag);

        thread::spawn(move || {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut handle) => {
                    let mut current_player: Option<Player> = None;
                    let mut current_volume: f32 = 1.0;

                    for cmd in rx {
                        match cmd {
                            OutputCommand::PlayBuffer {
                                samples,
                                channels,
                                sample_rate,
                                session_id: _,
                            } => {
                                // Stop and drop any active previous playback immediately
                                if let Some(p) = current_player.take() {
                                    p.stop();
                                }

                                let player = Player::connect_new(&handle.mixer());
                                player.set_volume(current_volume);

                                let ch = NonZeroU16::new(channels)
                                    .unwrap_or_else(|| NonZeroU16::new(1).unwrap());
                                let sr = NonZeroU32::new(sample_rate)
                                    .unwrap_or_else(|| NonZeroU32::new(24000).unwrap());

                                let source = SamplesBuffer::new(ch, sr, samples);
                                player.append(source);
                                player.play();

                                current_player = Some(player);
                                playing_flag.store(true, Ordering::SeqCst);
                            }
                            OutputCommand::PlayEncoded {
                                bytes,
                                session_id: _,
                            } => {
                                if let Some(p) = current_player.take() {
                                    p.stop();
                                }

                                let cursor = Cursor::new(bytes);
                                match Decoder::new(cursor) {
                                    Ok(source) => {
                                        let player = Player::connect_new(&handle.mixer());
                                        player.set_volume(current_volume);
                                        player.append(source);
                                        player.play();

                                        current_player = Some(player);
                                        playing_flag.store(true, Ordering::SeqCst);
                                    }
                                    Err(e) => {
                                        eprintln!("[AudioOutput] Failed to decode audio bytes: {}", e);
                                        playing_flag.store(false, Ordering::SeqCst);
                                    }
                                }
                            }
                            OutputCommand::Stop => {
                                if let Some(p) = current_player.take() {
                                    p.stop();
                                }
                                playing_flag.store(false, Ordering::SeqCst);
                            }
                            OutputCommand::Pause => {
                                if let Some(ref p) = current_player {
                                    p.pause();
                                }
                                playing_flag.store(false, Ordering::SeqCst);
                            }
                            OutputCommand::Resume => {
                                if let Some(ref p) = current_player {
                                    p.play();
                                    playing_flag.store(true, Ordering::SeqCst);
                                }
                            }
                            OutputCommand::SetVolume(vol) => {
                                current_volume = vol.clamp(0.0, 1.0);
                                if let Some(ref p) = current_player {
                                    p.set_volume(current_volume);
                                }
                            }
                            OutputCommand::SwitchDevice { device_name, device_id: _ } => {
                                if let Some(p) = current_player.take() {
                                    p.stop();
                                }
                                use rodio::cpal::traits::{DeviceTrait, HostTrait};
                                let new_sink_res = if let Some(ref name) = device_name {
                                    let host = rodio::cpal::default_host();
                                    if let Ok(mut devs) = host.output_devices() {
                                        if let Some(d) = devs.find(|dev| dev.name().map(|n| n == *name).unwrap_or(false)) {
                                            rodio::DeviceSinkBuilder::from_device(d).and_then(|b| b.open_sink_or_fallback())
                                        } else {
                                            rodio::DeviceSinkBuilder::open_default_sink()
                                        }
                                    } else {
                                        rodio::DeviceSinkBuilder::open_default_sink()
                                    }
                                } else {
                                    rodio::DeviceSinkBuilder::open_default_sink()
                                };

                                match new_sink_res {
                                    Ok(h) => {
                                        handle = h;
                                    }
                                    Err(e) => {
                                        eprintln!("[AudioOutput] Failed to switch audio output sink: {}", e);
                                    }
                                }
                                playing_flag.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                    playing_flag.store(false, Ordering::SeqCst);
                }
                Err(e) => {
                    eprintln!("[AudioOutput] CRITICAL: Failed to open default audio sink: {}", e);
                    playing_flag.store(false, Ordering::SeqCst);
                }
            }
        });
    }
}

impl Default for RodioAudioOutputDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutputDriver for RodioAudioOutputDriver {
    fn play(&self, buffer: AudioBuffer, session_id: VoiceSessionId) -> Result<(), VoiceError> {
        self.ensure_started();
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            tx.send(OutputCommand::PlayBuffer {
                samples: buffer.samples,
                channels: buffer.channels,
                sample_rate: buffer.sample_rate,
                session_id,
            })
            .map_err(|e| VoiceError::PlaybackFailure(format!("Failed to send buffer to audio player: {}", e)))?;
            Ok(())
        } else {
            Err(VoiceError::AudioDeviceUnavailable("Audio thread sender is closed".to_string()))
        }
    }

    fn play_encoded_bytes(&self, bytes: Vec<u8>, session_id: VoiceSessionId) -> Result<(), VoiceError> {
        self.ensure_started();
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            tx.send(OutputCommand::PlayEncoded { bytes, session_id })
                .map_err(|e| VoiceError::PlaybackFailure(format!("Failed to send encoded audio to player: {}", e)))?;
            Ok(())
        } else {
            Err(VoiceError::AudioDeviceUnavailable("Audio thread sender is closed".to_string()))
        }
    }

    fn stop(&self) -> Result<(), VoiceError> {
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            let _ = tx.send(OutputCommand::Stop);
        }
        self.is_playing_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn pause(&self) -> Result<(), VoiceError> {
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            let _ = tx.send(OutputCommand::Pause);
        }
        self.is_playing_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn resume(&self) -> Result<(), VoiceError> {
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            let _ = tx.send(OutputCommand::Resume);
        }
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), VoiceError> {
        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            let _ = tx.send(OutputCommand::SetVolume(volume));
        }
        Ok(())
    }

    fn is_playing(&self) -> bool {
        self.is_playing_flag.load(Ordering::SeqCst)
    }

    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError> {
        self.ensure_started();
        let dev_name = if let Some(ref id) = device_id {
            use rodio::cpal::traits::{DeviceTrait, HostTrait};
            let host = rodio::cpal::default_host();
            let mut found = None;
            if let Ok(devs) = host.output_devices() {
                for d in devs {
                    if let Ok(name) = d.name() {
                        let opaque_id = super::devices::compute_opaque_device_id(&name, false);
                        if &opaque_id == id || &name == id {
                            found = Some(name);
                            break;
                        }
                    }
                }
            }
            found
        } else {
            None
        };

        *self.current_device_id.write().unwrap() = device_id.clone();
        *self.current_device_name.write().unwrap() = dev_name.clone();

        let lock = self.sender.lock().unwrap();
        if let Some(ref tx) = *lock {
            let _ = tx.send(OutputCommand::SwitchDevice {
                device_name: dev_name,
                device_id,
            });
        }
        Ok(())
    }

    fn current_device_id(&self) -> Option<String> {
        self.current_device_id.read().unwrap().clone()
    }

    fn current_device_name(&self) -> Option<String> {
        self.current_device_name.read().unwrap().clone()
    }
}

/// In-memory mock audio output driver for unit and integration testing.
pub struct MockAudioOutputDriver {
    is_playing_flag: Arc<AtomicBool>,
    played_buffers: Arc<Mutex<Vec<AudioBuffer>>>,
    played_sessions: Arc<Mutex<Vec<VoiceSessionId>>>,
    volume: Arc<Mutex<f32>>,
    stop_count: Arc<Mutex<usize>>,
    current_device_id: Arc<std::sync::RwLock<Option<String>>>,
    current_device_name: Arc<std::sync::RwLock<Option<String>>>,
}

impl MockAudioOutputDriver {
    pub fn new() -> Self {
        Self {
            is_playing_flag: Arc::new(AtomicBool::new(false)),
            played_buffers: Arc::new(Mutex::new(Vec::new())),
            played_sessions: Arc::new(Mutex::new(Vec::new())),
            volume: Arc::new(Mutex::new(1.0)),
            stop_count: Arc::new(Mutex::new(0)),
            current_device_id: Arc::new(std::sync::RwLock::new(None)),
            current_device_name: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn get_played_buffers_count(&self) -> usize {
        self.played_buffers.lock().unwrap().len()
    }

    pub fn get_stop_count(&self) -> usize {
        *self.stop_count.lock().unwrap()
    }

    pub fn last_played_session(&self) -> Option<VoiceSessionId> {
        self.played_sessions.lock().unwrap().last().cloned()
    }
}

impl Default for MockAudioOutputDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutputDriver for MockAudioOutputDriver {
    fn play(&self, buffer: AudioBuffer, session_id: VoiceSessionId) -> Result<(), VoiceError> {
        self.played_buffers.lock().unwrap().push(buffer);
        self.played_sessions.lock().unwrap().push(session_id);
        self.is_playing_flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn play_encoded_bytes(&self, bytes: Vec<u8>, session_id: VoiceSessionId) -> Result<(), VoiceError> {
        let dummy = AudioBuffer::new(24000, 1, vec![0.0; bytes.len().min(100)]);
        self.played_buffers.lock().unwrap().push(dummy);
        self.played_sessions.lock().unwrap().push(session_id);
        self.is_playing_flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) -> Result<(), VoiceError> {
        *self.stop_count.lock().unwrap() += 1;
        self.is_playing_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn pause(&self) -> Result<(), VoiceError> {
        self.is_playing_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn resume(&self) -> Result<(), VoiceError> {
        self.is_playing_flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), VoiceError> {
        *self.volume.lock().unwrap() = volume;
        Ok(())
    }

    fn is_playing(&self) -> bool {
        self.is_playing_flag.load(Ordering::SeqCst)
    }

    fn set_device(&self, device_id: Option<String>) -> Result<(), VoiceError> {
        let name = device_id.as_ref().map(|id| format!("Mock Output Device ({})", id));
        *self.current_device_id.write().unwrap() = device_id;
        *self.current_device_name.write().unwrap() = name;
        Ok(())
    }

    fn current_device_id(&self) -> Option<String> {
        self.current_device_id.read().unwrap().clone()
    }

    fn current_device_name(&self) -> Option<String> {
        self.current_device_name.read().unwrap().clone()
    }
}
