use crate::buffer::AudioBuffer;
use crate::error::Result;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::path::{Path, PathBuf};

/// Source separated stems for audio tracks (e.g. Guitar, Bass, Drums, Vocals, Piano, Other)
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

/// Enhanced Harmonic-Percussive Source Separation (HPSS) with 2D Median Filtering,
/// Spectral Gating, and Noise Floor Attenuation
#[derive(Debug, Clone)]
pub struct HarmonicPercussiveSeparator {
    pub window_size: usize,
    pub hop_size: usize,
    pub harmonic_kernel_size: usize,   // Horizontal median across time (default 31)
    pub percussive_kernel_size: usize, // Vertical median across frequency (default 31)
    pub power: f32,                    // Mask exponent (default 2.5 for sharper separation)
    pub noise_floor_ratio: f32,        // Spectral noise gate threshold (default 0.015)
}

impl Default for HarmonicPercussiveSeparator {
    fn default() -> Self {
        Self {
            window_size: 2048,
            hop_size: 512,
            harmonic_kernel_size: 31,
            percussive_kernel_size: 31,
            power: 2.5,
            noise_floor_ratio: 0.015,
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
        let mut max_global_mag = 0.0f32;

        for i in 0..num_frames {
            let start = i * hop;
            let window_slice = &samples[start..start + n];

            let mut fft_buf: Vec<Complex<f32>> = (0..n)
                .map(|j| Complex::new(window_slice[j] * hann[j], 0.0))
                .collect();

            fft_forward.process(&mut fft_buf);

            let mut frame_mag = Vec::with_capacity(half_w);
            for k in 0..half_w {
                let mag = (fft_buf[k].re * fft_buf[k].re + fft_buf[k].im * fft_buf[k].im).sqrt();
                if mag > max_global_mag {
                    max_global_mag = mag;
                }
                frame_mag.push(mag);
            }

            stft_frames.push(fft_buf);
            mag_spectrogram.push(frame_mag);
        }

        let noise_gate = max_global_mag * self.noise_floor_ratio;

        // 3. Fast 2D Median Filtering (Zero Heap Allocation)
        let half_h = (self.harmonic_kernel_size / 2).min(31);
        let mut harm_mag = vec![vec![0.0f32; half_w]; num_frames];

        for k in 0..half_w {
            for t in 0..num_frames {
                let start_t = t.saturating_sub(half_h);
                let end_t = (t + half_h + 1).min(num_frames);
                let count = end_t - start_t;
                let mut buf = [0.0f32; 64];
                for (idx, slot) in (start_t..end_t).zip(buf.iter_mut()) {
                    *slot = mag_spectrogram[idx][k];
                }
                let slice = &mut buf[..count];
                let mid = count / 2;
                slice.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                harm_mag[t][k] = slice[mid];
            }
        }

        let half_p = (self.percussive_kernel_size / 2).min(31);
        let mut perc_mag = vec![vec![0.0f32; half_w]; num_frames];

        for t in 0..num_frames {
            for k in 0..half_w {
                let start_k = k.saturating_sub(half_p);
                let end_k = (k + half_p + 1).min(half_w);
                let count = end_k - start_k;
                let mut buf = [0.0f32; 64];
                for (idx, slot) in (start_k..end_k).zip(buf.iter_mut()) {
                    *slot = mag_spectrogram[t][idx];
                }
                let slice = &mut buf[..count];
                let mid = count / 2;
                slice.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                perc_mag[t][k] = slice[mid];
            }
        }

        // 4. Soft Masking with Wiener-style Power Exponent and Sub-Band Spectral Filtering
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
                let orig_mag = mag_spectrogram[t][k];
                if orig_mag < noise_gate {
                    // Suppress noise floor
                    continue;
                }

                let h_val = harm_mag[t][k].powf(self.power);
                let p_val = perc_mag[t][k].powf(self.power);
                let sum_val = (h_val + p_val).max(1e-8);

                let mask_h = (h_val / sum_val).clamp(0.0, 1.0);
                let mask_p = (p_val / sum_val).clamp(0.0, 1.0);

                let orig_c = stft_frames[t][k];
                let freq = k as f32 * bin_hz;

                // Suppress vocal formants/consonants bleeding into drums:
                // When sustained harmonic energy exists in the vocal formant band (200 - 4500 Hz),
                // transient energy belongs to singing articulation rather than drum hits.
                let vocal_harmonic_ratio = harm_mag[t][k] / (perc_mag[t][k] + 1e-6);
                let vocal_bleed_suppression = if freq >= 180.0 && freq <= 4500.0 && vocal_harmonic_ratio > 0.30 {
                    (0.30 / vocal_harmonic_ratio).min(1.0).powi(2)
                } else {
                    1.0
                };

                // 1. Drums stem: Percussive mask + frequency band shaping + vocal bleed reduction
                let drum_band_gain = if freq < 120.0 {
                    1.0 // Kick drum
                } else if freq <= 4500.0 {
                    0.85 * vocal_bleed_suppression // Snare, Toms with vocal sibilance suppression
                } else if freq <= 9000.0 {
                    0.70 // Hi-hats, Cymbals
                } else {
                    0.30 // Cut ultra-high vocal sibilance bleed
                };
                let c_drums = orig_c * (mask_p * drum_band_gain);
                drums_spec[k] = c_drums;
                if k > 0 && k < n / 2 {
                    drums_spec[n - k] = c_drums.conj();
                }

                // 2. Harmonic components routing
                let c_harm = orig_c * mask_h;

                // Bass stem: Low harmonic (< 260 Hz) with steep roll-off
                let bass_gain = if freq < 240.0 {
                    1.0
                } else if freq < 360.0 {
                    (1.0 - (freq - 240.0) / 120.0).max(0.0)
                } else {
                    0.0
                };
                let c_bass = c_harm * bass_gain;
                bass_spec[k] = c_bass;
                if k > 0 && k < n / 2 {
                    bass_spec[n - k] = c_bass.conj();
                }

                // Guitar stem: 80 Hz to 4500 Hz (rejects sub-bass rumble and high air hiss)
                let guitar_gain = if freq >= 80.0 && freq <= 4200.0 {
                    if freq < 180.0 {
                        0.7 // Avoid bass overlap
                    } else {
                        1.0
                    }
                } else if freq < 80.0 {
                    0.0
                } else {
                    0.15
                };
                let c_guitar = c_harm * guitar_gain;
                guitar_spec[k] = c_guitar;
                if k > 0 && k < n / 2 {
                    guitar_spec[n - k] = c_guitar.conj();
                }

                // Vocals stem: 200 Hz to 7500 Hz
                let vocal_gain = if freq >= 200.0 && freq <= 7000.0 { 1.0 } else { 0.05 };
                let c_vocals = c_harm * vocal_gain;
                vocals_spec[k] = c_vocals;
                if k > 0 && k < n / 2 {
                    vocals_spec[n - k] = c_vocals.conj();
                }

                // Other stem: Remaining harmonic energy
                other_spec[k] = c_harm * 0.8;
                if k > 0 && k < n / 2 {
                    other_spec[n - k] = (c_harm * 0.8).conj();
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

/// Neural Network / ONNX Runtime Source Separator with sliding window chunking and crossfades
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
            chunk_duration_secs: 8.0,
            overlap_duration_secs: 2.0,
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

    /// Try loading an ONNX Runtime session from the model path
    fn load_onnx_session(&self, path: &Path) -> Option<(ort::session::Session, Vec<String>)> {
        if !path.exists() {
            return None;
        }
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() < 1_000_000 {
                tracing::warn!("Model file at {} is smaller than 1MB (likely a placeholder marker), falling back to DSP", path.display());
                return None;
            }
        }

        tracing::info!("Initializing ONNX Runtime session for model: {}", path.display());
        let session = match ort::session::Session::builder()
            .and_then(|mut b| {
                b = b.with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?;
                b.commit_from_file(path)
            })
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to create ONNX session from {}: {}. Falling back to DSP.", path.display(), e);
                return None;
            }
        };

        // Determine stem mapping based on filename / model characteristics
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let stems = if filename.contains("6s") {
            vec![
                "drums".to_string(),
                "bass".to_string(),
                "other".to_string(),
                "vocals".to_string(),
                "guitar".to_string(),
                "piano".to_string(),
            ]
        } else if filename.contains("roformer") {
            vec!["vocals".to_string(), "other".to_string()]
        } else {
            vec![
                "drums".to_string(),
                "bass".to_string(),
                "other".to_string(),
                "vocals".to_string(),
            ]
        };

        Some((session, stems))
    }

