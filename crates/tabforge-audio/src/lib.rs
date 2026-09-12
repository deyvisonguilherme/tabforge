//! # tabforge-audio
//!
//! Audio decoding (Symphonia), DSP analysis, pitch detection (YIN),
//! onset detection, beat estimation, and source separation interfaces.

pub mod adsr;
pub mod articulation_detector;
pub mod beat_detector;
pub mod buffer;
pub mod decoder;
pub mod error;
pub mod model_manager;
pub mod onset_detector;
pub mod pitch_detector;
pub mod quantizer;
pub mod separator;
pub mod traits;
pub mod wav_writer;

pub use adsr::AdsrSegmenter;
pub use articulation_detector::ArticulationDetector;
pub use beat_detector::AutoCorrelationBeatDetector;
pub use buffer::AudioBuffer;
pub use decoder::{AudioDecoder, SymphoniaDecoder};
pub use error::{AudioError, Result};
pub use model_manager::{ModelInfo, ModelManager, KNOWN_MODELS};
pub use onset_detector::{EnergyOnsetDetector, SpectralOnsetDetector};
pub use pitch_detector::YinPitchDetector;
pub use quantizer::{segment_notes, SimpleQuantizer};
pub use separator::{
    HarmonicPercussiveSeparator, NeuralSourceSeparator, PassthroughSeparator, SeparatedTracks,
    SourceSeparator,
};
pub use traits::{BeatDetector, BeatGrid, OnsetDetector, OnsetEvent, PitchDetector, PitchEvent, Quantizer, RawNoteEvent};
pub use wav_writer::write_wav;
