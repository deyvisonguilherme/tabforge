use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CoreError {
    #[error("Invalid MIDI pitch number: {0}")]
    InvalidMidiPitch(u8),

    #[error("Invalid pitch notation: {0}")]
    InvalidPitchNotation(String),

    #[error("Invalid time signature: {numerator}/{denominator}")]
    InvalidTimeSignature { numerator: u32, denominator: u32 },

    #[error("Invalid duration value: {0}")]
    InvalidDuration(String),

    #[error("Measure overflow: total beats duration exceeds measure capacity")]
    MeasureOverflow,
}

pub type Result<T> = std::result::Result<T, CoreError>;
