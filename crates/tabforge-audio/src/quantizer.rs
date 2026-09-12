use crate::error::Result;
use crate::traits::{BeatGrid, OnsetEvent, PitchEvent, Quantizer, RawNoteEvent};
use tabforge_core::{Beat, Duration, Measure, Note, Pitch};

/// Rhythmic quantizer converting continuous event timestamps into exact rational Beats and Measures
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
    /// Approximate continuous duration in quarter-notes to the nearest canonical musical duration
    pub fn approximate_duration(&self, quarters: f64) -> Duration {
        if quarters >= 3.5 {
            Duration::WHOLE
        } else if quarters >= 1.75 {
            Duration::HALF
        } else if quarters >= 0.75 {
            Duration::QUARTER
        } else if quarters >= 0.375 {
            Duration::EIGHTH
        } else {
            Duration::SIXTEENTH
        }
    }

    /// Decompose a gap duration into a sequence of canonical rest durations
    pub fn decompose_rest_duration(mut gap: Duration) -> Vec<Duration> {
        let mut rests = Vec::new();
        let denominations = [
            Duration::WHOLE,
            Duration::HALF,
            Duration::QUARTER,
            Duration::EIGHTH,
            Duration::SIXTEENTH,
        ];

        while gap >= Duration::SIXTEENTH {
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

    /// Quantize raw note events into perfectly formatted 4/4 musical measures with rest filling
    pub fn quantize_to_measures(
        &self,
        raw_events: &[RawNoteEvent],
        beat_grid: &BeatGrid,
    ) -> Result<Vec<Measure>> {
        if raw_events.is_empty() {
            let mut empty_measure = Measure::new(1);
            empty_measure.add_beat(Beat::new(Duration::ZERO, Duration::WHOLE));
            return Ok(vec![empty_measure]);
        }

        let quarter_secs = beat_grid.tempo.quarter_note_seconds();
        let measure_capacity = Duration::WHOLE; // 4/4 = 1 Whole Note (4 Quarters)

        // 1. Sort raw events by start time
        let mut sorted_events = raw_events.to_vec();
        sorted_events.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));

        let mut timeline_beats: Vec<(Duration, Duration, Vec<Note>)> = Vec::new();
        let mut current_timeline = Duration::ZERO;

        for event in sorted_events {
            let q_start = (event.start_time / quarter_secs).max(0.0);
            let q_dur = ((event.end_time - event.start_time).max(quarter_secs * 0.25)) / quarter_secs;

            // Quantize start position to 16th grid (fraction of whole note)
            let raw_16th = (q_start * 4.0).round() as i32;
            let note_start = Duration::new(raw_16th, 16);
            let note_dur = self.approximate_duration(q_dur);

            // If there's a rest gap before this note, insert rests
            if note_start > current_timeline {
                let gap = note_start - current_timeline;
                let rest_pieces = Self::decompose_rest_duration(gap);
                let mut rest_pos = current_timeline;
                for piece in rest_pieces {
                    timeline_beats.push((rest_pos, piece, Vec::new()));
                    rest_pos += piece;
                }
                current_timeline = note_start;
            }

            let mut note = Note::new(event.pitch, note_dur).with_velocity(event.velocity);
            if let Some(art) = event.articulation {
                note = note.with_articulation(art);
            }
            timeline_beats.push((current_timeline, note_dur, vec![note]));
            current_timeline += note_dur;
        }

        // 2. Partition global timeline beats into Measures of 4/4
        let mut measures: Vec<Measure> = Vec::new();
        let mut current_measure = Measure::new(1);
        let mut measure_filled = Duration::ZERO;

        for (_pos, dur, notes) in timeline_beats {
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
                    let next_num = measures.len() as u32 + 1;
                    current_measure = Measure::new(next_num);
                    measure_filled = Duration::ZERO;
                }
            }
        }

        // Fill trailing rest in last measure if partially filled
        if measure_filled > Duration::ZERO && measure_filled < measure_capacity {
            let trailing_gap = measure_capacity - measure_filled;
            let rest_pieces = Self::decompose_rest_duration(trailing_gap);
            for piece in rest_pieces {
                current_measure.add_beat(Beat::new(measure_filled, piece));
                measure_filled += piece;
            }
            measures.push(current_measure);
        } else if measure_filled == Duration::ZERO && !measures.is_empty() {
            // Already clean
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
    use tabforge_core::{Pitch, Tempo};

    #[test]
    fn test_quantize_to_measures_with_rest_filling() {
        let quantizer = SimpleQuantizer::default();
        let beat_grid = BeatGrid {
            tempo: Tempo::new(120.0).unwrap(), // 1 quarter = 0.5s
            beat_times_seconds: vec![0.0, 0.5, 1.0, 1.5, 2.0],
        };

        // Two quarter notes at 0.0s and 1.0s (leaving 0.5s rest in between, and rest at end of 4/4 measure)
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

        // Total duration of beats in the measure must equal Duration::WHOLE (1/1)
        let total_measure_dur: Duration = m.beats.iter().map(|b| b.duration).fold(Duration::ZERO, |acc, d| acc + d);
        assert_eq!(total_measure_dur, Duration::WHOLE, "Measure total duration must be exactly 1 whole note (4/4)");

        // Verify notes and rests sequence: Note, Rest, Note, Rest
        assert_eq!(m.beats.len(), 4);
        assert!(!m.beats[0].is_rest());
        assert_eq!(m.beats[0].notes[0].pitch, Pitch::E2);
        assert!(m.beats[1].is_rest());
        assert!(!m.beats[2].is_rest());
        assert_eq!(m.beats[2].notes[0].pitch, Pitch::A2);
        assert_eq!(m.beats[2].notes[0].articulation, Some(tabforge_core::Articulation::HammerOn));
        assert!(m.beats[3].is_rest());
    }
}
