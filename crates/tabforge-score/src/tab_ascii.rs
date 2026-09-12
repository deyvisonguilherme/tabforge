use tabforge_core::Track;
use tabforge_guitar::{Fretboard, FingeringOptimizer};

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
                let pitches: Vec<_> = beat.notes.iter().map(|n| n.pitch).collect();
                if let Ok(fingerings) = self.optimizer.optimize_melody(&pitches) {
                    // String 1 is high 'e', string 6 is low 'E'
                    let mut fret_char = ["-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()];
                    for pos in fingerings {
                        if pos.string >= 1 && pos.string <= 6 {
                            let idx = (pos.string - 1) as usize;
                            fret_char[idx] = pos.fret.to_string();
                        }
                    }

                    for i in 0..6 {
                        lines[i].push_str(&format!("{}-", fret_char[i]));
                    }
                }
            }
            // Add bar line
            for i in 0..6 {
                lines[i].push('|');
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
    fn test_ascii_tab_rendering() {
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
}
