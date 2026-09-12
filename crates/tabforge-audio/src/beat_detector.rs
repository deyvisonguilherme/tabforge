use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{BeatDetector, BeatGrid};
use tabforge_core::Tempo;

/// BPM and Beat Grid detector
pub struct AutoCorrelationBeatDetector;

impl Default for AutoCorrelationBeatDetector {
    fn default() -> Self {
        Self
    }
}

impl BeatDetector for AutoCorrelationBeatDetector {
    fn detect_beat_grid(&self, audio: &AudioBuffer) -> Result<BeatGrid> {
        let duration = audio.duration_seconds();
        // Default tempo estimation baseline (120 BPM)
        let bpm = 120.0;
        let tempo = Tempo::new(bpm).unwrap();
        let beat_interval = 60.0 / bpm;

        let mut beat_times = Vec::new();
        let mut t = 0.0;
        while t < duration {
            beat_times.push(t);
            t += beat_interval;
        }

        Ok(BeatGrid {
            tempo,
            beat_times_seconds: beat_times,
        })
    }
}
