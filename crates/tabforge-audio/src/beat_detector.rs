use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{BeatDetector, BeatGrid};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::f32::consts::PI;
use tabforge_core::Tempo;

/// Dynamic Autocorrelation Beat and Tempo (BPM) Detector
pub struct AutoCorrelationBeatDetector {
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub window_size: usize,
    pub hop_size: usize,
    pub tempo_prior_bpm: f64,
}

impl Default for AutoCorrelationBeatDetector {
    fn default() -> Self {
        Self {
            min_bpm: 60.0,
            max_bpm: 220.0,
            window_size: 1024,
            hop_size: 512,
            tempo_prior_bpm: 120.0,
        }
    }
}

impl BeatDetector for AutoCorrelationBeatDetector {
    fn detect_beat_grid(&self, audio: &AudioBuffer) -> Result<BeatGrid> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;
        let duration = mono.duration_seconds();

        if samples.len() < self.window_size * 2 || duration < 0.5 {
            let default_tempo = Tempo::new(120.0).unwrap();
            let beat_interval = 60.0 / 120.0;
            let mut beat_times = Vec::new();
            let mut t = 0.0;
            while t < duration {
                beat_times.push(t);
                t += beat_interval;
            }
            return Ok(BeatGrid {
                tempo: default_tempo,
                beat_times_seconds: beat_times,
            });
        }

        // 1. Compute Onset Strength Envelope (Spectral Flux)
        let envelope = self.compute_onset_strength_envelope(samples);
        let env_fps = sample_rate / self.hop_size as f64;

        // 2. Estimate BPM via Autocorrelation & Gaussian Tempo Prior
        let estimated_bpm = self.estimate_bpm(&envelope, env_fps);
        let tempo = Tempo::new(estimated_bpm).unwrap_or_else(|_| Tempo::new(120.0).unwrap());
        let beat_interval_secs = 60.0 / tempo.bpm();

        // 3. Phase Alignment: Determine best starting offset t0 to maximize energy at beat times
        let t0 = self.align_beat_phase(&envelope, env_fps, beat_interval_secs, duration);

        // 4. Generate aligned BeatGrid
        let mut beat_times = Vec::new();
        let mut t = t0;
        while t < duration {
            if t >= 0.0 {
                beat_times.push(t);
            }
            t += beat_interval_secs;
        }

        Ok(BeatGrid {
            tempo,
            beat_times_seconds: beat_times,
        })
    }
}

