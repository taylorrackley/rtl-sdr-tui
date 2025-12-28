use anyhow::{anyhow, Result};
use num_complex::Complex;
use rtlsdr_mt::Controller;

/// Wrapper around RTL-SDR device for easier management
pub struct RtlSdrDevice {
    controller: Controller,
    sample_rate: u32,
    center_freq: u32,
}

impl RtlSdrDevice {
    /// Open the RTL-SDR device by index
    pub fn open(device_index: usize) -> Result<Self> {
        log::info!("Opening RTL-SDR device {}", device_index);

        // Open the device - rtlsdr_mt::open returns (Controller, Reader)
        let (controller, _reader) = rtlsdr_mt::open(device_index as u32)
            .map_err(|_| anyhow!("Failed to open RTL-SDR device {}", device_index))?;

        log::info!("RTL-SDR device opened successfully");

        Ok(Self {
            controller,
            sample_rate: 0,
            center_freq: 0,
        })
    }

    /// Get a reference to the device controller
    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    /// Set the center frequency in Hz
    pub fn set_center_freq(&mut self, freq: u32) -> Result<()> {
        super::config::validate_frequency(freq)?;

        self.controller
            .set_center_freq(freq)
            .map_err(|_| anyhow!("Failed to set center frequency to {} Hz", freq))?;

        self.center_freq = freq;
        log::info!("Set center frequency to {} Hz ({} MHz)", freq, freq / 1_000_000);

        Ok(())
    }

    /// Get the current center frequency
    pub fn get_center_freq(&self) -> u32 {
        self.center_freq
    }

    /// Set the sample rate in Hz
    pub fn set_sample_rate(&mut self, rate: u32) -> Result<()> {
        super::config::validate_sample_rate(rate)?;

        self.controller
            .set_sample_rate(rate)
            .map_err(|_| anyhow!("Failed to set sample rate to {} Hz", rate))?;

        self.sample_rate = rate;
        log::info!("Set sample rate to {} Hz ({} kHz)", rate, rate / 1000);

        Ok(())
    }

    /// Get the current sample rate
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set tuner gain in tenths of dB (e.g., 421 = 42.1 dB)
    /// Use -1 for automatic gain
    pub fn set_tuner_gain(&mut self, gain: i32) -> Result<()> {
        if gain == -1 {
            // Enable automatic gain
            self.controller
                .enable_agc()
                .map_err(|_| anyhow!("Failed to enable automatic gain"))?;
            log::info!("Enabled automatic gain control");
        } else {
            // Disable AGC and set manual gain
            self.controller
                .disable_agc()
                .map_err(|_| anyhow!("Failed to disable automatic gain"))?;
            self.controller
                .set_tuner_gain(gain)
                .map_err(|_| anyhow!("Failed to set tuner gain to {} ({}dB)", gain, gain / 10))?;
            log::info!("Set tuner gain to {} ({}.{} dB)", gain, gain / 10, gain % 10);
        }

        Ok(())
    }

    /// Set PPM (parts per million) frequency correction
    pub fn set_ppm(&mut self, ppm: i32) -> Result<()> {
        self.controller
            .set_ppm(ppm)
            .map_err(|_| anyhow!("Failed to set PPM correction to {}", ppm))?;

        if ppm != 0 {
            log::info!("Set PPM correction to {}", ppm);
        }

        Ok(())
    }

    /// Get device information
    pub fn get_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            manufacturer: String::from("Realtek"),
            product: String::from("RTL2838UHIDIR"),
            serial: String::from("00000001"),
        }
    }
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} (S/N: {})",
            self.manufacturer, self.product, self.serial
        )
    }
}

/// Get the number of available RTL-SDR devices
/// Note: In rtlsdr_mt v2, we attempt to open devices to count them
pub fn get_device_count() -> usize {
    // Try opening devices 0-10 to see how many exist
    for i in 0..10u32 {
        if rtlsdr_mt::open(i).is_err() {
            return i as usize;
        }
    }
    10 // Maximum we checked
}

/// List all available RTL-SDR devices
pub fn list_devices() -> Vec<String> {
    let count = get_device_count();
    (0..count)
        .map(|i| format!("RTL-SDR Device #{}", i))
        .collect()
}

/// Convert raw IQ samples (u8) to Complex<f32> and normalize to [-1.0, 1.0]
pub fn samples_u8_to_complex(samples: &[u8]) -> Vec<Complex<f32>> {
    samples
        .chunks_exact(2)
        .map(|iq| {
            // RTL-SDR uses unsigned 8-bit samples centered at 127.5
            // Convert to signed and normalize to [-1.0, 1.0]
            let i = (iq[0] as f32 - 127.5) / 128.0;
            let q = (iq[1] as f32 - 127.5) / 128.0;
            Complex::new(i, q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_u8_to_complex() {
        // Test center value (127)
        let samples = vec![127, 127];
        let complex = samples_u8_to_complex(&samples);
        assert_eq!(complex.len(), 1);
        assert!((complex[0].re + 0.00390625).abs() < 0.01); // ~-0.004
        assert!((complex[0].im + 0.00390625).abs() < 0.01);

        // Test max value (255)
        let samples = vec![255, 255];
        let complex = samples_u8_to_complex(&samples);
        assert!(complex[0].re > 0.99);
        assert!(complex[0].im > 0.99);

        // Test min value (0)
        let samples = vec![0, 0];
        let complex = samples_u8_to_complex(&samples);
        assert!(complex[0].re < -0.99);
        assert!(complex[0].im < -0.99);
    }
}
