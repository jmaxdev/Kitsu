use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use kitsu_core::Repository;
use kitsu_core::objects::{Checkpoint, Map};
use kitsu_core::remote::{
    GitBridge, LocalBridge, RemoteRegistry, SshTransport, default_remote_name, is_git_url,
    is_local_path,
};
use kitsu_core::storage::ObjectType;
use std::path::Path;

pub fn execute(
    current_dir: &Path,
    remote: Option<String>,
    target: Option<String>,
    branch: Option<String>,
) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let repo_dir = repo.repo_dir();
    let r_name =
        remote.unwrap_or_else(|| default_remote_name(&repo_dir).unwrap_or("origin".into()));

    let remote_entry = RemoteRegistry::get(&repo_dir, &r_name)?;
    let r_url = remote_entry.url;
    let data_branch = branch.or(remote_entry.branch);
    let t_name = target.unwrap_or_else(|| "latest".to_string());

    if is_git_url(&r_url) {
        let branch_name = data_branch.as_deref().unwrap_or("kitsu-data");
        println!(
            "Pulling from Git Registry: {} (branch: {})",
            r_url.cyan(),
            branch_name.yellow()
        );
        let result = GitBridge::pull(
            repo.storage(),
            &repo_dir,
            &r_url,
            &t_name,
            Some(branch_name),
        )?;
        if let Some(hash) = result {
            println!(
                "{} Pulled {} from Git Registry.",
                "✓".green().bold(),
                hash.yellow()
            );
        } else {
            println!("No matching seal found on remote.");
        }
    } else if is_local_path(&r_url) {
        println!("Pulling from Local Registry: {}", r_url.cyan());
        let result = LocalBridge::pull(repo.storage(), &repo_dir, &r_url, &t_name)?;
        if let Some(hash) = result {
            println!(
                "{} Pulled {} from local remote.",
                "✓".green().bold(),
                hash.yellow()
            );
        } else {
            println!("No matching seal found in local remote.");
        }
    } else {
        println!("Pulling from Sovereign Registry: {}", r_url.cyan());
        let transport = SshTransport::new(r_url.clone());
        let sess = connect_with_fallback(&transport)?;
        let r_repo = "kitsu_repo";
        let hash = transport.fetch_seal(&sess, &t_name, r_repo)?;

        let mut queue = vec![hash.clone()];
        let mut done = std::collections::HashSet::new();
        while let Some(h) = queue.pop() {
            if done.contains(&h) {
                continue;
            }
            let data = transport.fetch_object(&sess, &h, r_repo)?;
            let (t, _) = repo.storage().write_raw(&h, &data)?;
            done.insert(h.clone());
            match t {
                ObjectType::Checkpoint => {
                    queue.push(Checkpoint::deserialize(&data)?.map_hash);
                }
                ObjectType::Map => {
                    for e in Map::deserialize(&data)?.entries {
                        queue.push(e.hash);
                    }
                }
                _ => {}
            }
        }
        std::fs::write(repo_dir.join("seals").join(&t_name), format!("{}\n", hash))?;
        println!("{} Pulled from SFTP.", "✓".green().bold());
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
