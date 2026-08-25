use anyhow::Result;
use colored::*;
use kitsu_core::update::{REPO_URL, check_for_update, perform_update};

pub fn execute(tag: Option<String>, check_only: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!(
        "{} Current Kitsu version: {}",
        "ℹ".cyan(),
        current_version.bold()
    );

    if check_only {
        println!("Checking for updates on GitHub...");
        match check_for_update(current_version) {
            Some(release) => {
                println!(
                    "{} A new version is available: {} ({})",
                    "★".yellow().bold(),
                    release.tag_name.green().bold(),
                    release.html_url.cyan()
                );
                println!(
                    "Run {} to update automatically.",
                    "kitsu update".bold().yellow()
                );
            }
            None => {
                println!(
                    "{} Kitsu is up to date (version {}).",
                    "✓".green().bold(),
                    current_version
                );
            }
        }
        return Ok(());
    }

    println!("Contacting GitHub Releases ({}/releases)...", REPO_URL);
    match perform_update(current_version, tag.as_deref()) {
        Ok((release, version)) => {
            println!(
                "{} Successfully updated Kitsu to {} ({})!",
                "✓".green().bold(),
                version.to_string().green().bold(),
                release.tag_name.yellow()
            );
            println!("Release notes: {}", release.html_url.cyan().underline());
        }
        Err(e) => {
            println!("{} Update failed: {}", "✗".red().bold(), e);
            println!(
                "You can manually download the latest release from: {}/releases",
                REPO_URL
            );
        }
    }

    Ok(())
}
