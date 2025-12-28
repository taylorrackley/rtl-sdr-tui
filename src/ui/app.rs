use crate::state::SharedState;
use crate::types::Command;
use anyhow::Result;
use crossbeam::channel::Sender;

/// TUI Application structure
pub struct App {
    /// Shared application state
    pub state: SharedState,
    /// Command sender to control threads
    pub command_tx: Option<Sender<Command>>,
}

impl App {
    /// Create a new TUI application
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            command_tx: None,
        }
    }

    /// Set the command sender for controlling threads
    pub fn set_command_tx(&mut self, tx: Sender<Command>) {
        self.command_tx = Some(tx);
    }

    /// Send a command to the application threads
    pub fn send_command(&self, command: Command) -> Result<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(command)?;
        }
        Ok(())
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.state.read().ui.should_quit
    }

    /// Handle application quit
    pub fn quit(&mut self) {
        self.state.write().ui.should_quit = true;
        let _ = self.send_command(Command::Quit);
    }

    /// Update status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.state.write().ui.status_message = message.into();
    }

    /// Get current frequency in Hz
    pub fn get_frequency(&self) -> u32 {
        self.state.read().sdr.frequency
    }

    /// Get current sample rate in Hz
    pub fn get_sample_rate(&self) -> u32 {
        self.state.read().sdr.sample_rate
    }

    /// Get current mode
    pub fn get_mode(&self) -> crate::types::DemodMode {
        self.state.read().decoder.mode
    }

    /// Get current gain
    pub fn get_gain(&self) -> i32 {
        self.state.read().sdr.tuner_gain
    }

    /// Check if recording is active
    pub fn is_recording(&self) -> bool {
        self.state.read().recording.is_recording
    }

    /// Get status message
    pub fn get_status(&self) -> String {
        self.state.read().ui.status_message.clone()
    }
}
