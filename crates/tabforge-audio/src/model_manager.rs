use crate::error::{AudioError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Metadata and specifications for available neural source separation models
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub stems: &'static [&'static str],
    pub sdr_score_db: f32,
    pub download_url: &'static str,
    pub filename: &'static str,
    pub size_mb: f32,
}

pub const KNOWN_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "htdemucs_6s",
        display_name: "HTDemucs v4 (6-Stems)",
        description: "Meta AI Hybrid Transformer Demucs with dedicated Guitar and Piano stems (SOTA for guitar)",
        stems: &["drums", "bass", "other", "vocals", "guitar", "piano"],
        sdr_score_db: 9.2,
        download_url: "https://huggingface.co/StemSplitio/htdemucs-6s-onnx/resolve/main/htdemucs_6s.onnx",
        filename: "htdemucs_6s.onnx",
        size_mb: 118.0,
    },
    ModelInfo {
        name: "htdemucs_4s",
        display_name: "HTDemucs v4 (4-Stems)",
        description: "Meta AI Hybrid Transformer Demucs 4-stem model (Drums, Bass, Other, Vocals)",
        stems: &["drums", "bass", "other", "vocals"],
        sdr_score_db: 8.8,
        download_url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx",
        filename: "htdemucs_4s.onnx",
        size_mb: 79.0,
    },
    ModelInfo {
        name: "bs_roformer",
        display_name: "BS-Roformer (Vocal & Inst)",
        description: "Band-Split Roformer with Rotary Position Embeddings (Music Demixing Challenge Winner)",
        stems: &["vocals", "other"],
        sdr_score_db: 9.6,
        download_url: "https://huggingface.co/ZFTurbo/BS-Roformer/resolve/main/bs_roformer.onnx",
        filename: "bs_roformer.onnx",
        size_mb: 145.0,
    },
];

/// Model Manager responsible for discovering, caching, and downloading neural models
pub struct ModelManager;

impl ModelManager {
    /// Return canonical directory for storing neural models
    pub fn cache_dir() -> PathBuf {
        if let Ok(custom) = std::env::var("TABFORGE_CACHE_DIR") {
            let p = PathBuf::from(custom).join("models");
            let _ = fs::create_dir_all(&p);
            return p;
        }

        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
            let p = PathBuf::from(xdg).join("tabforge").join("models");
            let _ = fs::create_dir_all(&p);
            return p;
        }

        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".cache").join("tabforge").join("models");
            let _ = fs::create_dir_all(&p);
            return p;
        }

        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(appdata).join("tabforge").join("models");
            let _ = fs::create_dir_all(&p);
            return p;
        }

        let p = std::env::temp_dir().join("tabforge").join("models");
        let _ = fs::create_dir_all(&p);
        p
    }

    /// List all known models with their installation status
    pub fn list_models() -> Vec<(ModelInfo, bool, PathBuf)> {
        let dir = Self::cache_dir();
        KNOWN_MODELS
            .iter()
            .map(|m| {
                let model_path = dir.join(m.filename);
                let is_installed = model_path.exists() && fs::metadata(&model_path).map(|meta| meta.len() > 1_000_000).unwrap_or(false);
                (m.clone(), is_installed, model_path)
            })
            .collect()
    }

    /// Find model info by name or alias
    pub fn find_model(name: &str) -> Option<&'static ModelInfo> {
        let clean = name.trim().to_lowercase();
        KNOWN_MODELS
            .iter()
            .find(|m| m.name == clean || m.filename.eq_ignore_ascii_case(&clean))
    }

    /// Check if a model file is currently installed in the cache
    pub fn get_installed_model_path(name: &str) -> Option<PathBuf> {
        let model_info = Self::find_model(name)?;
        let path = Self::cache_dir().join(model_info.filename);
        if path.exists() && fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
            Some(path)
        } else {
            None
        }
    }

    /// Get installed model path or return instructions if not present
    pub fn get_or_default_model_path(name: &str) -> Option<PathBuf> {
        if let Some(p) = Self::get_installed_model_path(name) {
            return Some(p);
        }

        // Check if custom path was specified in environment variable
        if let Ok(env_path) = std::env::var("TABFORGE_MODEL_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() && fs::metadata(&p).map(|m| m.len() > 1_000_000).unwrap_or(false) {
                return Some(p);
            }
        }

        None
    }

    /// Download model weights into local cache directory
    pub fn download_model(name: &str) -> Result<PathBuf> {
        let model_info = Self::find_model(name)
            .ok_or_else(|| AudioError::DecodeError(format!("Unknown neural model: '{name}'")))?;

        let dest_path = Self::cache_dir().join(model_info.filename);
        let tmp_path = Self::cache_dir().join(format!("{}.download.tmp", model_info.filename));

        if dest_path.exists() {
            if let Ok(meta) = fs::metadata(&dest_path) {
                if meta.len() > 1_000_000 {
                    tracing::info!("Model '{}' already present at {}", model_info.display_name, dest_path.display());
                    return Ok(dest_path);
                } else {
                    tracing::warn!("Existing model file is corrupted/too small ({} bytes), redownloading...", meta.len());
                    let _ = fs::remove_file(&dest_path);
                }
            }
        }

        println!("⬇️  Downloading {} (~{:.1} MB)...", model_info.display_name, model_info.size_mb);
        println!("   Source: {}", model_info.download_url);
        println!("   Target: {}", dest_path.display());

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(600))
            .redirects(10)
            .build();

        let response = agent.get(model_info.download_url)
            .set("User-Agent", "TabForge/0.1.0")
            .call()
            .map_err(|e| AudioError::DspError(format!("Failed to connect to model download URL: {}", e)))?;

        let total_size = response
            .header("content-length")
            .and_then(|l| l.parse::<u64>().ok())
            .unwrap_or((model_info.size_mb * 1024.0 * 1024.0) as u64);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );

        let mut reader = response.into_reader();
        let mut file = File::create(&tmp_path)?;
        let mut buffer = [0u8; 65536];
        let mut downloaded: u64 = 0;

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n])?;
            downloaded += n as u64;
            pb.set_position(downloaded);
        }

        file.flush()?;
        drop(file);
        pb.finish_with_message("Download complete!");

        fs::rename(&tmp_path, &dest_path)?;
        println!("✅ Model saved successfully to: {}\n", dest_path.display());

        Ok(dest_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_manager_discovery() {
        let models = ModelManager::list_models();
        assert_eq!(models.len(), 3);
        assert!(models.iter().any(|(m, _, _)| m.name == "htdemucs_6s"));
    }

    #[test]
    fn test_model_manager_path_resolution() {
        let cache_dir = ModelManager::cache_dir();
        assert!(cache_dir.exists());

        let htdemucs = ModelManager::find_model("htdemucs_6s");
        assert!(htdemucs.is_some());
        assert_eq!(htdemucs.unwrap().stems.len(), 6);
        assert!(htdemucs.unwrap().stems.contains(&"guitar"));
    }
}
