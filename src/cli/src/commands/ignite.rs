use anyhow::Result;
use colored::*;
use dialoguer::{Confirm, Input, Select};
use kitsu_core::Repository;
use kitsu_core::remote::RemoteRegistry;
use std::path::Path;

pub fn execute(current_dir: &Path) -> Result<()> {
    println!("{}", "--- Kitsu Ignite Assistant ---".cyan().bold());
    let repo = Repository::init(current_dir)?;

    if Confirm::new()
        .with_prompt("Configure a remote registry now?")
        .interact()?
    {
        let types = vec!["GitHub / GitLab", "Custom SSH Server"];
        let selection = Select::new()
            .with_prompt("Select registry type")
            .items(&types)
            .default(0)
            .interact()?;
        let url: String = if selection == 0 {
            let user: String = Input::new()
                .with_prompt("GitHub/GitLab username")
                .interact_text()?;
            let repo_name: String = Input::new()
                .with_prompt("Repository name")
                .interact_text()?;
            format!("https://github.com/{}/{}.git", user, repo_name)
        } else {
            let host: String = Input::new().with_prompt("Server Host").interact_text()?;
            let user: String = Input::new()
                .with_prompt("User")
                .default("root".into())
                .interact_text()?;
            let path: String = Input::new()
                .with_prompt("Path on server")
                .default("/opt/kitsu/repo".into())
                .interact_text()?;
            format!("ssh://{}@{}{}", user, host, path)
        };
        RemoteRegistry::add(&repo.repo_dir(), "origin", &url)?;
        RemoteRegistry::set_default(&repo.repo_dir(), "origin")?;
        println!(
            "{} Remote 'origin' configured: {}",
            "SUCCESS".green().bold(),
            url
        );
    }
    println!("Repository ignited successfully.");
    Ok(())
}
