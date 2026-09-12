use crate::buffer::AudioBuffer;
use crate::error::Result;
use crate::traits::{PitchDetector, PitchEvent};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::sync::Arc;
use tabforge_core::Pitch;

/// FFT-accelerated YIN Monophonic Pitch Detector with Sub-sample Parabolic Interpolation
pub struct YinPitchDetector {
    pub window_size: usize,
    pub hop_size: usize,
    pub threshold: f32,
    pub min_frequency: f64,
    pub max_frequency: f64,
    pub silence_threshold: f32,
    pub median_filter_size: usize,
}

impl Default for YinPitchDetector {
    fn default() -> Self {
        Self {
            window_size: 2048,
            hop_size: 512,
            threshold: 0.15,
            min_frequency: 60.0,   // ~B1 / guitar low drop tuning
            max_frequency: 1400.0, // High fret guitar harmonics / E6
            silence_threshold: 0.005,
            median_filter_size: 5,
        }
    }
}

impl PitchDetector for YinPitchDetector {
    fn detect_pitch(&self, audio: &AudioBuffer) -> Result<Vec<PitchEvent>> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;
        let mut raw_events: Vec<Option<PitchEvent>> = Vec::new();

        if samples.len() < self.window_size {
            return Ok(Vec::new());
        }

        let mut planner = FftPlanner::new();
        let fft_forward = planner.plan_fft_forward(self.window_size);
        let fft_inverse = planner.plan_fft_inverse(self.window_size);

        let num_frames = (samples.len() - self.window_size) / self.hop_size;
        for i in 0..num_frames {
            let start = i * self.hop_size;
            let window = &samples[start..start + self.window_size];
            let time = start as f64 / sample_rate;

            // Compute RMS to filter silence/noise
            let rms: f32 = (window.iter().map(|&x| x * x).sum::<f32>() / window.len() as f32).sqrt();
            if rms < self.silence_threshold {
                raw_events.push(None);
                continue;
            }

            // Estimate pitch with FFT-accelerated YIN
            if let Some((freq, confidence)) = self.yin_pitch_estimate(
                window,
                sample_rate,
                &fft_forward,
                &fft_inverse,
            ) {
                if freq >= self.min_frequency && freq <= self.max_frequency {
                    if let Some(pitch) = Pitch::from_frequency(freq) {
                        raw_events.push(Some(PitchEvent {
                            time_seconds: time,
                            pitch,
                            confidence,
                            frequency: freq,
                        }));
                        continue;
                    }
                }
            }

            raw_events.push(None);
        }

        // Apply temporal median filter to stabilize pitch tracking and suppress spurious jumps
        let smoothed_events = self.apply_median_filter(&raw_events);

        Ok(smoothed_events)
    }
}

