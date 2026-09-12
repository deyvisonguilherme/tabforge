use thiserror::Error;

#[derive(Error, Debug)]
pub enum MidiError {
    #[error("MIDI parse error: {0}")]
    ParseError(String),

    #[error("MIDI write error: {0}")]
    WriteError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MidiError>;
