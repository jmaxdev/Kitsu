use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use kitsu_core::Repository;
use kitsu_core::config::AppConfig;
use kitsu_core::global_registry::GlobalRegistry;
use kitsu_core::objects::{Checkpoint, Map};
use kitsu_core::remote::{
    GitBridge, LocalBridge, RemoteRegistry, SshTransport, is_git_url, is_local_path,
};
use kitsu_core::storage::{ObjectType, Storage};
use std::fs;
use std::path::{Path, PathBuf};

/// Copies/clones a repository from a remote URL.
pub fn execute(_current_dir: &Path, url: &str, directory: Option<PathBuf>) -> Result<()> {
    let dir_name = directory.unwrap_or_else(|| {
        let name = url.replace('\\', "/");
        let last = name.split('/').rfind(|s| !s.is_empty()).unwrap_or("repo");
        PathBuf::from(last.trim_end_matches(".git"))
    });
    if dir_name.exists() {
        return Err(anyhow::anyhow!("Directory {:?} already exists", dir_name));
    }

    println!("Copying from {}...", url);

    if is_git_url(url) {
        println!("Checking Git Registry...");
        fs::create_dir_all(&dir_name)?;
        let config = AppConfig::load();
        let r_dir = dir_name.join(&config.dir_name);
        fs::create_dir_all(r_dir.join(&config.objects_dir))?;
        fs::create_dir_all(r_dir.join(&config.streams_dir))?;
        fs::create_dir_all(r_dir.join("seals"))?;
        fs::create_dir_all(r_dir.join("remotes"))?;
        fs::write(r_dir.join(&config.current_file), "stream: main\n")?;
        RemoteRegistry::add(&r_dir, "origin", url, Some("kitsu-data"))?;
        RemoteRegistry::set_default(&r_dir, "origin")?;

        let new_storage = Storage::new(dir_name.clone(), config.clone());
        let pull_res = GitBridge::pull(&new_storage, &r_dir, url, "main", None);

        let mut imported_success = false;
        if let Ok(Some(hash)) = pull_res
            && let Ok((ObjectType::Checkpoint, cp_data)) = new_storage.read_object(&hash)
            && let Ok(cp) = Checkpoint::deserialize(&cp_data)
        {
            Repository::open(&dir_name)?.apply_map_to_disk(&cp.map_hash, &dir_name)?;
            fs::write(r_dir.join(&config.current_file), format!("{}\n", hash))?;
            imported_success = true;
        }

        if !imported_success {
            // Standard GitHub/Git repository detected without kitsu-data branch -> auto clone & import
            println!(
                "{}",
                "Standard Git repository detected. Importing Git history and HEAD tree...".yellow()
            );
            let _ = fs::remove_dir_all(&dir_name);
            git2::Repository::clone(url, &dir_name)
                .map_err(|e| anyhow::anyhow!("Failed to clone Git repository: {}", e))?;
            kitsu_core::import_git_repository(&dir_name)?;
        }
    } else if is_local_path(url) {
        fs::create_dir_all(&dir_name)?;
        let config = AppConfig::load();
        let r_dir = dir_name.join(&config.dir_name);
        fs::create_dir_all(r_dir.join(&config.objects_dir))?;
        fs::create_dir_all(r_dir.join(&config.streams_dir))?;
        fs::create_dir_all(r_dir.join("seals"))?;
        fs::create_dir_all(r_dir.join("remotes"))?;
        fs::write(r_dir.join(&config.current_file), "stream: main\n")?;
        RemoteRegistry::add(&r_dir, "origin", url, None)?;
        RemoteRegistry::set_default(&r_dir, "origin")?;

        let new_storage = Storage::new(dir_name.clone(), config.clone());
        println!("Copying from local repository...");
        let result = LocalBridge::pull(&new_storage, &r_dir, url, "main")
            .or_else(|_| LocalBridge::pull(&new_storage, &r_dir, url, "latest"))?;
        if let Some(hash) = result
            && let Ok((ObjectType::Checkpoint, cp_data)) = new_storage.read_object(&hash)
            && let Ok(cp) = Checkpoint::deserialize(&cp_data)
        {
            Repository::open(&dir_name)?.apply_map_to_disk(&cp.map_hash, &dir_name)?;
            fs::write(r_dir.join(&config.current_file), format!("{}\n", hash))?;
        }
    } else {
        fs::create_dir_all(&dir_name)?;
        let config = AppConfig::load();
        let r_dir = dir_name.join(&config.dir_name);
        fs::create_dir_all(r_dir.join(&config.objects_dir))?;
        fs::create_dir_all(r_dir.join(&config.streams_dir))?;
        fs::create_dir_all(r_dir.join("seals"))?;
        fs::create_dir_all(r_dir.join("remotes"))?;
        fs::write(r_dir.join(&config.current_file), "stream: main\n")?;
        RemoteRegistry::add(&r_dir, "origin", url, None)?;
        RemoteRegistry::set_default(&r_dir, "origin")?;

        let new_storage = Storage::new(dir_name.clone(), config.clone());
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

    let _ = GlobalRegistry::register(&dir_name);
    println!("{} Project copied to {:?}", "✓".green().bold(), dir_name);
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
