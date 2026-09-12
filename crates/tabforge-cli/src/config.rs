use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub transcription: TranscriptionConfig,
    pub guitar: GuitarConfig,
    pub quantization: QuantizationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    pub sample_rate: u32,
    pub hop_size: usize,
    pub window_size: usize,
    pub min_frequency: f64,
    pub max_frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuitarConfig {
    pub strings: usize,
    pub frets: u8,
    pub tuning: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    pub enabled: bool,
    pub min_subdivision: String,
    pub swing: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            transcription: TranscriptionConfig {
                sample_rate: 44100,
                hop_size: 512,
                window_size: 2048,
                min_frequency: 60.0,
                max_frequency: 1200.0,
            },
            guitar: GuitarConfig {
                strings: 6,
                frets: 24,
                tuning: vec![
                    "E2".to_string(),
                    "A2".to_string(),
                    "D3".to_string(),
                    "G3".to_string(),
                    "B3".to_string(),
                    "E4".to_string(),
                ],
            },
            quantization: QuantizationConfig {
                enabled: true,
                min_subdivision: "1/16".to_string(),
                swing: 0.0,
            },
        }
    }
}

impl Config {
    pub fn load_or_default(path: Option<&Path>) -> Self {
        if let Some(p) = path {
            if let Ok(content) = fs::read_to_string(p) {
                if let Ok(cfg) = toml::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }
}
