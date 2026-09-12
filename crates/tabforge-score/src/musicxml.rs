use crate::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tabforge_core::{Articulation, Duration, Song};

pub trait ScoreWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()>;
}

/// MusicXML 4.0 Exporter for notation software (MuseScore, Guitar Pro, Finale, Sibelius)
pub struct MusicXmlWriter;

impl Default for MusicXmlWriter {
    fn default() -> Self {
        Self
    }
}

impl MusicXmlWriter {
    fn duration_to_divisions(dur: Duration) -> u32 {
        // Divisions per quarter note = 4 (sixteenth note = 1 division, quarter note = 4 divisions, whole = 16)
        let quarters = dur.to_quarter_notes();
        ((quarters * 4.0).round() as u32).max(1)
    }

    fn duration_to_type(dur: Duration) -> &'static str {
        let quarters = dur.to_quarter_notes();
        if quarters >= 3.5 {
            "whole"
        } else if quarters >= 1.75 {
            "half"
        } else if quarters >= 0.75 {
            "quarter"
        } else if quarters >= 0.375 {
            "eighth"
        } else {
            "16th"
        }
    }
}

impl ScoreWriter for MusicXmlWriter {
    fn write(&self, song: &Song, output: &Path) -> Result<()> {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n");
        xml.push_str("<score-partwise version=\"4.0\">\n");

        // Work title
        xml.push_str("  <work>\n");
        xml.push_str(&format!("    <work-title>{}</work-title>\n", song.title));
        xml.push_str("  </work>\n");

        // Part list
        xml.push_str("  <part-list>\n");
        for (i, track) in song.tracks.iter().enumerate() {
            xml.push_str(&format!("    <score-part id=\"P{}\">\n", i + 1));
            xml.push_str(&format!("      <part-name>{}</part-name>\n", track.name));
            xml.push_str("    </score-part>\n");
        }
        xml.push_str("  </part-list>\n");

        // Parts
        for (i, track) in song.tracks.iter().enumerate() {
            xml.push_str(&format!("  <part id=\"P{}\">\n", i + 1));

            for (m_idx, measure) in track.measures.iter().enumerate() {
                xml.push_str(&format!("    <measure number=\"{}\">\n", m_idx + 1));

                // Attributes in first measure
                if m_idx == 0 {
                    xml.push_str("      <attributes>\n");
                    xml.push_str("        <divisions>4</divisions>\n");
                    xml.push_str("        <key>\n          <fifths>0</fifths>\n        </key>\n");
                    xml.push_str("        <time>\n");
                    xml.push_str(&format!("          <beats>{}</beats>\n", song.time_signature.numerator));
                    xml.push_str(&format!("          <beat-type>{}</beat-type>\n", song.time_signature.denominator));
                    xml.push_str("        </time>\n");
                    xml.push_str("        <clef>\n          <sign>G</sign>\n          <line>2</line>\n        </clef>\n");
                    xml.push_str("      </attributes>\n");
                }

                for beat in &measure.beats {
                    let divisions = Self::duration_to_divisions(beat.duration);
                    let note_type = Self::duration_to_type(beat.duration);

                    if beat.is_rest() {
                        xml.push_str("      <note>\n");
                        xml.push_str("        <rest/>\n");
                        xml.push_str(&format!("        <duration>{}</duration>\n", divisions));
                        xml.push_str(&format!("        <type>{}</type>\n", note_type));
                        xml.push_str("      </note>\n");
                    } else {
                        for note in &beat.notes {
                            xml.push_str("      <note>\n");
                            xml.push_str("        <pitch>\n");
                            let step = match note.pitch.note_name() {
                                tabforge_core::NoteName::C | tabforge_core::NoteName::CSharp => "C",
                                tabforge_core::NoteName::D | tabforge_core::NoteName::DSharp => "D",
                                tabforge_core::NoteName::E => "E",
                                tabforge_core::NoteName::F | tabforge_core::NoteName::FSharp => "F",
                                tabforge_core::NoteName::G | tabforge_core::NoteName::GSharp => "G",
                                tabforge_core::NoteName::A | tabforge_core::NoteName::ASharp => "A",
                                tabforge_core::NoteName::B => "B",
                            };
                            let is_sharp = match note.pitch.note_name() {
                                tabforge_core::NoteName::CSharp
                                | tabforge_core::NoteName::DSharp
                                | tabforge_core::NoteName::FSharp
                                | tabforge_core::NoteName::GSharp
                                | tabforge_core::NoteName::ASharp => true,
                                _ => false,
                            };

                            xml.push_str(&format!("          <step>{}</step>\n", step));
                            if is_sharp {
                                xml.push_str("          <alter>1</alter>\n");
                            }
                            xml.push_str(&format!("          <octave>{}</octave>\n", note.pitch.octave()));
                            xml.push_str("        </pitch>\n");
                            xml.push_str(&format!("        <duration>{}</duration>\n", divisions));
                            xml.push_str(&format!("        <type>{}</type>\n", note_type));

                            // Articulations / Technical notations
                            if let Some(art) = note.articulation {
                                xml.push_str("        <notations>\n");
                                match art {
                                    Articulation::Bend => {
                                        xml.push_str("          <technical><bend><bend-alter>2</bend-alter></bend></technical>\n");
                                    }
                                    Articulation::ReleaseBend => {
                                        xml.push_str("          <technical><bend><bend-alter>2</bend-alter><release/></bend></technical>\n");
                                    }
                                    Articulation::HammerOn => {
                                        xml.push_str("          <technical><hammer-on type=\"start\">H</hammer-on></technical>\n");
                                    }
                                    Articulation::PullOff => {
                                        xml.push_str("          <technical><pull-off type=\"start\">P</pull-off></technical>\n");
                                    }
                                    Articulation::SlideUp | Articulation::SlideDown => {
                                        xml.push_str("          <slide type=\"start\"/>\n");
                                    }
                                    Articulation::Harmonic => {
                                        xml.push_str("          <technical><harmonic><natural/></harmonic></technical>\n");
                                    }
                                    Articulation::Vibrato => {
                                        xml.push_str("          <ornaments><wavy-line type=\"start\"/></ornaments>\n");
                                    }
                                    Articulation::PalmMute => {
                                        xml.push_str("          <technical><other-technical>P.M.</other-technical></technical>\n");
                                    }
                                    Articulation::Staccato => {
                                        xml.push_str("          <articulations><staccato/></articulations>\n");
                                    }
                                    Articulation::Accent => {
                                        xml.push_str("          <articulations><accent/></articulations>\n");
                                    }
                                    Articulation::Tenuto => {
                                        xml.push_str("          <articulations><tenuto/></articulations>\n");
                                    }
                                    _ => {}
                                }
                                xml.push_str("        </notations>\n");
                            }

                            xml.push_str("      </note>\n");
                        }
                    }
                }

                xml.push_str("    </measure>\n");
            }

            xml.push_str("  </part>\n");
        }

        xml.push_str("</score-partwise>\n");

        let mut file = File::create(output)?;
        file.write_all(xml.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tabforge_core::{Beat, Instrument, Measure, Note, Pitch, Track};

    #[test]
    fn test_musicxml_generation_with_rests_and_articulation() {
        let mut song = Song::new("Test Song");
        let mut track = Track::new("Guitar", Instrument::ElectricGuitarClean);
        let mut measure = Measure::new(1);

        let mut note = Note::new(Pitch::from_str("A4").unwrap(), Duration::QUARTER);
        note = note.with_articulation(Articulation::Bend);
        measure.add_beat(Beat::with_notes(Duration::ZERO, Duration::QUARTER, vec![note]));
        measure.add_beat(Beat::new(Duration::QUARTER, Duration::HALF)); // Rest

        track.add_measure(measure);
        song.add_track(track);

        let temp_dir = std::env::temp_dir();
        let out_path = temp_dir.join("test_tabforge_score.musicxml");

        let writer = MusicXmlWriter::default();
        writer.write(&song, &out_path).unwrap();

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("<rest/>"));
        assert!(content.contains("<bend>"));
        assert!(content.contains("<work-title>Test Song</work-title>"));

        let _ = std::fs::remove_file(out_path);
    }
}
