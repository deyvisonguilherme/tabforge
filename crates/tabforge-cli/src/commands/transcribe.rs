use anyhow::Result;
use std::path::Path;
use std::str::FromStr;
use tabforge_audio::{
    AdsrSegmenter, AudioDecoder, AutoCorrelationBeatDetector, BeatDetector, NeuralSourceSeparator,
    OnsetDetector, PitchDetector, SimpleQuantizer, SourceSeparator, SpectralOnsetDetector,
    SymphoniaDecoder, YinPitchDetector,
};
use tabforge_core::{Instrument, Song, TimeSignature, Track};
use tabforge_midi::{MidiWriter, MidlyWriter};
use tabforge_score::{AsciiTabWriter, MusicXmlWriter, ScoreWriter, TabWriter};
use tracing::info;

pub fn execute(
    input: &Path,
    instrument: &str,
    output: Option<&Path>,
    separate: bool,
    stem: &str,
    time_signature_opt: Option<&str>,
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

    // 3. Pitch Detection
    let pitch_detector = YinPitchDetector::default();
    let pitch_events = pitch_detector.detect_pitch(&audio)?;
    info!("Detected {} pitch events", pitch_events.len());

    // 4. Onset Detection
    let onset_detector = SpectralOnsetDetector::default();
    let onsets = onset_detector.detect_onsets(&audio)?;
    info!("Detected {} note onsets", onsets.len());

    // 5. Beat Grid & Metric Detection
    let beat_detector = AutoCorrelationBeatDetector::default();
    let mut beat_grid = beat_detector.detect_beat_grid(&audio)?;
    
    // Apply time signature override if specified
    if let Some(ts_str) = time_signature_opt {
        let parsed_ts = TimeSignature::from_str(ts_str)
            .map_err(|e| anyhow::anyhow!("Invalid time signature '{}': {}", ts_str, e))?;
        info!("Using manual time signature override: {}", parsed_ts);
        beat_grid.time_signature = parsed_ts;
    }

    info!(
        "Tempo: {:.1} BPM, Metric: {} (dynamic tempo changes: {})",
        beat_grid.tempo.bpm(),
        beat_grid.time_signature,
        beat_grid.tempo_changes.len()
    );

    // 6. ADSR Note Segmentation with Articulation Tracking
    let adsr_segmenter = AdsrSegmenter::default();
    let raw_notes = adsr_segmenter.segment(&audio, &onsets, &pitch_events);
    info!("Segmented {} discrete musical notes via ADSR", raw_notes.len());

    // 7. Rhythmic Quantization into structured Measures with rest filling
    let quantizer = SimpleQuantizer::default();
    let measures = quantizer.quantize_to_measures(&raw_notes, &beat_grid)?;
    info!("Quantized into {} structured measures", measures.len());

    // 8. Build Music IR Song
    let mut song = Song::new(input.file_stem().unwrap().to_str().unwrap())
        .with_tempo(beat_grid.tempo)
        .with_time_signature(beat_grid.time_signature);

    let target_instrument = match instrument.to_lowercase().as_str() {
        "bass" => Instrument::ElectricBass,
        "acoustic" | "acoustic_guitar" => Instrument::AcousticGuitar,
        _ => Instrument::ElectricGuitarClean,
    };

    let mut track = Track::new(format!("{instrument} Track"), target_instrument);
    for m in measures {
        track.add_measure(m);
    }
    song.add_track(track.clone());

    // 9. Render Tablature & Output Formats
    let tab_renderer = AsciiTabWriter::default();
    let ascii_tab = tab_renderer.render(&track);

    println!("\n=== TabForge Audio Transcription ===");
    println!("File: {}", input.display());
    println!("Tempo: {:.1} BPM", song.tempo.bpm());
    println!("Time Signature: {}", song.time_signature);
    println!("Instrument: {}", instrument);
    println!("\n{}", ascii_tab);
    println!("====================================\n");

    if let Some(out_path) = output {
        let ext = out_path.extension().and_then(|s| s.to_str()).unwrap_or("txt").to_lowercase();
        match ext.as_str() {
            "musicxml" | "xml" => {
                let writer = MusicXmlWriter::default();
                writer.write(&song, out_path)?;
                println!("Exported MusicXML score to: {}", out_path.display());
            }
            "mid" | "midi" => {
                let writer = MidlyWriter::default();
                writer.write(&song, out_path)?;
                println!("Exported Standard MIDI to: {}", out_path.display());
            }
            _ => {
                std::fs::write(out_path, &ascii_tab)?;
                println!("Saved ASCII tablature to: {}", out_path.display());
            }
        }
    }

    Ok(())
}
