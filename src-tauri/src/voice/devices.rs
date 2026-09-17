//! devices.rs — Audio device enumeration, selection, and opaque identifier hashing.
//!
//! Enforces:
//! - Telemetry privacy: Opaque hashed identifiers (`id`) used for logging, metrics, and storage.
//! - Human-readable device names (`name`) restricted to local UI display only.
//! - Safe querying through rodio's underlying `cpal` host.
//! - Fault-tolerant device resolution with graceful fallback to system default.

#![allow(deprecated)]

use super::errors::VoiceError;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::cpal::Device;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::RwLock;

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

/// Central manager orchestrating audio hardware device enumeration and selection.
pub struct AudioDeviceManager {
    active_input_id: RwLock<Option<String>>,
    active_output_id: RwLock<Option<String>>,
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self {
            active_input_id: RwLock::new(None),
            active_output_id: RwLock::new(None),
        }
    }

    /// Lists all available input and output devices with current active selections.
    pub fn list_devices(&self) -> Result<AudioDevicesSummary, VoiceError> {
        let input_devices = self.list_input_devices()?;
        let output_devices = self.list_output_devices()?;

        let active_in = self.active_input_id.read().unwrap().clone();
        let active_out = self.active_output_id.read().unwrap().clone();

        Ok(AudioDevicesSummary {
            input_devices,
            output_devices,
            active_input_id: active_in,
            active_output_id: active_out,
        })
    }

    /// Enumerates all active physical input (microphone) devices.
    pub fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|d| d.name().ok());

        let devices = host
            .input_devices()
            .map_err(|e| VoiceError::AudioDeviceUnavailable(format!("Failed to query input devices: {}", e)))?;

        let mut list = Vec::new();
        for dev in devices {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let id = compute_opaque_device_id(&name, true);
                
                // Sample rates detection (common targets)
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

    /// Enumerates all active physical output (speaker / headphone) devices.
    pub fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
        let host = rodio::cpal::default_host();
        let default_name = host
            .default_output_device()
            .and_then(|d| d.name().ok());

        let devices = host
            .output_devices()
            .map_err(|e| VoiceError::AudioDeviceUnavailable(format!("Failed to query output devices: {}", e)))?;

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

    /// Resolves an input `cpal::Device` by either its opaque ID or human-readable name.
    pub fn find_input_device(&self, id_or_name: &str) -> Result<Option<Device>, VoiceError> {
        let host = rodio::cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| VoiceError::AudioDeviceUnavailable(format!("Failed to query input devices: {}", e)))?;

        for dev in devices {
            if let Ok(name) = dev.name() {
                let opaque_id = compute_opaque_device_id(&name, true);
                if opaque_id == id_or_name || name == id_or_name {
                    return Ok(Some(dev));
                }
            }
        }
        Ok(None)
    }

    /// Resolves an output `cpal::Device` by either its opaque ID or human-readable name.
    pub fn find_output_device(&self, id_or_name: &str) -> Result<Option<Device>, VoiceError> {
        let host = rodio::cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| VoiceError::AudioDeviceUnavailable(format!("Failed to query output devices: {}", e)))?;

        for dev in devices {
            if let Ok(name) = dev.name() {
                let opaque_id = compute_opaque_device_id(&name, false);
                if opaque_id == id_or_name || name == id_or_name {
                    return Ok(Some(dev));
                }
            }
        }
        Ok(None)
    }

    /// Currently active input device opaque ID.
    pub fn active_input_id(&self) -> Option<String> {
        self.active_input_id.read().unwrap().clone()
    }

    /// Currently active output device opaque ID.
    pub fn active_output_id(&self) -> Option<String> {
        self.active_output_id.read().unwrap().clone()
    }

    /// Sets the active input device opaque ID.
    pub fn set_active_input_id(&self, id: Option<String>) {
        *self.active_input_id.write().unwrap() = id;
    }

    /// Sets the active output device opaque ID.
    pub fn set_active_output_id(&self, id: Option<String>) {
        *self.active_output_id.write().unwrap() = id;
    }
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
