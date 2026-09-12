use crate::duration::Duration;
use crate::pitch::Pitch;
use serde::{Deserialize, Serialize};

/// Articulations and instrument playing techniques
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Articulation {
    Staccato,
    Accent,
    Tenuto,
    HammerOn,
    PullOff,
    SlideUp,
    SlideDown,
    Bend,
    ReleaseBend,
    Vibrato,
    PalmMute,
    Tapping,
    Harmonic,
    GhostNote,
}

/// A discrete musical note event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub pitch: Pitch,
    pub duration: Duration,
    pub velocity: u8,
    pub articulation: Option<Articulation>,
    pub tie_forward: bool,
}

impl Note {
    pub fn new(pitch: Pitch, duration: Duration) -> Self {
        Self {
            pitch,
            duration,
            velocity: 80,
            articulation: None,
            tie_forward: false,
        }
    }

    pub fn with_velocity(mut self, velocity: u8) -> Self {
        self.velocity = velocity.min(127);
        self
    }

    pub fn with_articulation(mut self, articulation: Articulation) -> Self {
        self.articulation = Some(articulation);
        self
    }
}
