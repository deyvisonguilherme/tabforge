use crate::cli::ModelsAction;
use anyhow::Result;
use tabforge_audio::ModelManager;

pub fn execute(action: &ModelsAction) -> Result<()> {
    match action {
        ModelsAction::List => {
            println!("\n=== TabForge Neural Separation Models ===");
            println!("Cache Directory: {}\n", ModelManager::cache_dir().display());

            let models = ModelManager::list_models();
            for (info, is_installed, path) in models {
                let status = if is_installed {
                    "✓ INSTALLED"
                } else {
                    "○ AVAILABLE"
                };

                println!("• {} [{}]", info.display_name, info.name);
                println!("  Status:      {}", status);
                println!("  SDR Quality: {:.1} dB", info.sdr_score_db);
                println!("  Stems:       {}", info.stems.join(", "));
                println!("  Size:        ~{:.1} MB", info.size_mb);
                println!("  Description: {}", info.description);
                println!("  Path:        {}", path.display());
                println!();
            }
            println!("Use `tabforge models download <name>` to fetch a model.\n");
        }
        ModelsAction::Download { name } => {
            println!("\n=== TabForge Model Downloader ===");
            println!("Requested model: {}", name);

            match ModelManager::download_model(name) {
                Ok(path) => {
                    println!("Successfully installed model to: {}", path.display());
                }
                Err(e) => {
                    eprintln!("Error downloading model: {}", e);
                }
            }
            println!("=================================\n");
        }
        ModelsAction::Path => {
            println!("{}", ModelManager::cache_dir().display());
        }
    }

    Ok(())
}
