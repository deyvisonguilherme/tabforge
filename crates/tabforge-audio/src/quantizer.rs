use crate::error::Result;
use crate::traits::{BeatGrid, OnsetEvent, PitchEvent, Quantizer, RawNoteEvent};
use tabforge_core::{Beat, Duration, Measure, Note, Pitch, Tempo, TimeSignature};

/// Rhythmic quantizer converting continuous event timestamps into exact rational Beats and Measures,
/// supporting flexible time signatures (4/4, 3/4, 6/8, 7/8, etc.) and dynamic tempo changes.
pub struct SimpleQuantizer {
    pub min_subdivision: Duration,
}

impl Default for SimpleQuantizer {
    fn default() -> Self {
        Self {
            min_subdivision: Duration::SIXTEENTH,
        }
    }
}

impl Quantizer for SimpleQuantizer {
    fn quantize(
        &self,
        raw_events: &[RawNoteEvent],
        beat_grid: &BeatGrid,
    ) -> Result<Vec<Beat>> {
        let measures = self.quantize_to_measures(raw_events, beat_grid)?;
        let mut all_beats = Vec::new();
        for m in measures {
            all_beats.extend(m.beats);
        }
        Ok(all_beats)
    }
}

impl SimpleQuantizer {
    /// Approximate continuous duration in quarter-notes to canonical musical duration
    pub fn approximate_duration(&self, quarters: f64) -> Duration {
        if quarters >= 3.5 {
            Duration::WHOLE
        } else if quarters >= 2.6 {
            Duration::DOTTED_HALF
        } else if quarters >= 1.75 {
            Duration::HALF
        } else if quarters >= 1.25 {
            Duration::DOTTED_QUARTER
        } else if quarters >= 0.75 {
            Duration::QUARTER
        } else if quarters >= 0.375 {
            Duration::EIGHTH
        } else {
            Duration::SIXTEENTH
        }
    }

    /// Decompose a gap duration into canonical musical rest durations (including dotted rests)
    pub fn decompose_rest_duration(mut gap: Duration) -> Vec<Duration> {
        let mut rests = Vec::new();
        let denominations = [
            Duration::WHOLE,          // 1/1
            Duration::DOTTED_HALF,    // 3/4
            Duration::HALF,           // 1/2
            Duration::DOTTED_QUARTER, // 3/8 (canonical 6/8 compound rest)
            Duration::QUARTER,        // 1/4
            Duration::DOTTED_EIGHTH,  // 3/16
            Duration::EIGHTH,         // 1/8
            Duration::SIXTEENTH,      // 1/16
            Duration::THIRTY_SECOND,  // 1/32
        ];

        while gap >= Duration::THIRTY_SECOND {
            let mut fitted = false;
            for &denom in &denominations {
                if gap >= denom {
                    rests.push(denom);
                    gap -= denom;
                    fitted = true;
                    break;
                }
            }
            if !fitted {
                break;
            }
        }

        rests
    }

