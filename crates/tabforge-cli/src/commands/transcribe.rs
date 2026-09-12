use anyhow::Result;
use std::path::Path;
use tabforge_audio::{
    AudioDecoder, AutoCorrelationBeatDetector, BeatDetector, EnergyOnsetDetector, OnsetDetector,
    PitchDetector, SymphoniaDecoder, YinPitchDetector,
};
use tabforge_core::{Instrument, Measure, Song, Track};
use tabforge_score::{AsciiTabWriter, TabWriter};
use tracing::info;

pub fn execute(input: &Path, instrument: &str, output: Option<&Path>) -> Result<()> {
    info!("Starting transcription for audio file: {}", input.display());

    // 1. Decode Audio
    let decoder = SymphoniaDecoder::default();
    let audio = decoder.decode(input)?;
    info!(
        "Decoded {} frames ({:.2}s, {} Hz)",
        audio.samples.len(),
        audio.duration_seconds(),
        audio.sample_rate
    );

    // 2. Pitch Detection
    let pitch_detector = YinPitchDetector::default();
    let pitch_events = pitch_detector.detect_pitch(&audio)?;
    info!("Detected {} pitch events", pitch_events.len());

    // 3. Onset Detection
    let onset_detector = EnergyOnsetDetector::default();
    let onsets = onset_detector.detect_onsets(&audio)?;
    info!("Detected {} note onsets", onsets.len());

    // 4. Beat Grid Detection
    let beat_detector = AutoCorrelationBeatDetector::default();
    let beat_grid = beat_detector.detect_beat_grid(&audio)?;
    info!("Detected BPM: {:.1}", beat_grid.tempo.bpm());

    // 5. Build Music IR Song
    let mut song = Song::new(input.file_stem().unwrap().to_str().unwrap()).with_tempo(beat_grid.tempo);
    let mut track = Track::new(format!("{instrument} Track"), Instrument::ElectricGuitarClean);
    let mut measure = Measure::new(1);

    // Filter and collect pitches
    for p_event in pitch_events.iter().step_by(4).take(16) {
        let note = tabforge_core::Note::new(p_event.pitch, tabforge_core::Duration::QUARTER);
        measure.add_beat(tabforge_core::Beat::with_notes(
            tabforge_core::Duration::ZERO,
            tabforge_core::Duration::QUARTER,
            vec![note],
        ));
    }
    track.add_measure(measure);
    song.add_track(track.clone());

    // 6. Render Tablature
    let tab_renderer = AsciiTabWriter::default();
    let ascii_tab = tab_renderer.render(&track);

    println!("\n=== TabForge Audio Transcription ===");
    println!("File: {}", input.display());
    println!("Tempo: {:.1} BPM", song.tempo.bpm());
    println!("Instrument: {}", instrument);
    println!("\n{}", ascii_tab);
    println!("====================================\n");

    if let Some(out_path) = output {
        std::fs::write(out_path, ascii_tab)?;
        println!("Saved transcription to: {}", out_path.display());
    }

    Ok(())
}
