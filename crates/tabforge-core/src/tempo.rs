use crate::duration::Duration;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} BPM", self.bpm)
    }
}

/// Dynamic tempo change event at a specific timestamp or timeline offset
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    pub time_seconds: f64,
    pub tempo: Tempo,
}

/// Dynamic time signature change event at a specific timestamp or timeline offset
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeSignatureChange {
    pub time_seconds: f64,
    pub time_signature: TimeSignature,
}

/// Time Signature (e.g. 4/4, 3/4, 6/8, 7/8, 5/8, 12/8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u32,
    pub denominator: u32,
}

impl TimeSignature {
    pub const FOUR_FOUR: Self = Self { numerator: 4, denominator: 4 };
    pub const THREE_FOUR: Self = Self { numerator: 3, denominator: 4 };
    pub const TWO_FOUR: Self = Self { numerator: 2, denominator: 4 };
    pub const FIVE_FOUR: Self = Self { numerator: 5, denominator: 4 };
    pub const SIX_EIGHT: Self = Self { numerator: 6, denominator: 8 };
    pub const SEVEN_EIGHT: Self = Self { numerator: 7, denominator: 8 };
    pub const FIVE_EIGHT: Self = Self { numerator: 5, denominator: 8 };
    pub const NINE_EIGHT: Self = Self { numerator: 9, denominator: 8 };
    pub const TWELVE_EIGHT: Self = Self { numerator: 12, denominator: 8 };

    pub fn new(numerator: u32, denominator: u32) -> Result<Self> {
        if numerator == 0 || denominator == 0 || !denominator.is_power_of_two() {
            return Err(CoreError::InvalidTimeSignature { numerator, denominator });
        }
        Ok(Self { numerator, denominator })
    }

    /// Total duration of one complete measure in this time signature
    pub fn measure_duration(&self) -> Duration {
        Duration::new(self.numerator as i32, self.denominator as i32)
    }

    /// Check if this time signature is compound (e.g. 6/8, 9/8, 12/8 with dotted quarter pulses)
    pub fn is_compound(&self) -> bool {
        self.denominator == 8 && (self.numerator % 3 == 0) && self.numerator >= 6
    }

    /// Check if this time signature is odd / asymmetric (e.g. 5/8, 7/8)
    pub fn is_asymmetric(&self) -> bool {
        self.denominator == 8 && (self.numerator == 5 || self.numerator == 7 || self.numerator == 11)
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

impl FromStr for TimeSignature {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.trim().split('/').collect();
        if parts.len() != 2 {
            return Err(CoreError::InvalidPitchNotation(format!(
                "Invalid time signature format (expected e.g. 4/4 or 6/8): '{s}'"
            )));
        }

        let numerator = parts[0]
            .parse::<u32>()
            .map_err(|_| CoreError::InvalidPitchNotation(format!("Invalid numerator: '{}'", parts[0])))?;
        let denominator = parts[1]
            .parse::<u32>()
            .map_err(|_| CoreError::InvalidPitchNotation(format!("Invalid denominator: '{}'", parts[1])))?;

        Self::new(numerator, denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_signature_parsing_and_capacity() {
        let ts_6_8 = TimeSignature::from_str("6/8").unwrap();
        assert_eq!(ts_6_8, TimeSignature::SIX_EIGHT);
        assert!(ts_6_8.is_compound());
        assert_eq!(ts_6_8.measure_duration(), Duration::new(6, 8)); // 3/4 rational

        let ts_7_8 = TimeSignature::from_str("7/8").unwrap();
        assert_eq!(ts_7_8, TimeSignature::SEVEN_EIGHT);
        assert!(ts_7_8.is_asymmetric());
        assert_eq!(ts_7_8.measure_duration(), Duration::new(7, 8));

        let ts_3_4 = TimeSignature::from_str("3/4").unwrap();
        assert_eq!(ts_3_4, TimeSignature::THREE_FOUR);
        assert_eq!(ts_3_4.measure_duration(), Duration::new(3, 4));
    }
}
