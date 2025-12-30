use crate::events::AppEvent;
use crate::model::ModelManager;
use crate::vad::VadProcessor;
use crossbeam_channel::{bounded, Sender};
use std::thread;
use tao::event_loop::EventLoopProxy;

/// Request to transcribe audio samples
pub struct TranscriptionRequest {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

/// Background worker that handles transcription off the main thread.
/// This keeps the UI responsive during VAD processing and inference.
pub struct TranscriptionWorker {
    request_tx: Sender<TranscriptionRequest>,
}

impl TranscriptionWorker {
    /// Create a new worker that owns the ModelManager and optional VadProcessor.
    /// Results are sent back via the EventLoopProxy.
    pub fn new(
        model_manager: ModelManager,
        vad_processor: Option<VadProcessor>,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Self {
        // Bounded channel with capacity 1 - if a new request comes in while
        // processing, we can buffer exactly one. This prevents memory buildup
        // from rapid requests.
        let (request_tx, request_rx) = bounded::<TranscriptionRequest>(1);

        thread::spawn(move || {
            log::info!("Transcription worker started");
            
            loop {
                // Block until we receive a request (no busy polling = lightweight)
                match request_rx.recv() {
                    Ok(request) => {
                        Self::process_request(
                            &request,
                            &model_manager,
                            &vad_processor,
                            &proxy,
                        );
                    }
                    Err(_) => {
                        // Channel closed, worker should exit
                        log::info!("Transcription worker shutting down");
                        break;
                    }
                }
            }
        });

        Self { request_tx }
    }

    /// Submit a transcription request to the worker.
    /// Returns immediately - transcription happens in background.
    pub fn submit(&self, request: TranscriptionRequest) {
        // Use try_send to avoid blocking if channel is full
        // If channel is full, the old request gets dropped (which is fine -
        // we only care about the latest audio)
        match self.request_tx.try_send(request) {
            Ok(_) => log::debug!("Transcription request submitted"),
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                log::warn!("Transcription queue full, request dropped");
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                log::error!("Transcription worker disconnected");
            }
        }
    }

    fn process_request(
        request: &TranscriptionRequest,
        model_manager: &ModelManager,
        vad_processor: &Option<VadProcessor>,
        proxy: &EventLoopProxy<AppEvent>,
    ) {
        // Step 1: VAD processing (trim silence)
        let samples_to_transcribe = if let Some(vad) = vad_processor {
            match vad.process(&request.samples, request.sample_rate) {
                Ok(Some(trimmed)) => trimmed,
                Ok(None) => {
                    log::info!("No speech detected, skipping transcription");
                    let _ = proxy.send_event(AppEvent::TranscriptionFailed);
                    return;
                }
                Err(e) => {
                    log::warn!("VAD failed: {}, using original samples", e);
                    request.samples.clone()
                }
            }
        } else {
            request.samples.clone()
        };

        // Step 2: Check minimum length
        if samples_to_transcribe.len() <= 1600 {
            log::warn!("Recording too short, ignoring");
            let _ = proxy.send_event(AppEvent::TranscriptionFailed);
            return;
        }

        // Step 3: Transcription (includes resampling if needed)
        match model_manager.transcribe(&samples_to_transcribe, request.sample_rate) {
            Ok(text) => {
                log::info!("Transcribed: {}", text);
                if text.is_empty() {
                    let _ = proxy.send_event(AppEvent::TranscriptionFailed);
                } else {
                    let _ = proxy.send_event(AppEvent::TranscriptionComplete(text));
                }
            }
            Err(e) => {
                log::error!("Transcription failed: {}", e);
                let _ = proxy.send_event(AppEvent::TranscriptionFailed);
            }
        }
    }
}
