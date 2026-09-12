use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{OnsetDetector, OnsetEvent};

/// Spectral flux / Energy-based Onset Detector
pub struct EnergyOnsetDetector {
    pub hop_size: usize,
    pub threshold: f32,
}

impl Default for EnergyOnsetDetector {
    fn default() -> Self {
        Self {
            hop_size: 512,
            threshold: 1.5,
        }
    }
}

impl OnsetDetector for EnergyOnsetDetector {
    fn detect_onsets(&self, audio: &AudioBuffer) -> Result<Vec<OnsetEvent>> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;
        let mut onsets = Vec::new();

        let num_frames = samples.len() / self.hop_size;
        let mut energies = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * self.hop_size;
            let end = (start + self.hop_size).min(samples.len());
            let energy: f32 = samples[start..end].iter().map(|s| s * s).sum();
            energies.push(energy);
        }

        // Detect sudden positive energy flux
        for i in 1..energies.len() {
            let diff = energies[i] - energies[i - 1];
            if diff > self.threshold && energies[i] > 0.01 {
                let time = (i * self.hop_size) as f64 / sample_rate;
                onsets.push(OnsetEvent {
                    time_seconds: time,
                    strength: diff,
                });
            }
        }

        Ok(onsets)
    }
}
