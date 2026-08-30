mod app;
mod commands;

use anyhow::Result;
use app::{Cli, Commands};

use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let current_dir = std::env::current_dir()?;

    let is_update = matches!(cli.command, Commands::Update { .. });
    let is_server_cmd = matches!(cli.command, Commands::Server { .. });

    // Auto-start persistent local background server on first execution of general Kitsu commands
    if !is_server_cmd {
        let _ = kitsu_core::server::ensure_server_started();
    }

    // Auto-register current repository if inside a valid Kitsu directory
    if current_dir.join(".kitsu").exists() {
        let _ = kitsu_core::global_registry::GlobalRegistry::register(&current_dir);
    }

    match cli.command {
        Commands::Ignite => commands::ignite::execute(&current_dir),
        Commands::Copy { url, directory } => commands::copy::execute(&current_dir, &url, directory),
        Commands::Track { files } => commands::track::execute(&current_dir, files),
        Commands::Freeze { message, sign } => {
            commands::freeze::execute(&current_dir, &message, sign)
        }
        Commands::Timeline => commands::timeline::execute(&current_dir),
        Commands::Diff { old, new } => commands::diff::execute(&current_dir, old, new),
        Commands::Rollback { target } => commands::rollback::execute(&current_dir, target),
        Commands::Seal {
            version,
            bump,
            list,
        } => commands::seal::execute(&current_dir, version, bump, list),
        Commands::Switch { target } => commands::switch::execute(&current_dir, &target),
        Commands::Export { target, output } => {
            commands::export::execute(&current_dir, &target, &output)
        }
        Commands::Import { input } => commands::import::execute(&current_dir, &input),
        Commands::Push {
            remote,
            target,
            branch,
        } => commands::push::execute(&current_dir, remote, target, branch),
        Commands::Pull {
            remote,
            target,
            branch,
        } => commands::pull::execute(&current_dir, remote, target, branch),
        Commands::Contents { target } => commands::contents::execute(&current_dir, target),
        Commands::Hash { file } => commands::hash::execute(&file),
        Commands::Repository { action } => commands::repository::execute(&current_dir, action),
        Commands::Persona { action } => commands::persona::execute(&current_dir, action),
        Commands::Server { action } => commands::server::execute(action),
        Commands::Burn { hash, aggressive } => {
            commands::burn::execute(&current_dir, hash, aggressive)
        }
        Commands::State => commands::state::execute(&current_dir),
        Commands::Peek { hash } => commands::peek::execute(&current_dir, &hash),
        Commands::Update { tag, check } => commands::update::execute(tag, check),
    }?;

    if !is_update
        && let Some(release) = kitsu_core::update::check_for_update(env!("CARGO_PKG_VERSION"))
    {
        use colored::*;
        eprintln!(
            "\n{}\n  A new version of Kitsu is available: {} -> {}\n  Run '{}' to update your binary.\n{}\n",
            "┌────────────────────────────────────────────────────────────┐".yellow(),
            env!("CARGO_PKG_VERSION").bright_black(),
            release.tag_name.green().bold(),
            "kitsu update".bold().cyan(),
            "└────────────────────────────────────────────────────────────┘".yellow()
        );
    }

    Ok(())
}
