use super::app::App;
use crate::state::ControlId;
use crate::types::{Command, DemodMode};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

/// Handle keyboard input events
pub fn handle_input(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            handle_key_event(app, key)?;
        }
    }
    Ok(())
}

/// Handle a single key event
fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // Global key bindings (work regardless of selected control)
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.quit();
            return Ok(());
        }

        // Toggle recording
        (KeyCode::Char('r'), KeyModifiers::NONE) => {
            toggle_recording(app)?;
            return Ok(());
        }

        // Navigation between controls
        (KeyCode::Tab, KeyModifiers::NONE) => {
            let current = app.state.read().ui.selected_control;
            app.state.write().ui.selected_control = current.next();
            return Ok(());
        }
        (KeyCode::BackTab, KeyModifiers::SHIFT) => {
            let current = app.state.read().ui.selected_control;
            app.state.write().ui.selected_control = current.prev();
            return Ok(());
        }

        _ => {}
    }

    // Control-specific key bindings
    let selected = app.state.read().ui.selected_control;
    match selected {
        ControlId::Frequency => handle_frequency_keys(app, key)?,
        ControlId::Mode => handle_mode_keys(app, key)?,
        ControlId::Gain => handle_gain_keys(app, key)?,
        ControlId::SampleRate => handle_sample_rate_keys(app, key)?,
        ControlId::Record => handle_record_keys(app, key)?,
    }

    Ok(())
}

/// Handle frequency control keys
fn handle_frequency_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            // Increase frequency by 100 kHz
            app.send_command(Command::IncreaseFrequency(100_000))?;
            app.set_status("Frequency +100 kHz");
        }
        KeyCode::Down | KeyCode::Char('j') => {
            // Decrease frequency by 100 kHz
            app.send_command(Command::DecreaseFrequency(100_000))?;
            app.set_status("Frequency -100 kHz");
        }
        KeyCode::Right | KeyCode::Char('l') => {
            // Increase frequency by 1 MHz
            app.send_command(Command::IncreaseFrequency(1_000_000))?;
            app.set_status("Frequency +1 MHz");
        }
        KeyCode::Left | KeyCode::Char('h') => {
            // Decrease frequency by 1 MHz
            app.send_command(Command::DecreaseFrequency(1_000_000))?;
            app.set_status("Frequency -1 MHz");
        }
        _ => {}
    }
    Ok(())
}

/// Handle mode control keys
fn handle_mode_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    let current_mode = app.get_mode();
    let modes = DemodMode::all();
    let current_idx = modes.iter().position(|&m| m == current_mode).unwrap_or(0);

    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Right | KeyCode::Char('l') => {
            let next_idx = (current_idx + 1) % modes.len();
            let next_mode = modes[next_idx];
            app.send_command(Command::SetMode(next_mode))?;
            app.set_status(format!("Mode: {}", next_mode.name()));
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Left | KeyCode::Char('h') => {
            let prev_idx = if current_idx == 0 {
                modes.len() - 1
            } else {
                current_idx - 1
            };
            let prev_mode = modes[prev_idx];
            app.send_command(Command::SetMode(prev_mode))?;
            app.set_status(format!("Mode: {}", prev_mode.name()));
        }
        _ => {}
    }
    Ok(())
}

/// Handle gain control keys
fn handle_gain_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    let current_gain = app.get_gain();

    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Right | KeyCode::Char('l') => {
            if current_gain == -1 {
                // Switch from auto to manual (start at 200 = 20.0 dB)
                app.send_command(Command::SetTunerGain(200))?;
                app.set_status("Gain: 20.0 dB");
            } else {
                // Increase gain by 5 dB (50 tenths)
                let new_gain = (current_gain + 50).min(500);
                app.send_command(Command::SetTunerGain(new_gain))?;
                app.set_status(format!("Gain: {}.{} dB", new_gain / 10, new_gain % 10));
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Left | KeyCode::Char('h') => {
            if current_gain == -1 {
                // Already on auto
            } else {
                // Decrease gain by 5 dB (50 tenths)
                let new_gain = (current_gain - 50).max(0);
                app.send_command(Command::SetTunerGain(new_gain))?;
                app.set_status(format!("Gain: {}.{} dB", new_gain / 10, new_gain % 10));
            }
        }
        KeyCode::Char('a') => {
            // Toggle auto gain
            app.send_command(Command::SetAutoGain(true))?;
            app.set_status("Gain: Auto");
        }
        _ => {}
    }
    Ok(())
}

/// Handle sample rate control keys
fn handle_sample_rate_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    let rates = crate::sdr::config::COMMON_SAMPLE_RATES;
    let current_rate = app.get_sample_rate();
    let current_idx = rates.iter().position(|&r| r == current_rate).unwrap_or(6); // Default to 2.048 MHz

    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Right | KeyCode::Char('l') => {
            let next_idx = (current_idx + 1).min(rates.len() - 1);
            let next_rate = rates[next_idx];
            app.send_command(Command::SetSampleRate(next_rate))?;
            app.set_status(format!("Sample Rate: {} kHz", next_rate / 1000));
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Left | KeyCode::Char('h') => {
            let prev_idx = current_idx.saturating_sub(1);
            let prev_rate = rates[prev_idx];
            app.send_command(Command::SetSampleRate(prev_rate))?;
            app.set_status(format!("Sample Rate: {} kHz", prev_rate / 1000));
        }
        _ => {}
    }
    Ok(())
}

/// Handle record control keys
fn handle_record_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter | KeyCode::Char(' ') => {
            toggle_recording(app)?;
        }
        _ => {}
    }
    Ok(())
}

/// Toggle recording on/off
fn toggle_recording(app: &mut App) -> Result<()> {
    let is_recording = app.is_recording();
    if is_recording {
        app.send_command(Command::StopRecording)?;
        app.set_status("Recording stopped");
    } else {
        // Generate filename with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("recording_{}.iq", timestamp);
        let path = std::path::PathBuf::from(filename);
        app.send_command(Command::StartRecording(path))?;
        app.set_status("Recording started");
    }
    Ok(())
}
