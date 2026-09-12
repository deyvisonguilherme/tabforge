//! # tabforge-guitar
//!
//! Guitar physical engine, tunings, fretboard position mapper,
//! chord databases, and Dynamic Programming fingering optimizer.

pub mod chords;
pub mod error;
pub mod fretboard;
pub mod optimizer;
pub mod tuning;

pub use chords::ChordShape;
pub use error::{GuitarError, Result};
pub use fretboard::{FretPosition, Fretboard};
pub use optimizer::{CostWeights, FingeringOptimizer};
pub use tuning::Tuning;
