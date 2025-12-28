use super::FftProcessor;
use crate::state::SharedState;
use crate::types::DemodMode;
use crossbeam::channel::{Receiver, Sender};
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
    stream_tx: Option<Sender<Vec<f32>>>,
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

                    // Demodulate to get audio samples
                    let audio: Option<Vec<f32>> = match mode {
                        DemodMode::FmNarrow | DemodMode::FmWide => {
                            Some(demodulate_fm(&samples, mode == DemodMode::FmWide))
                        }
                        DemodMode::Am => {
                            Some(demodulate_am(&samples))
                        }
                        DemodMode::Usb => {
                            Some(demodulate_ssb(&samples, true))
                        }
                        DemodMode::Lsb => {
                            Some(demodulate_ssb(&samples, false))
                        }
                        DemodMode::Aprs | DemodMode::Adsb => {
                            // Digital modes - demodulate FM for APRS, raw for ADS-B
                            // TODO: Add packet decoding
                            Some(demodulate_fm(&samples, false))
                        }
                        DemodMode::Raw => {
                            // No demodulation, just visualization
                            None
                        }
                    };

                    // Send audio to local output and/or network stream
                    if let Some(ref audio_samples) = audio {
                        // Send to local audio output
                        if let Some(audio_producer) = audio_tx.as_mut() {
                            send_audio_samples(audio_producer, audio_samples);
                        }

                        // Send to network stream
                        if let Some(ref stream) = stream_tx {
                            let _ = stream.try_send(audio_samples.clone());
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

/// FM demodulator using phase difference with de-emphasis
fn demodulate_fm(samples: &[Complex<f32>], wideband: bool) -> Vec<f32> {
    if samples.len() < 2 {
        return vec![];
    }

    let mut audio = Vec::with_capacity(samples.len());

    // FM demodulation via phase difference (polar discriminator)
    for window in samples.windows(2) {
        // Phase difference between consecutive samples
        // This gives us the instantaneous frequency deviation
        let phase_diff = (window[1] * window[0].conj()).arg();

        // Normalize to audio range
        let sample = phase_diff / std::f32::consts::PI;
        audio.push(sample);
    }

    // Apply lowpass filtering
    // Wideband FM (broadcast): ~15 kHz audio bandwidth
    // Narrowband FM (NOAA, voice): ~3 kHz audio bandwidth
    let filter_size = if wideband { 8 } else { 4 };
    let filtered = lowpass_filter(&audio, filter_size);

    // Apply de-emphasis filter (75µs for NA, 50µs for EU)
    // This compensates for the pre-emphasis used in FM transmission
    // Improves audio quality significantly for NOAA and FM broadcast
    apply_deemphasis(&filtered, wideband)
}

/// Apply de-emphasis filter to FM audio
/// FM broadcasts use pre-emphasis to boost high frequencies
/// We need de-emphasis to restore flat frequency response
fn apply_deemphasis(input: &[f32], wideband: bool) -> Vec<f32> {
    if input.is_empty() {
        return vec![];
    }

    // De-emphasis time constant
    // 75µs for North America, 50µs for Europe
    // Using 75µs as default (good for NOAA in NA)
    let tau = if wideband { 75e-6 } else { 50e-6 };

    // Assume ~48kHz sample rate after decimation
    let sample_rate = 48000.0;

    // Single-pole IIR lowpass filter coefficient
    // alpha = 1 / (1 + 2*pi*tau*fs)
    let alpha = 1.0 / (1.0 + 2.0 * std::f32::consts::PI * tau * sample_rate);

    let mut output = Vec::with_capacity(input.len());
    let mut prev = input[0];

    for &sample in input.iter() {
        // IIR filter: y[n] = alpha * x[n] + (1 - alpha) * y[n-1]
        let filtered = alpha * sample + (1.0 - alpha) * prev;
        output.push(filtered);
        prev = filtered;
    }

    output
}

/// Simple AM demodulator using envelope detection
fn demodulate_am(samples: &[Complex<f32>]) -> Vec<f32> {
    // Envelope detection with DC removal
    let envelope: Vec<f32> = samples.iter().map(|s| s.norm()).collect();

    // Remove DC offset
    let dc: f32 = envelope.iter().sum::<f32>() / envelope.len() as f32;
    envelope.iter().map(|s| s - dc).collect()
}

/// SSB (Single Sideband) demodulator
/// For USB: use upper sideband (positive frequencies)
/// For LSB: use lower sideband (negative frequencies)
fn demodulate_ssb(samples: &[Complex<f32>], upper: bool) -> Vec<f32> {
    // SSB demodulation using the Weaver method (simplified)
    // The IQ samples from the SDR already give us the analytic signal
    // For USB: take the real part directly (I channel)
    // For LSB: negate Q before combining (effectively flipping the spectrum)

    let mut audio = Vec::with_capacity(samples.len());

    // Simple SSB demodulation:
    // USB: output = I * cos(wt) + Q * sin(wt) -> for baseband, just I
    // LSB: output = I * cos(wt) - Q * sin(wt) -> for baseband, just I with inverted Q

    // Apply a simple BFO (Beat Frequency Oscillator) mixing
    // This shifts the sideband to audio frequencies
    let bfo_freq = 1500.0; // 1.5 kHz BFO offset for typical SSB
    let sample_rate = 48000.0; // Assumed audio sample rate

    for (i, sample) in samples.iter().enumerate() {
        let t = i as f32 / sample_rate;
        let bfo_phase = 2.0 * std::f32::consts::PI * bfo_freq * t;

        let audio_sample = if upper {
            // USB: mix with positive frequency
            sample.re * bfo_phase.cos() - sample.im * bfo_phase.sin()
        } else {
            // LSB: mix with negative frequency (inverted)
            sample.re * bfo_phase.cos() + sample.im * bfo_phase.sin()
        };

        audio.push(audio_sample);
    }

    // Apply lowpass filter to clean up
    lowpass_filter(&audio, 4)
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
    fn test_demodulate_fm_wideband() {
        let samples: Vec<Complex<f32>> = (0..100)
            .map(|i| {
                let phase = i as f32 * 0.1;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let audio = demodulate_fm(&samples, true);
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
    fn test_demodulate_ssb_usb() {
        let samples: Vec<Complex<f32>> = (0..100)
            .map(|i| {
                let phase = i as f32 * 0.05;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let audio = demodulate_ssb(&samples, true);
        assert_eq!(audio.len(), samples.len());
    }

    #[test]
    fn test_demodulate_ssb_lsb() {
        let samples: Vec<Complex<f32>> = (0..100)
            .map(|i| {
                let phase = i as f32 * 0.05;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let audio = demodulate_ssb(&samples, false);
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

    #[test]
    fn test_deemphasis() {
        let input = vec![1.0, 0.5, 0.0, -0.5, -1.0];
        let output = apply_deemphasis(&input, false);
        assert_eq!(output.len(), input.len());
    }
}
