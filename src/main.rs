// Module declarations
mod audio;
mod dsp;
mod recorder;
mod sdr;
mod state;
mod types;
mod ui;

use anyhow::Result;
use audio::AudioOutput;
use crossbeam::channel;
use ringbuf::{traits::Split, HeapRb};
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

    // Create shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));

    // Create channel for IQ samples (SDR -> DSP)
    let (samples_tx, samples_rx) = channel::bounded(64);

    // Create channel for commands (UI -> SDR)
    let (command_tx, command_rx) = channel::unbounded();

    // Create ring buffer for audio (DSP -> Audio)
    const AUDIO_BUFFER_SIZE: usize = 48000; // 1 second at 48kHz
    let audio_ring = HeapRb::<f32>::new(AUDIO_BUFFER_SIZE);
    let (audio_producer, audio_consumer) = audio_ring.split();

    // Start SDR thread (currently simulated data)
    log::info!("Starting SDR thread...");
    let sdr_thread = sdr::start_sdr_thread(
        0, // device index
        state.clone(),
        samples_tx,
        command_rx,
        shutdown.clone(),
    )?;

    // Start DSP processing thread
    log::info!("Starting DSP thread...");
    let dsp_thread = dsp::start_dsp_thread(
        state.clone(),
        samples_rx,
        Some(audio_producer),
        shutdown.clone(),
    );

    // Initialize audio output
    log::info!("Starting audio output...");
    let _audio_output = AudioOutput::new(audio_consumer)?;

    // Initialize the UI app
    let mut app = App::new(state);
    app.set_command_tx(command_tx);

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

    // Signal all threads to stop
    log::info!("Shutting down threads...");
    shutdown.store(true, Ordering::Relaxed);

    // Wait for threads to finish
    let _ = sdr_thread.join();
    let _ = dsp_thread.join();

    log::info!("RTL-SDR TUI shutting down");
    Ok(())
}
