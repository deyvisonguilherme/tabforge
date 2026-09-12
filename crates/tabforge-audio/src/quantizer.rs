use crate::error::Result;
use crate::traits::{BeatGrid, Quantizer, RawNoteEvent};
use tabforge_core::{Beat, Duration, Note};

/// Rhythmic quantizer converting continuous event timestamps into exact rational Beats
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
        let beat_duration_secs = beat_grid.tempo.quarter_note_seconds();
        let mut beats = Vec::new();

        for event in raw_events {
            let note_duration_secs = (event.end_time - event.start_time).max(beat_duration_secs * 0.25);
            let quarters = note_duration_secs / beat_duration_secs;

            // Approximate to closest musical note duration
            let duration = if quarters >= 3.5 {
                Duration::WHOLE
            } else if quarters >= 1.75 {
                Duration::HALF
            } else if quarters >= 0.75 {
                Duration::QUARTER
            } else if quarters >= 0.375 {
                Duration::EIGHTH
            } else {
                Duration::SIXTEENTH
            };

            let note = Note::new(event.pitch, duration).with_velocity(event.velocity);
            let beat_pos = Duration::new((event.start_time / beat_duration_secs).round() as i32, 4);
            beats.push(Beat::with_notes(beat_pos, duration, vec![note]));
        }

        Ok(beats)
    }
}
