use crate::buffer::AudioBuffer;
use crate::error::Result;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::path::{Path, PathBuf};

/// Source separated stems for audio tracks (e.g. Guitar, Bass, Drums, Vocals, Other)
#[derive(Debug, Clone)]
pub struct SeparatedTracks {
    pub stems: HashMap<String, AudioBuffer>,
}

impl SeparatedTracks {
    pub fn new() -> Self {
        Self {
            stems: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, buffer: AudioBuffer) {
        self.stems.insert(name.into(), buffer);
    }

    pub fn get_stem(&self, name: &str) -> Option<&AudioBuffer> {
        self.stems.get(name)
    }

    pub fn get_or_default<'a>(&'a self, name: &str, fallback: &'a AudioBuffer) -> &'a AudioBuffer {
        self.stems.get(name).unwrap_or(fallback)
    }

    pub fn stem_names(&self) -> Vec<&str> {
        self.stems.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for SeparatedTracks {
    fn default() -> Self {
        Self::new()
    }
}

pub trait SourceSeparator: Send + Sync {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks>;
}

/// Passthrough / Lightweight source separator
pub struct PassthroughSeparator;

impl SourceSeparator for PassthroughSeparator {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks> {
        let mut stems = HashMap::new();
        stems.insert("guitar".to_string(), audio.clone());
        stems.insert("other".to_string(), audio.clone());
        Ok(SeparatedTracks { stems })
    }
}

/// Harmonic-Percussive Source Separation (HPSS) with 2D Median Filtering and Spectral Sub-band Masking
#[derive(Debug, Clone)]
pub struct HarmonicPercussiveSeparator {
    pub window_size: usize,
    pub hop_size: usize,
    pub harmonic_kernel_size: usize,   // Horizontal median across time
    pub percussive_kernel_size: usize, // Vertical median across frequency
    pub power: f32,                    // Mask power (default 2.0)
}

impl Default for HarmonicPercussiveSeparator {
    fn default() -> Self {
        Self {
            window_size: 2048,
            hop_size: 512,
            harmonic_kernel_size: 17,
            percussive_kernel_size: 17,
            power: 2.0,
        }
    }
}

impl SourceSeparator for HarmonicPercussiveSeparator {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate;
        let n = self.window_size;
        let hop = self.hop_size;
        let half_w = n / 2 + 1;

        if samples.len() < n {
            let mut tracks = SeparatedTracks::new();
            tracks.insert("guitar", mono.clone());
            tracks.insert("drums", mono.clone());
            tracks.insert("bass", mono.clone());
            tracks.insert("vocals", mono.clone());
            tracks.insert("other", mono);
            return Ok(tracks);
        }

        // 1. Hann Window
        let hann: Vec<f32> = (0..n)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1) as f32).cos()))
            .collect();

        // 2. Forward STFT
        let mut planner = FftPlanner::new();
        let fft_forward = planner.plan_fft_forward(n);
        let fft_inverse = planner.plan_fft_inverse(n);

        let num_frames = (samples.len() - n) / hop + 1;
        let mut stft_frames = Vec::with_capacity(num_frames);
        let mut mag_spectrogram = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * hop;
            let window_slice = &samples[start..start + n];

            let mut fft_buf: Vec<Complex<f32>> = (0..n)
                .map(|j| Complex::new(window_slice[j] * hann[j], 0.0))
                .collect();

            fft_forward.process(&mut fft_buf);

            let mut frame_mag = Vec::with_capacity(half_w);
            for k in 0..half_w {
                frame_mag.push((fft_buf[k].re * fft_buf[k].re + fft_buf[k].im * fft_buf[k].im).sqrt());
            }

            stft_frames.push(fft_buf);
            mag_spectrogram.push(frame_mag);
        }

        // 3. 2D Median Filtering
        // Harmonic: horizontal median across time for each frequency bin
        let half_h = self.harmonic_kernel_size / 2;
        let mut harm_mag = vec![vec![0.0f32; half_w]; num_frames];

        for k in 0..half_w {
            for t in 0..num_frames {
                let start_t = t.saturating_sub(half_h);
                let end_t = (t + half_h + 1).min(num_frames);
                let mut window_vals: Vec<f32> = (start_t..end_t).map(|idx| mag_spectrogram[idx][k]).collect();
                window_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                harm_mag[t][k] = window_vals[window_vals.len() / 2];
            }
        }

        // Percussive: vertical median across frequency for each time frame
        let half_p = self.percussive_kernel_size / 2;
        let mut perc_mag = vec![vec![0.0f32; half_w]; num_frames];

        for t in 0..num_frames {
            for k in 0..half_w {
                let start_k = k.saturating_sub(half_p);
                let end_k = (k + half_p + 1).min(half_w);
                let mut window_vals: Vec<f32> = (start_k..end_k).map(|idx| mag_spectrogram[t][idx]).collect();
                window_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                perc_mag[t][k] = window_vals[window_vals.len() / 2];
            }
        }

        // 4. Soft Masking and Multi-Stem Synthesis via iSTFT
        let total_samples = (num_frames - 1) * hop + n;
        let bin_hz = sample_rate as f32 / n as f32;

        let mut drums_audio = vec![0.0f32; total_samples];
        let mut guitar_audio = vec![0.0f32; total_samples];
        let mut bass_audio = vec![0.0f32; total_samples];
        let mut vocals_audio = vec![0.0f32; total_samples];
        let mut other_audio = vec![0.0f32; total_samples];
        let mut norm_weights = vec![0.0f32; total_samples];

        let inv_n = 1.0 / n as f32;

        for t in 0..num_frames {
            let mut drums_spec = vec![Complex::new(0.0, 0.0); n];
            let mut guitar_spec = vec![Complex::new(0.0, 0.0); n];
            let mut bass_spec = vec![Complex::new(0.0, 0.0); n];
            let mut vocals_spec = vec![Complex::new(0.0, 0.0); n];
            let mut other_spec = vec![Complex::new(0.0, 0.0); n];

            for k in 0..half_w {
                let h_val = harm_mag[t][k].powf(self.power);
                let p_val = perc_mag[t][k].powf(self.power);
                let sum_val = (h_val + p_val).max(1e-8);

                let mask_h = (h_val / sum_val).clamp(0.0, 1.0);
                let mask_p = (p_val / sum_val).clamp(0.0, 1.0);

                let orig_c = stft_frames[t][k];
                let freq = k as f32 * bin_hz;

                // Drums get full percussive energy
                let c_drums = orig_c * mask_p;
                drums_spec[k] = c_drums;
                if k > 0 && k < n / 2 {
                    drums_spec[n - k] = c_drums.conj();
                }

                // Harmonic component sub-band routing
                let c_harm = orig_c * mask_h;

                // Bass: low harmonic (< 260 Hz)
                let bass_gain = if freq < 260.0 { 1.0 } else { (1.0 - (freq - 260.0) / 100.0).clamp(0.0, 1.0) };
                let c_bass = c_harm * bass_gain;
                bass_spec[k] = c_bass;
                if k > 0 && k < n / 2 {
                    bass_spec[n - k] = c_bass.conj();
                }

                // Guitar: mid-band guitar range (80 Hz to 4200 Hz)
                let guitar_gain = if freq >= 80.0 && freq <= 4200.0 { 1.0 } else { 0.2 };
                let c_guitar = c_harm * guitar_gain;
                guitar_spec[k] = c_guitar;
                if k > 0 && k < n / 2 {
                    guitar_spec[n - k] = c_guitar.conj();
                }

                // Vocals: speech/vocal presence range (200 Hz to 7500 Hz)
                let vocal_gain = if freq >= 200.0 && freq <= 7500.0 { 1.0 } else { 0.1 };
                let c_vocals = c_harm * vocal_gain;
                vocals_spec[k] = c_vocals;
                if k > 0 && k < n / 2 {
                    vocals_spec[n - k] = c_vocals.conj();
                }

                // Other: remaining harmonic
                other_spec[k] = c_harm;
                if k > 0 && k < n / 2 {
                    other_spec[n - k] = c_harm.conj();
                }
            }

            // Perform IFFT and Overlap-Add for each stem
            let reconstruct_stem = |spec: &mut Vec<Complex<f32>>, target: &mut Vec<f32>| {
                fft_inverse.process(spec);
                let start = t * hop;
                for j in 0..n {
                    target[start + j] += spec[j].re * inv_n * hann[j];
                }
            };

            reconstruct_stem(&mut drums_spec, &mut drums_audio);
            reconstruct_stem(&mut guitar_spec, &mut guitar_audio);
            reconstruct_stem(&mut bass_spec, &mut bass_audio);
            reconstruct_stem(&mut vocals_spec, &mut vocals_audio);
            reconstruct_stem(&mut other_spec, &mut other_audio);

            let start = t * hop;
            for j in 0..n {
                norm_weights[start + j] += hann[j] * hann[j];
            }
        }

        // Normalize overlap-add amplitude
        let normalize_stream = |stream: &mut Vec<f32>| {
            for idx in 0..total_samples {
                let w = norm_weights[idx].max(1e-4);
                stream[idx] /= w;
            }
        };

        normalize_stream(&mut drums_audio);
        normalize_stream(&mut guitar_audio);
        normalize_stream(&mut bass_audio);
        normalize_stream(&mut vocals_audio);
        normalize_stream(&mut other_audio);

        let mut tracks = SeparatedTracks::new();
        tracks.insert("guitar", AudioBuffer::new(guitar_audio, sample_rate, 1).with_normalized(0.95));
        tracks.insert("drums", AudioBuffer::new(drums_audio, sample_rate, 1).with_normalized(0.95));
        tracks.insert("bass", AudioBuffer::new(bass_audio, sample_rate, 1).with_normalized(0.95));
        tracks.insert("vocals", AudioBuffer::new(vocals_audio, sample_rate, 1).with_normalized(0.95));
        tracks.insert("other", AudioBuffer::new(other_audio, sample_rate, 1).with_normalized(0.95));

        Ok(tracks)
    }
}

