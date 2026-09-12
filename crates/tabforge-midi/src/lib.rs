//! # tabforge-midi
//!
//! Fast, pure-Rust MIDI import and export engine for TabForge Music IR.

pub mod error;
pub mod reader;
pub mod writer;

pub use error::{MidiError, Result};
pub use reader::{MidiReader, MidlyReader};
pub use writer::{MidiWriter, MidlyWriter};
