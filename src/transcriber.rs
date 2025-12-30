use anyhow::{anyhow, Result};
use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn new(model_path: PathBuf) -> Result<Self> {
        log::info!("Loading Whisper model from {:?}", model_path);

        if !model_path.exists() {
            return Err(anyhow!(
                "Model not found at {:?}. Please download a model first.",
                model_path
            ));
        }

        let num_threads = (num_cpus::get() / 2).max(1);
        log::info!("Using {} threads for Whisper", num_threads);

        let mut params = WhisperContextParameters::default();
        params.use_gpu(false);

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            params,
        )
        .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        log::info!("Model loaded successfully");
        Ok(Self { ctx })
    }

    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<String> {
        let samples = if sample_rate != 16000 {
            resample_high_quality(samples, sample_rate, 16000)?
        } else {
            samples.to_vec()
        };

        let mut state = self.ctx.create_state()?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        let num_threads = (num_cpus::get() / 2).max(1);
        params.set_n_threads(num_threads as i32);
        
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_single_segment(true);
        params.set_no_context(true);
        params.set_max_len(1);

        state.full(params, &samples)?;

        let num_segments = state.full_n_segments()?;
        let mut text = String::new();

        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        let mut result = text.trim().to_string();
        if result == "[BLANK_AUDIO]" {
            result.clear();
        }

        Ok(result)
    }

    pub fn warmup(&self) -> Result<()> {
        log::info!("Warming up model...");
        let silent = vec![0.0f32; 16000];
        let mut state = self.ctx.create_state()?;
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        state.full(params, &silent)?;
        log::info!("Warmup complete");
        Ok(())
    }
}

fn resample_high_quality(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    let params = SincInterpolationParameters {
        sinc_len: 64,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        to_rate as f64 / from_rate as f64,
        2.0,
        params,
        samples.len(),
        1,
    ).map_err(|e| anyhow!("Resampler creation failed: {}", e))?;

    let input = vec![samples.to_vec()];
    let output = resampler.process(&input, None)
        .map_err(|e| anyhow!("Resampling failed: {}", e))?;

    Ok(output[0].clone())
}
