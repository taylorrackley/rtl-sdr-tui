use super::samples_u8_to_complex;
use crate::state::SharedState;
use crate::types::Command;
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use num_complex::Complex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Start the SDR acquisition thread with real RTL-SDR hardware
pub fn start_sdr_thread(
    device_index: usize,
    state: SharedState,
    samples_tx: Sender<Vec<Complex<f32>>>,
    command_rx: Receiver<Command>,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>> {
    log::info!("Opening RTL-SDR device {}...", device_index);

    // Open RTL-SDR device
    let (mut controller, mut reader) = rtlsdr_mt::open(device_index as u32)
        .map_err(|e| anyhow::anyhow!("Failed to open RTL-SDR device {}: {:?}", device_index, e))?;

    // Get initial configuration from state
    let initial_freq = state.read().sdr.frequency;
    let initial_rate = state.read().sdr.sample_rate;
    let initial_gain = state.read().sdr.tuner_gain;

    // Configure device
    log::info!("Configuring RTL-SDR...");
    controller.set_center_freq(initial_freq)
        .map_err(|e| anyhow::anyhow!("Failed to set frequency: {:?}", e))?;
    controller.set_sample_rate(initial_rate)
        .map_err(|e| anyhow::anyhow!("Failed to set sample rate: {:?}", e))?;

    if initial_gain == -1 {
        controller.enable_agc()
            .map_err(|e| anyhow::anyhow!("Failed to enable AGC: {:?}", e))?;
        log::info!("AGC enabled");
    } else {
        controller.disable_agc()
            .map_err(|e| anyhow::anyhow!("Failed to disable AGC: {:?}", e))?;
        controller.set_tuner_gain(initial_gain)
            .map_err(|e| anyhow::anyhow!("Failed to set gain: {:?}", e))?;
        log::info!("Gain set to {}.{} dB", initial_gain / 10, initial_gain % 10);
    }

    log::info!("RTL-SDR configured: {} Hz, {} S/s", initial_freq, initial_rate);

    // Spawn command processing thread
    let cmd_shutdown = shutdown.clone();
    let cmd_state = state.clone();
    thread::spawn(move || {
        log::info!("SDR command processing thread started");

        loop {
            // Check for shutdown
            if cmd_shutdown.load(Ordering::Relaxed) {
                log::info!("SDR command thread shutting down");
                break;
            }

            // Process commands (blocking with timeout)
            match command_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(command) => {
                    match command {
                        Command::SetFrequency(freq) => {
                            if let Err(e) = controller.set_center_freq(freq) {
                                log::error!("Failed to set frequency: {:?}", e);
                            } else {
                                cmd_state.write().sdr.frequency = freq;
                                log::info!("Frequency changed to {} Hz", freq);
                            }
                        }
                        Command::IncreaseFrequency(delta) => {
                            let mut state_guard = cmd_state.write();
                            let new_freq = state_guard.sdr.frequency.saturating_add(delta as u32);
                            drop(state_guard); // Release lock before device call

                            if let Err(e) = controller.set_center_freq(new_freq) {
                                log::error!("Failed to set frequency: {:?}", e);
                            } else {
                                cmd_state.write().sdr.frequency = new_freq;
                                log::info!("Frequency increased to {} Hz", new_freq);
                            }
                        }
                        Command::DecreaseFrequency(delta) => {
                            let mut state_guard = cmd_state.write();
                            let new_freq = state_guard.sdr.frequency.saturating_sub(delta as u32);
                            drop(state_guard); // Release lock before device call

                            if let Err(e) = controller.set_center_freq(new_freq) {
                                log::error!("Failed to set frequency: {:?}", e);
                            } else {
                                cmd_state.write().sdr.frequency = new_freq;
                                log::info!("Frequency decreased to {} Hz", new_freq);
                            }
                        }
                        Command::SetSampleRate(rate) => {
                            if let Err(e) = controller.set_sample_rate(rate) {
                                log::error!("Failed to set sample rate: {:?}", e);
                            } else {
                                cmd_state.write().sdr.sample_rate = rate;
                                log::info!("Sample rate changed to {} Hz", rate);
                            }
                        }
                        Command::SetTunerGain(gain) => {
                            if let Err(e) = controller.set_tuner_gain(gain) {
                                log::error!("Failed to set gain: {:?}", e);
                            } else {
                                cmd_state.write().sdr.tuner_gain = gain;
                                cmd_state.write().sdr.auto_gain = false;
                                log::info!("Gain set to {}.{} dB", gain / 10, gain % 10);
                            }
                        }
                        Command::SetAutoGain(auto) => {
                            if auto {
                                if let Err(e) = controller.enable_agc() {
                                    log::error!("Failed to enable AGC: {:?}", e);
                                } else {
                                    cmd_state.write().sdr.tuner_gain = -1;
                                    cmd_state.write().sdr.auto_gain = true;
                                    log::info!("AGC enabled");
                                }
                            } else {
                                if let Err(e) = controller.disable_agc() {
                                    log::error!("Failed to disable AGC: {:?}", e);
                                } else {
                                    cmd_state.write().sdr.auto_gain = false;
                                    log::info!("AGC disabled");
                                }
                            }
                        }
                        Command::SetPpmError(ppm) => {
                            if let Err(e) = controller.set_ppm(ppm) {
                                log::error!("Failed to set PPM: {:?}", e);
                            } else {
                                cmd_state.write().sdr.ppm_error = ppm;
                                log::info!("PPM set to {}", ppm);
                            }
                        }
                        Command::Quit => {
                            log::info!("SDR command thread received quit command");
                            break;
                        }
                        _ => {} // Ignore other commands
                    }
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                    // No command, continue
                }
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    log::info!("Command channel disconnected");
                    break;
                }
            }
        }

        log::info!("SDR command processing thread stopped");
    });

    // Spawn the sample reading thread
    let handle = thread::spawn(move || {
        log::info!("SDR acquisition thread started");

        // Read samples asynchronously
        // Buffer params: 32 buffers of 16384 samples each (must be multiple of 512)
        let result = reader.read_async(32, 16384, |bytes| {
            // Check for shutdown (note: we can't early return from this callback,
            // so we just skip processing when shutting down)
            if !shutdown.load(Ordering::Relaxed) {
                // Convert u8 I/Q samples to Complex<f32>
                let samples = samples_u8_to_complex(bytes);

                // Send to DSP thread (non-blocking)
                if samples_tx.try_send(samples).is_err() {
                    // DSP thread is slow, drop this buffer
                    log::warn!("Dropping samples due to backpressure");
                }
            }
        });

        if let Err(e) = result {
            log::error!("SDR read_async error: {:?}", e);
        }

        log::info!("SDR acquisition thread stopped");
    });

    Ok(handle)
}
