use anyhow::Result;
use std::path::{Path, PathBuf};
use tabforge_core::{Duration, Instrument, Measure, Note, Pitch, Song, Track};
use tabforge_midi::{MidiWriter, MidlyWriter};
use tracing::info;

pub fn execute(input: &Path, output: Option<&Path>) -> Result<()> {
    info!("Processing MIDI command for: {}", input.display());
    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("mid"));

    // Build or import Song IR
    let mut song = Song::new(input.file_stem().unwrap().to_str().unwrap());
    let mut track = Track::new("Guitar Track", Instrument::ElectricGuitarClean);
    let mut measure = Measure::new(1);

    // Demonstration scale note sequence: C4, D4, E4, F4, G4, A4, B4, C5
    let pitches = ["C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5"];
    for p_str in pitches {
        let pitch: Pitch = p_str.parse().unwrap();
        let note = Note::new(pitch, Duration::QUARTER);
        measure.add_beat(tabforge_core::Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![note]));
    }

    track.add_measure(measure);
    song.add_track(track);

    let writer = MidlyWriter::default();
    writer.write(&song, &out_path)?;

    println!("Successfully wrote MIDI file to: {}", out_path.display());
    Ok(())
}