impl AutoCorrelationBeatDetector {
    /// Compute spectral flux onset strength envelope
    fn compute_onset_strength_envelope(&self, samples: &[f32]) -> Vec<f32> {
        let hann: Vec<f32> = (0..self.window_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (self.window_size - 1) as f32).cos()))
            .collect();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.window_size);

        let num_frames = (samples.len() - self.window_size) / self.hop_size;
        let half_w = self.window_size / 2;
        let mut prev_magnitude = vec![0.0f32; half_w];
        let mut envelope = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * self.hop_size;
            let window = &samples[start..start + self.window_size];

            let mut fft_buf: Vec<Complex<f32>> = (0..self.window_size)
                .map(|j| Complex::new(window[j] * hann[j], 0.0))
                .collect();

            fft.process(&mut fft_buf);

            let mut flux = 0.0f32;
            for k in 0..half_w {
                let mag = (fft_buf[k].re * fft_buf[k].re + fft_buf[k].im * fft_buf[k].im).sqrt();
                let diff = mag - prev_magnitude[k];
                if diff > 0.0 {
                    flux += diff;
                }
                prev_magnitude[k] = mag;
            }

            envelope.push(flux);
        }

        envelope
    }

    /// Autocorrelate the onset strength envelope and apply tempo prior weighting
    fn estimate_bpm(&self, envelope: &[f32], env_fps: f64) -> f64 {
        if envelope.is_empty() {
            return self.tempo_prior_bpm;
        }

        let min_lag = (60.0 * env_fps / self.max_bpm).floor() as usize;
        let max_lag = (60.0 * env_fps / self.min_bpm).ceil() as usize;

        if min_lag >= max_lag || max_lag >= envelope.len() {
            return self.tempo_prior_bpm;
        }

        let mut max_score = -1.0f64;
        let mut best_lag = (min_lag + max_lag) / 2;

        let mut corr = vec![0.0f64; max_lag + 2];

        for lag in min_lag..=max_lag {
            let mut sum = 0.0f64;
            let count = envelope.len() - lag;
            for i in 0..count {
                sum += envelope[i] as f64 * envelope[i + lag] as f64;
            }
            let normalized_corr = sum / (count as f64).max(1.0);
            corr[lag] = normalized_corr;

            let bpm = 60.0 * env_fps / lag as f64;
            // Log-Gaussian prior centered around tempo_prior_bpm (120 BPM)
            let log_ratio = (bpm / self.tempo_prior_bpm).log2();
            let prior_weight = (-0.5 * (log_ratio / 0.7).powi(2)).exp();

            let score = normalized_corr * prior_weight;
            if score > max_score {
                max_score = score;
                best_lag = lag;
            }
        }

        // Parabolic peak interpolation on correlation curve
        let refined_lag = if best_lag > min_lag && best_lag < max_lag {
            let y0 = corr[best_lag - 1];
            let y1 = corr[best_lag];
            let y2 = corr[best_lag + 1];
            let denom = 2.0 * (2.0 * y1 - y0 - y2);
            if denom.abs() > 1e-9 {
                let delta = (y2 - y0) / denom;
                (best_lag as f64 + delta.clamp(-1.0, 1.0)).max(1.0)
            } else {
                best_lag as f64
            }
        } else {
            best_lag as f64
        };

        let calculated_bpm = (60.0 * env_fps / refined_lag).clamp(self.min_bpm, self.max_bpm);
        (calculated_bpm * 10.0).round() / 10.0
    }

    /// Find optimum phase offset t0 by cross-evaluating onset envelope values on grid
    fn align_beat_phase(
        &self,
        envelope: &[f32],
        env_fps: f64,
        beat_interval_secs: f64,
        duration: f64,
    ) -> f64 {
        if envelope.is_empty() || beat_interval_secs <= 0.0 {
            return 0.0;
        }

        let num_candidates = 32;
        let step = beat_interval_secs / num_candidates as f64;
        let mut best_score = -1.0f32;
        let mut best_t0 = 0.0f64;

        for step_idx in 0..num_candidates {
            let candidate_t0 = step_idx as f64 * step;
            let mut score = 0.0f32;
            let mut t = candidate_t0;

            while t < duration {
                let frame_idx = (t * env_fps).round() as usize;
                if frame_idx < envelope.len() {
                    score += envelope[frame_idx];
                }
                t += beat_interval_secs;
            }

            if score > best_score {
                best_score = score;
                best_t0 = candidate_t0;
            }
        }

        best_t0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_rhythmic_clicks(sample_rate: u32, bpm: f64, total_secs: f64) -> AudioBuffer {
        let total_samples = (sample_rate as f64 * total_secs) as usize;
        let mut samples = vec![0.0f32; total_samples];
        let interval_secs = 60.0 / bpm;

        let mut t = 0.1; // 100ms initial offset
        while t < total_secs {
            let start_idx = (t * sample_rate as f64) as usize;
            for j in 0..600 {
                if start_idx + j < total_samples {
                    let decay = (-(j as f32) / 120.0).exp();
                    // Sharp transient burst
                    samples[start_idx + j] += 0.9 * decay * ((j as f32 * 0.1).sin());
                }
            }
            t += interval_secs;
        }

        AudioBuffer::new(samples, sample_rate, 1)
    }

    #[test]
    fn test_autocorrelation_beat_detection_120bpm() {
        let sample_rate = 44100;
        let audio = generate_rhythmic_clicks(sample_rate, 120.0, 5.0);
        let detector = AutoCorrelationBeatDetector::default();
        let beat_grid = detector.detect_beat_grid(&audio).expect("beat detection failed");

        let detected_bpm = beat_grid.tempo.bpm();
        let error = (detected_bpm - 120.0).abs();
        assert!(
            error < 3.0,
            "Expected ~120.0 BPM, got {:.1} BPM (error: {:.2})",
            detected_bpm,
            error
        );
        assert!(!beat_grid.beat_times_seconds.is_empty());
    }

    #[test]
    fn test_autocorrelation_beat_detection_90bpm() {
        let sample_rate = 44100;
        let audio = generate_rhythmic_clicks(sample_rate, 90.0, 6.0);
        let detector = AutoCorrelationBeatDetector::default();
        let beat_grid = detector.detect_beat_grid(&audio).expect("beat detection failed");

        let detected_bpm = beat_grid.tempo.bpm();
        let error = (detected_bpm - 90.0).abs();
        assert!(
            error < 3.0,
            "Expected ~90.0 BPM, got {:.1} BPM (error: {:.2})",
            detected_bpm,
            error
        );
    }
}
