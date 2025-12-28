/// Simple linear interpolation resampler
pub struct Resampler {
    /// Input sample rate
    input_rate: u32,
    /// Output sample rate
    output_rate: u32,
    /// Resampling ratio (output / input)
    ratio: f32,
    /// Accumulated phase
    phase: f32,
}

impl Resampler {
    /// Create a new resampler
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let ratio = output_rate as f32 / input_rate as f32;

        Self {
            input_rate,
            output_rate,
            ratio,
            phase: 0.0,
        }
    }

    /// Resample audio samples
    ///
    /// Uses linear interpolation for simplicity.
    /// For production use, consider using a proper polyphase filter.
    pub fn resample(&mut self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return vec![];
        }

        // Calculate expected output size
        let output_len = (input.len() as f32 * self.ratio) as usize;
        let mut output = Vec::with_capacity(output_len);

        let mut pos = self.phase;

        while pos < input.len() as f32 - 1.0 {
            let idx = pos as usize;
            let frac = pos - idx as f32;

            // Linear interpolation
            let sample = input[idx] * (1.0 - frac) + input[idx + 1] * frac;
            output.push(sample);

            pos += 1.0 / self.ratio;
        }

        // Save phase for next call (for continuity between buffers)
        self.phase = pos - (input.len() - 1) as f32;
        if self.phase < 0.0 {
            self.phase = 0.0;
        }

        output
    }

    /// Reset the resampler state
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Get the resampling ratio
    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    /// Set new sample rates
    pub fn set_rates(&mut self, input_rate: u32, output_rate: u32) {
        self.input_rate = input_rate;
        self.output_rate = output_rate;
        self.ratio = output_rate as f32 / input_rate as f32;
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_downsample() {
        let mut resampler = Resampler::new(48000, 24000); // 2:1 downsampling

        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let output = resampler.resample(&input);

        // Should have roughly half the samples
        assert!(output.len() >= 45 && output.len() <= 55);
    }

    #[test]
    fn test_resampler_upsample() {
        let mut resampler = Resampler::new(24000, 48000); // 1:2 upsampling

        let input: Vec<f32> = (0..50).map(|i| i as f32).collect();
        let output = resampler.resample(&input);

        // Should have roughly double the samples
        assert!(output.len() >= 95 && output.len() <= 105);
    }

    #[test]
    fn test_resampler_reset() {
        let mut resampler = Resampler::new(48000, 44100);

        let input = vec![1.0; 100];
        let _ = resampler.resample(&input);

        resampler.reset();
        assert_eq!(resampler.phase, 0.0);
    }

    #[test]
    fn test_resampler_ratio() {
        let resampler = Resampler::new(48000, 24000);
        assert!((resampler.ratio() - 0.5).abs() < 0.001);

        let resampler = Resampler::new(24000, 48000);
        assert!((resampler.ratio() - 2.0).abs() < 0.001);
    }
}
