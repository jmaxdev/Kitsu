use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use kitsu_core::Repository;
use kitsu_core::config::AppConfig;
use kitsu_core::objects::{Checkpoint, Map};
use kitsu_core::remote::{SshTransport, is_git_url};
use kitsu_core::storage::{ObjectType, Storage};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute(_current_dir: &Path, url: &str, directory: Option<PathBuf>) -> Result<()> {
    let dir_name = directory.unwrap_or_else(|| {
        let name = url.split('/').next_back().unwrap_or("repo");
        PathBuf::from(name.trim_end_matches(".git"))
    });
    if dir_name.exists() {
        return Err(anyhow::anyhow!("Directory {:?} already exists", dir_name));
    }

    fs::create_dir_all(&dir_name)?;
    let config = AppConfig::load();
    let r_dir = dir_name.join(&config.dir_name);
    fs::create_dir_all(r_dir.join(&config.objects_dir))?;
    fs::create_dir_all(r_dir.join(&config.streams_dir))?;
    fs::create_dir_all(r_dir.join("seals"))?;
    fs::create_dir_all(r_dir.join("remotes"))?;
    fs::write(r_dir.join(&config.current_file), "stream: main\n")?;
    fs::write(r_dir.join("remotes").join("origin"), url)?;
    fs::write(r_dir.join("default_remote"), "origin")?;

    println!("Copying from {}...", url);
    let new_storage = Storage::new(dir_name.clone(), config.clone());

    if is_git_url(url) {
        println!("Pulling from Git Registry...");
        let result = kitsu_core::remote::GitBridge::pull(&new_storage, &r_dir, url, "main")?;
        if let Some(hash) = result
            && let Ok((ObjectType::Checkpoint, cp_data)) = new_storage.read_object(&hash)
            && let Ok(cp) = Checkpoint::deserialize(&cp_data)
        {
            kitsu_core::repository::Repository::open(&dir_name)?
                .apply_map_to_disk(&cp.map_hash, &dir_name)?;
            fs::write(r_dir.join(&config.current_file), format!("{}\n", hash))?;
        }
    } else {
        let transport = SshTransport::new(url.to_string());
        let sess = connect_with_fallback(&transport)?;
        let r_repo = "kitsu_repo";
        let hash = transport
            .fetch_seal(&sess, "latest", r_repo)
            .or_else(|_| transport.fetch_seal(&sess, "main", r_repo))?;

        let mut queue = vec![hash.clone()];
        let mut done = std::collections::HashSet::new();
        while let Some(h) = queue.pop() {
            if done.contains(&h) {
                continue;
            }
            let data = transport.fetch_object(&sess, &h, r_repo)?;
            let (t, _) = new_storage.write_raw(&h, &data)?;
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
        fs::write(r_dir.join("seals").join("latest"), format!("{}\n", hash))?;
        let cp = Checkpoint::deserialize(&new_storage.read_object(&hash)?.1)?;
        let new_repo = Repository::open(&dir_name)?;
        new_repo.apply_map_to_disk(&cp.map_hash, &dir_name)?;
        fs::write(r_dir.join(&config.current_file), format!("{}\n", hash))?;
    }
    println!("Done. Project copied to {:?}", dir_name);
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
