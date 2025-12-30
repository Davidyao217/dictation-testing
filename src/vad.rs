use anyhow::Result;
use voice_activity_detector::VoiceActivityDetector;

pub struct VadProcessor {
    threshold: f32,
    min_speech_samples: usize,
    padding_samples: usize,
}

impl VadProcessor {
    pub fn new(threshold: f32, sample_rate: u32) -> Self {
        Self {
            threshold,
            min_speech_samples: (sample_rate as f32 * 0.1) as usize,
            padding_samples: (sample_rate as f32 * 0.05) as usize,
        }
    }

    pub fn process(&self, samples: &[f32], sample_rate: u32) -> Result<Option<Vec<f32>>> {
        let chunk_size = if sample_rate == 8000 { 256 } else { 512 };
        
        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        let mut vad = VoiceActivityDetector::builder()
            .sample_rate(sample_rate as i64)
            .chunk_size(chunk_size)
            .build()
            .map_err(|e| anyhow::anyhow!("VAD creation failed: {}", e))?;

        let mut speech_start: Option<usize> = None;
        let mut speech_end: Option<usize> = None;
        let mut has_speech = false;

        for (i, chunk) in samples_i16.chunks(chunk_size).enumerate() {
            if chunk.len() < chunk_size {
                break;
            }
            
            let probability = vad.predict(chunk.iter().copied());
            let chunk_start = i * chunk_size;
            
            if probability > self.threshold {
                has_speech = true;
                if speech_start.is_none() {
                    speech_start = Some(chunk_start);
                }
                speech_end = Some(chunk_start + chunk_size);
            }
        }

        if !has_speech {
            log::info!("VAD: No speech detected");
            return Ok(None);
        }

        let start = speech_start.unwrap_or(0);
        let end = speech_end.unwrap_or(samples.len());
        
        let padded_start = start.saturating_sub(self.padding_samples);
        let padded_end = (end + self.padding_samples).min(samples.len());
        
        let trimmed_len = padded_end - padded_start;
        if trimmed_len < self.min_speech_samples {
            log::info!("VAD: Speech too short ({} samples)", trimmed_len);
            return Ok(None);
        }

        log::info!(
            "VAD: Trimmed {} -> {} samples ({}ms)",
            samples.len(),
            trimmed_len,
            trimmed_len * 1000 / sample_rate as usize
        );

        Ok(Some(samples[padded_start..padded_end].to_vec()))
    }
}
