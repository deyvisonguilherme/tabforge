use crate::buffer::AudioBuffer;
use crate::error::{AudioError, Result};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub trait AudioDecoder {
    fn decode(&self, path: &Path) -> Result<AudioBuffer>;
}

/// Pure-Rust audio decoder powered by Symphonia
pub struct SymphoniaDecoder;

impl Default for SymphoniaDecoder {
    fn default() -> Self {
        Self
    }
}

impl AudioDecoder for SymphoniaDecoder {
    fn decode(&self, path: &Path) -> Result<AudioBuffer> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let mut format = probed.format;

        // Find the default audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(AudioError::NoTrackFound)?;

        let track_id = track.id;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let mut samples = Vec::new();
        let mut sample_rate = 44100;
        let mut channels = 1;

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    match decoded {
                        AudioBufferRef::F32(buf) => {
                            sample_rate = buf.spec().rate;
                            channels = buf.spec().channels.count() as u16;
                            for frame in 0..buf.frames() {
                                for ch in 0..buf.spec().channels.count() {
                                    samples.push(buf.chan(ch)[frame]);
                                }
                            }
                        }
                        AudioBufferRef::S16(buf) => {
                            sample_rate = buf.spec().rate;
                            channels = buf.spec().channels.count() as u16;
                            for frame in 0..buf.frames() {
                                for ch in 0..buf.spec().channels.count() {
                                    let sample_s16 = buf.chan(ch)[frame];
                                    samples.push(sample_s16 as f32 / 32768.0);
                                }
                            }
                        }
                        _ => {
                            // Unsupported sample format in this fast path
                        }
                    }
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Skip corrupt packet
                    continue;
                }
                Err(e) => {
                    return Err(AudioError::DecodeError(e.to_string()));
                }
            }
        }

        Ok(AudioBuffer::new(samples, sample_rate, channels))
    }
}
