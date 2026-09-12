//! # tabforge-core
//!
//! Independent Music Intermediate Representation (IR) for TabForge.
//! This crate contains pure musical concepts, data types, and arithmetic
//! completely decoupled from audio engines, file codecs, and presentation layers.

pub mod beat;
pub mod duration;
pub mod error;
pub mod instrument;
pub mod measure;
pub mod note;
pub mod pitch;
pub mod song;
pub mod tempo;
pub mod track;

pub use beat::Beat;
pub use duration::Duration;
pub use error::{CoreError, Result};
pub use instrument::Instrument;
pub use measure::Measure;
pub use note::{Articulation, Note};
pub use pitch::{NoteName, Pitch};
pub use song::Song;
pub use tempo::{Tempo, TempoChange, TimeSignature, TimeSignatureChange};
pub use track::Track;
