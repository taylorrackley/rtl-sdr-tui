// Module declarations
mod audio;
mod dsp;
mod recorder;
mod sdr;
mod state;
mod types;
mod ui;

use anyhow::Result;
use dsp::FftProcessor;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
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

    // Start test signal generator thread
    let shutdown = Arc::new(AtomicBool::new(false));
    let test_thread = start_test_signal_generator(state.clone(), shutdown.clone());

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

    // Signal test thread to stop
    shutdown.store(true, Ordering::Relaxed);
    let _ = test_thread.join();

    log::info!("RTL-SDR TUI shutting down");
    Ok(())
}

/// Start a test signal generator thread for demonstration
fn start_test_signal_generator(
    state: state::SharedState,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut fft_processor = FftProcessor::new(2048);
        let mut frame_count = 0u32;

        while !shutdown.load(Ordering::Relaxed) {
            // Get current sample rate from state
            let sample_rate = state.read().sdr.sample_rate;

            // Generate test signal with moving frequencies
            let freq1 = 100_000.0 + (frame_count as f32 * 1000.0).sin() * 50_000.0;
            let freq2 = -150_000.0 + (frame_count as f32 * 500.0).cos() * 30_000.0;
            let freq3 = 50_000.0;

            let test_signal = FftProcessor::generate_test_signal(
                2048,
                sample_rate,
                &[
                    (freq1, 0.8),
                    (freq2, 0.6),
                    (freq3, 0.4),
                ],
            );

            // Process with FFT
            let fft_data = fft_processor.process(&test_signal);

            // Update state
            state.write().spectrum.add_fft_data(fft_data);

            frame_count = frame_count.wrapping_add(1);

            // Update at ~20 FPS
            thread::sleep(Duration::from_millis(50));
        }
    })
}
