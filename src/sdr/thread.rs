use crate::dsp::FftProcessor;
use crate::state::SharedState;
use crate::types::Command;
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use num_complex::Complex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Start the SDR acquisition thread
///
/// NOTE: This currently uses simulated data. The actual rtlsdr_mt v2 integration
/// requires API-specific adjustments for reading samples. The architecture (threading,
/// channels, command processing) is complete and ready for real device integration.
pub fn start_sdr_thread(
    _device_index: usize,
    state: SharedState,
    samples_tx: Sender<Vec<Complex<f32>>>,
    command_rx: Receiver<Command>,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>> {
    log::info!("SDR thread starting (simulated mode)");

    // Spawn the acquisition thread
    let handle = thread::spawn(move || {
        log::info!("SDR acquisition thread started");
        let mut frame_count = 0u32;

        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Relaxed) {
                log::info!("SDR thread shutting down");
                break;
            }

            // Check for commands (non-blocking)
            if let Ok(command) = command_rx.try_recv() {
                match command {
                    Command::SetFrequency(freq) => {
                        state.write().sdr.frequency = freq;
                        log::info!("Frequency changed to {} Hz", freq);
                    }
                    Command::IncreaseFrequency(delta) => {
                        let mut state_guard = state.write();
                        state_guard.sdr.frequency = state_guard.sdr.frequency.saturating_add(delta as u32);
                        log::info!("Frequency increased to {} Hz", state_guard.sdr.frequency);
                    }
                    Command::DecreaseFrequency(delta) => {
                        let mut state_guard = state.write();
                        state_guard.sdr.frequency = state_guard.sdr.frequency.saturating_sub(delta as u32);
                        log::info!("Frequency decreased to {} Hz", state_guard.sdr.frequency);
                    }
                    Command::SetSampleRate(rate) => {
                        state.write().sdr.sample_rate = rate;
                        log::info!("Sample rate changed to {} Hz", rate);
                    }
                    Command::SetTunerGain(gain) => {
                        state.write().sdr.tuner_gain = gain;
                        state.write().sdr.auto_gain = gain == -1;
                        log::info!("Gain set to {}", if gain == -1 { "Auto".to_string() } else { format!("{}.{} dB", gain / 10, gain % 10) });
                    }
                    Command::SetAutoGain(auto) => {
                        if auto {
                            state.write().sdr.tuner_gain = -1;
                            state.write().sdr.auto_gain = true;
                            log::info!("AGC enabled");
                        }
                    }
                    Command::SetPpmError(ppm) => {
                        state.write().sdr.ppm_error = ppm;
                        log::info!("PPM set to {}", ppm);
                    }
                    Command::Quit => {
                        log::info!("SDR thread received quit command");
                        break;
                    }
                    _ => {} // Ignore other commands
                }
            }

            // Generate simulated IQ samples
            // TODO: Replace with actual RTL-SDR device reads
            let current_freq = state.read().sdr.frequency;
            let sample_rate = state.read().sdr.sample_rate;

            // Simulate some "stations" at specific frequencies
            // Signals will only appear when tuned near these frequencies
            let stations = [
                (100_000_000, 200_000.0, 0.7),  // FM station at 100 MHz, 200 kHz offset
                (144_390_000, 0.0, 0.9),        // APRS at 144.390 MHz (strong, centered)
                (144_390_000, 25_000.0, 0.3),   // Weak signal near APRS
                (162_550_000, 0.0, 0.8),        // NOAA Weather at 162.550 MHz
                (433_000_000, -100_000.0, 0.6), // 433 MHz signal
                (1_090_000_000, 0.0, 0.7),      // ADS-B at 1090 MHz
            ];

            // Build signal list based on current frequency
            let mut signals = Vec::new();
            for (station_freq, offset, strength) in &stations {
                // Calculate how far this station is from our tuned frequency
                let freq_diff = (*station_freq as i64) - (current_freq as i64);
                let bandwidth = (sample_rate / 2) as i64;

                // Only include signals within our visible bandwidth
                if freq_diff.abs() < bandwidth {
                    // Signal offset relative to our tuned frequency
                    let signal_offset = freq_diff as f32 + offset;
                    signals.push((signal_offset, *strength));
                }
            }

            // Add a slowly moving signal for demonstration
            let demo_offset = (frame_count as f32 * 0.01).sin() * 100_000.0;
            signals.push((demo_offset, 0.5));

            let samples = if signals.is_empty() {
                // No signals, generate just noise
                FftProcessor::generate_test_signal(16384, sample_rate, &[(0.0, 0.05)])
            } else {
                FftProcessor::generate_test_signal(16384, sample_rate, &signals)
            };

            // Send samples to DSP thread (non-blocking)
            if samples_tx.try_send(samples).is_err() {
                // DSP thread is slow, drop this buffer
                log::warn!("Dropping samples due to backpressure");
            }

            frame_count = frame_count.wrapping_add(1);

            // Simulate USB sample rate (~512 samples at 2.048 MSPS = ~4ms)
            thread::sleep(Duration::from_millis(4));
        }

        log::info!("SDR acquisition thread stopped");
    });

    Ok(handle)
}

// TODO: Real RTL-SDR integration template
//
// When integrating with actual rtlsdr_mt v2:
// 1. Open device with: rtlsdr_mt::open(device_index as u32)
// 2. Configure Controller (frequency, sample rate, gain, PPM)
// 3. Use Reader to get sample blocks (API-specific - check rtlsdr_mt docs)
// 4. Convert u8 samples: crate::sdr::samples_u8_to_complex(&block)
// 5. Send through samples_tx channel as shown above
