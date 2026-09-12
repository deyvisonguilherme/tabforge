use crate::buffer::AudioBuffer;
use crate::error::Result;
use std::collections::HashMap;

/// Source separated stems for audio tracks (e.g. Guitar, Bass, Drums, Vocals)
#[derive(Debug, Clone)]
pub struct SeparatedTracks {
    pub stems: HashMap<String, AudioBuffer>,
}

pub trait SourceSeparator: Send + Sync {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks>;
}

/// Passthrough / Mock source separator
pub struct PassthroughSeparator;

impl SourceSeparator for PassthroughSeparator {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks> {
        let mut stems = HashMap::new();
        stems.insert("guitar".to_string(), audio.clone());
        Ok(SeparatedTracks { stems })
    }
}
