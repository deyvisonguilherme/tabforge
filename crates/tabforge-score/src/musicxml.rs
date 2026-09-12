use crate::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tabforge_core::Song;

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
                        xml.push_str("        <duration>4</duration>\n");
                        xml.push_str("        <type>quarter</type>\n");
                        xml.push_str("      </note>\n");
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
