use crate::instrument::Instrument;
use crate::measure::Measure;
use serde::{Deserialize, Serialize};

/// A musical track containing measures for a specific instrument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub instrument: Instrument,
    pub channel: u8,
    pub measures: Vec<Measure>,
}

impl Track {
    pub fn new(name: impl Into<String>, instrument: Instrument) -> Self {
        Self {
            name: name.into(),
            instrument,
            channel: 0,
            measures: Vec::new(),
        }
    }

    pub fn add_measure(&mut self, measure: Measure) {
        self.measures.push(measure);
    }
}
