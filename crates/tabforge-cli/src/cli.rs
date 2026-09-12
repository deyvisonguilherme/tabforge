use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tabforge", author, version, about = "Audio to MIDI, Score and Guitar Tablature Transcriber Engine", long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Path to custom configuration file")]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Initialize a new TabForge music project")]
    New {
        #[arg(help = "Project or song name")]
        name: String,
    },

    #[command(about = "Inspect an audio file (metadata, duration, sample rate, channels)")]
    Inspect {
        #[arg(help = "Path to audio file (MP3/WAV/FLAC)")]
        input: PathBuf,
    },

    #[command(about = "Transcribe an audio file into Music IR / Tablature")]
    Transcribe {
        #[arg(help = "Path to audio file (MP3/WAV/FLAC)")]
        input: PathBuf,

        #[arg(short, long, default_value = "guitar", help = "Target instrument")]
        instrument: String,

        #[arg(short, long, help = "Optional output file")]
        output: Option<PathBuf>,

        #[arg(long, help = "Enable neural / DSP source separation before transcription")]
        separate: bool,

        #[arg(long, default_value = "guitar", help = "Stem to transcribe if separation is enabled")]
        stem: String,
    },

    #[command(about = "Separate audio into individual instrument stems (guitar, bass, drums, vocals, other)")]
    Stems {
        #[arg(help = "Path to audio file (MP3/WAV/FLAC)")]
        input: PathBuf,

        #[arg(short, long, help = "Output directory for separated stem audio files")]
        output_dir: Option<PathBuf>,

        #[arg(long, help = "Optional path to custom ONNX source separation model")]
        model: Option<PathBuf>,
    },

    #[command(about = "Convert audio or Music IR to MIDI")]
    Midi {
        #[arg(help = "Input file (audio or MIDI)")]
        input: PathBuf,

        #[arg(short, long, help = "Output MIDI file path")]
        output: Option<PathBuf>,
    },

    #[command(about = "Export score to MusicXML")]
    Score {
        #[arg(help = "Input file (audio or MIDI)")]
        input: PathBuf,

        #[arg(short, long, help = "Output MusicXML file path")]
        output: Option<PathBuf>,
    },

    #[command(about = "Render guitar tablature in ASCII")]
    Tab {
        #[arg(help = "Input file (audio or MIDI)")]
        input: PathBuf,
    },

    #[command(about = "Manage neural source separation models (HTDemucs v4, Roformer)")]
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelsAction {
    #[command(about = "List available and installed neural separation models")]
    List,

    #[command(about = "Download pre-trained neural model weights (e.g. htdemucs_6s)")]
    Download {
        #[arg(default_value = "htdemucs_6s", help = "Model name (htdemucs_6s, htdemucs_4s, bs_roformer)")]
        name: String,
    },

    #[command(about = "Show local model cache directory path")]
    Path,
}
