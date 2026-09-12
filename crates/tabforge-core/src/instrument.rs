use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instrument {
    AcousticGuitar,
    ElectricGuitarClean,
    ElectricGuitarOverdrive,
    ElectricBass,
    Piano,
    Generic,
}

impl Default for Instrument {
    fn default() -> Self {
        Instrument::ElectricGuitarClean
    }
}
