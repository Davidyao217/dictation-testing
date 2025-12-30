#[macro_use]
extern crate objc;

mod audio;
mod config;
mod events;
mod hotkey;
mod indicator;
mod model;
mod output;
mod state;
mod transcriber;
mod triggers;
mod tray;
mod vad;
mod worker;

use anyhow::Result;
use crossbeam_channel::unbounded;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use crate::audio::AudioCapture;
use crate::config::{Config, RecordingMode};
use crate::events::AppEvent;
use crate::hotkey::{HotkeyEvent, HotkeyHandler};
use crate::indicator::RecordingIndicator;
use crate::model::ModelManager;
use crate::output::OutputHandler;
use crate::state::StateManager;
use crate::tray::TrayIcon;
use crate::vad::VadProcessor;
use crate::worker::{TranscriptionRequest, TranscriptionWorker};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Dictation App");

    let config = Config::load()?;
    log::info!("Config loaded: {:?}", config);

    if !Config::models_dir().exists() {
        fs::create_dir_all(Config::models_dir())?;
    }

    if !config.model_path().exists() {
        log::error!(
            "Model not found at {:?}. Please download a Whisper model.",
            config.model_path()
        );
        log::info!("Download from: {}", config.model.download_url());
        log::info!("Place the model file in: {:?}", Config::models_dir());
        return Err(anyhow::anyhow!("Model not found"));
    }

    // Build event loop with our custom AppEvent type
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // State management
    let state = StateManager::new();

    // Model manager (will be moved to worker)
    let model_manager = ModelManager::new(state.clone(), config.clone());
    log::info!("Pre-loading model...");
    model_manager.load_async();

    // VAD processor (will be moved to worker)
    let mut audio_capture = AudioCapture::new()?;
    let vad_processor = if config.vad_enabled {
        Some(VadProcessor::new(config.vad_threshold, audio_capture.sample_rate()))
    } else {
        None
    };

    // Create transcription worker - takes ownership of model_manager and vad_processor
    let worker = TranscriptionWorker::new(model_manager, vad_processor, proxy.clone());

    // Tray icon
    let _tray = TrayIcon::new(proxy)?;

    // Hotkey handling
    let hotkey_handler = HotkeyHandler::new()?;
    let hotkey_id = hotkey_handler.hotkey_id();

    let (hotkey_tx, hotkey_rx) = unbounded::<HotkeyEvent>();
    HotkeyHandler::listen(hotkey_tx, hotkey_id);

    // Output handler and indicator
    let mut output_handler = OutputHandler::new(config.output_mode)?;
    let indicator = Arc::new(RecordingIndicator::new());

    let recording_mode = config.recording_mode;
    let mut is_toggle_recording = false;

    log::info!("Dictation App ready. Press Cmd+Shift+D to dictate.");
    log::info!("Recording mode: {:?}", recording_mode);

    let check_interval = Duration::from_millis(100);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + check_interval);

        match event {
            Event::NewEvents(StartCause::Poll | StartCause::ResumeTimeReached { .. }) => {
                // Process hotkey events
                while let Ok(evt) = hotkey_rx.try_recv() {
                    match recording_mode {
                        RecordingMode::PushToTalk => {
                            match evt {
                                HotkeyEvent::Pressed => {
                                    start_recording(&mut audio_capture, &indicator, &state);
                                }
                                HotkeyEvent::Released => {
                                    stop_and_submit(
                                        &mut audio_capture,
                                        &worker,
                                        &indicator,
                                        &state,
                                    );
                                }
                            }
                        }
                        RecordingMode::Toggle => {
                            if matches!(evt, HotkeyEvent::Pressed) {
                                if !is_toggle_recording {
                                    start_recording(&mut audio_capture, &indicator, &state);
                                    is_toggle_recording = true;
                                } else {
                                    stop_and_submit(
                                        &mut audio_capture,
                                        &worker,
                                        &indicator,
                                        &state,
                                    );
                                    is_toggle_recording = false;
                                }
                            }
                        }
                    }
                }
            }

            // Handle transcription results from worker
            Event::UserEvent(AppEvent::TranscriptionComplete(text)) => {
                log::info!("Transcription complete, outputting text");
                if let Err(e) = output_handler.output_text(&text) {
                    log::error!("Failed to output text: {}", e);
                }
                indicator.hide();
                state.transition_to_idle();
            }

            Event::UserEvent(AppEvent::TranscriptionFailed) => {
                log::info!("Transcription failed or no speech detected");
                indicator.flash_error();
                state.transition_to_idle();
            }

            Event::UserEvent(AppEvent::Quit) => {
                log::info!("Quit requested");
                *control_flow = ControlFlow::Exit;
            }

            _ => {}
        }
    });
}

fn start_recording(
    audio_capture: &mut AudioCapture,
    indicator: &RecordingIndicator,
    state: &StateManager,
) {
    log::info!("Starting recording");
    indicator.show();
    indicator.set_color_recording();
    if let Err(e) = audio_capture.start_recording() {
        log::error!("Failed to start recording: {}", e);
    }
    state.transition_to_recording();
}

fn stop_and_submit(
    audio_capture: &mut AudioCapture,
    worker: &TranscriptionWorker,
    indicator: &RecordingIndicator,
    state: &StateManager,
) {
    log::info!("Stopping recording");
    let samples = audio_capture.stop_recording();
    let sample_rate = audio_capture.sample_rate();

    if samples.len() > 1600 {
        // Change indicator to processing color
        indicator.set_color_processing();
        state.transition_to_transcribing();

        // Submit to worker - this returns immediately
        worker.submit(TranscriptionRequest { samples, sample_rate });
        // UI stays responsive, indicator stays visible until worker completes
    } else {
        log::warn!("Recording too short, ignoring");
        indicator.hide();
        state.transition_to_idle();
    }
}
