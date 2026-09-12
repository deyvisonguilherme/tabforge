use crate::buffer::AudioBuffer;
use crate::traits::{OnsetEvent, PitchEvent, RawNoteEvent};
use tabforge_core::Pitch;

/// ADSR (Attack, Decay, Sustain, Release) Note Segmenter
///
/// Traces signal envelope and fundamental pitch to determine exact note-on,
/// velocity, pitch sustain, and note-off events (including staccato and rests).
#[derive(Debug, Clone)]
pub struct AdsrSegmenter {
    pub hop_size: usize,
    pub sustain_drop_ratio: f32, // -20 dB = ~0.1
    pub silence_rms_threshold: f32,
    pub max_note_duration_secs: f64,
    pub min_note_duration_secs: f64,
}

impl Default for AdsrSegmenter {
    fn default() -> Self {
        Self {
            hop_size: 256,
            sustain_drop_ratio: 0.10, // Cut off when RMS drops below 10% (-20dB) of attack peak
            silence_rms_threshold: 0.005,
            max_note_duration_secs: 4.0,
            min_note_duration_secs: 0.05,
        }
    }
}

impl AdsrSegmenter {
    /// Segment continuous audio into discrete musical RawNoteEvents with ADSR tracking
    pub fn segment(
        &self,
        audio: &AudioBuffer,
        onsets: &[OnsetEvent],
        pitches: &[PitchEvent],
    ) -> Vec<RawNoteEvent> {
        let mono = audio.to_mono();
        let samples = &mono.samples;
        let sample_rate = mono.sample_rate as f64;
        let total_duration = mono.duration_seconds();

        if samples.is_empty() || (onsets.is_empty() && pitches.is_empty()) {
            return Vec::new();
        }

        // 1. Compute fine-grained RMS energy envelope
        let num_frames = samples.len() / self.hop_size;
        let mut rms_envelope = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let start = i * self.hop_size;
            let end = (start + self.hop_size).min(samples.len());
            let sum_sq: f32 = samples[start..end].iter().map(|&s| s * s).sum();
            let rms = (sum_sq / (end - start) as f32).sqrt();
            rms_envelope.push(rms);
        }

        let frame_to_time = |frame: usize| (frame * self.hop_size) as f64 / sample_rate;
        let time_to_frame = |t: f64| ((t * sample_rate / self.hop_size as f64).round() as usize).min(num_frames.saturating_sub(1));

        // Fallback if no onsets detected: segment based on continuous pitch regions
        if onsets.is_empty() {
            return self.segment_from_pitches_only(pitches, &rms_envelope, sample_rate);
        }

        let mut raw_notes = Vec::new();

        for (i, onset) in onsets.iter().enumerate() {
            let t_start = onset.time_seconds;
            let t_next_onset = onsets
                .get(i + 1)
                .map(|o| o.time_seconds)
                .unwrap_or(total_duration);

            let max_end = (t_start + self.max_note_duration_secs).min(t_next_onset).min(total_duration);
            let start_frame = time_to_frame(t_start);
            let max_frame = time_to_frame(max_end);

            if start_frame >= max_frame {
                continue;
            }

            // 2. Attack Phase: Find peak RMS within first 35ms of onset
            let attack_window_frames = ((0.035 * sample_rate / self.hop_size as f64).ceil() as usize).max(1);
            let attack_end_frame = (start_frame + attack_window_frames).min(max_frame);

            let mut peak_rms = self.silence_rms_threshold;
            for f in start_frame..attack_end_frame {
                if rms_envelope[f] > peak_rms {
                    peak_rms = rms_envelope[f];
                }
            }

            // Map peak RMS to MIDI velocity (30 to 127)
            let velocity = ((peak_rms * 150.0).clamp(30.0, 127.0)) as u8;

            // 3. Find Note Pitch: Select pitch from pitch detector in onset window
            let search_pitch_end = (t_start + 0.08).min(max_end);
            let note_pitches: Vec<&PitchEvent> = pitches
                .iter()
                .filter(|p| p.time_seconds >= t_start - 0.02 && p.time_seconds <= search_pitch_end)
                .collect();

            let target_pitch = if let Some(best) = note_pitches
                .iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            {
                best.pitch
            } else {
                // Fallback: look slightly further ahead in sustain
                let next_pitches: Vec<&PitchEvent> = pitches
                    .iter()
                    .filter(|p| p.time_seconds >= t_start && p.time_seconds < max_end)
                    .collect();
                if let Some(p) = next_pitches.first() {
                    p.pitch
                } else {
                    continue; // No pitch detected for this onset
                }
            };

            // 4. Sustain & Release Phase: Trace envelope and pitch stability
            let sustain_cutoff = (peak_rms * self.sustain_drop_ratio).max(self.silence_rms_threshold);
            let mut end_frame = max_frame;

            let mut consecutive_deviations = 0;
            for f in (start_frame + 1)..max_frame {
                let current_rms = rms_envelope[f];
                let current_time = frame_to_time(f);

                // Check RMS decay
                if current_rms < sustain_cutoff {
                    end_frame = f;
                    break;
                }

                // Check pitch stability during sustain
                if let Some(p_event) = pitches.iter().find(|p| (p.time_seconds - current_time).abs() < 0.02) {
                    let semitone_diff = (p_event.pitch.midi() as i32 - target_pitch.midi() as i32).abs();
                    if semitone_diff > 1 {
                        consecutive_deviations += 1;
                        if consecutive_deviations >= 2 {
                            end_frame = f;
                            break;
                        }
                    } else {
                        consecutive_deviations = 0;
                    }
                }
            }

            let t_end = frame_to_time(end_frame).min(t_next_onset).min(total_duration);
            if t_end - t_start >= self.min_note_duration_secs {
                let art_detector = crate::articulation_detector::ArticulationDetector::default();
                let prev_midi = raw_notes.last().map(|n: &RawNoteEvent| n.pitch.midi());
                let articulation = art_detector.detect(
                    t_start,
                    t_end,
                    Some(onset),
                    pitches,
                    &rms_envelope,
                    self.hop_size,
                    sample_rate,
                    prev_midi,
                );

                raw_notes.push(RawNoteEvent {
                    start_time: t_start,
                    end_time: t_end,
                    pitch: target_pitch,
                    velocity,
                    articulation,
                });
            }
        }

