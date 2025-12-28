use super::FftProcessor;
use crate::state::SharedState;
use crate::types::DemodMode;
use crossbeam::channel::Receiver;
use num_complex::Complex;
use ringbuf::traits::Producer;
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Start the DSP processing thread
pub fn start_dsp_thread<P>(
    state: SharedState,
    samples_rx: Receiver<Vec<Complex<f32>>>,
    mut audio_tx: Option<P>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()>
where
    P: Producer<Item = f32> + Send + 'static,
{
    thread::spawn(move || {
        log::info!("DSP processing thread started");

        // Create FFT processor
        let mut fft_processor = FftProcessor::new(2048);

        loop {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                log::info!("DSP thread shutting down");
                break;
            }

            // Receive samples from SDR thread (blocking with timeout)
            match samples_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(samples) => {
                    // 1. Compute FFT for spectrum display
                    let fft_data = fft_processor.process(&samples);

                    // Update spectrum state
                    state.write().spectrum.add_fft_data(fft_data);

                    // 2. Demodulate based on current mode
                    let mode = state.read().decoder.mode;

                    match mode {
                        DemodMode::FmNarrow | DemodMode::FmWide => {
                            // FM demodulation
                            if let Some(audio_producer) = audio_tx.as_mut() {
                                let audio = demodulate_fm(&samples, mode == DemodMode::FmWide);
                                send_audio_samples(audio_producer, &audio);
                            }
                        }
                        DemodMode::Am => {
                            // AM demodulation
                            if let Some(audio_producer) = audio_tx.as_mut() {
                                let audio = demodulate_am(&samples);
                                send_audio_samples(audio_producer, &audio);
                            }
                        }
                        DemodMode::Raw => {
                            // No demodulation, just visualization
                        }
                        _ => {
                            // Other modes not yet implemented
                        }
                    }
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                    // No samples available, continue
                    continue;
                }
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    log::info!("SDR thread disconnected, DSP thread exiting");
                    break;
                }
            }
        }

        log::info!("DSP processing thread stopped");
    })
}

/// Simple FM demodulator using phase difference
fn demodulate_fm(samples: &[Complex<f32>], wideband: bool) -> Vec<f32> {
    let mut audio = Vec::with_capacity(samples.len());

    for window in samples.windows(2) {
        // Phase difference between consecutive samples
        let phase_diff = (window[1] * window[0].conj()).arg();

        // Normalize to audio range
        let sample = phase_diff / std::f32::consts::PI;
        audio.push(sample);
    }

    // Apply simple lowpass filtering (moving average)
    let filter_size = if wideband { 4 } else { 2 };
    lowpass_filter(&audio, filter_size)
}

/// Simple AM demodulator using envelope detection
fn demodulate_am(samples: &[Complex<f32>]) -> Vec<f32> {
    samples.iter().map(|s| s.norm()).collect()
}

/// Simple lowpass filter using moving average
fn lowpass_filter(input: &[f32], window_size: usize) -> Vec<f32> {
    if window_size <= 1 {
        return input.to_vec();
    }

    let mut output = Vec::with_capacity(input.len());

    for i in 0..input.len() {
        let start = i.saturating_sub(window_size / 2);
        let end = (i + window_size / 2 + 1).min(input.len());

        let sum: f32 = input[start..end].iter().sum();
        let avg = sum / (end - start) as f32;
        output.push(avg);
    }

    output
}

/// Send audio samples to the ring buffer
fn send_audio_samples<P: Producer<Item = f32>>(producer: &mut P, samples: &[f32]) {
    for &sample in samples {
        // Clamp to valid audio range
        let clamped = sample.max(-1.0).min(1.0);

        // Try to push, drop if buffer is full
        let _ = producer.try_push(clamped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demodulate_fm() {
        // Create a simple test signal
        let samples: Vec<Complex<f32>> = (0..100)
            .map(|i| {
                let phase = i as f32 * 0.1;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let audio = demodulate_fm(&samples, false);
        assert_eq!(audio.len(), samples.len() - 1);
    }

    #[test]
    fn test_demodulate_am() {
        let samples: Vec<Complex<f32>> = (0..100)
            .map(|i| {
                let amp = (i as f32 * 0.1).sin().abs();
                Complex::new(amp, 0.0)
            })
            .collect();

        let audio = demodulate_am(&samples);
        assert_eq!(audio.len(), samples.len());
    }

    #[test]
    fn test_lowpass_filter() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let filtered = lowpass_filter(&input, 3);

        assert_eq!(filtered.len(), input.len());
        // Middle value should be average of surrounding values
        assert!((filtered[2] - 3.0).abs() < 0.1);
    }
}
