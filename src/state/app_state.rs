use crate::types::{DecodedMessage, DemodMode};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared application state accessible from all threads
pub type SharedState = Arc<RwLock<AppState>>;

/// Main application state
#[derive(Debug)]
pub struct AppState {
    pub sdr: SdrState,
    pub spectrum: SpectrumState,
    pub decoder: DecoderState,
    pub recording: RecordingState,
    pub ui: UiState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sdr: SdrState::default(),
            spectrum: SpectrumState::default(),
            decoder: DecoderState::default(),
            recording: RecordingState::default(),
            ui: UiState::default(),
        }
    }
}

impl AppState {
    /// Create a new shared state wrapped in Arc<RwLock>
    pub fn new_shared() -> SharedState {
        Arc::new(RwLock::new(Self::default()))
    }
}

/// SDR device state
#[derive(Debug)]
pub struct SdrState {
    /// Center frequency in Hz
    pub frequency: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Tuner gain in tenths of dB (-1 = auto)
    pub tuner_gain: i32,
    /// Automatic gain control enabled
    pub auto_gain: bool,
    /// PPM frequency correction
    pub ppm_error: i32,
    /// Whether the SDR is currently running
    pub is_running: bool,
    /// Device serial number
    pub device_serial: Option<String>,
}

impl Default for SdrState {
    fn default() -> Self {
        Self {
            frequency: 144_390_000,  // 144.390 MHz (APRS)
            sample_rate: 2_048_000,  // 2.048 MHz
            tuner_gain: -1,          // Auto gain
            auto_gain: true,
            ppm_error: 0,
            is_running: false,
            device_serial: None,
        }
    }
}

/// Spectrum analyzer and waterfall state
#[derive(Debug)]
pub struct SpectrumState {
    /// Current FFT magnitude data (in dB)
    pub fft_data: Vec<f32>,
    /// Waterfall history (ring buffer of FFT data)
    pub waterfall: Vec<Vec<f32>>,
    /// Current index in waterfall ring buffer
    pub waterfall_index: usize,
    /// Maximum waterfall history size
    pub max_waterfall_history: usize,
}

impl Default for SpectrumState {
    fn default() -> Self {
        Self {
            fft_data: vec![],
            waterfall: vec![],
            waterfall_index: 0,
            max_waterfall_history: 500,
        }
    }
}

impl SpectrumState {
    /// Add new FFT data to waterfall
    pub fn add_fft_data(&mut self, data: Vec<f32>) {
        self.fft_data = data.clone();

        // Initialize waterfall if empty
        if self.waterfall.is_empty() {
            self.waterfall = vec![vec![0.0; data.len()]; self.max_waterfall_history];
        }

        // Add to ring buffer
        if self.waterfall_index < self.waterfall.len() {
            self.waterfall[self.waterfall_index] = data;
            self.waterfall_index = (self.waterfall_index + 1) % self.waterfall.len();
        }
    }

    /// Get waterfall data in display order (oldest to newest)
    pub fn get_waterfall_display(&self) -> Vec<&Vec<f32>> {
        if self.waterfall.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(self.waterfall.len());

        // Add from current index to end (oldest data)
        for i in self.waterfall_index..self.waterfall.len() {
            result.push(&self.waterfall[i]);
        }

        // Add from start to current index (newest data)
        for i in 0..self.waterfall_index {
            result.push(&self.waterfall[i]);
        }

        result
    }
}

/// Digital decoder state
#[derive(Debug)]
pub struct DecoderState {
    /// Current demodulation mode
    pub mode: DemodMode,
    /// Recent decoded messages
    pub messages: Vec<DecodedMessage>,
    /// Maximum number of messages to keep
    pub max_messages: usize,
}

impl Default for DecoderState {
    fn default() -> Self {
        Self {
            mode: DemodMode::default(),
            messages: Vec::new(),
            max_messages: 100,
        }
    }
}

impl DecoderState {
    /// Add a new decoded message
    pub fn add_message(&mut self, message: DecodedMessage) {
        self.messages.push(message);

        // Keep only the most recent messages
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
}

/// Recording state
#[derive(Debug)]
pub struct RecordingState {
    /// Whether recording is currently active
    pub is_recording: bool,
    /// Path to the recording file
    pub file_path: Option<PathBuf>,
    /// Number of samples recorded
    pub samples_recorded: u64,
    /// Recording start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            file_path: None,
            samples_recorded: 0,
            start_time: None,
        }
    }
}

impl RecordingState {
    /// Start recording to a file
    pub fn start(&mut self, path: PathBuf) {
        self.is_recording = true;
        self.file_path = Some(path);
        self.samples_recorded = 0;
        self.start_time = Some(chrono::Utc::now());
    }

    /// Stop recording
    pub fn stop(&mut self) {
        self.is_recording = false;
        self.file_path = None;
        self.start_time = None;
    }
}

/// UI state
#[derive(Debug)]
pub struct UiState {
    /// Currently selected control element
    pub selected_control: ControlId,
    /// Status bar message
    pub status_message: String,
    /// Whether the application should quit
    pub should_quit: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_control: ControlId::Frequency,
            status_message: String::from("Ready"),
            should_quit: false,
        }
    }
}

/// Control element identifiers for UI navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlId {
    Frequency,
    Mode,
    Gain,
    SampleRate,
    Record,
}

impl ControlId {
    /// Get all control IDs
    pub fn all() -> &'static [ControlId] {
        &[
            ControlId::Frequency,
            ControlId::Mode,
            ControlId::Gain,
            ControlId::SampleRate,
            ControlId::Record,
        ]
    }

    /// Get the next control in the cycle
    pub fn next(&self) -> Self {
        let all = Self::all();
        let current_idx = all.iter().position(|&c| c == *self).unwrap_or(0);
        all[(current_idx + 1) % all.len()]
    }

    /// Get the previous control in the cycle
    pub fn prev(&self) -> Self {
        let all = Self::all();
        let current_idx = all.iter().position(|&c| c == *self).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            all.len() - 1
        } else {
            current_idx - 1
        };
        all[prev_idx]
    }
}
