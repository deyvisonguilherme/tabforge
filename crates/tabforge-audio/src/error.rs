use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio decode error: {0}")]
    DecodeError(String),

    #[error("Unsupported audio format or codec")]
    UnsupportedFormat,

    #[error("No audio track found in file")]
    NoTrackFound,

    #[error("DSP processing error: {0}")]
    DspError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AudioError>;
