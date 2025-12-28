//! TCP Audio Streaming Server
//!
//! Streams raw PCM audio over TCP for remote listening.
//! Audio format: 16-bit signed little-endian, mono, 48kHz

use anyhow::Result;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use crossbeam::channel::{Receiver, Sender};

/// Audio sample rate for streaming
pub const STREAM_SAMPLE_RATE: u32 = 48000;

/// Start a TCP audio streaming server
///
/// Returns a sender channel to push audio samples to stream
pub fn start_streaming_server(
    port: u16,
    shutdown: Arc<AtomicBool>,
) -> Result<Sender<Vec<f32>>> {
    let (tx, rx) = crossbeam::channel::bounded::<Vec<f32>>(64);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    listener.set_nonblocking(true)?;

    log::info!("Audio streaming server started on port {}", port);
    log::info!("Connect with: nc localhost {} | aplay -r 48000 -f S16_LE -c 1", port);

    thread::spawn(move || {
        let mut clients: Vec<TcpStream> = Vec::new();

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Accept new connections (non-blocking)
            match listener.accept() {
                Ok((stream, addr)) => {
                    log::info!("Audio client connected from {}", addr);
                    if let Err(e) = stream.set_nonblocking(false) {
                        log::warn!("Failed to set stream blocking: {}", e);
                    }
                    // Set TCP_NODELAY for lower latency
                    if let Err(e) = stream.set_nodelay(true) {
                        log::warn!("Failed to set TCP_NODELAY: {}", e);
                    }
                    clients.push(stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connections, continue
                }
                Err(e) => {
                    log::warn!("Accept error: {}", e);
                }
            }

            // Receive audio samples
            match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(samples) => {
                    // Convert f32 samples to i16 PCM
                    let pcm_data: Vec<u8> = samples
                        .iter()
                        .flat_map(|&sample| {
                            // Clamp and convert to i16
                            let clamped = sample.max(-1.0).min(1.0);
                            let i16_sample = (clamped * 32767.0) as i16;
                            i16_sample.to_le_bytes()
                        })
                        .collect();

                    // Send to all connected clients
                    clients.retain_mut(|client| {
                        match client.write_all(&pcm_data) {
                            Ok(_) => true,
                            Err(e) => {
                                log::info!("Client disconnected: {}", e);
                                false
                            }
                        }
                    });
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    log::info!("Audio stream channel disconnected");
                    break;
                }
            }
        }

        log::info!("Streaming server stopped");
    });

    Ok(tx)
}

/// Audio streaming sink that sends samples to the TCP server
pub struct StreamingSink {
    tx: Sender<Vec<f32>>,
    buffer: Vec<f32>,
    buffer_size: usize,
}

impl StreamingSink {
    pub fn new(tx: Sender<Vec<f32>>) -> Self {
        Self {
            tx,
            buffer: Vec::with_capacity(4096),
            buffer_size: 4096, // Buffer ~85ms at 48kHz
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.buffer.push(sample);

        if self.buffer.len() >= self.buffer_size {
            let _ = self.tx.try_send(std::mem::take(&mut self.buffer));
            self.buffer = Vec::with_capacity(self.buffer_size);
        }
    }

    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let _ = self.tx.try_send(std::mem::take(&mut self.buffer));
        }
    }
}
