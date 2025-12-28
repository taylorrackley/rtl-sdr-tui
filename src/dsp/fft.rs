use num_complex::Complex;
use rustfft::{FftPlanner, num_complex::Complex32};
use std::f32::consts::PI;

/// FFT processor for spectrum analysis
pub struct FftProcessor {
    /// FFT size
    size: usize,
    /// FFT planner (reused for efficiency)
    planner: FftPlanner<f32>,
    /// Input buffer for FFT
    input_buffer: Vec<Complex32>,
    /// Output buffer for FFT
    output_buffer: Vec<Complex32>,
    /// Window function coefficients
    window: Vec<f32>,
}

impl FftProcessor {
    /// Create a new FFT processor
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let window = Self::hann_window(size);

        Self {
            size,
            planner,
            input_buffer: vec![Complex32::new(0.0, 0.0); size],
            output_buffer: vec![Complex32::new(0.0, 0.0); size],
            window,
        }
    }

    /// Process IQ samples and return FFT magnitude in dB
    pub fn process(&mut self, samples: &[Complex<f32>]) -> Vec<f32> {
        // Take only the required number of samples
        let sample_count = samples.len().min(self.size);

        // Apply window function and copy to input buffer
        for i in 0..sample_count {
            self.input_buffer[i] = samples[i] * self.window[i];
        }

        // Zero-pad if needed
        for i in sample_count..self.size {
            self.input_buffer[i] = Complex32::new(0.0, 0.0);
        }

        // Compute FFT
        let fft = self.planner.plan_fft_forward(self.size);
        self.output_buffer.copy_from_slice(&self.input_buffer);
        fft.process(&mut self.output_buffer);

        // Convert to magnitude in dB and apply FFT shift
        self.fft_shift_and_magnitude()
    }

    /// Apply FFT shift (move DC to center) and convert to dB magnitude
    fn fft_shift_and_magnitude(&self) -> Vec<f32> {
        let mut result = vec![0.0; self.size];
        let half = self.size / 2;

        for i in 0..self.size {
            // FFT shift: move second half to first half and vice versa
            let shifted_idx = if i < half { i + half } else { i - half };

            // Calculate magnitude
            let magnitude = self.output_buffer[i].norm();

            // Convert to dB (with floor to avoid log(0))
            let db = if magnitude > 1e-10 {
                20.0 * magnitude.log10()
            } else {
                -100.0 // Floor at -100 dB
            };

            result[shifted_idx] = db;
        }

        result
    }

    /// Generate Hann window coefficients
    fn hann_window(size: usize) -> Vec<f32> {
        (0..size)
            .map(|i| {
                let angle = 2.0 * PI * i as f32 / (size - 1) as f32;
                0.5 * (1.0 - angle.cos())
            })
            .collect()
    }

    /// Get FFT size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Generate test signal (for demonstration)
    pub fn generate_test_signal(
        size: usize,
        sample_rate: u32,
        frequencies: &[(f32, f32)], // (frequency_hz, amplitude)
    ) -> Vec<Complex<f32>> {
        let mut signal = vec![Complex::new(0.0, 0.0); size];

        for (freq_hz, amplitude) in frequencies {
            let omega = 2.0 * PI * freq_hz / sample_rate as f32;

            for (i, sample) in signal.iter_mut().enumerate() {
                let phase = omega * i as f32;
                *sample += Complex::new(
                    amplitude * phase.cos(),
                    amplitude * phase.sin(),
                );
            }
        }

        // Add some noise
        for sample in signal.iter_mut() {
            let noise_i = (rand::random::<f32>() - 0.5) * 0.1;
            let noise_q = (rand::random::<f32>() - 0.5) * 0.1;
            *sample += Complex::new(noise_i, noise_q);
        }

        signal
    }
}

/// Utility function to normalize FFT output to a specified range
pub fn normalize_fft(fft_data: &[f32], min_db: f32, max_db: f32) -> Vec<f32> {
    fft_data
        .iter()
        .map(|&db| {
            // Clamp to range
            let clamped = db.max(min_db).min(max_db);
            // Normalize to 0.0-1.0
            (clamped - min_db) / (max_db - min_db)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_processor() {
        let mut processor = FftProcessor::new(1024);

        // Generate a test signal with a single frequency
        let signal = FftProcessor::generate_test_signal(
            1024,
            2_048_000,
            &[(100_000.0, 1.0)], // 100 kHz tone
        );

        // Process the signal
        let spectrum = processor.process(&signal);

        // Verify output size
        assert_eq!(spectrum.len(), 1024);

        // Verify there's a peak somewhere (basic sanity check)
        let max_value = spectrum.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max_value > -50.0); // Should have a significant peak
    }

    #[test]
    fn test_normalize_fft() {
        let data = vec![-100.0, -80.0, -60.0, -40.0, -20.0, 0.0];
        let normalized = normalize_fft(&data, -100.0, 0.0);

        assert!((normalized[0] - 0.0).abs() < 0.01);
        assert!((normalized[5] - 1.0).abs() < 0.01);
        assert!((normalized[2] - 0.4).abs() < 0.01);
    }
}
