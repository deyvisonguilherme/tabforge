use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{PitchDetector, PitchEvent};
use tabforge_core::Pitch;

/// Autocorrelation / YIN-based monophonic Pitch Detector
pub struct YinPitchDetector {
    pub window_size: usize,
    pub hop_size: usize,
    pub threshold: f32,
}

impl Default for YinPitchDetector {
    fn default() -> Self {
        Self {
            window_size: 2048,
            hop_size: 512,
            threshold: 0.15,
        }
    }
}

impl PitchDetector for YinPitchDetector {
    fn detect_pitch(&self, audio: &AudioBuffer) -> Result<Vec<PitchEvent>> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;
        let mut events = Vec::new();

        if samples.len() < self.window_size {
            return Ok(events);
        }

        let num_frames = (samples.len() - self.window_size) / self.hop_size;
        for i in 0..num_frames {
            let start = i * self.hop_size;
            let window = &samples[start..start + self.window_size];
            let time = start as f64 / sample_rate;

            // Simplified YIN difference function
            if let Some((freq, confidence)) = self.yin_pitch_estimate(window, sample_rate) {
                if let Some(pitch) = Pitch::from_frequency(freq) {
                    events.push(PitchEvent {
                        time_seconds: time,
                        pitch,
                        confidence,
                        frequency: freq,
                    });
                }
            }
        }

        Ok(events)
    }
}

impl YinPitchDetector {
    fn yin_pitch_estimate(&self, window: &[f32], sample_rate: f64) -> Option<(f64, f32)> {
        let half_w = window.len() / 2;
        let mut d = vec![0.0f32; half_w];

        // Step 1: Difference function
        for tau in 0..half_w {
            for j in 0..half_w {
                let diff = window[j] - window[j + tau];
                d[tau] += diff * diff;
            }
        }

        // Step 2: Cumulative mean normalized difference
        let mut cmndf = vec![0.0f32; half_w];
        cmndf[0] = 1.0;
        let mut running_sum = 0.0f32;
        for tau in 1..half_w {
            running_sum += d[tau];
            if running_sum > 0.0 {
                cmndf[tau] = d[tau] * (tau as f32) / running_sum;
            } else {
                cmndf[tau] = 1.0;
            }
        }

        // Step 3: Absolute threshold
        let min_tau = (sample_rate / 1200.0) as usize; // Max freq 1200 Hz
        let max_tau = (sample_rate / 60.0) as usize;   // Min freq 60 Hz
        let end_tau = max_tau.min(half_w);

        for tau in min_tau..end_tau {
            if cmndf[tau] < self.threshold {
                let mut best_tau = tau;
                while best_tau + 1 < end_tau && cmndf[best_tau + 1] < cmndf[best_tau] {
                    best_tau += 1;
                }
                let confidence = 1.0 - cmndf[best_tau];
                let freq = sample_rate / best_tau as f64;
                return Some((freq, confidence));
            }
        }

        None
    }
}
