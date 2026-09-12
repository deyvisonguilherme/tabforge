use std::str::FromStr;
use tabforge_core::Pitch;
use serde::{Deserialize, Serialize};

/// Tuning configuration for a stringed instrument
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuning {
    pub name: String,
    /// Pitches of strings from lowest pitch (thickest string) to highest pitch (thinnest string)
    pub strings: Vec<Pitch>,
}

impl Tuning {
    /// Standard 6-string guitar tuning: E2, A2, D3, G3, B3, E4
    pub fn standard_6_string() -> Self {
        Self {
            name: "Standard".to_string(),
            strings: vec![
                Pitch::from_str("E2").unwrap(),
                Pitch::from_str("A2").unwrap(),
                Pitch::from_str("D3").unwrap(),
                Pitch::from_str("G3").unwrap(),
                Pitch::from_str("B3").unwrap(),
                Pitch::from_str("E4").unwrap(),
            ],
        }
    }

    /// Drop D tuning: D2, A2, D3, G3, B3, E4
    pub fn drop_d() -> Self {
        Self {
            name: "Drop D".to_string(),
            strings: vec![
                Pitch::from_str("D2").unwrap(),
                Pitch::from_str("A2").unwrap(),
                Pitch::from_str("D3").unwrap(),
                Pitch::from_str("G3").unwrap(),
                Pitch::from_str("B3").unwrap(),
                Pitch::from_str("E4").unwrap(),
            ],
        }
    }

    /// 4-string Bass standard tuning: E1, A1, D2, G2
    pub fn bass_standard() -> Self {
        Self {
            name: "Bass Standard".to_string(),
            strings: vec![
                Pitch::from_str("E1").unwrap(),
                Pitch::from_str("A1").unwrap(),
                Pitch::from_str("D2").unwrap(),
                Pitch::from_str("G2").unwrap(),
            ],
        }
    }

    pub fn string_count(&self) -> usize {
        self.strings.len()
    }
}

impl Default for Tuning {
    fn default() -> Self {
        Self::standard_6_string()
    }
}
