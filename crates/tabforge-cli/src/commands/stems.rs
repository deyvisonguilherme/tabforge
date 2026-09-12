use anyhow::Result;
use std::path::{Path, PathBuf};
use tabforge_audio::{AudioDecoder, NeuralSourceSeparator, SourceSeparator, SymphoniaDecoder};
use tracing::info;

pub fn execute(input: &Path, output_dir: Option<&Path>, model: Option<&Path>) -> Result<()> {
    info!("Starting neural / DSP source separation for: {}", input.display());

    // 1. Decode Audio
    let decoder = SymphoniaDecoder::default();
    let audio = decoder.decode(input)?;
    info!(
        "Decoded {} frames ({:.2}s, {} Hz)",
        audio.samples.len(),
        audio.duration_seconds(),
        audio.sample_rate
    );

    // 2. Perform Source Separation
    let mut separator = NeuralSourceSeparator::new();
    if let Some(m) = model {
        separator = separator.with_model_path(m);
    } else if let Some(cached_model) = tabforge_audio::ModelManager::get_or_default_model_path("htdemucs_6s") {
        separator = separator.with_model_path(cached_model);
    }

    let separated = separator.separate(&audio)?;

    // 3. Prepare Output Directory
    let default_dir_name = format!("{}_stems", input.file_stem().and_then(|s| s.to_str()).unwrap_or("track"));
    let target_dir = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")).join(default_dir_name));

    std::fs::create_dir_all(&target_dir)?;

    println!("\n=== TabForge Audio Stem Separation ===");
    println!("Source: {}", input.display());
    println!("Output Directory: {}", target_dir.display());
    println!("---------------------------------------");

    // 4. Save each stem as WAV
    for (name, stem_buffer) in &separated.stems {
        let stem_file = target_dir.join(format!("{name}.wav"));
        stem_buffer.save_wav(&stem_file)?;
        println!(
            "  -> Stem: {:<8} | Duration: {:.2}s | File: {}",
            name,
            stem_buffer.duration_seconds(),
            stem_file.display()
        );
    }

    println!("=======================================\n");
    println!("Successfully exported {} stems to: {}", separated.stems.len(), target_dir.display());

    Ok(())
}
