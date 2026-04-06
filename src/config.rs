use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkeys: HotkeyConfig,
    pub stt: SttConfig,
    pub ui: UiConfig,
    pub audio: AudioConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub start_stop: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SttConfig {
    pub model_path: String,
    pub language: String,
    pub beam_size: i32,
    pub patience: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub opacity: u8,
    pub size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub microphone_only: bool,
    pub preferred_input_device: String,
    pub worker_sleep_ms: u64,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            start_stop: "F9".to_string(),
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_path: "models/ggml-large-v3-turbo.bin".to_string(),
            language: "es".to_string(),
            beam_size: 5,
            patience: 1.0,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            opacity: 220,
            size: 48,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            microphone_only: true,
            preferred_input_device: String::new(),
            worker_sleep_ms: 10,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkeys: HotkeyConfig::default(),
            stt: SttConfig::default(),
            ui: UiConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "simplestt", "simplestt")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

impl AppConfig {
    pub fn load() -> Self {
        let path = match config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            let default = Self::default();
            let _ = default.save();
            return default;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        toml::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "could not determine config directory")
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&path, contents)
    }
}
