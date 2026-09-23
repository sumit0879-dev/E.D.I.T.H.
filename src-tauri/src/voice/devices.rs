//! devices.rs — Audio device enumeration, selection, and opaque identifier hashing.
//!
//! Enforces:
//! - Telemetry privacy: Opaque hashed identifiers (`id`, `opaque_id`) used for logging, metrics, and storage.
//! - Human-readable device names (`name`) restricted to local UI display only.
//! - Provider abstraction: `AudioDeviceProvider` trait decoupling native CPAL / WASAPI hardware
//!   interactions from deterministic test environments.
//! - Deterministic unit tests: `MockAudioDeviceProvider` for zero-hardware CI testing.
//! - Safe querying through rodio's underlying `cpal` host for production.
//! - Fault-tolerant device resolution with graceful fallback to system default.

#![allow(deprecated)]

use super::errors::VoiceError;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, RwLock};

/// Opaque descriptor of an audio hardware device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    /// Opaque, stable identifier (safe for telemetry, persistence, and correlation)
    pub id: String,
    pub opaque_id: String,
    /// Human-readable device name (UI-only display, strictly scrubbed from telemetry)
    pub name: String,
    /// Whether this device is the current platform default
    pub is_default: bool,
    /// Typical supported sample rates (e.g. 16000, 24000, 44100, 48000)
    pub sample_rates: Vec<u32>,
    /// Number of channels (typically 1 for mic, 2 for speaker)
    pub channels: u16,
}

/// Summary of all connected audio hardware devices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioDevicesSummary {
    pub input_devices: Vec<AudioDeviceInfo>,
    pub output_devices: Vec<AudioDeviceInfo>,
    pub active_input_id: Option<String>,
    pub active_output_id: Option<String>,
}

/// Computes an opaque, deterministic, telemetry-safe identifier from a device name and direction.
pub fn compute_opaque_device_id(name: &str, is_input: bool) -> String {
    let mut hasher = Sha256::new();
    let prefix_bytes: &[u8] = if is_input { b"in:" } else { b"out:" };
    hasher.update(prefix_bytes);
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    let prefix = if is_input { "in_" } else { "out_" };
    format!("{}{}", prefix, hex::encode(&result[0..8]))
}

// Fallback hex encoder if hex crate is not present
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Abstract provider boundary for enumerating and resolving audio hardware devices.
/// Isolates native host (CPAL / Windows WASAPI) interactions from unit test runners.
pub trait AudioDeviceProvider: Send + Sync {
    /// Enumerate all available audio input (microphone) devices.
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError>;

    /// Enumerate all available audio output (speaker / headphone) devices.
    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError>;

    /// Return the system default input device, if available.
    fn default_input_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError>;

    /// Return the system default output device, if available.
    fn default_output_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError>;

    /// Resolve an input device by opaque ID or human-readable name.
    fn find_input_device(&self, id_or_name: &str) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        let devices = self.list_input_devices()?;
        Ok(devices
            .into_iter()
            .find(|d| d.id == id_or_name || d.opaque_id == id_or_name || d.name == id_or_name))
    }

    /// Resolve an output device by opaque ID or human-readable name.
    fn find_output_device(&self, id_or_name: &str) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        let devices = self.list_output_devices()?;
        Ok(devices
            .into_iter()
            .find(|d| d.id == id_or_name || d.opaque_id == id_or_name || d.name == id_or_name))
    }
}

/// Production audio device provider querying native host audio endpoints via CPAL.
pub struct CpalAudioDeviceProvider;

impl CpalAudioDeviceProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CpalAudioDeviceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDeviceProvider for CpalAudioDeviceProvider {
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let default_name = host.default_input_device().and_then(|d| d.name().ok());

