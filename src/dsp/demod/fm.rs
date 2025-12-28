use num_complex::Complex;
use std::f32::consts::PI;

/// FM Demodulator
pub struct FmDemodulator {
    /// Previous sample for phase difference calculation
    prev_sample: Complex<f32>,
    /// De-emphasis filter state
    deemph_state: f32,
    /// De-emphasis time constant
    deemph_alpha: f32,
}

impl FmDemodulator {
    /// Create a new FM demodulator
    ///
    /// # Arguments
    /// * `sample_rate` - Input sample rate in Hz
    /// * `tau` - De-emphasis time constant in microseconds (50 for EU, 75 for US)
    pub fn new(sample_rate: u32, tau_us: f32) -> Self {
        // Calculate de-emphasis filter coefficient
        let tau = tau_us * 1e-6; // Convert to seconds
        let deemph_alpha = 1.0 / (1.0 + sample_rate as f32 * tau);

        Self {
            prev_sample: Complex::new(1.0, 0.0),
            deemph_state: 0.0,
            deemph_alpha,
        }
    }

    /// Demodulate FM samples
    ///
    /// Returns demodulated audio samples in the range [-1.0, 1.0]
    pub fn demodulate(&mut self, samples: &[Complex<f32>]) -> Vec<f32> {
        let mut audio = Vec::with_capacity(samples.len());

        for &sample in samples {
            // Calculate phase difference (discriminator)
            let phase_diff = (sample * self.prev_sample.conj()).arg();

            // Normalize to audio range [-1.0, 1.0]
            let mut demod = phase_diff / PI;

            // Apply de-emphasis filter (simple IIR lowpass)
            self.deemph_state = self.deemph_state * (1.0 - self.deemph_alpha)
                + demod * self.deemph_alpha;
            demod = self.deemph_state;

            // Clamp output
            demod = demod.max(-1.0).min(1.0);

            audio.push(demod);
            self.prev_sample = sample;
        }

        audio
    }

    /// Reset the demodulator state
    pub fn reset(&mut self) {
        self.prev_sample = Complex::new(1.0, 0.0);
        self.deemph_state = 0.0;
    }
}

impl Default for FmDemodulator {
    fn default() -> Self {
        // Default to 2.048 MHz sample rate, 75us de-emphasis (US)
        Self::new(2_048_000, 75.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fm_demodulator() {
        let mut demod = FmDemodulator::new(2_048_000, 75.0);

        // Generate a test signal with constant frequency deviation
        let samples: Vec<Complex<f32>> = (0..1000)
            .map(|i| {
                let phase = i as f32 * 0.1;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let audio = demod.demodulate(&samples);

        assert_eq!(audio.len(), samples.len());

        // All audio samples should be in valid range
        for sample in &audio {
            assert!(sample.abs() <= 1.0);
        }
    }

    #[test]
    fn test_fm_demodulator_reset() {
        let mut demod = FmDemodulator::new(2_048_000, 75.0);

        let samples: Vec<Complex<f32>> = (0..10)
            .map(|i| {
                let phase = i as f32 * 0.5;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let _ = demod.demodulate(&samples);

        // Reset should clear state
        demod.reset();
        assert_eq!(demod.prev_sample, Complex::new(1.0, 0.0));
        assert_eq!(demod.deemph_state, 0.0);
    }
}
