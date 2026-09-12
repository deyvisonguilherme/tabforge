use serde::{Deserialize, Serialize};

/// Chord shape definition on a guitar fretboard
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChordShape {
    pub name: String,
    /// Frets for strings [E, A, D, G, B, e] where None means muted (x)
    pub frets: Vec<Option<u8>>,
}

impl ChordShape {
    pub fn new(name: impl Into<String>, frets: Vec<Option<u8>>) -> Self {
        Self {
            name: name.into(),
            frets,
        }
    }

    /// Common standard open chords dictionary
    pub fn standard_open_chords() -> Vec<Self> {
        vec![
            ChordShape::new("C", vec![None, Some(3), Some(2), Some(0), Some(1), Some(0)]),
            ChordShape::new("D", vec![None, None, Some(0), Some(2), Some(3), Some(2)]),
            ChordShape::new("E", vec![Some(0), Some(2), Some(2), Some(1), Some(0), Some(0)]),
            ChordShape::new("G", vec![Some(3), Some(2), Some(0), Some(0), Some(0), Some(3)]),
            ChordShape::new("A", vec![None, Some(0), Some(2), Some(2), Some(2), Some(0)]),
            ChordShape::new("Em", vec![Some(0), Some(2), Some(2), Some(0), Some(0), Some(0)]),
            ChordShape::new("Am", vec![None, Some(0), Some(2), Some(2), Some(1), Some(0)]),
            ChordShape::new("Dm", vec![None, None, Some(0), Some(2), Some(3), Some(1)]),
        ]
    }
}
