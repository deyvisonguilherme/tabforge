use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{OnsetDetector, OnsetEvent};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::f32::consts::PI;

/// High-precision Spectral Flux Onset Detector using STFT and Adaptive Median Peak-Picking
pub struct SpectralOnsetDetector {
    pub window_size: usize,
    pub hop_size: usize,
    pub threshold_multiplier: f32,
    pub threshold_offset: f32,
    pub median_window_size: usize,
    pub min_interval_seconds: f64,
}

impl Default for SpectralOnsetDetector {
    fn default() -> Self {
        Self {
            window_size: 1024,
            hop_size: 256,
            threshold_multiplier: 1.5,
            threshold_offset: 0.04,
            median_window_size: 11,
            min_interval_seconds: 0.04, // 40ms refractory period
        }
    }
}

/// Backwards compatibility alias
pub type EnergyOnsetDetector = SpectralOnsetDetector;

impl OnsetDetector for SpectralOnsetDetector {
    fn detect_onsets(&self, audio: &AudioBuffer) -> Result<Vec<OnsetEvent>> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;

        if samples.len() < self.window_size {
            return Ok(Vec::new());
        }

        // 1. Prepare Hann window
        let hann: Vec<f32> = (0..self.window_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (self.window_size - 1) as f32).cos()))
            .collect();

        // 2. Compute STFT magnitude spectra
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.window_size);

        let num_frames = (samples.len() - self.window_size) / self.hop_size;
        let half_w = self.window_size / 2;
        let mut prev_magnitude = vec![0.0f32; half_w];
        let mut spectral_flux = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * self.hop_size;
            let window = &samples[start..start + self.window_size];

            let mut fft_buf: Vec<Complex<f32>> = (0..self.window_size)
                .map(|j| Complex::new(window[j] * hann[j], 0.0))
                .collect();

            fft.process(&mut fft_buf);

            // Compute half-wave rectified spectral flux
            let mut flux = 0.0f32;
            for k in 0..half_w {
                let mag = (fft_buf[k].re * fft_buf[k].re + fft_buf[k].im * fft_buf[k].im).sqrt();
                let diff = mag - prev_magnitude[k];
                if diff > 0.0 {
                    flux += diff;
                }
                prev_magnitude[k] = mag;
            }

            spectral_flux.push(flux);
        }

        // 3. Adaptive dynamic threshold peak-picking with moving median
        let mut onsets = Vec::new();
        let half_median = self.median_window_size / 2;
        let mut last_onset_time = -1.0;

        for i in 1..spectral_flux.len().saturating_sub(1) {
            let cur_flux = spectral_flux[i];

            // Local median over window
            let start = i.saturating_sub(half_median);
            let end = (i + half_median + 1).min(spectral_flux.len());
            let mut local_slice = spectral_flux[start..end].to_vec();
            local_slice.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = local_slice[local_slice.len() / 2];

            let dynamic_threshold = median * self.threshold_multiplier + self.threshold_offset;

            // Peak conditions: strictly greater than adaptive threshold, and a local maximum
            if cur_flux > dynamic_threshold
                && cur_flux >= spectral_flux[i - 1]
                && cur_flux >= spectral_flux[i + 1]
            {
                let time = (i * self.hop_size) as f64 / sample_rate;
                if time - last_onset_time >= self.min_interval_seconds {
                    let strength = cur_flux - dynamic_threshold;
                    onsets.push(OnsetEvent {
                        time_seconds: time,
                        strength,
                    });
                    last_onset_time = time;
                }
            }
        }

        Ok(onsets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_impulse_bursts(sample_rate: u32, burst_times: &[f32], total_secs: f32) -> AudioBuffer {
        let total_samples = (sample_rate as f32 * total_secs) as usize;
        let mut samples = vec![0.0f32; total_samples];

        for &bt in burst_times {
            let start_idx = (bt * sample_rate as f32) as usize;
            for j in 0..500 {
                if start_idx + j < total_samples {
                    // Decaying impulse burst
                    let decay = (-(j as f32) / 100.0).exp();
                    samples[start_idx + j] += 0.9 * decay;
                }
            }
        }

        AudioBuffer::new(samples, sample_rate, 1)
    }

    #[test]
    fn test_spectral_onset_detection_bursts() {
        let sample_rate = 44100;
        let expected_bursts = [0.25f32, 0.5f32, 0.75f32];
        let audio = generate_impulse_bursts(sample_rate, &expected_bursts, 1.0);

        let detector = SpectralOnsetDetector::default();
        let onsets = detector.detect_onsets(&audio).expect("onset detection failed");

        assert_eq!(onsets.len(), 3, "Expected 3 onsets, got {:?}", onsets);
        for (i, expected_time) in expected_bursts.iter().enumerate() {
            let error = (onsets[i].time_seconds - *expected_time as f64).abs();
            assert!(
                error < 0.03,
                "Onset {} error too high: expected {}, got {}",
                i,
                expected_time,
                onsets[i].time_seconds
            );
        }
    }

    #[test]
    fn test_spectral_onset_silence() {
        let sample_rate = 44100;
        let audio = AudioBuffer::new(vec![0.0; sample_rate as usize], sample_rate, 1);
        let detector = SpectralOnsetDetector::default();
        let onsets = detector.detect_onsets(&audio).unwrap();
        assert!(onsets.is_empty(), "Silence should yield no onsets");
    }
}
