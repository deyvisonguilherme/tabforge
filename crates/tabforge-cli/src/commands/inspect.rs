use anyhow::Result;
use std::path::Path;
use tabforge_audio::{AudioDecoder, SymphoniaDecoder};
use tracing::info;

pub fn execute(input: &Path) -> Result<()> {
    info!("Inspecting audio file: {}", input.display());
    let decoder = SymphoniaDecoder::default();
    let audio = decoder.decode(input)?;

    println!("=========================================");
    println!(" TabForge Audio Inspection: {}", input.display());
    println!("=========================================");
    println!(" Sample Rate : {} Hz", audio.sample_rate);
    println!(" Channels    : {}", audio.channels);
    println!(" Duration    : {:.2} seconds", audio.duration_seconds());
    println!(" Samples     : {}", audio.samples.len());
    println!("=========================================");

    Ok(())
}