        let devices = match host.input_devices() {
            Ok(devs) => devs,
            Err(e) => {
                eprintln!("[AudioDeviceManager] Warning querying input devices: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut list = Vec::new();
        for dev in devices {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let id = compute_opaque_device_id(&name, true);
                let sample_rates = vec![16000, 24000, 44100, 48000];
                let channels = dev
                    .default_input_config()
                    .map(|c| c.channels())
                    .unwrap_or(1);

                list.push(AudioDeviceInfo {
                    opaque_id: id.clone(),
                    id,
                    name,
                    is_default,
                    sample_rates,
                    channels,
                });
            }
        }
        Ok(list)
    }

    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let default_name = host.default_output_device().and_then(|d| d.name().ok());

        let devices = match host.output_devices() {
            Ok(devs) => devs,
            Err(e) => {
                eprintln!(
                    "[AudioDeviceManager] Warning querying output devices: {}",
                    e
                );
                return Ok(Vec::new());
            }
        };

        let mut list = Vec::new();
        for dev in devices {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let id = compute_opaque_device_id(&name, false);
                let sample_rates = vec![24000, 44100, 48000];
                let channels = dev
                    .default_output_config()
                    .map(|c| c.channels())
                    .unwrap_or(2);

                list.push(AudioDeviceInfo {
                    opaque_id: id.clone(),
                    id,
                    name,
                    is_default,
                    sample_rates,
                    channels,
                });
            }
        }
        Ok(list)
    }

    fn default_input_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let dev = match host.default_input_device() {
            Some(d) => d,
            None => return Ok(None),
        };
        let name = match dev.name() {
            Ok(n) => n,
            Err(_) => return Ok(None),
        };
        let id = compute_opaque_device_id(&name, true);
        let channels = dev
            .default_input_config()
            .map(|c| c.channels())
            .unwrap_or(1);
        Ok(Some(AudioDeviceInfo {
            opaque_id: id.clone(),
            id,
            name,
            is_default: true,
            sample_rates: vec![16000, 24000, 44100, 48000],
            channels,
        }))
    }

    fn default_output_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let dev = match host.default_output_device() {
            Some(d) => d,
            None => return Ok(None),
        };
        let name = match dev.name() {
            Ok(n) => n,
            Err(_) => return Ok(None),
        };
        let id = compute_opaque_device_id(&name, false);
        let channels = dev
            .default_output_config()
            .map(|c| c.channels())
            .unwrap_or(2);
        Ok(Some(AudioDeviceInfo {
            opaque_id: id.clone(),
            id,
            name,
            is_default: true,
            sample_rates: vec![24000, 44100, 48000],
            channels,
        }))
    }
}

/// Deterministic mock audio device provider for unit and integration testing.
/// Does NOT touch native Windows WASAPI / CPAL audio subsystems.
pub struct MockAudioDeviceProvider {
    input_devices: RwLock<Vec<AudioDeviceInfo>>,
    output_devices: RwLock<Vec<AudioDeviceInfo>>,
}

impl MockAudioDeviceProvider {
    pub fn new() -> Self {
        Self {
            input_devices: RwLock::new(Vec::new()),
            output_devices: RwLock::new(Vec::new()),
        }
    }

    /// Creates a mock provider pre-populated with realistic desktop audio devices.
    pub fn with_realistic_devices() -> Self {
        let mic_default_name = "Built-in Microphone";
        let mic_default_id = compute_opaque_device_id(mic_default_name, true);
        let mic_usb_name = "USB Studio Condenser Mic";
        let mic_usb_id = compute_opaque_device_id(mic_usb_name, true);
        let mic_bt_name = "Bluetooth Headset Microphone";
        let mic_bt_id = compute_opaque_device_id(mic_bt_name, true);

        let inputs = vec![
            AudioDeviceInfo {
                id: mic_default_id.clone(),
                opaque_id: mic_default_id,
                name: mic_default_name.to_string(),
                is_default: true,
                sample_rates: vec![16000, 24000, 44100, 48000],
                channels: 1,
            },
            AudioDeviceInfo {
                id: mic_usb_id.clone(),
                opaque_id: mic_usb_id,
                name: mic_usb_name.to_string(),
                is_default: false,
                sample_rates: vec![44100, 48000, 96000],
                channels: 2,
            },
            AudioDeviceInfo {
                id: mic_bt_id.clone(),
                opaque_id: mic_bt_id,
                name: mic_bt_name.to_string(),
                is_default: false,
                sample_rates: vec![16000],
                channels: 1,
            },
        ];

        let out_default_name = "Realtek High Definition Audio (Speakers)";
        let out_default_id = compute_opaque_device_id(out_default_name, false);
        let out_dac_name = "USB DAC Audio Interface";
        let out_dac_id = compute_opaque_device_id(out_dac_name, false);
        let out_bt_name = "Wireless Noise-Cancelling Headphones";
        let out_bt_id = compute_opaque_device_id(out_bt_name, false);

        let outputs = vec![
            AudioDeviceInfo {
                id: out_default_id.clone(),
                opaque_id: out_default_id,
                name: out_default_name.to_string(),
                is_default: true,
                sample_rates: vec![24000, 44100, 48000],
                channels: 2,
            },
            AudioDeviceInfo {
                id: out_dac_id.clone(),
                opaque_id: out_dac_id,
                name: out_dac_name.to_string(),
                is_default: false,
                sample_rates: vec![44100, 48000, 96000, 192000],
                channels: 2,
            },
            AudioDeviceInfo {
                id: out_bt_id.clone(),
                opaque_id: out_bt_id,
                name: out_bt_name.to_string(),
                is_default: false,
                sample_rates: vec![44100, 48000],
                channels: 2,
            },
        ];

        Self {
            input_devices: RwLock::new(inputs),
            output_devices: RwLock::new(outputs),
        }
    }

