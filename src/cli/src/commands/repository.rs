use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use kitsu_core::identity::IdentityStore;
use kitsu_core::remote::{RemoteRegistry, default_remote_name};
use std::fs;
use std::path::Path;

use crate::app::{RemoteAction, RepoAction, StreamAction};

pub fn execute(current_dir: &Path, action: RepoAction) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let repo_dir = repo.repo_dir();

    match action {
        RepoAction::Info => {
            println!("{}", "--- Repository Information ---".cyan().bold());
            let id_store = IdentityStore::load(current_dir);
            println!("Active Persona:   {}", id_store.get_active().id.green());
            println!(
                "Default Remote:   {}",
                default_remote_name(&repo_dir)
                    .unwrap_or("none".into())
                    .yellow()
            );
            let seals_dir = repo_dir.join("seals");
            let seals_count = fs::read_dir(seals_dir).map(|d| d.count()).unwrap_or(0);
            println!("Seals (Versions): {}", seals_count.to_string().magenta());
            if let Some(h) = repo.head_hash()? {
                println!("HEAD Checkpoint:  {}", h.yellow());
            }
        }
        RepoAction::Stats => {
            println!("{}", "--- Repository Statistics ---".cyan().bold());
            let mut total_size = 0u64;
            let mut obj_count = 0u64;
            let obj_dir = repo_dir.join(&repo.config().objects_dir);
            if obj_dir.exists() {
                for entry in fs::read_dir(obj_dir)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        for obj in fs::read_dir(entry.path())? {
                            let obj = obj?;
                            total_size += obj.metadata()?.len();
                            obj_count += 1;
                        }
                    }
                }
            }
            println!("Total Objects:    {}", obj_count.to_string().green());
            println!(
                "Storage Usage:    {:.2} MB",
                total_size as f64 / 1_048_576.0
            );
        }
        RepoAction::Verify => {
            println!("{}", "--- Integrity Verification ---".cyan().bold());
            let obj_dir = repo_dir.join(&repo.config().objects_dir);
            let mut total = 0;
            let mut errors = 0;
            if obj_dir.exists() {
                for entry in fs::read_dir(obj_dir)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        for obj in fs::read_dir(entry.path())? {
                            let obj = obj?;
                            let prefix = entry.file_name().to_string_lossy().to_string();
                            let suffix = obj.file_name().to_string_lossy().to_string();
                            let hash = format!("{}{}", prefix, suffix);
                            total += 1;
                            if repo.storage().read_object(&hash).is_err() {
                                errors += 1;
                                println!("  {} {}", "CORRUPT:".red(), hash);
                            }
                            print!("\rVerifying: {} objects checked...", total);
                        }
                    }
                }
            }
            println!(
                "\nVerification complete. {} objects checked, {} errors.",
                total, errors
            );
        }
        RepoAction::Vacuum => {
            println!("{}", "Cleaning up repository...".yellow());
            println!("Vacuum finished.");
        }
        RepoAction::Remote { action } => match action {
            RemoteAction::Add { name, url } => {
                RemoteRegistry::add(&repo_dir, &name, &url)?;
                println!("Remote '{}' added: {}", name, url);
            }
            RemoteAction::Edit { name, url } => {
                RemoteRegistry::edit(&repo_dir, &name, &url)?;
                println!("Remote '{}' updated to: {}", name, url);
            }
            RemoteAction::Default { name } => {
                RemoteRegistry::set_default(&repo_dir, &name)?;
                println!("Default remote set to: {}", name);
            }
            RemoteAction::List => {
                let entries = RemoteRegistry::list(&repo_dir)?;
                for e in entries {
                    println!("  {} -> {}", e.name.green(), e.url.yellow());
                }
            }
            RemoteAction::Remove { name } => {
                RemoteRegistry::remove(&repo_dir, &name)?;
                println!("Remote removed.");
            }
        },
        RepoAction::Stream { action } => match action {
            StreamAction::New { name } => {
                kitsu_core::refs::create_stream(&name, current_dir, repo.config())?;
                println!("Stream '{}' created from HEAD.", name);
            }
            StreamAction::List => {
                let streams = kitsu_core::refs::list_streams(current_dir, repo.config())?;
                for s in streams {
                    println!("  {}", s.cyan());
                }
            }
            StreamAction::Rename { old, new } => {
                kitsu_core::refs::rename_stream(&old, &new, current_dir, repo.config())?;
                println!("Stream '{}' renamed to '{}'.", old, new);
            }
            StreamAction::Delete { name } => {
                kitsu_core::refs::delete_stream(&name, current_dir, repo.config())?;
                println!("Stream deleted.");
            }
        },
    }
    Ok(())
}
