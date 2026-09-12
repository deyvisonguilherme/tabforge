use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum GuitarError {
    #[error("Invalid tuning string count: {0}")]
    InvalidStringCount(usize),

    #[error("Note out of guitar range: {0}")]
    OutOfRange(String),

    #[error("No viable fingering path found")]
    NoViableFingering,
}

pub type Result<T> = std::result::Result<T, GuitarError>;