impl YinPitchDetector {
    /// Compute fundamental frequency and confidence using FFT-accelerated YIN
    fn yin_pitch_estimate(
        &self,
        window: &[f32],
        sample_rate: f64,
        fft_forward: &Arc<dyn rustfft::Fft<f32>>,
        fft_inverse: &Arc<dyn rustfft::Fft<f32>>,
    ) -> Option<(f64, f32)> {
        let n = self.window_size;
        let half_w = n / 2;

        // 1. Prepare buffers for FFT cross-correlation
        // x1: window[0..half_w] zero-padded to N
        let mut buf_x1: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); n];
        for j in 0..half_w {
            buf_x1[j] = Complex::new(window[j], 0.0);
        }

        // x2: window[0..n]
        let mut buf_x2: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); n];
        for j in 0..n {
            buf_x2[j] = Complex::new(window[j], 0.0);
        }

        // 2. Perform FFT on both
        fft_forward.process(&mut buf_x1);
        fft_forward.process(&mut buf_x2);

        // 3. Spectral cross multiplication: X1* * X2
        let mut buf_r: Vec<Complex<f32>> = Vec::with_capacity(n);
        for j in 0..n {
            buf_r.push(buf_x1[j].conj() * buf_x2[j]);
        }

        // 4. IFFT to obtain cross-correlation
        fft_inverse.process(&mut buf_r);

        let inv_n = 1.0 / (n as f32);

        // 5. Compute Energy terms
        let mut e0 = 0.0f32;
        for j in 0..half_w {
            e0 += window[j] * window[j];
        }

        let mut e_tau = e0;
        let mut d = vec![0.0f32; half_w];

        for tau in 0..half_w {
            if tau > 0 {
                let leaving = window[tau - 1];
                let entering = window[tau - 1 + half_w];
                e_tau = (e_tau - leaving * leaving + entering * entering).max(0.0);
            }
            let r_tau = buf_r[tau].re * inv_n;
            let diff = e0 + e_tau - 2.0 * r_tau;
            d[tau] = diff.max(0.0);
        }

        // 6. Cumulative mean normalized difference function (CMNDF)
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

        // 7. Search for first local minimum below threshold
        let min_tau = ((sample_rate / self.max_frequency) as usize).max(2);
        let max_tau = ((sample_rate / self.min_frequency) as usize).min(half_w - 2);

        if min_tau >= max_tau {
            return None;
        }

        let mut best_tau = None;

        for tau in min_tau..max_tau {
            if cmndf[tau] < self.threshold {
                let mut current = tau;
                while current + 1 < max_tau && cmndf[current + 1] < cmndf[current] {
                    current += 1;
                }
                best_tau = Some(current);
                break;
            }
        }

        // Fallback: Global minimum if confidence is reasonably strong
        if best_tau.is_none() {
            let mut min_val = f32::MAX;
            let mut min_idx = 0;
            for tau in min_tau..max_tau {
                if cmndf[tau] < min_val {
                    min_val = cmndf[tau];
                    min_idx = tau;
                }
            }
            if min_val < self.threshold * 1.5 && min_idx > 0 {
                best_tau = Some(min_idx);
            }
        }

        let tau = best_tau?;
        let confidence = (1.0 - cmndf[tau]).clamp(0.0, 1.0);

        // 8. Sub-sample Parabolic Interpolation for exact frequency
        let refined_tau = if tau > 0 && tau + 1 < half_w {
            let s0 = cmndf[tau - 1];
            let s1 = cmndf[tau];
            let s2 = cmndf[tau + 1];
            let denom = 2.0 * (2.0 * s1 - s0 - s2);
            if denom.abs() > 1e-6 {
                let delta = (s2 - s0) / denom;
                (tau as f64 + delta.clamp(-1.0, 1.0) as f64).max(1.0)
            } else {
                tau as f64
            }
        } else {
            tau as f64
        };

        let freq = sample_rate / refined_tau;
        Some((freq, confidence))
    }

    /// Apply moving median filter over pitch frequency sequence
    fn apply_median_filter(&self, raw: &[Option<PitchEvent>]) -> Vec<PitchEvent> {
        let mut result = Vec::new();
        let half = self.median_filter_size / 2;

        for i in 0..raw.len() {
            if let Some(ref current) = raw[i] {
                let start = i.saturating_sub(half);
                let end = (i + half + 1).min(raw.len());

                let mut window_freqs: Vec<f64> = raw[start..end]
                    .iter()
                    .filter_map(|e| e.as_ref().map(|p| p.frequency))
                    .collect();

                if window_freqs.is_empty() {
                    continue;
                }

                window_freqs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let median_freq = window_freqs[window_freqs.len() / 2];

                // If current frequency is within acceptable deviation of median, keep it (or refine pitch)
                let freq = if (current.frequency / median_freq - 1.0).abs() < 0.1 {
                    current.frequency
                } else {
                    median_freq
                };

                if let Some(pitch) = Pitch::from_frequency(freq) {
                    result.push(PitchEvent {
                        time_seconds: current.time_seconds,
                        pitch,
                        confidence: current.confidence,
                        frequency: freq,
                    });
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine_buffer(freq_hz: f32, sample_rate: u32, duration_secs: f32) -> AudioBuffer {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((2.0 * PI * freq_hz * t).sin() * 0.8);
        }
        AudioBuffer::new(samples, sample_rate, 1)
    }

    #[test]
    fn test_yin_pitch_detection_a4_440hz() {
        let sample_rate = 44100;
        let audio = generate_sine_buffer(440.0, sample_rate, 0.5);
        let detector = YinPitchDetector::default();
        let events = detector.detect_pitch(&audio).expect("pitch detection failed");

        assert!(!events.is_empty(), "Expected pitch events for 440Hz tone");
        for event in &events {
            let error = (event.frequency - 440.0).abs();
            assert!(
                error < 2.0,
                "Expected ~440.0 Hz, got {:.2} Hz (error: {:.2})",
                event.frequency,
                error
            );
            assert_eq!(event.pitch, Pitch::A4);
            assert!(event.confidence > 0.85);
        }
    }

    #[test]
    fn test_yin_pitch_detection_guitar_e2_82hz() {
        let sample_rate = 44100;
        let audio = generate_sine_buffer(82.41, sample_rate, 0.6);
        let detector = YinPitchDetector::default();
        let events = detector.detect_pitch(&audio).expect("pitch detection failed");

        assert!(!events.is_empty(), "Expected pitch events for E2 tone");
        let avg_freq: f64 = events.iter().map(|e| e.frequency).sum::<f64>() / events.len() as f64;
        let error = (avg_freq - 82.41).abs();
        assert!(
            error < 1.5,
            "Expected ~82.41 Hz, got average {:.2} Hz",
            avg_freq
        );
        assert_eq!(events[0].pitch, Pitch::E2);
    }

    #[test]
    fn test_yin_silence_rejection() {
        let sample_rate = 44100;
        let audio = AudioBuffer::new(vec![0.0; sample_rate as usize], sample_rate, 1);
        let detector = YinPitchDetector::default();
        let events = detector.detect_pitch(&audio).unwrap();
        assert!(events.is_empty(), "Silence should yield no pitch events");
    }
}
