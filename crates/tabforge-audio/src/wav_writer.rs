use crate::buffer::AudioBuffer;
use crate::error::{AudioError, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Write AudioBuffer to standard 16-bit PCM WAV file
pub fn write_wav(buffer: &AudioBuffer, path: &Path) -> Result<()> {
    let mut file = File::create(path)?;

    let num_channels = buffer.channels;
    let sample_rate = buffer.sample_rate;
    let bits_per_sample = 16u16;
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;

    let num_samples = buffer.samples.len() as u32;
    let data_chunk_size = num_samples * bytes_per_sample;
    let riff_chunk_size = 36 + data_chunk_size;

    // 1. RIFF Header
    file.write_all(b"RIFF")?;
    file.write_all(&riff_chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // 2. fmt Chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size for PCM = 16
    file.write_all(&1u16.to_le_bytes())?;  // AudioFormat 1 = PCM
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // 3. data Chunk
    file.write_all(b"data")?;
    file.write_all(&data_chunk_size.to_le_bytes())?;

    // 4. Sample data as 16-bit signed LE PCM
    for &sample in &buffer.samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0).round() as i16;
        file.write_all(&s16.to_le_bytes())?;
    }

    file.flush().map_err(|e| AudioError::DecodeError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{AudioDecoder, SymphoniaDecoder};
    use std::f32::consts::PI;

    #[test]
    fn test_write_and_decode_wav_roundtrip() {
        let sample_rate = 44100;
        let num_samples = 4410; // 0.1s
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((2.0 * PI * 440.0 * t).sin() * 0.8);
        }

        let buffer = AudioBuffer::new(samples, sample_rate, 1);
        let tmp_path = std::env::temp_dir().join("tabforge_test_roundtrip.wav");

        write_wav(&buffer, &tmp_path).expect("Failed to write WAV");
        assert!(tmp_path.exists());

        let decoder = SymphoniaDecoder::new()
            .with_target_sample_rate(None)
            .with_auto_normalize(false);
        let decoded = decoder.decode(&tmp_path).expect("Failed to decode written WAV");

        assert_eq!(decoded.sample_rate, sample_rate);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), num_samples);

        // Check sample value correlation (>0.99)
        let mut diff_sum = 0.0;
        for i in 0..num_samples {
            diff_sum += (decoded.samples[i] - buffer.samples[i]).abs();
        }
        let avg_diff = diff_sum / num_samples as f32;
        assert!(avg_diff < 0.01, "Average difference too high: {}", avg_diff);

        let _ = std::fs::remove_file(tmp_path);
    }
}
