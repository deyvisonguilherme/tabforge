use anyhow::Result;
use std::path::{Path, PathBuf};
use tabforge_core::{Duration, Instrument, Measure, Note, Pitch, Song, Track};
use tabforge_score::{MusicXmlWriter, ScoreWriter};
use tracing::info;

pub fn execute(input: &Path, output: Option<&Path>) -> Result<()> {
    info!("Processing Score command for: {}", input.display());
    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("musicxml"));

    let mut song = Song::new(input.file_stem().unwrap().to_str().unwrap());
    let mut track = Track::new("Guitar", Instrument::ElectricGuitarClean);
    let mut measure = Measure::new(1);

    let pitches = ["E2", "G2", "A2", "B2", "D3", "E3"];
    for p_str in pitches {
        let pitch: Pitch = p_str.parse().unwrap();
        let note = Note::new(pitch, Duration::QUARTER);
        measure.add_beat(tabforge_core::Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![note]));
    }

    track.add_measure(measure);
    song.add_track(track);

    let writer = MusicXmlWriter::default();
    writer.write(&song, &out_path)?;

    println!("Successfully exported MusicXML score to: {}", out_path.display());
    Ok(())
}
