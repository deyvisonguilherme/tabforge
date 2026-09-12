use crate::buffer::AudioBuffer;
use crate::error::Result;
use tabforge_core::{Beat, Pitch, Tempo};

/// Pitch detection event with timestamp and confidence
#[derive(Debug, Clone, PartialEq)]
pub struct PitchEvent {
    pub time_seconds: f64,
    pub pitch: Pitch,
    pub confidence: f32,
    pub frequency: f64,
}

/// Note onset event (start of note attack)
#[derive(Debug, Clone, PartialEq)]
pub struct OnsetEvent {
    pub time_seconds: f64,
    pub strength: f32,
}

/// Beat grid with detected tempo and beat timestamps
#[derive(Debug, Clone, PartialEq)]
pub struct BeatGrid {
    pub tempo: Tempo,
    pub beat_times_seconds: Vec<f64>,
}

/// Raw detected note event before musical quantization
#[derive(Debug, Clone, PartialEq)]
pub struct RawNoteEvent {
    pub start_time: f64,
    pub end_time: f64,
    pub pitch: Pitch,
    pub velocity: u8,
}

pub trait PitchDetector: Send + Sync {
    fn detect_pitch(&self, audio: &AudioBuffer) -> Result<Vec<PitchEvent>>;
}

pub trait OnsetDetector: Send + Sync {
    fn detect_onsets(&self, audio: &AudioBuffer) -> Result<Vec<OnsetEvent>>;
}

pub trait BeatDetector: Send + Sync {
    fn detect_beat_grid(&self, audio: &AudioBuffer) -> Result<BeatGrid>;
}

pub trait Quantizer: Send + Sync {
    fn quantize(
        &self,
        raw_events: &[RawNoteEvent],
        beat_grid: &BeatGrid,
    ) -> Result<Vec<Beat>>;
}
