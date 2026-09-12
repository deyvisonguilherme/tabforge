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

    /// Convert multi-channel buffer to mono using standard ITU-R downmixing
    pub fn to_mono(&self) -> Self {
        self.to_mono_weighted()
    }

    /// Convert multi-channel buffer to a single mono channel using ITU-R BS.775 weighting
    pub fn to_mono_weighted(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }

        let num_frames = self.samples.len() / self.channels as usize;
        let mut mono_samples = Vec::with_capacity(num_frames);
        let ch = self.channels as usize;

        for frame in 0..num_frames {
            let offset = frame * ch;
            let sample = match self.channels {
                2 => {
                    // Standard Stereo: (L + R) / 2
                    (self.samples[offset] + self.samples[offset + 1]) * 0.5
                }
                4 => {
                    // Quadraphonic: (L + R + 0.707 Ls + 0.707 Rs) / 3.414
                    (self.samples[offset]
                        + self.samples[offset + 1]
                        + 0.707 * self.samples[offset + 2]
                        + 0.707 * self.samples[offset + 3])
                        / 3.414
                }
                6 => {
                    // 5.1 Surround (L, R, C, LFE, Ls, Rs): exclude LFE sub to avoid LF pitch rumble
                    let l = self.samples[offset];
                    let r = self.samples[offset + 1];
                    let c = self.samples[offset + 2];
                    let ls = self.samples[offset + 4];
                    let rs = self.samples[offset + 5];
                    (l + r + 0.707 * c + 0.707 * ls + 0.707 * rs) / 4.121
                }
                8 => {
                    // 7.1 Surround (L, R, C, LFE, Ls, Rs, Rls, Rrs)
                    let l = self.samples[offset];
                    let r = self.samples[offset + 1];
                    let c = self.samples[offset + 2];
                    let ls = self.samples[offset + 4];
                    let rs = self.samples[offset + 5];
                    let rls = self.samples[offset + 6];
                    let rrs = self.samples[offset + 7];
                    (l + r + 0.707 * c + 0.5 * ls + 0.5 * rs + 0.5 * rls + 0.5 * rrs) / 4.707
                }
                _ => {
                    // Unweighted arithmetic average for arbitrary channel counts
                    let mut sum = 0.0;
                    for c_idx in 0..ch {
                        sum += self.samples[offset + c_idx];
                    }
                    sum / ch as f32
                }
            };

            mono_samples.push(sample);
        }

        Self {
            samples: mono_samples,
            sample_rate: self.sample_rate,
            channels: 1,
        }
    }

    /// Resample audio to target sample rate using Catmull-Rom cubic interpolation
    pub fn resample(&self, target_sample_rate: u32) -> Self {
        if self.sample_rate == target_sample_rate || self.sample_rate == 0 || target_sample_rate == 0 || self.samples.is_empty() {
            return self.clone();
        }

        let ch = self.channels as usize;
        let num_src_frames = self.samples.len() / ch;
        let ratio = self.sample_rate as f64 / target_sample_rate as f64;
        let num_target_frames = ((num_src_frames as f64) / ratio).round() as usize;

        let mut resampled_samples = Vec::with_capacity(num_target_frames * ch);

        for target_frame in 0..num_target_frames {
            let src_pos = target_frame as f64 * ratio;
            let k = src_pos.floor() as usize;
            let alpha = (src_pos - k as f64) as f32;

            for c in 0..ch {
                let get_sample = |idx: isize| -> f32 {
                    let clamped = idx.clamp(0, num_src_frames as isize - 1) as usize;
                    self.samples[clamped * ch + c]
                };

                let y0 = get_sample(k as isize - 1);
                let y1 = get_sample(k as isize);
                let y2 = get_sample(k as isize + 1);
                let y3 = get_sample(k as isize + 2);

                // Catmull-Rom cubic polynomial interpolation
                let c0 = y1;
                let c1 = 0.5 * (y2 - y0);
                let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                let c3 = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;

                let val = ((c3 * alpha + c2) * alpha + c1) * alpha + c0;
                resampled_samples.push(val);
            }
        }

        Self {
            samples: resampled_samples,
            sample_rate: target_sample_rate,
            channels: self.channels,
        }
    }

    /// Remove DC offset and normalize peak amplitude to target peak level (e.g. 0.95 / -0.1 dBFS)
    pub fn normalize(&mut self, peak_target: f32) -> &mut Self {
        if self.samples.is_empty() {
            return self;
        }

        // 1. Remove DC Offset
        let sum: f64 = self.samples.iter().map(|&s| s as f64).sum();
        let dc_offset = (sum / self.samples.len() as f64) as f32;
        for s in &mut self.samples {
            *s -= dc_offset;
        }

        // 2. Find Peak Amplitude
        let mut max_abs = 0.0f32;
        for &s in &self.samples {
            let abs_val = s.abs();
            if abs_val > max_abs {
                max_abs = abs_val;
            }
        }

        // 3. Scale to Peak Target
        if max_abs > 1e-6 {
            let scale = peak_target / max_abs;
            for s in &mut self.samples {
                *s *= scale;
            }
        }

        self
    }

    pub fn with_normalized(mut self, peak_target: f32) -> Self {
        self.normalize(peak_target);
        self
    }

    /// Save buffer as 16-bit PCM WAV file
    pub fn save_wav(&self, path: &std::path::Path) -> crate::error::Result<()> {
        crate::wav_writer::write_wav(self, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_audio_buffer_duration_and_mono_downmix() {
        let mut stereo_samples = Vec::new();
        for _ in 0..44100 {
            stereo_samples.push(0.5);
            stereo_samples.push(-0.5);
        }
        let buffer = AudioBuffer::new(stereo_samples, 44100, 2);
        assert!((buffer.duration_seconds() - 1.0).abs() < 1e-5);

        let mono = buffer.to_mono_weighted();
        assert_eq!(mono.channels, 1);
        assert_eq!(mono.samples.len(), 44100);
        assert!((mono.samples[0] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_5_1_surround_downmix() {
        // 5.1 frame: L=1.0, R=1.0, C=1.0, LFE=99.0 (should be excluded), Ls=0.0, Rs=0.0
        let frame_5_1 = vec![1.0, 1.0, 1.0, 99.0, 0.0, 0.0];
        let buffer = AudioBuffer::new(frame_5_1, 48000, 6);
        let mono = buffer.to_mono_weighted();
        assert_eq!(mono.channels, 1);
        assert_eq!(mono.samples.len(), 1);
        // LFE should not explode the output
        assert!(mono.samples[0] < 1.0);
    }

    #[test]
    fn test_cubic_resampling_sine_wave() {
        let src_rate = 48000;
        let target_rate = 44100;
        let freq = 440.0;
        let duration = 0.2;

        let num_src = (src_rate as f32 * duration) as usize;
        let mut src_samples = Vec::with_capacity(num_src);
        for i in 0..num_src {
            let t = i as f32 / src_rate as f32;
            src_samples.push((2.0 * PI * freq * t).sin());
        }

        let buffer = AudioBuffer::new(src_samples, src_rate, 1);
        let resampled = buffer.resample(target_rate);

        assert_eq!(resampled.sample_rate, target_rate);
        let expected_samples = (target_rate as f32 * duration).round() as usize;
        assert!((resampled.samples.len() as i32 - expected_samples as i32).abs() <= 2);

        // Check waveform accuracy at midpoint
        let mid_idx = resampled.samples.len() / 2;
        let t_mid = mid_idx as f32 / target_rate as f32;
        let expected_val = (2.0 * PI * freq * t_mid).sin();
        assert!((resampled.samples[mid_idx] - expected_val).abs() < 0.05);
    }

    #[test]
    fn test_dc_offset_removal_and_normalization() {
        let samples = vec![1.2, 1.4, 1.6, 1.8]; // DC offset = 1.5, zero-mean = [-0.3, -0.1, 0.1, 0.3]
        let mut buffer = AudioBuffer::new(samples, 44100, 1);
        buffer.normalize(0.95);

        let mean: f32 = buffer.samples.iter().sum::<f32>() / buffer.samples.len() as f32;
        assert!(mean.abs() < 1e-6, "DC offset was not removed: mean = {}", mean);

        let peak: f32 = buffer.samples.iter().map(|&s| s.abs()).fold(0.0, f32::max);
        assert!((peak - 0.95).abs() < 1e-5, "Peak was not normalized to 0.95, got {}", peak);
    }
}
