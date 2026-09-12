use tabforge_core::{Articulation, Track};
use tabforge_guitar::{FingeringOptimizer, Fretboard};

pub trait TabWriter {
    fn render(&self, track: &Track) -> String;
}

pub struct AsciiTabWriter {
    pub fretboard: Fretboard,
    pub optimizer: FingeringOptimizer,
}

impl Default for AsciiTabWriter {
    fn default() -> Self {
        let fretboard = Fretboard::default();
        let optimizer = FingeringOptimizer::new(fretboard.clone());
        Self {
            fretboard,
            optimizer,
        }
    }
}

impl AsciiTabWriter {
    fn format_fret_with_articulation(fret: u8, articulation: Option<Articulation>) -> String {
        match articulation {
            Some(Articulation::Bend) => format!("{}b", fret),
            Some(Articulation::ReleaseBend) => format!("{}r", fret),
            Some(Articulation::Vibrato) => format!("{}~", fret),
            Some(Articulation::SlideUp) => format!("{}/", fret),
            Some(Articulation::SlideDown) => format!("{}\\", fret),
            Some(Articulation::HammerOn) => format!("h{}", fret),
            Some(Articulation::PullOff) => format!("p{}", fret),
            Some(Articulation::PalmMute) => format!("{}.", fret),
            Some(Articulation::Harmonic) => format!("<{}>", fret),
            Some(Articulation::Tapping) => format!("t{}", fret),
            Some(Articulation::GhostNote) => format!("({})", fret),
            _ => fret.to_string(),
        }
    }
}

impl TabWriter for AsciiTabWriter {
    fn render(&self, track: &Track) -> String {
        let string_names = ["e", "B", "G", "D", "A", "E"];
        let mut lines = vec![
            format!("{:<2}|-", string_names[0]),
            format!("{:<2}|-", string_names[1]),
            format!("{:<2}|-", string_names[2]),
            format!("{:<2}|-", string_names[3]),
            format!("{:<2}|-", string_names[4]),
            format!("{:<2}|-", string_names[5]),
        ];

        for measure in &track.measures {
            for beat in &measure.beats {
                if beat.is_rest() {
                    // Render rest column
                    for line in lines.iter_mut().take(6) {
                        line.push_str("---");
                    }
                    continue;
                }

                let pitches: Vec<_> = beat.notes.iter().map(|n| n.pitch).collect();
                if let Ok(fingerings) = self.optimizer.optimize_melody(&pitches) {
                    // String 1 is high 'e' (idx 0), string 6 is low 'E' (idx 5)
                    let mut fret_entries: [Option<String>; 6] = [None, None, None, None, None, None];

                    for (pos_idx, pos) in fingerings.iter().enumerate() {
                        if pos.string >= 1 && pos.string <= 6 {
                            let s_idx = (pos.string - 1) as usize;
                            let art = beat.notes.get(pos_idx).and_then(|n| n.articulation);
                            fret_entries[s_idx] = Some(Self::format_fret_with_articulation(pos.fret, art));
                        }
                    }

                    // Determine column character width for perfect vertical alignment
                    let max_width = fret_entries
                        .iter()
                        .filter_map(|opt| opt.as_ref().map(|s| s.len()))
                        .max()
                        .unwrap_or(1)
                        .max(1);

                    for i in 0..6 {
                        if let Some(ref text) = fret_entries[i] {
                            let pad = max_width.saturating_sub(text.len());
                            lines[i].push_str(text);
                            for _ in 0..pad {
                                lines[i].push('-');
                            }
                        } else {
                            for _ in 0..max_width {
                                lines[i].push('-');
                            }
                        }
                        lines[i].push('-');
                    }
                }
            }
            // Add measure bar line
            for line in lines.iter_mut().take(6) {
                line.push('|');
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tabforge_core::{Beat, Duration, Instrument, Measure, Note, Pitch};

    #[test]
    fn test_ascii_tab_rendering_basic() {
        let mut track = Track::new("Guitar", Instrument::ElectricGuitarClean);
        let mut measure = Measure::new(1);
        let pitch = Pitch::from_str("E2").unwrap();
        measure.add_beat(Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![Note::new(pitch, Duration::QUARTER)]));
        track.add_measure(measure);

        let writer = AsciiTabWriter::default();
        let rendered = writer.render(&track);
        assert!(rendered.contains("E |"));
        assert!(rendered.contains("0"));
    }

    #[test]
    fn test_ascii_tab_rendering_articulations() {
        let mut track = Track::new("Guitar Lead", Instrument::ElectricGuitarClean);
        let mut measure = Measure::new(1);
        
        let pitch_bend = Pitch::from_str("D4").unwrap();
        let mut note_bend = Note::new(pitch_bend, Duration::QUARTER);
        note_bend = note_bend.with_articulation(Articulation::Bend);
        measure.add_beat(Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![note_bend]));

        let pitch_vib = Pitch::from_str("E4").unwrap();
        let mut note_vib = Note::new(pitch_vib, Duration::QUARTER);
        note_vib = note_vib.with_articulation(Articulation::Vibrato);
        measure.add_beat(Beat::with_notes(Duration::QUARTER, Duration::QUARTER, vec![note_vib]));

        let pitch_h = Pitch::from_str("G4").unwrap();
        let mut note_h = Note::new(pitch_h, Duration::QUARTER);
        note_h = note_h.with_articulation(Articulation::HammerOn);
        measure.add_beat(Beat::with_notes(Duration::HALF, Duration::QUARTER, vec![note_h]));

        track.add_measure(measure);

        let writer = AsciiTabWriter::default();
        let rendered = writer.render(&track);
        
        assert!(rendered.contains("b-"), "Should render bend marker 'b'");
        assert!(rendered.contains("~-"), "Should render vibrato marker '~'");
        assert!(rendered.contains("h"), "Should render hammer-on marker 'h'");
    }
}
