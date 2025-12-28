use super::commands::DemodMode;

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sdr: SdrConfig,
    pub ui: UiConfig,
    pub audio: AudioConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sdr: SdrConfig::default(),
            ui: UiConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

/// SDR device configuration
#[derive(Debug, Clone)]
pub struct SdrConfig {
    /// Center frequency in Hz
    pub frequency: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Tuner gain in tenths of dB (e.g., 421 = 42.1 dB)
    /// Use -1 for automatic gain
    pub tuner_gain: i32,
    /// PPM (Parts Per Million) frequency correction
    pub ppm_error: i32,
    /// Device index (0 for first device)
    pub device_index: usize,
}

impl Default for SdrConfig {
    fn default() -> Self {
        Self {
            frequency: 144_390_000,  // 144.390 MHz (APRS frequency)
            sample_rate: 2_048_000,  // 2.048 MHz
            tuner_gain: -1,          // Auto gain
            ppm_error: 0,
            device_index: 0,
        }
    }
}

impl SdrConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), String> {
        // RTL-SDR frequency range: 24 MHz to 1.7 GHz
        if self.frequency < 24_000_000 || self.frequency > 1_700_000_000 {
            return Err(format!(
                "Frequency {} Hz is out of range (24 MHz - 1.7 GHz)",
                self.frequency
            ));
        }

        // Common RTL-SDR sample rates
        let valid_sample_rates = [
            225_000, 900_000, 1_024_000, 1_400_000, 1_800_000, 1_920_000, 2_048_000, 2_400_000,
            2_560_000, 2_800_000, 3_200_000,
        ];

        if !valid_sample_rates.contains(&self.sample_rate) {
            log::warn!(
                "Sample rate {} Hz is not a common RTL-SDR rate, may cause issues",
                self.sample_rate
            );
        }

        Ok(())
    }
}

/// UI configuration
#[derive(Debug, Clone)]
pub struct UiConfig {
    /// FFT size for spectrum display
    pub fft_size: usize,
    /// Number of waterfall history lines to keep
    pub waterfall_history: usize,
    /// Target frames per second for UI updates
    pub fps: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            waterfall_history: 500,
            fps: 30,
        }
    }
}

/// Audio output configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Audio sample rate in Hz
    pub sample_rate: u32,
    /// Audio buffer size in samples
    pub buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            buffer_size: 4096,
        }
    }
}

/// Decoded message from digital modes
#[derive(Debug, Clone)]
pub struct DecodedMessage {
    pub mode: DemodMode,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content: String,
}

impl DecodedMessage {
    pub fn new(mode: DemodMode, content: String) -> Self {
        Self {
            mode,
            timestamp: chrono::Utc::now(),
            content,
        }
    }
}