    pub fn add_input_device(&self, info: AudioDeviceInfo) {
        self.input_devices.write().unwrap().push(info);
    }

    pub fn add_output_device(&self, info: AudioDeviceInfo) {
        self.output_devices.write().unwrap().push(info);
    }

    pub fn set_inputs(&self, inputs: Vec<AudioDeviceInfo>) {
        *self.input_devices.write().unwrap() = inputs;
    }

    pub fn set_outputs(&self, outputs: Vec<AudioDeviceInfo>) {
        *self.output_devices.write().unwrap() = outputs;
    }
}

impl Default for MockAudioDeviceProvider {
    fn default() -> Self {
        Self::with_realistic_devices()
    }
}

impl AudioDeviceProvider for MockAudioDeviceProvider {
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        Ok(self.input_devices.read().unwrap().clone())
    }

    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        Ok(self.output_devices.read().unwrap().clone())
    }

    fn default_input_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        Ok(self
            .input_devices
            .read()
            .unwrap()
            .iter()
            .find(|d| d.is_default)
            .cloned())
    }

    fn default_output_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        Ok(self
            .output_devices
            .read()
            .unwrap()
            .iter()
            .find(|d| d.is_default)
            .cloned())
    }
}

/// Central manager orchestrating audio hardware device enumeration, selection, and tracking.
pub struct AudioDeviceManager {
    provider: Arc<dyn AudioDeviceProvider>,
    active_input_id: RwLock<Option<String>>,
    active_output_id: RwLock<Option<String>>,
}

impl AudioDeviceManager {
    /// Production constructor using CpalAudioDeviceProvider.
    pub fn new() -> Self {
        Self::with_provider(Arc::new(CpalAudioDeviceProvider::new()))
    }

    /// Injects an explicit AudioDeviceProvider (e.g. MockAudioDeviceProvider).
    pub fn with_provider(provider: Arc<dyn AudioDeviceProvider>) -> Self {
        Self {
            provider,
            active_input_id: RwLock::new(None),
            active_output_id: RwLock::new(None),
        }
    }

    /// Convenience constructor for testing with mock provider.
    pub fn mock() -> Self {
        Self::with_provider(Arc::new(MockAudioDeviceProvider::default()))
    }

    /// Reference to the underlying AudioDeviceProvider.
    pub fn provider(&self) -> &Arc<dyn AudioDeviceProvider> {
        &self.provider
    }

    /// Lists all available input and output devices with current active selections.
    pub fn list_devices(&self) -> Result<AudioDevicesSummary, VoiceError> {
        let input_devices = self.provider.list_input_devices()?;
        let output_devices = self.provider.list_output_devices()?;

        let active_in = self.active_input_id.read().unwrap().clone();
        let active_out = self.active_output_id.read().unwrap().clone();

        Ok(AudioDevicesSummary {
            input_devices,
            output_devices,
            active_input_id: active_in,
            active_output_id: active_out,
        })
    }

    pub fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        self.provider.list_input_devices()
    }

    pub fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        self.provider.list_output_devices()
    }

    pub fn default_input_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        self.provider.default_input_device()
    }

    pub fn default_output_device(&self) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        self.provider.default_output_device()
    }

    pub fn find_input_device(
        &self,
        id_or_name: &str,
    ) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        self.provider.find_input_device(id_or_name)
    }

    pub fn find_output_device(
        &self,
        id_or_name: &str,
    ) -> Result<Option<AudioDeviceInfo>, VoiceError> {
        self.provider.find_output_device(id_or_name)
    }

    pub fn active_input_id(&self) -> Option<String> {
        self.active_input_id.read().unwrap().clone()
    }

    pub fn active_output_id(&self) -> Option<String> {
        self.active_output_id.read().unwrap().clone()
    }

    pub fn set_active_input_id(&self, id: Option<String>) {
        *self.active_input_id.write().unwrap() = id;
    }

    pub fn set_active_output_id(&self, id: Option<String>) {
        *self.active_output_id.write().unwrap() = id;
    }
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
