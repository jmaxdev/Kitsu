use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use kitsu_core::Repository;
use kitsu_core::objects::{Checkpoint, Map};
use kitsu_core::remote::{GitBridge, SshTransport, default_remote_name, is_git_url};
use kitsu_core::storage::ObjectType;
use std::path::Path;

pub fn execute(current_dir: &Path, remote: Option<String>, target: Option<String>) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let repo_dir = repo.repo_dir();
    let r_name =
        remote.unwrap_or_else(|| default_remote_name(&repo_dir).unwrap_or("origin".into()));
    let r_url = std::fs::read_to_string(repo_dir.join("remotes").join(&r_name))?
        .trim()
        .to_string();
    let t_name = target.unwrap_or_else(|| "latest".to_string());

    if is_git_url(&r_url) {
        println!("Pulling from Git Registry: {}", r_url);
        let result = GitBridge::pull(repo.storage(), &repo_dir, &r_url, &t_name)?;
        if let Some(hash) = result {
            println!("Pulled {} from Git Registry.", hash);
        } else {
            println!("No matching seal found on remote.");
        }
    } else {
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
        println!("Pulled from SFTP.");
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
