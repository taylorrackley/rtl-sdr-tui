use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use ringbuf::traits::Consumer;

/// Audio output manager
pub struct AudioOutput {
    _host: Host,
    _device: Device,
    _config: StreamConfig,
    stream: Stream,
}

impl AudioOutput {
    /// Create and start an audio output stream
    ///
    /// # Arguments
    /// * `consumer` - Ring buffer consumer for audio samples
    pub fn new<C: Consumer<Item = f32> + Send + 'static>(mut consumer: C) -> Result<Self> {
        // Get default audio output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default audio output device"))?;

        log::info!("Audio output device: {}", device.name()?);

        // Get default output config
        let config = device.default_output_config()?;
        log::info!(
            "Audio config: {} Hz, {} channels",
            config.sample_rate().0,
            config.channels()
        );

        let config: StreamConfig = config.into();

        // Create output stream
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Fill output buffer from ring buffer
                for sample in data.iter_mut() {
                    *sample = consumer.try_pop().unwrap_or(0.0);
                }
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )?;

        // Start the stream
        stream.play()?;
        log::info!("Audio stream started");

        Ok(Self {
            _host: host,
            _device: device,
            _config: config,
            stream,
        })
    }

    /// Pause the audio stream
    pub fn pause(&self) -> Result<()> {
        self.stream.pause()?;
        Ok(())
    }

    /// Resume the audio stream
    pub fn play(&self) -> Result<()> {
        self.stream.play()?;
        Ok(())
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        log::info!("Audio output stopped");
    }
}
