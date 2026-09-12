//! # tabforge-audio
//!
//! Audio decoding (Symphonia), DSP analysis, pitch detection (YIN),
//! onset detection, beat estimation, and source separation interfaces.

pub mod beat_detector;
pub mod buffer;
pub mod decoder;
pub mod error;
pub mod onset_detector;
pub mod pitch_detector;
pub mod quantizer;
pub mod separator;
pub mod traits;

pub use beat_detector::AutoCorrelationBeatDetector;
pub use buffer::AudioBuffer;
pub use decoder::{AudioDecoder, SymphoniaDecoder};
pub use error::{AudioError, Result};
pub use onset_detector::EnergyOnsetDetector;
pub use pitch_detector::YinPitchDetector;
pub use quantizer::SimpleQuantizer;
pub use separator::{PassthroughSeparator, SeparatedTracks, SourceSeparator};
pub use traits::{BeatDetector, BeatGrid, OnsetDetector, OnsetEvent, PitchDetector, PitchEvent, Quantizer, RawNoteEvent};
