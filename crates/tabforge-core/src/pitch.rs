use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Note letter in the chromatic scale
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoteName {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl NoteName {
    pub fn from_semitone(semitone: u8) -> Self {
        match semitone % 12 {
            0 => NoteName::C,
            1 => NoteName::CSharp,
            2 => NoteName::D,
            3 => NoteName::DSharp,
            4 => NoteName::E,
            5 => NoteName::F,
            6 => NoteName::FSharp,
            7 => NoteName::G,
            8 => NoteName::GSharp,
            9 => NoteName::A,
            10 => NoteName::ASharp,
            11 => NoteName::B,
            _ => unreachable!(),
        }
    }

    pub fn to_semitone(self) -> u8 {
        match self {
            NoteName::C => 0,
            NoteName::CSharp => 1,
            NoteName::D => 2,
            NoteName::DSharp => 3,
            NoteName::E => 4,
            NoteName::F => 5,
            NoteName::FSharp => 6,
            NoteName::G => 7,
            NoteName::GSharp => 8,
            NoteName::A => 9,
            NoteName::ASharp => 10,
            NoteName::B => 11,
        }
    }

    pub fn name_with_sharp(self) -> &'static str {
        match self {
            NoteName::C => "C",
            NoteName::CSharp => "C#",
            NoteName::D => "D",
            NoteName::DSharp => "D#",
            NoteName::E => "E",
            NoteName::F => "F",
            NoteName::FSharp => "F#",
            NoteName::G => "G",
            NoteName::GSharp => "G#",
            NoteName::A => "A",
            NoteName::ASharp => "A#",
            NoteName::B => "B",
        }
    }
}

/// Discrete Musical Pitch representation (0..=127 MIDI standard)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pitch {
    midi: u8,
}

impl Pitch {
    /// Creates a pitch from a standard MIDI note number (0 to 127)
    pub fn from_midi(midi: u8) -> Result<Self> {
        if midi > 127 {
            return Err(CoreError::InvalidMidiPitch(midi));
        }
        Ok(Self { midi })
    }

    /// Creates a pitch from note name and octave (-1 to 9)
    pub fn from_name_and_octave(name: NoteName, octave: i8) -> Result<Self> {
        let midi_val = (octave + 1) * 12 + name.to_semitone() as i8;
        if !(0..=127).contains(&midi_val) {
            return Err(CoreError::InvalidPitchNotation(format!("{name:?}{octave}")));
        }
        Ok(Self {
            midi: midi_val as u8,
        })
    }

    pub fn midi(self) -> u8 {
        self.midi
    }

    pub fn note_name(self) -> NoteName {
        NoteName::from_semitone(self.midi % 12)
    }

    pub fn octave(self) -> i8 {
        (self.midi as i8 / 12) - 1
    }

    /// Calculate frequency in Hertz (A4 = 440 Hz)
    pub fn frequency(self) -> f64 {
        440.0 * 2.0_f64.powf((self.midi as f64 - 69.0) / 12.0)
    }

    /// Find closest pitch from frequency in Hertz
    pub fn from_frequency(hz: f64) -> Option<Self> {
        if hz <= 0.0 {
            return None;
        }
        let midi_f = 69.0 + 12.0 * (hz / 440.0).log2();
        let midi_round = midi_f.round() as i32;
        if (0..=127).contains(&midi_round) {
            Some(Self {
                midi: midi_round as u8,
            })
        } else {
            None
        }
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.note_name().name_with_sharp(), self.octave())
    }
}

impl FromStr for Pitch {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(CoreError::InvalidPitchNotation(s.to_string()));
        }

        let (note_str, octave_str) = if trimmed.len() >= 2 && (trimmed.as_bytes()[1] == b'#' || trimmed.as_bytes()[1] == b'b') {
            (&trimmed[0..2], &trimmed[2..])
        } else {
            (&trimmed[0..1], &trimmed[1..])
        };

        let octave: i8 = octave_str
            .parse()
            .map_err(|_| CoreError::InvalidPitchNotation(s.to_string()))?;

        let note_name = match note_str.to_uppercase().as_str() {
            "C" => NoteName::C,
            "C#" | "DB" => NoteName::CSharp,
            "D" => NoteName::D,
            "D#" | "EB" => NoteName::DSharp,
            "E" => NoteName::E,
            "F" => NoteName::F,
            "F#" | "GB" => NoteName::FSharp,
            "G" => NoteName::G,
            "G#" | "AB" => NoteName::GSharp,
            "A" => NoteName::A,
            "A#" | "BB" => NoteName::ASharp,
            "B" => NoteName::B,
            _ => return Err(CoreError::InvalidPitchNotation(s.to_string())),
        };

        Pitch::from_name_and_octave(note_name, octave)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_midi_conversion() {
        let a4 = Pitch::from_str("A4").unwrap();
        assert_eq!(a4.midi(), 69);
        assert!((a4.frequency() - 440.0).abs() < 1e-3);
        assert_eq!(a4.to_string(), "A4");

        let c4 = Pitch::from_str("C4").unwrap();
        assert_eq!(c4.midi(), 60);

        let e2 = Pitch::from_str("E2").unwrap();
        assert_eq!(e2.midi(), 40);
    }
}
