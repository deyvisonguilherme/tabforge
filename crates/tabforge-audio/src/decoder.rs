use crate::buffer::AudioBuffer;
use crate::error::{AudioError, Result};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub trait AudioDecoder {
    fn decode(&self, path: &Path) -> Result<AudioBuffer>;
}

/// Robust audio decoder powered by Symphonia supporting universal sample formats,
/// dynamic packet buffering, stream error recovery, automatic resampling and peak normalization.
#[derive(Debug, Clone)]
pub struct SymphoniaDecoder {
    pub target_sample_rate: Option<u32>,
    pub auto_normalize: bool,
    pub force_mono: bool,
    pub peak_target: f32,
}

impl Default for SymphoniaDecoder {
    fn default() -> Self {
        Self {
            target_sample_rate: Some(44100),
            auto_normalize: true,
            force_mono: true,
            peak_target: 0.95,
        }
    }
}

impl SymphoniaDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_target_sample_rate(mut self, target_sample_rate: Option<u32>) -> Self {
        self.target_sample_rate = target_sample_rate;
        self
    }

    pub fn with_auto_normalize(mut self, auto_normalize: bool) -> Self {
        self.auto_normalize = auto_normalize;
        self
    }

    pub fn with_force_mono(mut self, force_mono: bool) -> Self {
        self.force_mono = force_mono;
        self
    }

    pub fn with_peak_target(mut self, peak_target: f32) -> Self {
        self.peak_target = peak_target;
        self
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
        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;
                    sample_rate = spec.rate;
                    channels = spec.channels.count() as u16;

                    let need_realloc = match &sample_buf {
                        Some(buf) => (buf.capacity() as u64) < duration,
                        None => true,
                    };

                    if need_realloc {
                        sample_buf = Some(SampleBuffer::<f32>::new(duration.max(4096), spec));
                    }

                    if let Some(ref mut s_buf) = sample_buf {
                        s_buf.copy_interleaved_ref(decoded);
                        samples.extend_from_slice(s_buf.samples());
                    }
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(SymphoniaError::ResetRequired) => {
                    tracing::warn!("Reset required during audio decoding, resetting decoder");
                    decoder.reset();
                }
                Err(SymphoniaError::DecodeError(msg)) => {
                    tracing::warn!("Corrupt packet skipped: {msg}");
                    continue;
                }
                Err(SymphoniaError::SeekError(e)) => {
                    tracing::warn!("Seek error encountered during audio decoding: {e:?}");
                    continue;
                }
                Err(e) => {
                    return Err(AudioError::DecodeError(e.to_string()));
                }
            }
        }

        // Post-decoding pipeline: multi-channel downmix -> cubic resampling -> peak normalization
        let mut buffer = AudioBuffer::new(samples, sample_rate, channels);

        if self.force_mono && buffer.channels > 1 {
            buffer = buffer.to_mono_weighted();
        }

        if let Some(target_rate) = self.target_sample_rate {
            if buffer.sample_rate != target_rate && buffer.sample_rate != 0 {
                buffer = buffer.resample(target_rate);
            }
        }

        if self.auto_normalize {
            buffer.normalize(self.peak_target);
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_builder_defaults() {
        let decoder = SymphoniaDecoder::new()
            .with_target_sample_rate(Some(48000))
            .with_auto_normalize(false)
            .with_force_mono(true)
            .with_peak_target(0.90);

        assert_eq!(decoder.target_sample_rate, Some(48000));
        assert!(!decoder.auto_normalize);
        assert!(decoder.force_mono);
        assert_eq!(decoder.peak_target, 0.90);
    }
}