    /// Process audio using loaded ONNX Runtime model
    fn separate_neural(
        &self,
        session: &mut ort::session::Session,
        stem_names: &[String],
        audio: &AudioBuffer,
    ) -> Result<SeparatedTracks> {
        let sample_rate = audio.sample_rate;
        let mono = audio.to_mono();
        let total_samples = mono.samples.len();

        // HTDemucs v4 ONNX models are compiled with a fixed chunk size of 343980 samples (~7.8s at 44.1 kHz)
        let chunk_samples = 343980usize;
        let overlap_samples = chunk_samples / 4; // 25% overlap (85995 samples ~ 1.95s)
        let hop_samples = chunk_samples - overlap_samples;

        let mut output_stems: HashMap<String, Vec<f32>> = HashMap::new();
        for stem in stem_names {
            output_stems.insert(stem.clone(), vec![0.0f32; total_samples]);
        }
        let mut stem_weights = vec![0.0f32; total_samples];

        let mut start_idx = 0;
        let num_stems = stem_names.len();

        while start_idx < total_samples {
            let end_idx = (start_idx + chunk_samples).min(total_samples);
            let current_len = end_idx - start_idx;

            // Prepare stereo input tensor [1, 2, chunk_samples] with zero-padding if at end
            let mut input_data = vec![0.0f32; 2 * chunk_samples];
            for j in 0..current_len {
                let s = mono.samples[start_idx + j];
                input_data[j] = s;
                input_data[chunk_samples + j] = s;
            }

            let input_tensor = ort::value::Tensor::from_array(([1usize, 2, chunk_samples], input_data))
                .map_err(|e| crate::error::AudioError::DspError(format!("Failed to create ONNX input tensor: {e}")))?;

            let outputs = session
                .run(ort::inputs![input_tensor])
                .map_err(|e| crate::error::AudioError::DspError(format!("ONNX model execution failed: {e}")))?;

            let first_output = outputs
                .into_iter()
                .next()
                .ok_or_else(|| crate::error::AudioError::DspError("ONNX model produced empty output".to_string()))?;

            let (extracted_shape, extracted_data) = first_output.1
                .try_extract_tensor::<f32>()
                .map_err(|e| crate::error::AudioError::DspError(format!("Failed to extract output tensor: {e}")))?;

            // Build crossfade window
            let mut crossfade = vec![1.0f32; current_len];
            let fade_len = overlap_samples.min(current_len / 3);
            if start_idx > 0 && fade_len > 0 {
                for j in 0..fade_len {
                    crossfade[j] = 0.5 * (1.0 - (PI * j as f32 / fade_len as f32).cos());
                }
            }
            if end_idx < total_samples && fade_len > 0 {
                for j in 0..fade_len {
                    let idx = current_len - fade_len + j;
                    crossfade[idx] = 0.5 * (1.0 + (PI * j as f32 / fade_len as f32).cos());
                }
            }

            let is_4d = extracted_shape.len() == 4;
            let channels_per_stem = if is_4d { extracted_shape[2] as usize } else { 1 };
            let samples_per_stem = if is_4d { extracted_shape[3] as usize } else { extracted_shape[2] as usize };

            for (stem_idx, stem_name) in stem_names.iter().enumerate() {
                if stem_idx >= num_stems {
                    break;
                }
                if let Some(target_acc) = output_stems.get_mut(stem_name) {
                    let stem_offset = stem_idx * channels_per_stem * samples_per_stem;
                    for j in 0..current_len.min(samples_per_stem) {
                        let mut sample_val = 0.0f32;
                        for ch in 0..channels_per_stem {
                            let idx = stem_offset + ch * samples_per_stem + j;
                            if idx < extracted_data.len() {
                                sample_val += extracted_data[idx];
                            }
                        }
                        sample_val /= channels_per_stem as f32;
                        target_acc[start_idx + j] += sample_val * crossfade[j];
                    }
                }
            }

            for j in 0..current_len {
                stem_weights[start_idx + j] += crossfade[j];
            }

            if end_idx >= total_samples {
                break;
            }
            start_idx += hop_samples;
        }

        // Normalize weights and return SeparatedTracks
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

impl Default for NeuralSourceSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceSeparator for NeuralSourceSeparator {
    fn separate(&self, audio: &AudioBuffer) -> Result<SeparatedTracks> {
        if let Some(ref path) = self.model_path {
            if let Some((mut session, stems)) = self.load_onnx_session(path) {
                tracing::info!("Running neural ONNX inference with {} stems: {:?}", stems.len(), stems);
                match self.separate_neural(&mut session, &stems, audio) {
                    Ok(tracks) => return Ok(tracks),
                    Err(e) => {
                        tracing::warn!("Neural inference error: {}. Falling back to DSP HPSS.", e);
                    }
                }
            }
        }

        tracing::info!("Executing DSP HPSS source separation engine (fallback)");
        self.fallback_separator.separate(audio)
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
