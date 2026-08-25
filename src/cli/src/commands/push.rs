use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use kitsu_core::Repository;
use kitsu_core::remote::{GitBridge, SshTransport, default_remote_name, is_git_url};
use std::path::Path;

pub fn execute(current_dir: &Path, remote: Option<String>, target: Option<String>) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let repo_dir = repo.repo_dir();
    let r_name =
        remote.unwrap_or_else(|| default_remote_name(&repo_dir).unwrap_or("origin".into()));
    let r_url = std::fs::read_to_string(repo_dir.join("remotes").join(&r_name))?
        .trim()
        .to_string();
    let t_name = target.unwrap_or_else(|| {
        repo.current_stream()
            .ok()
            .flatten()
            .unwrap_or_else(|| "latest".to_string())
    });
    let hash = repo.resolve_target(&t_name)?;
    let reachable = repo.collect_reachable(&hash)?;

    if is_git_url(&r_url) {
        println!("Pushing to Git Registry: {}", r_url);
        GitBridge::push(repo.storage(), &repo_dir, &r_url, &t_name, &reachable)?;
        println!("Pushed to GitHub (kitsu-data branch).");
    } else {
        println!("Pushing to Sovereign Registry: {}", r_url);
        let transport = SshTransport::new(r_url.clone());
        let sess = connect_with_fallback(&transport)?;
        let r_repo = "kitsu_repo";
        transport.ensure_remote_dir(&sess, r_repo)?;
        for h in reachable {
            let (_, data) = repo.storage().read_object(&h)?;
            transport.push_object(&sess, &h, &data, r_repo)?;
        }
        transport.push_seal(&sess, &t_name, &hash, r_repo)?;
        println!("Pushed to SFTP.");
    }
    Ok(())
}

fn connect_with_fallback(transport: &SshTransport) -> Result<ssh2::Session> {
    match transport.connect(None) {
        Ok(sess) => Ok(sess),
        Err(_) => {
            println!("{}", "SSH Key authentication failed.".yellow());
            if Confirm::new()
                .with_prompt("Try password authentication?")
                .interact()?
            {
                let pass = rpassword::prompt_password("Enter SSH Password: ")?;
                transport.connect(Some(&pass))
            } else {
                Err(anyhow::anyhow!("Authentication aborted"))
            }
        }
    }
}