        raw_notes
    }

    /// Pitch-only segmentation fallback when onsets are absent
    fn segment_from_pitches_only(
        &self,
        pitches: &[PitchEvent],
        _rms_envelope: &[f32],
        _sample_rate: f64,
    ) -> Vec<RawNoteEvent> {
        let mut raw_notes = Vec::new();
        let mut current_pitch: Option<Pitch> = None;
        let mut start_time = 0.0;
        let mut last_time = 0.0;

        for p in pitches {
            match current_pitch {
                Some(cur) if cur == p.pitch => {
                    last_time = p.time_seconds;
                }
                Some(cur) => {
                    raw_notes.push(RawNoteEvent {
                        start_time,
                        end_time: last_time + 0.1,
                        pitch: cur,
                        velocity: 90,
                        articulation: None,
                    });
                    current_pitch = Some(p.pitch);
                    start_time = p.time_seconds;
                    last_time = p.time_seconds;
                }
                None => {
                    current_pitch = Some(p.pitch);
                    start_time = p.time_seconds;
                    last_time = p.time_seconds;
                }
            }
        }

        if let Some(cur) = current_pitch {
            raw_notes.push(RawNoteEvent {
                start_time,
                end_time: last_time + 0.1,
                pitch: cur,
                velocity: 90,
                articulation: None,
            });
        }

        raw_notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_staccato_note(freq_hz: f32, sample_rate: u32, note_secs: f32, silence_secs: f32) -> AudioBuffer {
        let note_samples = (sample_rate as f32 * note_secs) as usize;
        let total_samples = ((note_secs + silence_secs) * sample_rate as f32) as usize;
        let mut samples = vec![0.0f32; total_samples];

        for i in 0..note_samples {
            let t = i as f32 / sample_rate as f32;
            let env = (-(i as f32) / (sample_rate as f32 * 0.15)).exp();
            samples[i] = (2.0 * PI * freq_hz * t).sin() * 0.8 * env;
        }

        AudioBuffer::new(samples, sample_rate, 1)
    }

    #[test]
    fn test_adsr_staccato_segmentation() {
        let sample_rate = 44100;
        // 0.2s staccato sound followed by 0.8s silence
        let audio = generate_staccato_note(440.0, sample_rate, 0.2, 0.8);
        let onsets = vec![OnsetEvent {
            time_seconds: 0.0,
            strength: 1.5,
        }];
        let pitches = vec![
            PitchEvent {
                time_seconds: 0.05,
                pitch: Pitch::A4,
                confidence: 0.95,
                frequency: 440.0,
            },
            PitchEvent {
                time_seconds: 0.10,
                pitch: Pitch::A4,
                confidence: 0.90,
                frequency: 440.0,
            },
        ];

        let segmenter = AdsrSegmenter::default();
        let notes = segmenter.segment(&audio, &onsets, &pitches);

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, Pitch::A4);
        assert_eq!(notes[0].start_time, 0.0);
        // Staccato note should release well before 1.0s
        assert!(notes[0].end_time < 0.6, "Note duration should cut off early, got end_time={}", notes[0].end_time);
        assert!(notes[0].end_time > 0.1, "Note duration too short");
    }
}