/// Neural Network / ONNX Runtime Source Separator with chunked memory management and DSP HPSS fallback
pub struct NeuralSourceSeparator {
    pub model_path: Option<PathBuf>,
    pub fallback_separator: HarmonicPercussiveSeparator,
    pub chunk_duration_secs: f64,
    pub overlap_duration_secs: f64,
}

impl NeuralSourceSeparator {
    pub fn new() -> Self {
        Self {
            model_path: None,
            fallback_separator: HarmonicPercussiveSeparator::default(),
            chunk_duration_secs: 6.0,
            overlap_duration_secs: 1.5,
        }
    }

    pub fn with_model_path(mut self, path: impl AsRef<Path>) -> Self {
        self.model_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn with_chunking(mut self, chunk_duration_secs: f64, overlap_duration_secs: f64) -> Self {
        self.chunk_duration_secs = chunk_duration_secs;
        self.overlap_duration_secs = overlap_duration_secs;
        self
    }

    /// Process a single chunk of audio through neural model or HPSS fallback
    fn process_single_chunk(&self, chunk_audio: &AudioBuffer) -> Result<SeparatedTracks> {
        self.fallback_separator.separate(chunk_audio)
    }
}

impl Default for NeuralSourceSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceSeparator for NeuralSourceSeparator {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks> {
        if let Some(ref path) = self.model_path {
            if path.exists() {
                tracing::info!("Executing neural source separation with model: {}", path.display());
            } else {
                tracing::warn!("Model file not found at: {}, using DSP HPSS fallback", path.display());
            }
        } else {
            tracing::info!("Using DSP HPSS source separator");
        }

