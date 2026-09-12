use crate::tuning::Tuning;
use tabforge_core::Pitch;
use serde::{Deserialize, Serialize};

/// Exact fretboard position (string 1 = thinnest/highest pitch string, fret 0 = open string)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FretPosition {
    /// 1-indexed string number from highest pitch string (1) to lowest (e.g. 6)
    pub string: u8,
    /// Fret number (0 for open string)
    pub fret: u8,
}

/// Representation of the instrument fretboard
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fretboard {
    pub tuning: Tuning,
    pub fret_count: u8,
}

impl Fretboard {
    pub fn new(tuning: Tuning, fret_count: u8) -> Self {
        Self { tuning, fret_count }
    }

    /// Find all physical fretboard positions where a given pitch can be played
    pub fn find_positions(&self, pitch: &Pitch) -> Vec<FretPosition> {
        let mut positions = Vec::new();
        let target_midi = pitch.midi();
        let total_strings = self.tuning.strings.len();

        for (idx, open_pitch) in self.tuning.strings.iter().enumerate() {
            let open_midi = open_pitch.midi();
            if target_midi >= open_midi {
                let fret = (target_midi - open_midi) as u8;
                if fret <= self.fret_count {
                    // String 1 is the highest string (last in tuning array)
                    let string_num = (total_strings - idx) as u8;
                    positions.push(FretPosition {
                        string: string_num,
                        fret,
                    });
                }
            }
        }

        positions
    }
}

impl Default for Fretboard {
    fn default() -> Self {
        Self::new(Tuning::standard_6_string(), 24)
    }
}
