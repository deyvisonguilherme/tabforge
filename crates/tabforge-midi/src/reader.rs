use crate::error::{MidiError, Result};
use midly::{MidiMessage, Smf, TrackEventKind};
use std::fs;
use std::path::Path;
use tabforge_core::{Beat, Duration, Instrument, Measure, Note, Pitch, Song, Tempo, Track};

pub trait MidiReader {
    fn read(&self, input: &Path) -> Result<Song>;
}

pub struct MidlyReader;

impl Default for MidlyReader {
    fn default() -> Self {
        Self
    }
}

impl MidiReader for MidlyReader {
    fn read(&self, input: &Path) -> Result<Song> {
        let data = fs::read(input)?;
        let smf = Smf::parse(&data).map_err(|e| MidiError::ParseError(e.to_string()))?;

        let song_title = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Song")
            .to_string();

        let mut song = Song::new(song_title).with_tempo(Tempo::default());

        for (idx, track) in smf.tracks.iter().enumerate() {
            let mut music_track = Track::new(format!("Track {}", idx + 1), Instrument::Generic);
            let mut measure = Measure::new(1);

            for event in track {
                if let TrackEventKind::Midi { message, .. } = event.kind {
                    if let MidiMessage::NoteOn { key, vel } = message {
                        if vel.as_int() > 0 {
                            if let Ok(pitch) = Pitch::from_midi(key.as_int()) {
                                let note = Note::new(pitch, Duration::QUARTER).with_velocity(vel.as_int());
                                measure.add_beat(Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![note]));
                            }
                        }
                    }
                }
            }

            if !measure.beats.is_empty() {
                music_track.add_measure(measure);
                song.add_track(music_track);
            }
        }

        Ok(song)
    }
}
