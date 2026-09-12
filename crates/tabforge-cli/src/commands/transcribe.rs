use anyhow::Result;
use std::path::Path;
use tabforge_audio::{
    AdsrSegmenter, AudioDecoder, AutoCorrelationBeatDetector, BeatDetector, NeuralSourceSeparator,
    OnsetDetector, PitchDetector, SimpleQuantizer, SourceSeparator, SpectralOnsetDetector,
    SymphoniaDecoder, YinPitchDetector,
};
use tabforge_core::{Instrument, Song, Track};
use tabforge_score::{AsciiTabWriter, TabWriter};
use tracing::info;

pub fn execute(
    input: &Path,
    instrument: &str,
    output: Option<&Path>,
    separate: bool,
    stem: &str,
) -> Result<()> {
    info!("Starting transcription for audio file: {}", input.display());

    // 1. Decode Audio
    let decoder = SymphoniaDecoder::default();
    let decoded_audio = decoder.decode(input)?;
    info!(
        "Decoded {} frames ({:.2}s, {} Hz)",
        decoded_audio.samples.len(),
        decoded_audio.duration_seconds(),
        decoded_audio.sample_rate
    );

    // 2. Source Separation (Optional)
    let audio = if separate {
        info!("Applying neural / HPSS source separation to isolate stem: '{}'", stem);
        let mut separator = NeuralSourceSeparator::new();
        if let Some(cached_model) = tabforge_audio::ModelManager::get_or_default_model_path("htdemucs_6s") {
            separator = separator.with_model_path(cached_model);
        }
        let separated = separator.separate(&decoded_audio)?;
        let isolated = separated.get_or_default(stem, &decoded_audio).clone();
        info!("Isolated stem '{}' ({:.2}s)", stem, isolated.duration_seconds());
        isolated
    } else {
        decoded_audio
    };

    // 2. Pitch Detection
    let pitch_detector = YinPitchDetector::default();
    let pitch_events = pitch_detector.detect_pitch(&audio)?;
    info!("Detected {} pitch events", pitch_events.len());

    // 3. Onset Detection
    let onset_detector = SpectralOnsetDetector::default();
    let onsets = onset_detector.detect_onsets(&audio)?;
    info!("Detected {} note onsets", onsets.len());

    // 4. Beat Grid Detection
    let beat_detector = AutoCorrelationBeatDetector::default();
    let beat_grid = beat_detector.detect_beat_grid(&audio)?;
    info!("Detected BPM: {:.1}", beat_grid.tempo.bpm());

    // 5. ADSR Note Segmentation (Attack, Decay, Sustain, Release)
    let adsr_segmenter = AdsrSegmenter::default();
    let raw_notes = adsr_segmenter.segment(&audio, &onsets, &pitch_events);
    info!("Segmented {} discrete musical notes via ADSR", raw_notes.len());

    // 6. Rhythmic Quantization with Rest-filling into 4/4 Measures
    let quantizer = SimpleQuantizer::default();
    let measures = quantizer.quantize_to_measures(&raw_notes, &beat_grid)?;
    info!("Quantized into {} structured measures", measures.len());

    // 7. Build Music IR Song
    let mut song = Song::new(input.file_stem().unwrap().to_str().unwrap()).with_tempo(beat_grid.tempo);
    let mut track = Track::new(format!("{instrument} Track"), Instrument::ElectricGuitarClean);
    for m in measures {
        track.add_measure(m);
    }
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
