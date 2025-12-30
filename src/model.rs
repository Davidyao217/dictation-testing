use crate::config::Config;
use crate::state::{ReadinessState, StateManager};
use crate::transcriber::Transcriber;
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;

pub struct ModelManager {
    transcriber: Arc<Mutex<Option<Transcriber>>>,
    state: StateManager,
    config: Config,
}

impl ModelManager {
    pub fn new(state: StateManager, config: Config) -> Self {
        Self {
            transcriber: Arc::new(Mutex::new(None)),
            state,
            config,
        }
    }

    pub fn load_async(&self) {
        if !self.state.transition_to_loading() {
            return;
        }

        let transcriber = self.transcriber.clone();
        let state = self.state.clone();
        let model_path = self.config.model_path();

        thread::spawn(move || {
            match Transcriber::new(model_path) {
                Ok(t) => {
                    if let Err(e) = t.warmup() {
                        log::warn!("Warmup failed: {}", e);
                    }
                    *transcriber.lock() = Some(t);
                    state.transition_to_idle();
                }
                Err(e) => {
                    log::error!("Failed to load model: {}", e);
                    state.transition_to_cold();
                }
            }
        });
    }

    pub fn ensure_loaded(&self) {
        let current = self.state.get();
        if current == ReadinessState::Cold {
            self.load_async();
        }
    }

    pub fn unload(&self) {
        *self.transcriber.lock() = None;
        self.state.transition_to_cold();
        log::info!("Model unloaded");
    }

    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<String> {
        let guard = self.transcriber.lock();
        match guard.as_ref() {
            Some(t) => t.transcribe(samples, sample_rate),
            None => Err(anyhow::anyhow!("Model not loaded")),
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.transcriber.lock().is_some()
    }
}
