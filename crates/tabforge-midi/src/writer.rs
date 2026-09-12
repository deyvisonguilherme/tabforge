use crate::error::{MidiError, Result};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track as MidlyTrack, TrackEvent,
    TrackEventKind,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tabforge_core::Song;

pub trait MidiWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()>;
}

pub struct MidlyWriter;

impl Default for MidlyWriter {
    fn default() -> Self {
        Self
    }
}

impl MidiWriter for MidlyWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()> {
        let ticks_per_quarter = 480u16;
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(ticks_per_quarter.into()),
        ));

        // Create tempo / metadata master track
        let mut meta_track = MidlyTrack::new();
        let tempo_micros = (60_000_000.0 / song.tempo.bpm()).round() as u32;
        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(tempo_micros.into())),
        });
        meta_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(song.title.as_bytes())),
        });
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
                    for note in &beat.notes {
                        let midi_pitch = note.pitch.midi();
                        let velocity = note.velocity;
                        // Duration in ticks (duration fraction * 4 * ticks_per_quarter)
                        let duration_ticks = (note.duration.as_f64() * 4.0 * ticks_per_quarter as f64) as u32;

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
