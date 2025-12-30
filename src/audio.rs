use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct AudioCapture {
    device: Device,
    config: StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    stream: Option<Stream>,
}

impl AudioCapture {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input device available"))?;

        log::info!("Using input device: {}", device.name().unwrap_or_default());

        let supported_config = device
            .supported_input_configs()?
            .find(|c| c.sample_format() == SampleFormat::F32)
            .ok_or_else(|| anyhow!("No F32 config available"))?
            .with_max_sample_rate();

        let config: StreamConfig = supported_config.into();
        log::info!(
            "Audio config: {} channels, {} Hz",
            config.channels,
            config.sample_rate.0
        );

        Ok(Self {
            device,
            config,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(16000 * 30))),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    pub fn start_recording(&mut self) -> Result<()> {
        self.buffer.lock().clear();
        self.is_recording.store(true, Ordering::SeqCst);

        let buffer = self.buffer.clone();
        let is_recording = self.is_recording.clone();
        let channels = self.config.channels as usize;

        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::SeqCst) {
                    let mut buf = buffer.lock();
                    if channels == 1 {
                        buf.extend_from_slice(data);
                    } else {
                        for chunk in data.chunks(channels) {
                            let mono = chunk.iter().sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                    }
                }
            },
            |err| log::error!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        log::info!("Recording started");
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Vec<f32> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;
        let samples = std::mem::take(&mut *self.buffer.lock());
        log::info!("Recording stopped, captured {} samples", samples.len());
        samples
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    /// Pre-warm the audio stream without starting actual recording.
    /// This creates the stream so it's ready for instant recording start.
    /// The stream exists but doesn't buffer audio (is_recording is false).
    pub fn prewarm(&mut self) -> Result<()> {
        if self.stream.is_some() {
            // Already warm
            return Ok(());
        }

        let buffer = self.buffer.clone();
        let is_recording = self.is_recording.clone();
        let channels = self.config.channels as usize;

        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::SeqCst) {
                    let mut buf = buffer.lock();
                    if channels == 1 {
                        buf.extend_from_slice(data);
                    } else {
                        for chunk in data.chunks(channels) {
                            let mono = chunk.iter().sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                    }
                }
                // When is_recording is false, we just discard the samples (no CPU cost)
            },
            |err| log::error!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        log::info!("Audio prewarmed (stream ready)");
        Ok(())
    }

    /// Cool down the audio stream (destroy it) to save resources.
    /// Safe to call even if not warm or currently recording.
    pub fn cooldown(&mut self) {
        if self.is_recording.load(Ordering::SeqCst) {
            // Don't cooldown while actively recording
            return;
        }
        if self.stream.is_some() {
            self.stream = None;
            log::info!("Audio stream cooled down");
        }
    }

    /// Check if the audio stream is pre-warmed and ready.
    pub fn is_warm(&self) -> bool {
        self.stream.is_some()
    }
}
