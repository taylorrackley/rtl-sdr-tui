// Module declarations
mod audio;
mod dsp;
mod recorder;
mod sdr;
mod state;
mod types;
mod ui;

use anyhow::Result;
use state::AppState;
use ui::App;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    log::info!("RTL-SDR TUI v0.1.0 starting...");

    // Run the application
    if let Err(e) = run() {
        log::error!("Application error: {}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run() -> Result<()> {
    // Initialize shared state
    let state = AppState::new_shared();

    // Initialize the UI app
    let mut app = App::new(state);

    // Initialize terminal
    let mut terminal = ui::init()?;

    // Main application loop
    loop {
        // Render UI
        ui::render(&mut terminal, &app)?;

        // Handle input
        ui::input::handle_input(&mut app)?;

        // Check if we should quit
        if app.should_quit() {
            break;
        }
    }

    // Restore terminal
    ui::restore()?;

    log::info!("RTL-SDR TUI shutting down");
    Ok(())
}
