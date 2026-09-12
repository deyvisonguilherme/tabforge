use serde::{Deserialize, Serialize};

/// Canonical in-memory representation of raw PCM audio samples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBuffer {
    /// Interleaved or mono audio samples normalized to [-1.0, 1.0]
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
        }
    }

    /// Total duration of audio in seconds
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            0.0
        } else {
            self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
        }
    }

    /// Convert multi-channel buffer to a single mono channel by averaging channels
    pub fn to_mono(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }

        let num_frames = self.samples.len() / self.channels as usize;
        let mut mono_samples = Vec::with_capacity(num_frames);

        for frame in 0..num_frames {
            let mut sum = 0.0;
            for ch in 0..self.channels as usize {
                sum += self.samples[frame * self.channels as usize + ch];
            }
            mono_samples.push(sum / self.channels as f32);
        }

        Self {
            samples: mono_samples,
            sample_rate: self.sample_rate,
            channels: 1,
        }
    }
}