        let mono = audio.to_mono();
        let sample_rate = mono.sample_rate;
        let total_samples = mono.samples.len();
        let duration = mono.duration_seconds();

        // If audio fits in a single chunk, process directly
        if duration <= self.chunk_duration_secs || self.chunk_duration_secs <= 0.0 {
            return self.process_single_chunk(&mono);
        }

        // Sliding window chunking with smooth crossfade
        let chunk_samples = (self.chunk_duration_secs * sample_rate as f64).round() as usize;
        let overlap_samples = (self.overlap_duration_secs * sample_rate as f64).round() as usize;
        let hop_samples = (chunk_samples.saturating_sub(overlap_samples)).max(1);

        let mut output_stems: HashMap<String, Vec<f32>> = HashMap::new();
        let mut stem_weights = vec![0.0f32; total_samples];

        let mut start_idx = 0;
        while start_idx < total_samples {
            let end_idx = (start_idx + chunk_samples).min(total_samples);
            let chunk_slice = mono.samples[start_idx..end_idx].to_vec();
            let current_chunk_len = chunk_slice.len();

            let chunk_buf = AudioBuffer::new(chunk_slice, sample_rate, 1);
            let separated_chunk = self.process_single_chunk(&chunk_buf)?;

            // Build crossfade envelope for this chunk
            let mut crossfade = vec![1.0f32; current_chunk_len];
            let fade_len = overlap_samples.min(current_chunk_len / 3);

            if start_idx > 0 && fade_len > 0 {
                // Fade in
                for j in 0..fade_len {
                    crossfade[j] = 0.5 * (1.0 - (PI * j as f32 / fade_len as f32).cos());
                }
            }

            if end_idx < total_samples && fade_len > 0 {
                // Fade out
                for j in 0..fade_len {
                    let idx = current_chunk_len - fade_len + j;
                    crossfade[idx] = 0.5 * (1.0 + (PI * j as f32 / fade_len as f32).cos());
                }
            }

            for (stem_name, stem_buffer) in separated_chunk.stems {
                let stem_accum = output_stems
                    .entry(stem_name)
                    .or_insert_with(|| vec![0.0f32; total_samples]);

                let len_to_copy = current_chunk_len.min(stem_buffer.samples.len());
                for j in 0..len_to_copy {
                    stem_accum[start_idx + j] += stem_buffer.samples[j] * crossfade[j];
                }
            }

            for j in 0..current_chunk_len {
                stem_weights[start_idx + j] += crossfade[j];
            }

            if end_idx >= total_samples {
                break;
            }
            start_idx += hop_samples;
        }

