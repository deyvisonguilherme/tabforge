use crate::buffer::AudioBuffer;
use crate::error::Result;
use tabforge_core::{Beat, Pitch, Tempo, TempoChange, TimeSignature, TimeSignatureChange};

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

/// Beat grid with detected tempo, time signatures, dynamic changes and beat timestamps
#[derive(Debug, Clone, PartialEq)]
pub struct BeatGrid {
    pub tempo: Tempo,
    pub time_signature: TimeSignature,
    pub beat_times_seconds: Vec<f64>,
    pub tempo_changes: Vec<TempoChange>,
    pub time_sig_changes: Vec<TimeSignatureChange>,
}

impl BeatGrid {
    pub fn new(tempo: Tempo, beat_times_seconds: Vec<f64>) -> Self {
        Self {
            tempo,
            time_signature: TimeSignature::FOUR_FOUR,
            beat_times_seconds,
            tempo_changes: Vec::new(),
            time_sig_changes: Vec::new(),
        }
    }

    pub fn with_time_signature(mut self, time_signature: TimeSignature) -> Self {
        self.time_signature = time_signature;
        self
    }

    /// Get current effective tempo at a given time in seconds
    pub fn tempo_at(&self, time_seconds: f64) -> Tempo {
        if self.tempo_changes.is_empty() {
            return self.tempo;
        }

        let mut current = self.tempo;
        for change in &self.tempo_changes {
            if change.time_seconds <= time_seconds {
                current = change.tempo;
            } else {
                break;
            }
        }
        current
    }

    /// Get current effective time signature at a given time in seconds
    pub fn time_signature_at(&self, time_seconds: f64) -> TimeSignature {
        if self.time_sig_changes.is_empty() {
            return self.time_signature;
        }

        let mut current = self.time_signature;
        for change in &self.time_sig_changes {
            if change.time_seconds <= time_seconds {
                current = change.time_signature;
            } else {
                break;
            }
        }
        current
    }
}

/// Raw detected note event before musical quantization
#[derive(Debug, Clone, PartialEq)]
pub struct RawNoteEvent {
    pub start_time: f64,
    pub end_time: f64,
    pub pitch: Pitch,
    pub velocity: u8,
    pub articulation: Option<tabforge_core::Articulation>,
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
