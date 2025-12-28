// Module declarations
mod audio;
mod dsp;
mod recorder;
mod sdr;
mod state;
mod types;
mod ui;

fn main() {
    env_logger::init();
    log::info!("RTL-SDR TUI starting...");

    println!("RTL-SDR TUI v0.1.0");
    println!("Module skeleton initialized successfully");
}
