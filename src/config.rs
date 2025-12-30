use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    #[default]
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Clipboard,
    Keystroke,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    TinyEn,
    BaseEn,
    SmallEn,
    Tiny,
    Base,
    Small,
}

impl Default for WhisperModel {
    fn default() -> Self {
        Self::BaseEn
    }
}

impl WhisperModel {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::TinyEn => "ggml-tiny.en.bin",
            Self::BaseEn => "ggml-base.en.bin",
            Self::SmallEn => "ggml-small.en.bin",
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
        }
    }

    pub fn download_url(&self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub model: WhisperModel,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
    #[serde(default)]
    pub recording_mode: RecordingMode,
    #[serde(default)]
    pub output_mode: OutputMode,
    #[serde(default = "default_vad_enabled")]
    pub vad_enabled: bool,
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
}

fn default_idle_timeout() -> u64 {
    300
}

fn default_vad_enabled() -> bool {
    true
}

fn default_vad_threshold() -> f32 {
    0.5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: WhisperModel::default(),
            idle_timeout_secs: default_idle_timeout(),
            recording_mode: RecordingMode::default(),
            output_mode: OutputMode::default(),
            vad_enabled: default_vad_enabled(),
            vad_threshold: default_vad_threshold(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dictation")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn models_dir() -> PathBuf {
        Self::config_dir().join("models")
    }

    pub fn model_path(&self) -> PathBuf {
        Self::models_dir().join(self.model.filename())
    }
}
