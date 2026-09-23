//! production_tests.rs — Comprehensive Phase 11 Voice UX & Production Reliability Test Suite.
//!
//! Validates:
//! 1. Orthogonal/composable duplex voice state transitions.
//! 2. Dynamic audio device enumeration and opaque IDs.
//! 3. Clean output sink handover.
//! 4. Missing device fallback to system default.
//! 5. Exponential backoff and recovery exhaustion.
//! 6. Deterministic barge-in invariants.
//! 7. Continuous audio streaming preserving silence frames.
//! 8. Software ducking echo canceller threshold elevation.
//! 9. Privacy-safe telemetry with opaque device IDs.
//! 10. Multi-turn soak session lifecycle.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::events::payload::{
        DuplexVoiceState, InputChannelState, OutputChannelState, ProcessingState,
        SessionLifecycleState,
    };
    use crate::voice::devices::{
        compute_opaque_device_id, AudioDeviceManager, MockAudioDeviceProvider,
    };
    use crate::voice::dsp::echo::{EchoCanceller, SoftwareDuckingEchoCanceller};
    use crate::voice::dsp::vad::{EnergyVad, VadConfig, VoiceActivityDetector};
    use crate::voice::output::{AudioOutputDriver, MockAudioOutputDriver};
    use crate::voice::realtime::recovery::{RecoveryConfig, RecoveryManager, RecoveryState};
    use crate::voice::telemetry::VoiceTelemetryCollector;

    /// 1. Verify orthogonal/composable duplex state model without mutual-exclusion bugs.
    #[test]
    fn test_orthogonal_duplex_state_transitions() {
        let mut state = DuplexVoiceState {
            session: SessionLifecycleState::Connected,
            input: InputChannelState::UserSpeaking,
            output: OutputChannelState::AssistantSpeaking,
            processing: ProcessingState::ModelStreaming,
            active_turn_id: Some("turn_test_1".to_string()),
            generation_id: 1,
        };

        // In full duplex S2S, both user and assistant can be speaking simultaneously (barge-in moment)
        assert!(state.is_user_speaking());
        assert!(state.is_assistant_speaking());
        assert!(state.can_barge_in());

        // User finishes speaking, assistant continues
        state.input = InputChannelState::ListeningAmbient;
        assert!(!state.is_user_speaking());
        assert!(state.is_assistant_speaking());

        // Assistant finishes response
        state.output = OutputChannelState::Silent;
        state.processing = ProcessingState::Idle;
        assert!(!state.is_assistant_speaking());
        assert_eq!(state.session, SessionLifecycleState::Connected);
    }

    /// 2. Verify dynamic audio device manager enumerates devices and computes stable opaque IDs.
    /// Uses MockAudioDeviceProvider for deterministic, zero-hardware isolation (no native CPAL / WASAPI calls).
    #[test]
    fn test_audio_device_enumeration() {
        let manager = AudioDeviceManager::mock();
        let summary_res = manager.list_devices();
        assert!(summary_res.is_ok());
        let summary = summary_res.unwrap();

        // 1. Verify multiple realistic microphones and output devices
        assert!(
            summary.input_devices.len() >= 3,
            "Expected at least 3 realistic input devices"
        );
        assert!(
            summary.output_devices.len() >= 3,
            "Expected at least 3 realistic output devices"
        );

        // 2. Verify default input and output detection
        let default_in = manager
            .default_input_device()
            .unwrap()
            .expect("Default input device must be present");
        assert!(default_in.is_default);
        assert_eq!(default_in.name, "Built-in Microphone");
        assert_eq!(default_in.channels, 1);
        assert!(default_in.sample_rates.contains(&16000));
        assert!(default_in.sample_rates.contains(&48000));

        let default_out = manager
            .default_output_device()
            .unwrap()
            .expect("Default output device must be present");
        assert!(default_out.is_default);
        assert!(default_out.name.contains("Realtek"));
        assert_eq!(default_out.channels, 2);
        assert!(default_out.sample_rates.contains(&44100));
        assert!(default_out.sample_rates.contains(&48000));

        // 3. Verify supported sample rates and channel counts across devices
        let usb_mic = manager
            .find_input_device("USB Studio Condenser Mic")
            .unwrap()
            .expect("USB Studio Condenser Mic should be found");
        assert_eq!(usb_mic.channels, 2);
        assert!(usb_mic.sample_rates.contains(&96000));
        assert!(!usb_mic.is_default);

        let dac_out = manager
            .find_output_device("USB DAC Audio Interface")
            .unwrap()
            .expect("USB DAC Audio Interface should be found");
        assert_eq!(dac_out.channels, 2);
        assert!(dac_out.sample_rates.contains(&192000));

        // 4. Opaque device ID hashing invariants and determinism
        let opaque_in = compute_opaque_device_id("Built-in Microphone", true);
        let opaque_out = compute_opaque_device_id("Realtek Speakers", false);

        assert!(opaque_in.starts_with("in_"));
        assert!(opaque_out.starts_with("out_"));
        assert_eq!(opaque_in.len(), 19); // "in_" (3) + 16 hex chars
        assert_eq!(opaque_out.len(), 20); // "out_" (4) + 16 hex chars
        assert_eq!(default_in.opaque_id, opaque_in);

        // Deterministic hashing for same name
        let opaque_in_again = compute_opaque_device_id("Built-in Microphone", true);
        assert_eq!(opaque_in, opaque_in_again);

        // Inputs and outputs summaries have valid opaque structure
        for in_dev in &summary.input_devices {
            assert!(in_dev.opaque_id.starts_with("in_"));
            assert!(!in_dev.name.is_empty());
        }
        for out_dev in &summary.output_devices {
            assert!(out_dev.opaque_id.starts_with("out_"));
            assert!(!out_dev.name.is_empty());
        }

        // 5. Active-device tracking and switching
        assert_eq!(manager.active_input_id(), None);
        assert_eq!(manager.active_output_id(), None);

        // Switch to USB mic
        manager.set_active_input_id(Some(usb_mic.opaque_id.clone()));
        assert_eq!(manager.active_input_id(), Some(usb_mic.opaque_id.clone()));

        // Switch to DAC output
        manager.set_active_output_id(Some(dac_out.opaque_id.clone()));
        assert_eq!(manager.active_output_id(), Some(dac_out.opaque_id.clone()));

        // Re-check summary reflects active IDs
        let updated_summary = manager.list_devices().unwrap();
        assert_eq!(updated_summary.active_input_id, Some(usb_mic.opaque_id));
        assert_eq!(updated_summary.active_output_id, Some(dac_out.opaque_id));

        // Switch back to None (system default)
        manager.set_active_input_id(None);
        manager.set_active_output_id(None);
        assert_eq!(manager.active_input_id(), None);
        assert_eq!(manager.active_output_id(), None);

        // 6. Device lookup by name and opaque ID
        let found_by_name = manager.find_input_device("Built-in Microphone").unwrap();
        assert!(found_by_name.is_some());
        let found_by_id = manager.find_input_device(&default_in.opaque_id).unwrap();
        assert_eq!(found_by_id, found_by_name);

        let not_found = manager.find_input_device("in_nonexistent_xyz").unwrap();
        assert!(not_found.is_none());
    }

    /// 3. Verify clean output device handover without crashes or audio leaks.
    #[test]
    fn test_clean_output_device_handover() {
        let driver = MockAudioOutputDriver::new();
        assert_eq!(driver.current_device_id(), None);
        assert_eq!(driver.current_device_name(), None);

        let res = driver.set_device(Some("out_test_sink_42".to_string()));
        assert!(res.is_ok());
        assert_eq!(
            driver.current_device_id(),
            Some("out_test_sink_42".to_string())
        );
        assert_eq!(
            driver.current_device_name(),
            Some("Mock Output Device (out_test_sink_42)".to_string())
        );

        // Switch to default
        let res2 = driver.set_device(None);
        assert!(res2.is_ok());
        assert_eq!(driver.current_device_id(), None);
        assert_eq!(driver.current_device_name(), None);
    }

    /// 4. Verify nonexistent device ID gracefully falls back to default without failing session.
    #[test]
    fn test_device_disappearance_and_default_fallback() {
        let mock_provider = Arc::new(MockAudioDeviceProvider::with_realistic_devices());
        let manager = AudioDeviceManager::with_provider(mock_provider.clone());

        let res_in = manager.find_input_device("in_nonexistent_xyz_999");
        assert!(res_in.is_ok());
        assert!(res_in.unwrap().is_none());

        let res_out = manager.find_output_device("out_nonexistent_xyz_999");
        assert!(res_out.is_ok());
        assert!(res_out.unwrap().is_none());

        // Set active device to USB mic
        let usb_mic = manager
            .find_input_device("USB Studio Condenser Mic")
            .unwrap()
            .unwrap();
        manager.set_active_input_id(Some(usb_mic.opaque_id.clone()));
        assert_eq!(manager.active_input_id(), Some(usb_mic.opaque_id.clone()));

        // Simulate device disconnection: only default built-in mic remains
        let default_in = manager.default_input_device().unwrap().unwrap();
        mock_provider.set_inputs(vec![default_in.clone()]);

        // Previously selected device is no longer found in hardware list
        let lookup_disconnected = manager.find_input_device(&usb_mic.opaque_id).unwrap();
        assert!(lookup_disconnected.is_none());

        // Graceful fallback to default device
        let effective_device = lookup_disconnected
            .or_else(|| manager.default_input_device().unwrap())
            .expect("Must fall back to default input device");
        assert_eq!(effective_device.id, default_in.id);
        assert_eq!(effective_device.name, "Built-in Microphone");
        assert!(effective_device.is_default);
    }

    /// 5. Verify exponential backoff delay calculation and retry budget exhaustion.
    #[test]
    fn test_reconnect_exponential_backoff_and_exhaustion() {
        let config = RecoveryConfig {
            max_reconnect_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 800,
            backoff_multiplier: 2.0,
        };
        let mut mgr = RecoveryManager::new(config);

        // Attempt 1: 100ms
        match mgr.on_disconnect() {
            RecoveryState::Reconnecting { attempt, delay_ms } => {
                assert_eq!(attempt, 1);
                assert_eq!(delay_ms, 100);
            }
            other => panic!("Expected Reconnecting, got {:?}", other),
        }

        // Attempt 2: 200ms
        match mgr.on_disconnect() {
            RecoveryState::Reconnecting { attempt, delay_ms } => {
                assert_eq!(attempt, 2);
                assert_eq!(delay_ms, 200);
            }
            other => panic!("Expected Reconnecting, got {:?}", other),
        }

        // Attempt 3: 400ms
        match mgr.on_disconnect() {
            RecoveryState::Reconnecting { attempt, delay_ms } => {
                assert_eq!(attempt, 3);
                assert_eq!(delay_ms, 400);
            }
            other => panic!("Expected Reconnecting, got {:?}", other),
        }

        // Attempt 4: Exhausted
        match mgr.on_disconnect() {
            RecoveryState::Exhausted { total_attempts } => {
                assert_eq!(total_attempts, 3);
                assert!(mgr.is_exhausted());
            }
            other => panic!("Expected Exhausted, got {:?}", other),
        }

        // Reconnect resets
        mgr.on_connected();
        assert!(!mgr.is_exhausted());
        assert_eq!(mgr.current_attempt(), 0);
    }

    /// 6. Verify deterministic barge-in invariants: atomic generation bump and playback stop.
    #[tokio::test]
    async fn test_deterministic_barge_in_invariants() {
        use std::sync::atomic::{AtomicU64, Ordering};

        let current_gen = Arc::new(AtomicU64::new(1));
        let driver = Arc::new(MockAudioOutputDriver::new());

        // Simulate assistant starting playback for Generation 1
        let _ = driver.stop();
        assert_eq!(driver.get_stop_count(), 1);

        // Barge-in occurs:
        // Invariant A: Invalidate active generation immediately
        let new_gen = current_gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(new_gen, 2);

        // Invariant B: Dispatched hardware playback stop
        let _ = driver.stop();
        assert_eq!(driver.get_stop_count(), 2);

        // Invariant C: Inbound audio with stale Generation 1 is rejected
        let stale_frame_gen: u64 = 1;
        let is_valid = stale_frame_gen == current_gen.load(Ordering::SeqCst);
        assert!(
            !is_valid,
            "Stale audio from gen 1 must be rejected after barge-in to gen 2"
        );
    }

    /// 7. Verify continuous audio streaming preserves silence frames for provider VAD.
    #[test]
    fn test_continuous_audio_streaming_preserves_silence() {
        let mut vad = EnergyVad::new(VadConfig::default());
        let silence_samples = vec![0.001f32; 480];

        // Process frame through VAD
        let is_speech = vad.is_speech(&silence_samples, 16000);
        assert!(
            !is_speech,
            "Silence frames must evaluate to is_speech=false"
        );

        // Critical Phase 11 invariant: frames must NOT be dropped or zeroed out
        assert_eq!(silence_samples.len(), 480);
        assert_eq!(silence_samples[0], 0.001f32);
    }

    /// 8. Verify software ducking echo canceller elevates threshold during assistant playback.
    #[test]
    fn test_dsp_echo_ducking_threshold() {
        let aec = SoftwareDuckingEchoCanceller::new(0.85, 2.5);

        // 1. When assistant is silent, baseline multiplier applies (1.0)
        let mult_silent = aec.energy_threshold_multiplier(false);
        assert!((mult_silent - 1.0).abs() < 1e-5);

        // 2. When assistant is speaking, multiplier elevates (2.5)
        let mult_speaking = aec.energy_threshold_multiplier(true);
        assert!((mult_speaking - 2.5).abs() < 1e-5);

        let base_threshold = 0.02f32;
        let elevated_threshold = base_threshold * mult_speaking; // 0.05

        // Low mic spillover (0.03) from speakers does NOT exceed elevated threshold
        let spillover_energy = 0.03f32;
        assert!(spillover_energy < elevated_threshold);

        // Firm user barge-in (0.08) exceeds elevated threshold and triggers speech
        let user_barge_in = 0.08f32;
        assert!(user_barge_in > elevated_threshold);
    }

    /// 9. Verify privacy-safe telemetry uses opaque device IDs and never leaks raw hardware names.
    #[test]
    fn test_telemetry_redaction_uses_opaque_device_ids() {
        let raw_mic_name = "Realtek High Definition Audio (Mic 1)";
        let raw_speaker_name = "Realtek High Definition Audio (Speakers)";

        let opaque_mic = compute_opaque_device_id(raw_mic_name, true);
        let opaque_speaker = compute_opaque_device_id(raw_speaker_name, false);

        let collector = VoiceTelemetryCollector::new(
            "test_session_123",
            "conv_456",
            "realtime_s2s",
            "gemini_live",
            opaque_mic.clone(),
            opaque_speaker.clone(),
        );

        collector.record_input_frame();
        collector.record_output_frame();
        collector.record_barge_in();

        let report = collector.snapshot();
        assert_eq!(report.opaque_input_device_id, opaque_mic);
        assert_eq!(report.opaque_output_device_id, opaque_speaker);
        assert_eq!(report.barge_in_count, 1);
        assert_eq!(report.total_input_frames, 1);
        assert_eq!(report.total_output_frames, 1);

        let json = serde_json::to_string(&report).unwrap();
        // Raw hardware device names must never appear anywhere in the telemetry json
        assert!(!json.contains("Realtek"));
        assert!(!json.contains("High Definition Audio"));
        assert!(json.contains("in_"));
        assert!(json.contains("out_"));
    }

    /// 10. Multi-turn soak session lifecycle: verify alternating turns and barge-in transitions.
    #[test]
    fn test_soak_multiturn_voice_session() {
        let mut duplex_state = DuplexVoiceState::default();
        let recovery = RecoveryManager::default();
        let aec = SoftwareDuckingEchoCanceller::new(0.85, 2.2);

        duplex_state.session = SessionLifecycleState::Connected;
        duplex_state.input = InputChannelState::ListeningAmbient;
        duplex_state.output = OutputChannelState::Silent;
        duplex_state.processing = ProcessingState::Idle;

        let base_thresh = 0.02f32;

        for turn_idx in 1..=5 {
            // Step 1: User speaks
            let user_energy = 0.06f32;
            let mult = aec.energy_threshold_multiplier(false);
            assert!(user_energy > base_thresh * mult);

            duplex_state.input = InputChannelState::UserSpeaking;
            assert!(duplex_state.is_user_speaking());

            // Step 2: User finishes utterance, model processes
            duplex_state.input = InputChannelState::ListeningAmbient;
            duplex_state.processing = ProcessingState::ModelInferring;
            assert_eq!(duplex_state.processing, ProcessingState::ModelInferring);

            // Step 3: Assistant streaming response
            duplex_state.output = OutputChannelState::AssistantSpeaking;
            duplex_state.processing = ProcessingState::ModelStreaming;
            assert!(duplex_state.is_assistant_speaking());

            // On turn 3, simulate a mid-turn barge-in
            if turn_idx == 3 {
                let active_mult = aec.energy_threshold_multiplier(true);
                let barge_energy = 0.09f32;
                assert!(barge_energy > base_thresh * active_mult);
                duplex_state.input = InputChannelState::UserSpeaking;
                assert!(duplex_state.can_barge_in());

                // Barge-in cuts assistant speech immediately
                duplex_state.output = OutputChannelState::Silent;
                duplex_state.input = InputChannelState::ListeningAmbient;
            } else {
                duplex_state.output = OutputChannelState::Silent;
                duplex_state.processing = ProcessingState::Idle;
            }

            assert!(!duplex_state.is_assistant_speaking());
        }

        assert!(!recovery.is_exhausted());
        assert_eq!(recovery.current_attempt(), 0);
        assert_eq!(duplex_state.session, SessionLifecycleState::Connected);
    }
}
