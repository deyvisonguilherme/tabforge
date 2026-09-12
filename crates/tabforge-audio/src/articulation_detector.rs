use crate::traits::{OnsetEvent, PitchEvent};
use tabforge_core::Articulation;

/// Detector for guitar playing techniques and articulations based on pitch trajectory and spectral envelopes
#[derive(Debug, Clone)]
pub struct ArticulationDetector {
    pub vibrato_min_freq_hz: f64,
    pub vibrato_max_freq_hz: f64,
    pub vibrato_min_depth_semitones: f64,
    pub bend_min_semitones: f64,
}

impl Default for ArticulationDetector {
    fn default() -> Self {
        Self {
            vibrato_min_freq_hz: 4.0,
            vibrato_max_freq_hz: 8.5,
            vibrato_min_depth_semitones: 0.35,
            bend_min_semitones: 0.8,
        }
    }
}

impl ArticulationDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a note's pitch trajectory, RMS envelope, and onset context to detect articulation
    pub fn detect(
        &self,
        start_time: f64,
        end_time: f64,
        onset: Option<&OnsetEvent>,
        pitches: &[PitchEvent],
        rms_envelope: &[f32],
        hop_size: usize,
        sample_rate: f64,
        prev_note_pitch_midi: Option<u8>,
    ) -> Option<Articulation> {
        let note_pitches: Vec<&PitchEvent> = pitches
            .iter()
            .filter(|p| p.time_seconds >= start_time && p.time_seconds <= end_time)
            .collect();

        // 1. Palm mute detection: very fast RMS decay
        let note_dur = end_time - start_time;
        if note_dur > 0.04 && note_dur < 0.18 {
            let start_frame = ((start_time * sample_rate / hop_size as f64).round() as usize).min(rms_envelope.len().saturating_sub(1));
            let end_frame = ((end_time * sample_rate / hop_size as f64).round() as usize).min(rms_envelope.len());
            if end_frame > start_frame + 2 {
                let peak = rms_envelope[start_frame..end_frame].iter().copied().fold(0.0f32, f32::max);
                let tail = rms_envelope[end_frame.saturating_sub(2)..end_frame].iter().copied().sum::<f32>() / 2.0;
                if peak > 0.05 && tail < peak * 0.12 {
                    return Some(Articulation::PalmMute);
                }
            }
        }

        // 2. Hammer-on / Pull-off detection (legato onset without heavy transient)
        if let (Some(prev_midi), Some(on)) = (prev_note_pitch_midi, onset) {
            if on.strength < 0.8 && !note_pitches.is_empty() {
                let current_midi = note_pitches[0].pitch.midi();
                let diff = current_midi as i32 - prev_midi as i32;
                if diff > 0 && diff <= 5 {
                    return Some(Articulation::HammerOn);
                } else if diff < 0 && diff >= -5 {
                    return Some(Articulation::PullOff);
                }
            }
        }

        if note_pitches.len() < 4 {
            return None;
        }

        // Convert frequency trajectory to fractional MIDI pitches: m = 69 + 12 * log2(f / 440)
        let midi_trajectory: Vec<(f64, f64)> = note_pitches
            .iter()
            .filter(|p| p.frequency > 40.0)
            .map(|p| {
                let midi_val = 69.0 + 12.0 * (p.frequency / 440.0).log2();
                (p.time_seconds, midi_val)
            })
            .collect();

        if midi_trajectory.len() < 4 {
            return None;
        }

        // 3. Bend detection: monotonic rise in pitch of >= bend_min_semitones
        let first_pitch = midi_trajectory[0].1;
        let max_pitch = midi_trajectory.iter().map(|(_, m)| *m).fold(f64::MIN, f64::max);
        let last_pitch = midi_trajectory.last().unwrap().1;
        let pitch_rise = max_pitch - first_pitch;

        if pitch_rise >= self.bend_min_semitones {
            if (last_pitch - first_pitch).abs() < 0.4 && pitch_rise >= 1.0 {
                return Some(Articulation::ReleaseBend);
            }
            return Some(Articulation::Bend);
        }

        // 4. Slide detection: steady continuous glissando
        let total_delta = last_pitch - first_pitch;
        if total_delta >= 1.8 && note_dur < 0.6 {
            return Some(Articulation::SlideUp);
        } else if total_delta <= -1.8 && note_dur < 0.6 {
            return Some(Articulation::SlideDown);
        }

        // 5. Vibrato detection: sinusoidal pitch oscillation in 4-8.5 Hz
        if let Some(vibrato) = self.detect_vibrato(&midi_trajectory) {
            return Some(vibrato);
        }

        None
    }

    fn detect_vibrato(&self, trajectory: &[(f64, f64)]) -> Option<Articulation> {
        if trajectory.len() < 6 {
            return None;
        }

        let total_time = trajectory.last().unwrap().0 - trajectory.first().unwrap().0;
        if total_time < 0.25 {
            return None;
        }

        // Calculate mean pitch
        let mean: f64 = trajectory.iter().map(|(_, m)| *m).sum::<f64>() / trajectory.len() as f64;

        // Zero crossings of (pitch - mean)
        let mut zero_crossings = 0;
        let mut prev_val = trajectory[0].1 - mean;
        let mut max_dev: f64 = 0.0;

        for (_, m) in &trajectory[1..] {
            let val = m - mean;
            max_dev = max_dev.max(val.abs());
            if (val > 0.0 && prev_val <= 0.0) || (val < 0.0 && prev_val >= 0.0) {
                zero_crossings += 1;
            }
            prev_val = val;
        }

        // Peak-to-peak depth
        let depth = max_dev * 2.0;
        if depth < self.vibrato_min_depth_semitones || depth > 2.5 {
            return None;
        }

        // Frequency = zero_crossings / (2 * duration)
        let est_freq = (zero_crossings as f64) / (2.0 * total_time);
        if est_freq >= self.vibrato_min_freq_hz && est_freq <= self.vibrato_max_freq_hz {
            return Some(Articulation::Vibrato);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabforge_core::Pitch;

    #[test]
    fn test_vibrato_detection() {
        let detector = ArticulationDetector::default();
        let mut pitches = Vec::new();
        // 6 Hz vibrato around A4 (440 Hz = MIDI 69) with +/- 0.5 semitone depth
        for i in 0..50 {
            let t = i as f64 * 0.01; // 100 fps, 0.5s total
            let mod_semitones = 0.5 * (2.0 * std::f64::consts::PI * 6.0 * t).sin();
            let midi = 69.0 + mod_semitones;
            let freq = 440.0 * 2.0f64.powf((midi - 69.0) / 12.0);
            pitches.push(PitchEvent {
                time_seconds: t,
                pitch: Pitch::A4,
                confidence: 0.95,
                frequency: freq,
            });
        }

        let art = detector.detect(0.0, 0.5, None, &pitches, &[], 256, 44100.0, None);
        assert_eq!(art, Some(Articulation::Vibrato));
    }

    #[test]
    fn test_bend_detection() {
        let detector = ArticulationDetector::default();
        let mut pitches = Vec::new();
        // Pitch rises from 440 Hz (MIDI 69) to 493.88 Hz (MIDI 71, full tone bend)
        for i in 0..30 {
            let t = i as f64 * 0.01;
            let progress = (i as f64 / 29.0).min(1.0);
            let midi = 69.0 + 2.0 * progress;
            let freq = 440.0 * 2.0f64.powf((midi - 69.0) / 12.0);
            pitches.push(PitchEvent {
                time_seconds: t,
                pitch: Pitch::A4,
                confidence: 0.95,
                frequency: freq,
            });
        }

        let art = detector.detect(0.0, 0.3, None, &pitches, &[], 256, 44100.0, None);
        assert_eq!(art, Some(Articulation::Bend));
    }
}
