use crate::duration::Duration;
use crate::note::Note;
use serde::{Deserialize, Serialize};

/// A musical beat / voice event at a specific relative measure offset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beat {
    /// Relative position from the start of the measure
    pub position: Duration,
    /// Duration of this beat or chord slice
    pub duration: Duration,
    /// Simultaneous notes (monophonic = 1 note, chord/polyphony = >1 notes, rest = 0 notes)
    pub notes: Vec<Note>,
}

impl Beat {
    pub fn new(position: Duration, duration: Duration) -> Self {
        Self {
            position,
            duration,
            notes: Vec::new(),
        }
    }

    pub fn with_notes(position: Duration, duration: Duration, notes: Vec<Note>) -> Self {
        Self {
            position,
            duration,
            notes,
        }
    }

    pub fn is_rest(&self) -> bool {
        self.notes.is_empty()
    }
}
