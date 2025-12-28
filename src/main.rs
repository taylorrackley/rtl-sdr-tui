// Module declarations
mod audio;
mod dsp;
mod recorder;
mod sdr;
mod state;
mod streaming;
mod types;
mod ui;

use anyhow::Result;
use audio::AudioOutput;
use clap::Parser;
use crossbeam::channel;
use ringbuf::{traits::Split, HeapRb};
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ui::App;

/// RTL-SDR TUI - A terminal-based SDR receiver
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Initial frequency in MHz (e.g., 162.425 for NOAA)
    #[arg(short, long)]
    frequency: Option<f64>,

    /// Stream audio over TCP on specified port
    /// Connect with: nc localhost <port> | aplay -r 48000 -f S16_LE -c 1
    #[arg(short = 'p', long = "audio-port")]
    audio_port: Option<u16>,

    /// SDR device index (default: 0)
    #[arg(short, long, default_value_t = 0)]
    device: usize,

    /// Initial gain in dB (default: auto)
    #[arg(short, long)]
    gain: Option<f32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging to file to avoid corrupting TUI
    use std::fs::OpenOptions;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("rtl-sdr-tui.log")
        .expect("Failed to open log file");

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("RTL-SDR TUI v0.1.0 starting...");

    if let Some(port) = args.audio_port {
        log::info!("Audio streaming enabled on port {}", port);
        eprintln!("Audio streaming on port {}. Connect with:", port);
        eprintln!("  nc localhost {} | aplay -r 48000 -f S16_LE -c 1", port);
        eprintln!();
    }

    // Run the application
    if let Err(e) = run(args) {
        log::error!("Application error: {}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run(args: Args) -> Result<()> {
    // Initialize shared state
    let state = AppState::new_shared();

    // Apply command-line arguments to initial state
    if let Some(freq_mhz) = args.frequency {
        let freq_hz = (freq_mhz * 1_000_000.0) as u32;
        state.write().sdr.frequency = freq_hz;
        log::info!("Initial frequency set to {} MHz", freq_mhz);
    }

    if let Some(gain) = args.gain {
        let gain_tenths = (gain * 10.0) as i32;
        state.write().sdr.tuner_gain = gain_tenths;
        state.write().sdr.auto_gain = false;
        log::info!("Initial gain set to {} dB", gain);
    }

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

    // Start TCP streaming server if requested
    let stream_tx = if let Some(port) = args.audio_port {
        log::info!("Starting audio streaming server on port {}...", port);
        Some(streaming::start_streaming_server(port, shutdown.clone())?)
    } else {
        None
    };

    // Start SDR thread
    log::info!("Starting SDR thread...");
    let sdr_thread = sdr::start_sdr_thread(
        args.device,
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
        stream_tx,
        shutdown.clone(),
    );

    // Initialize audio output (local speaker)
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
