use crate::error::{MidiError, Result};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track as MidlyTrack, TrackEvent,
    TrackEventKind,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tabforge_core::{Song, TimeSignature};

pub trait MidiWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()>;
}

pub struct MidlyWriter;

impl Default for MidlyWriter {
    fn default() -> Self {
        Self
    }
}

impl MidlyWriter {
    fn time_sig_to_midly(ts: TimeSignature) -> (u8, u8, u8, u8) {
        let num = ts.numerator as u8;
        let denom_pow2 = match ts.denominator {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            16 => 4,
            32 => 5,
            _ => 2,
        };
        (num, denom_pow2, 24, 8)
    }
}

impl MidiWriter for MidlyWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()> {
        let ticks_per_quarter = 480u16;
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(ticks_per_quarter.into()),
        ));

        // Create tempo / time signature master track
        let mut meta_track = MidlyTrack::new();
        let tempo_micros = (60_000_000.0 / song.tempo.bpm()).round() as u32;
        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(tempo_micros.into())),
        });

        let (num, denom_pow2, clocks, thirty_seconds) = Self::time_sig_to_midly(song.time_signature);
        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_pow2, clocks, thirty_seconds)),
        });

        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(song.title.as_bytes())),
        });

        // Track dynamic tempo and time signature events across timeline
        let mut current_tick = 0u32;
        let mut last_meta_tick = 0u32;

        if let Some(first_track) = song.tracks.first() {
            for measure in &first_track.measures {
                let measure_ticks = (measure.total_duration().to_quarter_notes() * ticks_per_quarter as f64).round() as u32;

                if let Some(ts) = measure.time_signature {
                    let delta = current_tick.saturating_sub(last_meta_tick);
                    let (n, d, c, t) = Self::time_sig_to_midly(ts);
                    meta_track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, c, t)),
                    });
                    last_meta_tick = current_tick;
                }

                if let Some(tempo) = measure.tempo {
                    let delta = current_tick.saturating_sub(last_meta_tick);
                    let micros = (60_000_000.0 / tempo.bpm()).round() as u32;
                    meta_track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Meta(MetaMessage::Tempo(micros.into())),
                    });
                    last_meta_tick = current_tick;
                }

                current_tick += measure_ticks;
            }
        }

        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        smf.tracks.push(meta_track);

        // Convert each Track to MIDI events
        for track in &song.tracks {
            let mut midi_track = MidlyTrack::new();
            midi_track.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
            });

            for measure in &track.measures {
                for beat in &measure.beats {
                    if beat.is_rest() {
                        continue;
                    }
                    for note in &beat.notes {
                        let midi_pitch = note.pitch.midi();
                        let velocity = note.velocity;
                        let duration_ticks = (note.duration.to_quarter_notes() * ticks_per_quarter as f64) as u32;

                        // Note ON
                        midi_track.push(TrackEvent {
                            delta: 0.into(),
                            kind: TrackEventKind::Midi {
                                channel: track.channel.into(),
                                message: MidiMessage::NoteOn {
                                    key: midi_pitch.into(),
                                    vel: velocity.into(),
                                },
                            },
                        });

                        // Note OFF
                        midi_track.push(TrackEvent {
                            delta: duration_ticks.into(),
                            kind: TrackEventKind::Midi {
                                channel: track.channel.into(),
                                message: MidiMessage::NoteOff {
                                    key: midi_pitch.into(),
                                    vel: 0.into(),
                                },
                            },
                        });
                    }
                }
            }

            midi_track.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            smf.tracks.push(midi_track);
        }

        let mut out_bytes = Vec::new();
        smf.write(&mut out_bytes)
            .map_err(|e| MidiError::WriteError(e.to_string()))?;

        let mut file = File::create(output)?;
        file.write_all(&out_bytes)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tabforge_core::{Beat, Duration, Instrument, Measure, Note, Pitch, Tempo, TimeSignature, Track};

    #[test]
    fn test_midi_writer_with_odd_meter_and_tempo() {
        let mut song = Song::new("Odd Meter Song");
        song.time_signature = TimeSignature::SEVEN_EIGHT;
        song.tempo = Tempo::new(135.0).unwrap();

        let mut track = Track::new("Bass", Instrument::ElectricBass);
        let mut measure = Measure::new(1);
        let note = Note::new(Pitch::from_str("E2").unwrap(), Duration::DOTTED_QUARTER);
        measure.add_beat(Beat::with_notes(Duration::ZERO, Duration::DOTTED_QUARTER, vec![note]));
        track.add_measure(measure);
        song.add_track(track);

        let temp_dir = std::env::temp_dir();
        let out_path = temp_dir.join("test_odd_meter.mid");

        let writer = MidlyWriter::default();
        writer.write(&song, &out_path).unwrap();

        assert!(out_path.exists());
        let _ = std::fs::remove_file(out_path);
    }
}
