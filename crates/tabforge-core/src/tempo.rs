use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Musical Tempo in Beats Per Minute (BPM)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Tempo {
    bpm: f64,
}

impl Tempo {
    pub const DEFAULT: Self = Self { bpm: 120.0 };

    pub fn new(bpm: f64) -> Result<Self> {
        if bpm <= 0.0 || bpm > 1000.0 {
            return Err(CoreError::InvalidPitchNotation(format!("Invalid BPM: {bpm}")));
        }
        Ok(Self { bpm })
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Duration of a single quarter note in seconds
    pub fn quarter_note_seconds(&self) -> f64 {
        60.0 / self.bpm
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Time Signature (e.g. 4/4, 3/4, 6/8, 7/8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u32,
    pub denominator: u32,
}

impl TimeSignature {
    pub const FOUR_FOUR: Self = Self { numerator: 4, denominator: 4 };
    pub const THREE_FOUR: Self = Self { numerator: 3, denominator: 4 };
    pub const SIX_EIGHT: Self = Self { numerator: 6, denominator: 8 };

    pub fn new(numerator: u32, denominator: u32) -> Result<Self> {
        if numerator == 0 || denominator == 0 || !denominator.is_power_of_two() {
            return Err(CoreError::InvalidTimeSignature { numerator, denominator });
        }
        Ok(Self { numerator, denominator })
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::FOUR_FOUR
    }
}

impl fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}
