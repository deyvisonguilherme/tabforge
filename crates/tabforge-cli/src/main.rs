mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let args = Cli::parse();

    // Initialize structured logging
    let filter_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter_level)),
        )
        .init();

    let _config = config::Config::load_or_default(args.config.as_deref());

    match &args.command {
        Commands::New { name } => commands::new::execute(name)?,
        Commands::Inspect { input } => commands::inspect::execute(input)?,
        Commands::Transcribe {
            input,
            instrument,
            output,
            separate,
            stem,
        } => commands::transcribe::execute(input, instrument, output.as_deref(), *separate, stem)?,
        Commands::Stems {
            input,
            output_dir,
            model,
        } => commands::stems::execute(input, output_dir.as_deref(), model.as_deref())?,
        Commands::Midi { input, output } => commands::midi::execute(input, output.as_deref())?,
        Commands::Score { input, output } => commands::score::execute(input, output.as_deref())?,
        Commands::Tab { input } => commands::tab::execute(input)?,
        Commands::Models { action } => commands::models::execute(action)?,
    }

    Ok(())
}
