/// RTL-SDR specific configuration constants and utilities

/// Default RTL-SDR configuration values
pub mod defaults {
    /// Default center frequency (144.390 MHz - APRS)
    pub const FREQUENCY: u32 = 144_390_000;

    /// Default sample rate (2.048 MHz)
    pub const SAMPLE_RATE: u32 = 2_048_000;

    /// Automatic gain (-1)
    pub const AUTO_GAIN: i32 = -1;

    /// Default PPM correction
    pub const PPM_ERROR: i32 = 0;
}

/// RTL-SDR hardware constraints
pub mod constraints {
    /// Minimum frequency supported by RTL-SDR (24 MHz)
    pub const MIN_FREQUENCY: u32 = 24_000_000;

    /// Maximum frequency supported by RTL-SDR (1.766 GHz)
    pub const MAX_FREQUENCY: u32 = 1_766_000_000;

    /// Minimum sample rate (225 kHz)
    pub const MIN_SAMPLE_RATE: u32 = 225_000;

    /// Maximum sample rate (3.2 MHz)
    pub const MAX_SAMPLE_RATE: u32 = 3_200_000;
}

/// Common RTL-SDR sample rates that work well
pub const COMMON_SAMPLE_RATES: &[u32] = &[
    225_000,    // 225 kHz
    900_000,    // 900 kHz
    1_024_000,  // 1.024 MHz
    1_400_000,  // 1.4 MHz
    1_800_000,  // 1.8 MHz
    1_920_000,  // 1.92 MHz
    2_048_000,  // 2.048 MHz (default, good for most applications)
    2_400_000,  // 2.4 MHz
    2_560_000,  // 2.56 MHz
    2_800_000,  // 2.8 MHz
    3_200_000,  // 3.2 MHz (maximum)
];

/// Common frequency presets
pub struct FrequencyPreset {
    pub name: &'static str,
    pub frequency: u32,
    pub mode: &'static str,
}

pub const FREQUENCY_PRESETS: &[FrequencyPreset] = &[
    FrequencyPreset {
        name: "APRS North America",
        frequency: 144_390_000,
        mode: "FM-NFM",
    },
    FrequencyPreset {
        name: "APRS Europe",
        frequency: 144_800_000,
        mode: "FM-NFM",
    },
    FrequencyPreset {
        name: "ADS-B Aircraft",
        frequency: 1_090_000_000,
        mode: "ADS-B",
    },
    FrequencyPreset {
        name: "NOAA Weather 1",
        frequency: 162_550_000,
        mode: "FM-WFM",
    },
    FrequencyPreset {
        name: "NOAA Weather 2",
        frequency: 162_400_000,
        mode: "FM-WFM",
    },
    FrequencyPreset {
        name: "FM Broadcast",
        frequency: 98_500_000,
        mode: "FM-WFM",
    },
    FrequencyPreset {
        name: "ISS APRS Downlink",
        frequency: 145_825_000,
        mode: "FM-NFM",
    },
];

/// Validate frequency is within RTL-SDR range
pub fn validate_frequency(freq: u32) -> anyhow::Result<()> {
    if freq < constraints::MIN_FREQUENCY {
        anyhow::bail!(
            "Frequency {} Hz is below minimum {} Hz",
            freq,
            constraints::MIN_FREQUENCY
        );
    } else if freq > constraints::MAX_FREQUENCY {
        anyhow::bail!(
            "Frequency {} Hz is above maximum {} Hz",
            freq,
            constraints::MAX_FREQUENCY
        );
    }
    Ok(())
}

/// Validate sample rate is within RTL-SDR range
pub fn validate_sample_rate(rate: u32) -> anyhow::Result<()> {
    if rate < constraints::MIN_SAMPLE_RATE {
        anyhow::bail!(
            "Sample rate {} Hz is below minimum {} Hz",
            rate,
            constraints::MIN_SAMPLE_RATE
        );
    } else if rate > constraints::MAX_SAMPLE_RATE {
        anyhow::bail!(
            "Sample rate {} Hz is above maximum {} Hz",
            rate,
            constraints::MAX_SAMPLE_RATE
        );
    }

    // Warn if not a common sample rate
    if !COMMON_SAMPLE_RATES.contains(&rate) {
        log::warn!(
            "Sample rate {} Hz is not a common RTL-SDR rate, may cause issues",
            rate
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_frequency() {
        assert!(validate_frequency(144_390_000).is_ok());
        assert!(validate_frequency(1_090_000_000).is_ok());
        assert!(validate_frequency(1_000_000).is_err());
        assert!(validate_frequency(2_000_000_000).is_err());
    }

    #[test]
    fn test_validate_sample_rate() {
        assert!(validate_sample_rate(2_048_000).is_ok());
        assert!(validate_sample_rate(100_000).is_err());
        assert!(validate_sample_rate(5_000_000).is_err());
    }
}
