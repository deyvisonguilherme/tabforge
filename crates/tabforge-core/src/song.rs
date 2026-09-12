use crate::tempo::{Tempo, TimeSignature};
use crate::track::Track;
use serde::{Deserialize, Serialize};

/// Root Song model in TabForge Music IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
    pub artist: Option<String>,
    pub tempo: Tempo,
    pub time_signature: TimeSignature,
    pub tracks: Vec<Track>,
}

impl Song {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            artist: None,
            tempo: Tempo::default(),
            time_signature: TimeSignature::default(),
            tracks: Vec::new(),
        }
    }

    pub fn with_tempo(mut self, tempo: Tempo) -> Self {
        self.tempo = tempo;
        self
    }

    pub fn with_time_signature(mut self, time_sig: TimeSignature) -> Self {
        self.time_signature = time_sig;
        self
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }
}
