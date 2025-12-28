use std::path::PathBuf;

/// Commands sent from UI thread to control the application
#[derive(Debug, Clone)]
pub enum Command {
    // SDR Control Commands
    SetFrequency(u32),
    IncreaseFrequency(i32),
    DecreaseFrequency(i32),
    SetSampleRate(u32),
    SetTunerGain(i32),
    SetAutoGain(bool),
    SetPpmError(i32),

    // Demodulation Mode Commands
    SetMode(DemodMode),

    // Recording Commands
    StartRecording(PathBuf),
    StopRecording,

    // Application Commands
    Quit,
}

/// Demodulation modes supported by the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemodMode {
    /// Raw IQ samples, no demodulation
    Raw,
    /// Frequency Modulation (FM) - Narrowband
    FmNarrow,
    /// Frequency Modulation (FM) - Wideband
    FmWide,
    /// Amplitude Modulation (AM)
    Am,
    /// Single Sideband - Upper Sideband
    Usb,
    /// Single Sideband - Lower Sideband
    Lsb,
    /// APRS (Automatic Packet Reporting System) decoder
    Aprs,
    /// ADS-B (Automatic Dependent Surveillance-Broadcast) decoder
    Adsb,
}

impl DemodMode {
    /// Get human-readable name for the mode
    pub fn name(&self) -> &'static str {
        match self {
            DemodMode::Raw => "RAW",
            DemodMode::FmNarrow => "FM-NFM",
            DemodMode::FmWide => "FM-WFM",
            DemodMode::Am => "AM",
            DemodMode::Usb => "USB",
            DemodMode::Lsb => "LSB",
            DemodMode::Aprs => "APRS",
            DemodMode::Adsb => "ADS-B",
        }
    }

    /// Get all available modes
    pub fn all() -> &'static [DemodMode] {
        &[
            DemodMode::Raw,
            DemodMode::FmNarrow,
            DemodMode::FmWide,
            DemodMode::Am,
            DemodMode::Usb,
            DemodMode::Lsb,
            DemodMode::Aprs,
            DemodMode::Adsb,
        ]
    }
}

impl Default for DemodMode {
    fn default() -> Self {
        DemodMode::FmNarrow
    }
}