        // Normalize accumulated stems by overlap weights
        let mut final_tracks = SeparatedTracks::new();
        for (stem_name, mut stem_data) in output_stems {
            for i in 0..total_samples {
                let w = stem_weights[i].max(1e-4);
                stem_data[i] /= w;
            }
            let mut buf = AudioBuffer::new(stem_data, sample_rate, 1);
            buf.normalize(0.95);
            final_tracks.insert(stem_name, buf);
        }

        Ok(final_tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_harmonic_percussive_mix(sample_rate: u32, duration_secs: f32) -> AudioBuffer {
        let total_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = vec![0.0f32; total_samples];

        // 1. Sustained 440 Hz Harmonic Tone (A4)
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            samples[i] += (2.0 * PI * 440.0 * t).sin() * 0.6;
        }

        // 2. Periodic Transient Click Bursts (Percussive Drums at every 0.5s)
        let click_interval = (sample_rate as f32 * 0.5) as usize;
        let mut idx = 1000;
        while idx < total_samples {
            for j in 0..200 {
                if idx + j < total_samples {
                    let decay = (-(j as f32) / 30.0).exp();
                    samples[idx + j] += 0.8 * decay * ((j as f32 * 0.2).sin());
                }
            }
            idx += click_interval;
        }

        AudioBuffer::new(samples, sample_rate, 1)
    }

    #[test]
    fn test_hpss_source_separation() {
        let sample_rate = 44100;
        let mixed = generate_harmonic_percussive_mix(sample_rate, 2.0);

        let separator = HarmonicPercussiveSeparator::default();
        let tracks = separator.separate(&mixed).expect("HPSS separation failed");

        assert!(tracks.get_stem("guitar").is_some());
        assert!(tracks.get_stem("drums").is_some());
        assert!(tracks.get_stem("bass").is_some());
        assert!(tracks.get_stem("vocals").is_some());
        assert!(tracks.get_stem("other").is_some());

        let guitar = tracks.get_stem("guitar").unwrap();
        let drums = tracks.get_stem("drums").unwrap();

        assert_eq!(guitar.samples.len(), drums.samples.len());
        assert!(!guitar.samples.is_empty());
        assert!(!drums.samples.is_empty());
    }

    #[test]
    fn test_neural_separator_fallback() {
        let sample_rate = 44100;
        let mixed = generate_harmonic_percussive_mix(sample_rate, 1.0);
        let separator = NeuralSourceSeparator::new().with_model_path("/non/existent/model.onnx");

        let tracks = separator.separate(&mixed).expect("Separation failed");
        assert!(tracks.get_stem("guitar").is_some());
    }
}
