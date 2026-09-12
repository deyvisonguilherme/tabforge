use crate::beat::Beat;
use crate::tempo::TimeSignature;
use serde::{Deserialize, Serialize};

/// A single musical measure (compasso)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measure {
    pub number: u32,
    pub time_signature: Option<TimeSignature>,
    pub beats: Vec<Beat>,
}

impl Measure {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            time_signature: None,
            beats: Vec::new(),
        }
    }

    pub fn add_beat(&mut self, beat: Beat) {
        self.beats.push(beat);
    }
}