    /// Quantize raw note events into structured musical measures supporting flexible time signatures and dynamic tempo
    pub fn quantize_to_measures(
        &self,
        raw_events: &[RawNoteEvent],
        beat_grid: &BeatGrid,
    ) -> Result<Vec<Measure>> {
        let default_time_sig = beat_grid.time_signature;
        let default_capacity = default_time_sig.measure_duration();

        if raw_events.is_empty() {
            let mut empty_measure = Measure::new(1).with_time_signature(default_time_sig).with_tempo(beat_grid.tempo);
            let rest_pieces = Self::decompose_rest_duration(default_capacity);
            let mut pos = Duration::ZERO;
            for piece in rest_pieces {
                empty_measure.add_beat(Beat::new(pos, piece));
                pos += piece;
            }
            return Ok(vec![empty_measure]);
        }

        // 1. Sort raw events by start time
        let mut sorted_events = raw_events.to_vec();
        sorted_events.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));

        let mut timeline_beats: Vec<(Duration, Duration, f64, Vec<Note>)> = Vec::new();
        let mut current_timeline = Duration::ZERO;

        for event in sorted_events {
            let effective_tempo = beat_grid.tempo_at(event.start_time);
            let quarter_secs = effective_tempo.quarter_note_seconds();

            let q_start = (event.start_time / quarter_secs).max(0.0);
            let q_dur = ((event.end_time - event.start_time).max(quarter_secs * 0.25)) / quarter_secs;

            // Quantize start position to 16th grid (fraction of whole note: quarter / 4)
            let raw_16th = (q_start * 4.0).round() as i32;
            let note_start = Duration::new(raw_16th, 16);
            let note_dur = self.approximate_duration(q_dur);

            // If there's a rest gap before this note, insert rests
            if note_start > current_timeline {
                let gap = note_start - current_timeline;
                let rest_pieces = Self::decompose_rest_duration(gap);
                let mut rest_pos = current_timeline;
                for piece in rest_pieces {
                    timeline_beats.push((rest_pos, piece, event.start_time, Vec::new()));
                    rest_pos += piece;
                }
                current_timeline = note_start;
            }

            let mut note = Note::new(event.pitch, note_dur).with_velocity(event.velocity);
            if let Some(art) = event.articulation {
                note = note.with_articulation(art);
            }
            timeline_beats.push((current_timeline, note_dur, event.start_time, vec![note]));
            current_timeline += note_dur;
        }

        // 2. Partition global timeline beats into flexible Measures
        let mut measures: Vec<Measure> = Vec::new();
        let mut current_measure_num = 1;
        let mut current_measure = Measure::new(current_measure_num);
        let mut measure_filled = Duration::ZERO;
        let mut last_time_sig: Option<TimeSignature> = None;
        let mut last_tempo: Option<Tempo> = None;

        for (_pos, dur, start_time_secs, notes) in timeline_beats {
            let active_time_sig = beat_grid.time_signature_at(start_time_secs);
            let active_tempo = beat_grid.tempo_at(start_time_secs);
            let measure_capacity = active_time_sig.measure_duration();

            // Check if measure needs header attributes
            if measure_filled == Duration::ZERO {
                if last_time_sig != Some(active_time_sig) {
                    current_measure.time_signature = Some(active_time_sig);
                    last_time_sig = Some(active_time_sig);
                }
                if last_tempo != Some(active_tempo) {
                    current_measure.tempo = Some(active_tempo);
                    last_tempo = Some(active_tempo);
                }
            }

            let mut remaining_dur = dur;
            let current_notes = notes;

            while remaining_dur > Duration::ZERO {
                let space_in_measure = measure_capacity - measure_filled;
                let piece_dur = remaining_dur.min(space_in_measure);

                // Add beat to current measure at local offset
                if current_notes.is_empty() {
                    current_measure.add_beat(Beat::new(measure_filled, piece_dur));
                } else {
                    let note_instances: Vec<Note> = current_notes
                        .iter()
                        .map(|n| {
                            let mut new_n = Note::new(n.pitch, piece_dur).with_velocity(n.velocity);
                            if let Some(art) = n.articulation {
                                new_n = new_n.with_articulation(art);
                            }
                            new_n
                        })
                        .collect();
                    current_measure.add_beat(Beat::with_notes(measure_filled, piece_dur, note_instances));
                }

                measure_filled += piece_dur;
                remaining_dur -= piece_dur;

                if measure_filled >= measure_capacity {
                    measures.push(current_measure);
                    current_measure_num += 1;
                    current_measure = Measure::new(current_measure_num);
                    measure_filled = Duration::ZERO;
                }
            }
        }

        // Fill trailing rest in last measure if partially filled
        let active_time_sig = beat_grid.time_signature;
        let measure_capacity = active_time_sig.measure_duration();

        if measure_filled > Duration::ZERO && measure_filled < measure_capacity {
            let trailing_gap = measure_capacity - measure_filled;
            let rest_pieces = Self::decompose_rest_duration(trailing_gap);
            for piece in rest_pieces {
                current_measure.add_beat(Beat::new(measure_filled, piece));
                measure_filled += piece;
            }
            measures.push(current_measure);
        } else if measure_filled == Duration::ZERO && !measures.is_empty() {
            // Already cleanly closed
        } else if measures.is_empty() {
            measures.push(current_measure);
        }

        Ok(measures)
    }
}

