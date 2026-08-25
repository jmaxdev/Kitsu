use anyhow::Result;
use colored::*;
use dialoguer::{Input, Select};
use kitsu_core::Repository;
use kitsu_core::remote::RemoteRegistry;
use std::path::Path;

pub fn execute(current_dir: &Path) -> Result<()> {
    println!("{}", "=== Kitsu Ignite Assistant ===".cyan().bold());
    let repo = Repository::init(current_dir)?;

    let options = vec![
        "Local / Offline repository (no remote)",
        "GitHub repository",
        "GitLab repository",
        "Custom Git remote (HTTPS / SSH URL)",
        "Local directory / Backup remote",
        "Sovereign SSH / SFTP server",
    ];

    let selection = Select::new()
        .with_prompt("Choose repository mode")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            println!(
                "{} Repository ignited in 100% local / offline mode.",
                "✓".green().bold()
            );
            println!("All version history, objects, and streams will remain on your local disk.");
            println!(
                "You can add remote registries at any time using 'kitsu repository remote add'."
            );
        }
        1 => {
            let user: String = Input::new()
                .with_prompt("GitHub username or organization")
                .interact_text()?;
            let repo_name: String = Input::new()
                .with_prompt("GitHub repository name")
                .interact_text()?;
            let branch: String = Input::new()
                .with_prompt("Remote data branch name")
                .default("kitsu-data".into())
                .interact_text()?;
            let url = format!("https://github.com/{}/{}.git", user, repo_name);
            RemoteRegistry::add(&repo.repo_dir(), "origin", &url, Some(&branch))?;
            RemoteRegistry::set_default(&repo.repo_dir(), "origin")?;
            println!(
                "{} Configured GitHub remote 'origin' -> {} (branch: {})",
                "✓".green().bold(),
                url.cyan(),
                branch.yellow()
            );
        }
        2 => {
            let user: String = Input::new()
                .with_prompt("GitLab username or group")
                .interact_text()?;
            let repo_name: String = Input::new()
                .with_prompt("GitLab repository name")
                .interact_text()?;
            let branch: String = Input::new()
                .with_prompt("Remote data branch name")
                .default("kitsu-data".into())
                .interact_text()?;
            let url = format!("https://gitlab.com/{}/{}.git", user, repo_name);
            RemoteRegistry::add(&repo.repo_dir(), "origin", &url, Some(&branch))?;
            RemoteRegistry::set_default(&repo.repo_dir(), "origin")?;
            println!(
                "{} Configured GitLab remote 'origin' -> {} (branch: {})",
                "✓".green().bold(),
                url.cyan(),
                branch.yellow()
            );
        }
        3 => {
            let url: String = Input::new()
                .with_prompt("Git remote URL (HTTPS or SSH)")
                .interact_text()?;
            let branch: String = Input::new()
                .with_prompt("Remote data branch name")
                .default("kitsu-data".into())
                .interact_text()?;
            RemoteRegistry::add(&repo.repo_dir(), "origin", &url, Some(&branch))?;
            RemoteRegistry::set_default(&repo.repo_dir(), "origin")?;
            println!(
                "{} Configured custom Git remote 'origin' -> {} (branch: {})",
                "✓".green().bold(),
                url.cyan(),
                branch.yellow()
            );
        }
        4 => {
            let local_path: String = Input::new()
                .with_prompt("Target directory or backup path")
                .interact_text()?;
            RemoteRegistry::add(&repo.repo_dir(), "backup", &local_path, None)?;
            RemoteRegistry::set_default(&repo.repo_dir(), "backup")?;
            println!(
                "{} Configured local backup remote 'backup' -> {}",
                "✓".green().bold(),
                local_path.cyan()
            );
        }
        5 => {
            let host: String = Input::new()
                .with_prompt("SSH server host")
                .interact_text()?;
            let user: String = Input::new()
                .with_prompt("SSH username")
                .default("root".into())
                .interact_text()?;
            let path: String = Input::new()
                .with_prompt("Path on remote server")
                .default("/opt/kitsu/repo".into())
                .interact_text()?;
            let url = format!("ssh://{}@{}{}", user, host, path);
            RemoteRegistry::add(&repo.repo_dir(), "origin", &url, None)?;
            RemoteRegistry::set_default(&repo.repo_dir(), "origin")?;
            println!(
                "{} Configured sovereign SSH remote 'origin' -> {}",
                "✓".green().bold(),
                url.cyan()
            );
        }
        _ => {}
    }

    println!(
        "\n{} Repository initialized successfully.",
        "SUCCESS:".green().bold()
    );
    Ok(())
}
