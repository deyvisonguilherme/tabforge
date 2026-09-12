use anyhow::Result;
use std::path::Path;
use tabforge_core::{Duration, Instrument, Measure, Note, Pitch, Track};
use tabforge_score::{AsciiTabWriter, TabWriter};
use tracing::info;

pub fn execute(input: &Path) -> Result<()> {
    info!("Generating ASCII Tab for: {}", input.display());

    let mut track = Track::new("Guitar Track", Instrument::ElectricGuitarClean);
    let mut measure1 = Measure::new(1);

    // E Minor Pentatonic scale run
    let pentatonic = ["E2", "G2", "A2", "B2", "D3", "E3", "G3", "A3", "B3", "D4", "E4"];
    for p_str in pentatonic {
        let pitch: Pitch = p_str.parse().unwrap();
        let note = Note::new(pitch, Duration::EIGHTH);
        measure1.add_beat(tabforge_core::Beat::with_notes(Duration::ZERO, Duration::EIGHTH, vec![note]));
    }
    track.add_measure(measure1);

    let renderer = AsciiTabWriter::default();
    let tab_ascii = renderer.render(&track);

    println!("\n=== TabForge ASCII Tablature Output ===");
    println!("Song: {}", input.file_stem().unwrap().to_str().unwrap());
    println!("Tuning: Standard (E A D G B e)\n");
    println!("{}", tab_ascii);
    println!("========================================\n");

    Ok(())
}