/// Segment continuous pitch and onset streams into discrete RawNoteEvents
pub fn segment_notes(
    onsets: &[OnsetEvent],
    pitches: &[PitchEvent],
    audio_duration: f64,
) -> Vec<RawNoteEvent> {
    if pitches.is_empty() {
        return Vec::new();
    }

    let mut raw_notes = Vec::new();

    if onsets.is_empty() {
        // Fallback: group contiguous pitch regions
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

        return raw_notes;
    }

    for (i, onset) in onsets.iter().enumerate() {
        let start_time = onset.time_seconds;
        let next_onset_time = onsets
            .get(i + 1)
            .map(|o| o.time_seconds)
            .unwrap_or(audio_duration);
        let end_time = next_onset_time.min(start_time + 4.0);

        // Find pitch events within [start_time, end_time)
        let note_pitches: Vec<&PitchEvent> = pitches
            .iter()
            .filter(|p| p.time_seconds >= start_time && p.time_seconds < end_time)
            .collect();

        if let Some(first_p) = note_pitches.first() {
            let pitch = note_pitches
                .iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
                .map(|p| p.pitch)
                .unwrap_or(first_p.pitch);

            let velocity = (64.0 + (onset.strength * 30.0).clamp(0.0, 63.0)) as u8;

            raw_notes.push(RawNoteEvent {
                start_time,
                end_time,
                pitch,
                velocity,
                articulation: None,
            });
        }
    }

    raw_notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabforge_core::{Pitch, Tempo, TimeSignature};

    #[test]
    fn test_quantize_to_measures_4_4() {
        let quantizer = SimpleQuantizer::default();
        let beat_grid = BeatGrid::new(
            Tempo::new(120.0).unwrap(), // 1 quarter = 0.5s
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
        );

        let raw_events = vec![
            RawNoteEvent {
                start_time: 0.0,
                end_time: 0.5,
                pitch: Pitch::E2,
                velocity: 90,
                articulation: None,
            },
            RawNoteEvent {
                start_time: 1.0,
                end_time: 1.5,
                pitch: Pitch::A2,
                velocity: 85,
                articulation: Some(tabforge_core::Articulation::HammerOn),
            },
        ];

        let measures = quantizer.quantize_to_measures(&raw_events, &beat_grid).unwrap();
        assert_eq!(measures.len(), 1);
        let m = &measures[0];

        assert_eq!(m.total_duration(), Duration::WHOLE);
        assert_eq!(m.beats.len(), 4);
        assert!(!m.beats[0].is_rest());
        assert_eq!(m.beats[0].notes[0].pitch, Pitch::E2);
        assert!(m.beats[1].is_rest());
        assert!(!m.beats[2].is_rest());
        assert_eq!(m.beats[2].notes[0].pitch, Pitch::A2);
        assert_eq!(m.beats[2].notes[0].articulation, Some(tabforge_core::Articulation::HammerOn));
        assert!(m.beats[3].is_rest());
    }

    #[test]
    fn test_quantize_to_measures_3_4_waltz() {
        let quantizer = SimpleQuantizer::default();
        let mut beat_grid = BeatGrid::new(
            Tempo::new(120.0).unwrap(),
            vec![0.0, 0.5, 1.0, 1.5],
        );
        beat_grid.time_signature = TimeSignature::THREE_FOUR;

        let raw_events = vec![
            RawNoteEvent {
                start_time: 0.0,
                end_time: 0.5,
                pitch: Pitch::E2,
                velocity: 90,
                articulation: None,
            },
        ];

        let measures = quantizer.quantize_to_measures(&raw_events, &beat_grid).unwrap();
        assert_eq!(measures.len(), 1);
        let m = &measures[0];

        assert_eq!(m.total_duration(), Duration::new(3, 4), "3/4 measure must equal 3/4 duration");
        assert_eq!(m.time_signature, Some(TimeSignature::THREE_FOUR));
    }

    #[test]
    fn test_quantize_to_measures_6_8_compound() {
        let quantizer = SimpleQuantizer::default();
        let mut beat_grid = BeatGrid::new(
            Tempo::new(120.0).unwrap(),
            vec![0.0, 0.5, 1.0],
        );
        beat_grid.time_signature = TimeSignature::SIX_EIGHT;

        let raw_events = vec![
            RawNoteEvent {
                start_time: 0.0,
                end_time: 0.5,
                pitch: Pitch::D3,
                velocity: 90,
                articulation: None,
            },
        ];

        let measures = quantizer.quantize_to_measures(&raw_events, &beat_grid).unwrap();
        assert_eq!(measures.len(), 1);
        let m = &measures[0];

        assert_eq!(m.total_duration(), Duration::new(6, 8), "6/8 measure capacity must equal 6/8 duration");
        assert_eq!(m.time_signature, Some(TimeSignature::SIX_EIGHT));
    }

    #[test]
    fn test_quantize_to_measures_7_8_odd_meter() {
        let quantizer = SimpleQuantizer::default();
        let mut beat_grid = BeatGrid::new(
            Tempo::new(120.0).unwrap(),
            vec![0.0, 0.5, 1.0],
        );
        beat_grid.time_signature = TimeSignature::SEVEN_EIGHT;

        let raw_events = vec![
            RawNoteEvent {
                start_time: 0.0,
                end_time: 0.5,
                pitch: Pitch::G3,
                velocity: 95,
                articulation: None,
            },
        ];

        let measures = quantizer.quantize_to_measures(&raw_events, &beat_grid).unwrap();
        assert_eq!(measures.len(), 1);
        let m = &measures[0];

        assert_eq!(m.total_duration(), Duration::new(7, 8), "7/8 measure capacity must equal 7/8 duration");
        assert_eq!(m.time_signature, Some(TimeSignature::SEVEN_EIGHT));
    }
}
