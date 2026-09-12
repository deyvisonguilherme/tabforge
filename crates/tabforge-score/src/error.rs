use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScoreError {
    #[error("Score generation error: {0}")]
    GenerationError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ScoreError>;
