use crate::beat::Beat;
use crate::duration::Duration;
use crate::tempo::{Tempo, TimeSignature};
use serde::{Deserialize, Serialize};

/// A single musical measure (compasso)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measure {
    pub number: u32,
    pub time_signature: Option<TimeSignature>,
    pub tempo: Option<Tempo>,
    pub beats: Vec<Beat>,
}

impl Measure {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            time_signature: None,
            tempo: None,
            beats: Vec::new(),
        }
    }

    pub fn with_time_signature(mut self, time_signature: TimeSignature) -> Self {
        self.time_signature = Some(time_signature);
        self
    }

    pub fn with_tempo(mut self, tempo: Tempo) -> Self {
        self.tempo = Some(tempo);
        self
    }

    pub fn add_beat(&mut self, beat: Beat) {
        self.beats.push(beat);
    }

    /// Calculate total filled duration of beats in this measure
    pub fn total_duration(&self) -> Duration {
        self.beats.iter().map(|b| b.duration).fold(Duration::ZERO, |acc, d| acc + d)
    }
}
