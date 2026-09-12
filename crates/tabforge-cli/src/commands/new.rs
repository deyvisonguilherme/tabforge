use anyhow::Result;
use std::fs;
use std::path::Path;
use tracing::info;

pub fn execute(name: &str) -> Result<()> {
    info!("Creating new TabForge project: {}", name);
    let project_dir = Path::new(name);
    if project_dir.exists() {
        println!("Error: Directory '{}' already exists.", name);
        return Ok(());
    }

    fs::create_dir_all(project_dir.join("audio"))?;
    fs::create_dir_all(project_dir.join("output"))?;

    let default_config = r#"# TabForge Project Configuration
[song]
title = "%NAME%"
artist = "Unknown"
bpm = 120.0
time_signature = "4/4"

[guitar]
tuning = ["E2", "A2", "D3", "G3", "B3", "E4"]
frets = 24
"#.replace("%NAME%", name);

    fs::write(project_dir.join("tabforge.toml"), default_config)?;

    println!("Created new TabForge project in: ./{}", name);
    println!("Place your audio in ./{}/audio and run `tabforge transcribe`", name);

    Ok(())
}
