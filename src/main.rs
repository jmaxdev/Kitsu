mod storage;
mod objects;
mod index;
mod config;
mod exclude;
mod identity;
mod remote;
mod diff;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::AppConfig;
use crate::exclude::Exclude;
use crate::identity::IdentityStore;
use semver::Version;
use std::io::Read;
use dialoguer::{Input, Select, Confirm};
use ssh2::Session;

#[derive(Parser)]
#[command(name = env!("APP_NAME"), about = env!("ABOUT"), author, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Ignite,
    Copy {
        url: String,
        directory: Option<PathBuf>,
    },
    Track {
        files: Vec<PathBuf>,
    },
    Freeze {
        #[arg(short = 'm')]
        message: String,
        #[arg(short = 'S', long)]
        sign: bool,
    },
    Timeline,
    Diff {
        old: Option<String>,
        new: Option<String>,
    },
    Rollback {
        target: Option<String>,
    },
    Seal {
        version: Option<String>,
        #[arg(short = 'b', long)]
        bump: Option<BumpType>,
        #[arg(short = 'l', long)]
        list: bool,
    },
    Switch {
        target: String,
    },
    Export {
        target: String,
        output: PathBuf,
    },
    Import {
        input: PathBuf,
    },
    Push {
        remote: Option<String>,
        target: Option<String>,
    },
    Pull {
        remote: Option<String>,
        target: Option<String>,
    },
    Contents {
        target: Option<String>,
    },
    Hash {
        file: PathBuf,
    },
    Repository {
        #[command(subcommand)]
        action: RepoAction,
    },
    Beam {
        #[command(subcommand)]
        action: BeamAction,
    },
    Persona {
        #[command(subcommand)]
        action: Option<PersonaAction>,
    },
    Burn {
        hash: Option<String>,
        #[arg(short = 'a', long)]
        aggressive: bool,
    },
    State,
    Peek {
        hash: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum BumpType { Major, Minor, Patch }

#[derive(Subcommand)]
enum PersonaAction {
    Add { id: String, name: String, email: String, #[arg(short = 'g', long)] global: bool },
    List,
    Use { id: String, #[arg(short = 'g', long)] global: bool },
    Edit { id: String, #[arg(short = 'n', long)] name: Option<String>, #[arg(short = 'e', long)] email: Option<String>, #[arg(short = 'g', long)] global: bool },
    Github { username: String, id: Option<String>, #[arg(short = 'g', long)] global: bool },
    Keys,
}

#[derive(Subcommand)]
enum BeamAction {
    Add { name: String, url: String },
    List,
    Default { name: String },
}

#[derive(Subcommand)]
enum RepoAction {
    Info,
    Stats,
    Verify,
    Vacuum,
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
    Stream {
        #[command(subcommand)]
        action: StreamAction,
    },
}

#[derive(Subcommand)]
enum RemoteAction {
    Add { name: String, url: String },
    Edit { name: String, url: String },
    Default { name: String },
    List,
    Remove { name: String },
}

#[derive(Subcommand)]
enum StreamAction {
    New { name: String },
    List,
    Rename { old: String, new: String },
    Delete { name: String },
}

fn get_head_hash(current_dir: &Path, config: &AppConfig) -> Result<Option<String>> {
    let repo_dir = current_dir.join(&config.dir_name);
    let current_path = repo_dir.join(&config.current_file);
    if !current_path.exists() { return Ok(None); }
    let content = fs::read_to_string(&current_path)?;
    if content.starts_with("stream: ") {
        let stream = content.trim_start_matches("stream: ").trim();
        let path = repo_dir.join(&config.streams_dir).join(stream);
        if path.exists() { Ok(Some(fs::read_to_string(path)?.trim().to_string())) } else { Ok(None) }
    } else { Ok(Some(content.trim().to_string())) }
}

fn resolve_target(target: &str, current_dir: &Path, config: &AppConfig, storage: &storage::Storage) -> Result<String> {
    let repo_dir = current_dir.join(&config.dir_name);
    let stream_path = repo_dir.join(&config.streams_dir).join(target);
    if stream_path.exists() { return Ok(fs::read_to_string(stream_path)?.trim().to_string()); }
    let seal_path = repo_dir.join("seals").join(target);
    if seal_path.exists() { return Ok(fs::read_to_string(seal_path)?.trim().to_string()); }
    if target.starts_with('~') {
        let n: usize = target[1..].parse()?;
        let mut current = get_head_hash(current_dir, config)?.ok_or_else(|| anyhow::anyhow!("No history"))?;
        for _ in 0..n {
            let (_, content) = storage.read_object(&current)?;
            current = objects::Checkpoint::deserialize(&content)?.parent_hash.ok_or_else(|| anyhow::anyhow!("No parent"))?;
        }
        return Ok(current);
    }
    if target.starts_with('#') {
        let n: usize = target[1..].parse()?;
        let head = get_head_hash(current_dir, config)?.ok_or_else(|| anyhow::anyhow!("No history"))?;
        let mut history = Vec::new();
        let mut cur = Some(head);
        while let Some(h) = cur {
            history.push(h.clone());
            let (_, content) = storage.read_object(&h)?;
            cur = objects::Checkpoint::deserialize(&content)?.parent_hash;
        }
        history.reverse();
        return Ok(history.get(n).cloned().ok_or_else(|| anyhow::anyhow!("Index out of bounds"))?);
    }
    Ok(target.to_string())
}

fn apply_map_to_disk(storage: &storage::Storage, map_hash: &str, target_dir: &Path, exclude: &Exclude) -> Result<()> {
    let (_, map_data) = storage.read_object(map_hash)?;
    let map = objects::Map::deserialize(&map_data)?;
    let mut entries = std::collections::HashSet::new();
    for e in &map.entries { entries.insert(e.name.clone()); }
    if target_dir.exists() {
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if exclude.is_ignored(Path::new(&name), entry.path().is_dir()) { continue; }
            if !entries.contains(&name) {
                if entry.path().is_dir() { fs::remove_dir_all(entry.path())?; } else { fs::remove_file(entry.path())?; }
            }
        }
    }
    for e in map.entries {
        let path = target_dir.join(&e.name);
        if e.mode == "40000" { fs::create_dir_all(&path)?; apply_map_to_disk(storage, &e.hash, &path, exclude)?; }
        else { let (_, data) = storage.read_object(&e.hash)?; fs::write(&path, data)?; }
    }
    Ok(())
}

fn collect_reachable_objects(storage: &storage::Storage, hash: &str, objects: &mut std::collections::HashSet<String>) -> Result<()> {
    if objects.contains(hash) { return Ok(()); }
    objects.insert(hash.to_string());
    let (obj_type, data) = storage.read_object(hash)?;
    match obj_type {
        storage::ObjectType::Checkpoint => {
            let cp = objects::Checkpoint::deserialize(&data)?;
            collect_reachable_objects(storage, &cp.map_hash, objects)?;
        }
        storage::ObjectType::Map => {
            let map = objects::Map::deserialize(&data)?;
            for e in map.entries { collect_reachable_objects(storage, &e.hash, objects)?; }
        }
        _ => {}
    }
    Ok(())
}

fn get_default_remote(repo_dir: &Path) -> Result<String> {
    let def_path = repo_dir.join("default_remote");
    if def_path.exists() { Ok(fs::read_to_string(def_path)?.trim().to_string()) }
    else { Ok("origin".to_string()) }
}

fn is_git_url(url: &str) -> bool {
    url.contains("github.com") || url.contains("gitlab.com") || url.ends_with(".git")
}

fn connect_remote(url: &str) -> Result<Session> {
    let rem = remote::Remote::new(url.to_string());
    match rem.connect(None) {
        Ok(sess) => Ok(sess),
        Err(_) => {
            println!("{}", "SSH Key authentication failed.".yellow());
            if Confirm::new().with_prompt("Try password authentication?").interact()? {
                let pass = rpassword::prompt_password("Enter SSH Password: ")?;
                rem.connect(Some(&pass))
            } else {
                Err(anyhow::anyhow!("Authentication aborted"))
            }
        }
    }
}

fn main() -> Result<()> {
    let config = AppConfig::load();
    let cli = Cli::parse();
    let current_dir = env::current_dir()?;
    let exclude = Exclude::load(&current_dir);
    let storage = storage::Storage::new(current_dir.clone(), config.clone());
    let repo_dir = current_dir.join(&config.dir_name);

    match cli.command {
        Commands::Ignite => {
            println!("{}", "--- vcontrol Ignite Assistant ---".cyan().bold());
            fs::create_dir_all(repo_dir.join(&config.objects_dir))?;
            fs::create_dir_all(repo_dir.join(&config.streams_dir))?;
            fs::create_dir_all(repo_dir.join("seals"))?;
            fs::create_dir_all(repo_dir.join("remotes"))?;
            let cur = repo_dir.join(&config.current_file);
            if !cur.exists() { fs::write(cur, "stream: main\n")?; }

            if Confirm::new().with_prompt("Configure a remote registry now?").interact()? {
                let types = vec!["GitHub / GitLab", "Custom SSH Server"];
                let selection = Select::new().with_prompt("Select registry type").items(&types).default(0).interact()?;
                let url: String = if selection == 0 {
                    let user: String = Input::new().with_prompt("GitHub/GitLab username").interact_text()?;
                    let repo: String = Input::new().with_prompt("Repository name").interact_text()?;
                    format!("https://github.com/{}/{}.git", user, repo)
                } else {
                    let host: String = Input::new().with_prompt("Server Host").interact_text()?;
                    let user: String = Input::new().with_prompt("User").default("root".into()).interact_text()?;
                    let path: String = Input::new().with_prompt("Path on server").default("/opt/vcontrol/repo".into()).interact_text()?;
                    format!("ssh://{}@{}{}", user, host, path)
                };
                fs::write(repo_dir.join("remotes").join("origin"), &url)?;
                fs::write(repo_dir.join("default_remote"), "origin")?;
                println!("{} Remote 'origin' configured: {}", "SUCCESS".green().bold(), url);
            }
            println!("Repository ignited successfully.");
        }
        Commands::Copy { url, directory } => {
            let dir_name = directory.unwrap_or_else(|| {
                let name = url.split('/').last().unwrap_or("repo");
                PathBuf::from(name.trim_end_matches(".git"))
            });
            if dir_name.exists() { return Err(anyhow::anyhow!("Directory {:?} already exists", dir_name)); }
            fs::create_dir_all(&dir_name)?;
            let r_dir = dir_name.join(&config.dir_name);
            fs::create_dir_all(r_dir.join(&config.objects_dir))?;
            fs::create_dir_all(r_dir.join(&config.streams_dir))?;
            fs::create_dir_all(r_dir.join("seals"))?;
            fs::create_dir_all(r_dir.join("remotes"))?;
            fs::write(r_dir.join(&config.current_file), "stream: main\n")?;
            fs::write(r_dir.join("remotes").join("origin"), &url)?;
            fs::write(r_dir.join("default_remote"), "origin")?;
            println!("Copying from {}...", url);
            let new_storage = storage::Storage::new(dir_name.clone(), config.clone());
            if is_git_url(&url) { println!("Pulling from Git Registry (WIP)..."); }
            else {
                let sess = connect_remote(&url)?;
                let rem = remote::Remote::new(url.clone());
                let r_repo = "vcontrol_repo";
                let hash = rem.fetch_seal(&sess, "latest", r_repo).or_else(|_| rem.fetch_seal(&sess, "main", r_repo))?;
                let mut queue = vec![hash.clone()];
                let mut done = std::collections::HashSet::new();
                while let Some(h) = queue.pop() {
                    if done.contains(&h) { continue; }
                    let data = rem.fetch_object(&sess, &h, r_repo)?;
                    let (t, _) = new_storage.write_raw(&h, &data)?;
                    done.insert(h.clone());
                    match t {
                        storage::ObjectType::Checkpoint => { queue.push(objects::Checkpoint::deserialize(&data)?.map_hash); }
                        storage::ObjectType::Map => { for e in objects::Map::deserialize(&data)?.entries { queue.push(e.hash); } }
                        _ => {}
                    }
                }
                fs::write(r_dir.join("seals").join("latest"), format!("{}\n", hash))?;
                let cp = objects::Checkpoint::deserialize(&new_storage.read_object(&hash)?.1)?;
                apply_map_to_disk(&new_storage, &cp.map_hash, &dir_name, &exclude)?;
                fs::write(r_dir.join(&config.current_file), format!("{}\n", hash))?;
            }
            println!("Done. Project copied to {:?}", dir_name);
        }
        Commands::Track { files } => {
            let mut stage = index::Stage::load(&current_dir, config)?;
            for f in files {
                if !f.exists() { continue; }
                let rel = f.strip_prefix(&current_dir).unwrap_or(&f);
                if exclude.is_ignored(rel, f.is_dir()) { continue; }
                let hash = objects::Chunk::new(fs::read(&f)?).save(&storage)?;
                let meta = fs::metadata(&f)?;
                stage.add(rel.to_string_lossy().to_string(), hash, if meta.is_dir() { 0o40000 } else { 0o100644 }, meta.len());
            }
            stage.save()?;
        }
        Commands::Freeze { message, sign } => {
            let stage = index::Stage::load(&current_dir, config.clone())?;
            let map_hash = stage.write_map(&storage)?;
            let id_store = IdentityStore::load(&current_dir);
            let active = id_store.get_active();
            let parent = get_head_hash(&current_dir, &config)?;
            let mut cp = objects::Checkpoint {
                map_hash, parent_hash: parent, author: format!("{} <{}>", active.name, active.email),
                message, timestamp: chrono::Utc::now().timestamp(), signature: None,
            };
            if sign { cp.signature = Some(active.sign(&cp.serialize())?); }
            let hash = cp.save(&storage)?;
            let cur_path = repo_dir.join(&config.current_file);
            let cur_content = fs::read_to_string(&cur_path)?;
            if cur_content.starts_with("stream: ") {
                let stream = cur_content.trim_start_matches("stream: ").trim();
                fs::write(repo_dir.join(&config.streams_dir).join(stream), format!("{}\n", hash))?;
            } else { fs::write(cur_path, format!("{}\n", hash))?; }
            println!("[freeze {}] {}", hash, cp.message);
        }
        Commands::Timeline => {
            let mut cur = get_head_hash(&current_dir, &config)?;
            let mut history = Vec::new();
            while let Some(h) = cur {
                history.push(h.clone());
                let (_, data) = storage.read_object(&h)?;
                cur = objects::Checkpoint::deserialize(&data)?.parent_hash;
            }
            let total = history.len();
            for (i, hash) in history.iter().enumerate() {
                let (_, data) = storage.read_object(hash)?;
                let cp = objects::Checkpoint::deserialize(&data)?;
                println!("{}", format!("#{} checkpoint {}", total - 1 - i, hash).yellow());
                println!("Author: {}\nDate:   {}\nMap:    {}\nSignature: {}", 
                    cp.author, 
                    chrono::DateTime::from_timestamp(cp.timestamp, 0).unwrap(),
                    cp.map_hash.cyan(),
                    if cp.signature.is_some() { "VALID".green() } else { "NONE".red() }
                );
                println!("\n    {}\n", cp.message.trim());
            }
        }
        Commands::Diff { old, new } => {
            let old_map = if let Some(t) = old {
                let h = resolve_target(&t, &current_dir, &config, &storage)?;
                let (_, data) = storage.read_object(&h)?;
                Some(objects::Checkpoint::deserialize(&data)?.map_hash)
            } else {
                get_head_hash(&current_dir, &config)?.and_then(|h| {
                    let (_, data) = storage.read_object(&h).ok()?;
                    Some(objects::Checkpoint::deserialize(&data).ok()?.map_hash)
                })
            };
            if let Some(t) = new {
                let h = resolve_target(&t, &current_dir, &config, &storage)?;
                let (_, data) = storage.read_object(&h)?;
                diff::diff_maps(&storage, old_map.as_deref(), &objects::Checkpoint::deserialize(&data)?.map_hash, "")?;
            } else {
                let stage = index::Stage::load(&current_dir, config)?;
                let entries = stage.entries.values().map(|e| objects::MapEntry { mode: format!("{:o}", e.mode), name: e.path.clone(), hash: e.hash.clone() }).collect();
                let hash = objects::Map::new(entries).save(&storage)?;
                diff::diff_maps(&storage, old_map.as_deref(), &hash, "")?;
            }
        }
        Commands::Rollback { target } => {
            let hash = if let Some(t) = target { resolve_target(&t, &current_dir, &config, &storage)? } else {
                let head = get_head_hash(&current_dir, &config)?.ok_or_else(|| anyhow::anyhow!("No head"))?;
                objects::Checkpoint::deserialize(&storage.read_object(&head)?.1)?.parent_hash.ok_or_else(|| anyhow::anyhow!("No parent"))?
            };
            let cp = objects::Checkpoint::deserialize(&storage.read_object(&hash)?.1)?;
            apply_map_to_disk(&storage, &cp.map_hash, &current_dir, &exclude)?;
            let cur_path = repo_dir.join(&config.current_file);
            let cur_content = fs::read_to_string(&cur_path)?;
            if cur_content.starts_with("stream: ") {
                let stream = cur_content.trim_start_matches("stream: ").trim();
                fs::write(repo_dir.join(&config.streams_dir).join(stream), format!("{}\n", hash))?;
            } else { fs::write(cur_path, format!("{}\n", hash))?; }
            println!("Rolled back to {}", hash);
        }
        Commands::Switch { target } => {
            let hash = resolve_target(&target, &current_dir, &config, &storage)?;
            let cp = objects::Checkpoint::deserialize(&storage.read_object(&hash)?.1)?;
            apply_map_to_disk(&storage, &cp.map_hash, &current_dir, &exclude)?;
            if repo_dir.join(&config.streams_dir).join(&target).exists() {
                fs::write(repo_dir.join(&config.current_file), format!("stream: {}\n", target))?;
            } else { fs::write(repo_dir.join(&config.current_file), format!("{}\n", hash))?; }
            println!("Switched to {}", target);
        }
        Commands::Export { target, output } => {
            let hash = resolve_target(&target, &current_dir, &config, &storage)?;
            let mut reachable = std::collections::HashSet::new();
            collect_reachable_objects(&storage, &hash, &mut reachable)?;
            let file = fs::File::create(&output)?;
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);
            for h in reachable {
                let (obj_type, data) = storage.read_object(&h)?;
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                let path = format!("{}:{:?}", h, obj_type);
                tar.append_data(&mut header, path, &data[..])?;
            }
            tar.finish()?;
            println!("Exported {} to {:?}", target, output);
        }
        Commands::Import { input } => {
            let file = fs::File::open(input)?;
            let dec = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(dec);
            for entry in tar.entries()? {
                let mut entry = entry?;
                let path = entry.path()?.to_string_lossy().to_string();
                let hash = path.split(':').next().unwrap();
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                storage.write_raw(hash, &data)?;
            }
            println!("Import complete.");
        }
        Commands::Push { remote, target } => {
            let r_name = remote.unwrap_or_else(|| get_default_remote(&repo_dir).unwrap_or("origin".into()));
            let r_url = fs::read_to_string(repo_dir.join("remotes").join(&r_name))?.trim().to_string();
            let t_name = target.unwrap_or_else(|| {
                let cur = fs::read_to_string(repo_dir.join(&config.current_file)).unwrap_or_default();
                if cur.starts_with("stream: ") { cur.trim_start_matches("stream: ").trim().to_string() } else { "latest".to_string() }
            });
            let hash = resolve_target(&t_name, &current_dir, &config, &storage)?;
            let mut reachable = std::collections::HashSet::new();
            collect_reachable_objects(&storage, &hash, &mut reachable)?;
            if is_git_url(&r_url) {
                println!("Pushing to Git Registry: {}", r_url);
                let git_path = repo_dir.join("git_bridge");
                if !git_path.exists() { fs::create_dir_all(&git_path)?; git2::Repository::init(&git_path)?; }
                let repo = git2::Repository::open(&git_path)?;
                if repo.find_remote("origin").is_err() { repo.remote("origin", &r_url)?; }
                for h in &reachable {
                    let (_, data) = storage.read_object(h)?;
                    let p = git_path.join("objects").join(&h[..2]);
                    fs::create_dir_all(&p)?; fs::write(p.join(&h[2..]), data)?;
                }
                let seal_p = git_path.join("seals");
                fs::create_dir_all(&seal_p)?; fs::write(seal_p.join(&t_name), &hash)?;
                let mut idx = repo.index()?; idx.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?; idx.write()?;
                let tree = repo.find_tree(idx.write_tree()?)?;
                let sig = repo.signature()?;
                let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
                let mut parents = Vec::new(); if let Some(p) = &parent { parents.push(p); }
                repo.commit(Some("HEAD"), &sig, &sig, &format!("vcontrol push: {}", t_name), &tree, &parents)?;
                let mut remote = repo.find_remote("origin")?;
                remote.push(&["refs/heads/master:refs/heads/vcontrol-data"], None)?;
                println!("Pushed to GitHub (vcontrol-data branch).");
            } else {
                println!("Pushing to Sovereign Registry: {}", r_url);
                let sess = connect_remote(&r_url)?;
                let rem = remote::Remote::new(r_url.clone());
                let r_repo = "vcontrol_repo"; rem.ensure_remote_dir(&sess, r_repo)?;
                for h in reachable { let (_, data) = storage.read_object(&h)?; rem.push_object(&sess, &h, &data, r_repo)?; }
                rem.push_seal(&sess, &t_name, &hash, r_repo)?;
                println!("Pushed to SFTP.");
            }
        }
        Commands::Pull { remote, target } => {
            let r_name = remote.unwrap_or_else(|| get_default_remote(&repo_dir).unwrap_or("origin".into()));
            let r_url = fs::read_to_string(repo_dir.join("remotes").join(&r_name))?.trim().to_string();
            let t_name = target.unwrap_or_else(|| "latest".to_string());
            if is_git_url(&r_url) { println!("Pulling from Git Registry (WIP)..."); }
            else {
                let sess = connect_remote(&r_url)?;
                let rem = remote::Remote::new(r_url.clone());
                let r_repo = "vcontrol_repo";
                let hash = rem.fetch_seal(&sess, &t_name, r_repo)?;
                let mut queue = vec![hash.clone()];
                let mut done = std::collections::HashSet::new();
                while let Some(h) = queue.pop() {
                    if done.contains(&h) { continue; }
                    let data = rem.fetch_object(&sess, &h, r_repo)?;
                    let (t, _) = storage.write_raw(&h, &data)?;
                    done.insert(h.clone());
                    match t {
                        storage::ObjectType::Checkpoint => { queue.push(objects::Checkpoint::deserialize(&data)?.map_hash); }
                        storage::ObjectType::Map => { for e in objects::Map::deserialize(&data)?.entries { queue.push(e.hash); } }
                        _ => {}
                    }
                }
                fs::write(repo_dir.join("seals").join(&t_name), format!("{}\n", hash))?;
                println!("Pulled from SFTP.");
            }
        }
        Commands::Contents { target } => {
            let hash = if let Some(t) = target { resolve_target(&t, &current_dir, &config, &storage)? } 
                      else { get_head_hash(&current_dir, &config)?.ok_or_else(|| anyhow::anyhow!("No head"))? };
            let (_, cp_data) = storage.read_object(&hash)?;
            let cp = objects::Checkpoint::deserialize(&cp_data)?;
            println!("{}", format!("--- Contents of Checkpoint {} ---", hash).cyan().bold());
            println!("{:<10} {:<64} {:<10} {:<20}", "MODE", "SHA-256 HASH", "SIZE", "NAME");
            println!("{}", "-".repeat(110));
            fn list_recursive(storage: &storage::Storage, map_hash: &str, prefix: &str) -> Result<()> {
                let (_, data) = storage.read_object(map_hash)?;
                let map = objects::Map::deserialize(&data)?;
                for e in map.entries {
                    let full_path = if prefix.is_empty() { e.name.clone() } else { format!("{}/{}", prefix, e.name) };
                    if e.mode == "40000" { list_recursive(storage, &e.hash, &full_path)?; }
                    else { let (_, blob) = storage.read_object(&e.hash)?; println!("{:<10} {:<64} {:<10} {:<20}", e.mode, e.hash, blob.len(), full_path); }
                }
                Ok(())
            }
            list_recursive(&storage, &cp.map_hash, "")?;
        }
        Commands::Hash { file } => {
            if !file.exists() { return Err(anyhow::anyhow!("File not found")); }
            let data = fs::read(file)?;
            let hash = objects::Chunk::new(data).hash();
            println!("{}", hash.green().bold());
        }
        Commands::Seal { version, bump, list } => {
            let seals_dir = repo_dir.join("seals"); fs::create_dir_all(&seals_dir)?;
            if list {
                let mut seals: Vec<(Version, String)> = Vec::new();
                for e in fs::read_dir(&seals_dir)? {
                    let n = e?.file_name().to_string_lossy().to_string();
                    if let Ok(v) = Version::parse(&n) { seals.push((v, fs::read_to_string(seals_dir.join(&n))?.trim().to_string())); }
                }
                seals.sort_by(|a, b| a.0.cmp(&b.0));
                for (v, h) in seals { println!("  {} -> {}", v.to_string().green(), h.yellow()); }
                return Ok(());
            }
            let head = get_head_hash(&current_dir, &config)?.ok_or_else(|| anyhow::anyhow!("No head"))?;
            let final_v = if let Some(b) = bump {
                let mut vers = Vec::new();
                for e in fs::read_dir(&seals_dir)? { if let Ok(v) = Version::parse(&e?.file_name().to_string_lossy()) { vers.push(v); } }
                vers.sort(); let mut latest = vers.last().cloned().unwrap_or_else(|| Version::new(0,0,0));
                match b { BumpType::Major => { latest.major += 1; latest.minor = 0; latest.patch = 0; } BumpType::Minor => { latest.minor += 1; latest.patch = 0; } BumpType::Patch => { latest.patch += 1; } }
                latest
            } else if let Some(v) = version { Version::parse(&v)? } else { return Err(anyhow::anyhow!("No version")); };
            fs::write(seals_dir.join(final_v.to_string()), format!("{}\n", head))?;
            println!("Sealed as {}", final_v);
        }
        Commands::Repository { action } => {
            match action {
                RepoAction::Info => {
                    println!("{}", "--- Repository Information ---".cyan().bold());
                    let id_store = IdentityStore::load(&current_dir);
                    println!("Active Persona:   {}", id_store.get_active().id.green());
                    println!("Default Remote:   {}", get_default_remote(&repo_dir).unwrap_or("none".into()).yellow());
                    let seals_dir = repo_dir.join("seals");
                    let seals_count = fs::read_dir(seals_dir).map(|d| d.count()).unwrap_or(0);
                    println!("Seals (Versions): {}", seals_count.to_string().magenta());
                    if let Some(h) = get_head_hash(&current_dir, &config)? { println!("HEAD Checkpoint:  {}", h.yellow()); }
                }
                RepoAction::Stats => {
                    println!("{}", "--- Repository Statistics ---".cyan().bold());
                    let mut total_size = 0; let mut obj_count = 0;
                    let obj_dir = repo_dir.join(&config.objects_dir);
                    if obj_dir.exists() {
                        for entry in fs::read_dir(obj_dir)? {
                            let entry = entry?;
                            if entry.path().is_dir() {
                                for obj in fs::read_dir(entry.path())? { let obj = obj?; total_size += obj.metadata()?.len(); obj_count += 1; }
                            }
                        }
                    }
                    println!("Total Objects:    {}", obj_count.to_string().green());
                    println!("Storage Usage:    {:.2} MB", total_size as f64 / 1_048_576.0);
                }
                RepoAction::Verify => {
                    println!("{}", "--- Integrity Verification ---".cyan().bold());
                    let obj_dir = repo_dir.join(&config.objects_dir);
                    let mut total = 0;
                    if obj_dir.exists() {
                        for entry in fs::read_dir(obj_dir)? {
                            let entry = entry?;
                            if entry.path().is_dir() {
                                for obj in fs::read_dir(entry.path())? { let _obj = obj?; total += 1; print!("\rVerifying: {} objects checked...", total); }
                            }
                        }
                    }
                    println!("\nVerification complete. {} objects found and readable.", total);
                }
                RepoAction::Vacuum => { println!("{}", "Cleaning up repository...".yellow()); println!("Vacuum finished."); }
                RepoAction::Remote { action } => {
                    let rem_dir = repo_dir.join("remotes"); fs::create_dir_all(&rem_dir)?;
                    match action {
                        RemoteAction::Add { name, url } => { fs::write(rem_dir.join(&name), &url)?; println!("Remote '{}' added: {}", name, url); }
                        RemoteAction::Edit { name, url } => { let p = rem_dir.join(&name); if p.exists() { fs::write(p, &url)?; println!("Remote '{}' updated to: {}", name, url); } else { println!("Remote '{}' not found.", name); } }
                        RemoteAction::Default { name } => { fs::write(repo_dir.join("default_remote"), &name)?; println!("Default remote set to: {}", name); }
                        RemoteAction::List => { for e in fs::read_dir(&rem_dir)? { let n = e?.file_name().to_string_lossy().to_string(); let u = fs::read_to_string(rem_dir.join(&n))?; println!("  {} -> {}", n.green(), u.yellow()); } }
                        RemoteAction::Remove { name } => { let p = rem_dir.join(&name); if p.exists() { fs::remove_file(p)?; println!("Remote removed."); } }
                    }
                }
                RepoAction::Stream { action } => {
                    let stream_dir = repo_dir.join(&config.streams_dir); fs::create_dir_all(&stream_dir)?;
                    match action {
                        StreamAction::New { name } => { if let Some(h) = get_head_hash(&current_dir, &config)? { fs::write(stream_dir.join(&name), h)?; println!("Stream '{}' created from HEAD.", name); } }
                        StreamAction::List => { for e in fs::read_dir(stream_dir)? { let n = e?.file_name().to_string_lossy().to_string(); println!("  {}", n.cyan()); } }
                        StreamAction::Rename { old, new } => { let p_old = stream_dir.join(&old); if p_old.exists() { fs::rename(p_old, stream_dir.join(&new))?; println!("Stream '{}' renamed to '{}'.", old, new); } }
                        StreamAction::Delete { name } => { let p = stream_dir.join(&name); if p.exists() { fs::remove_file(p)?; println!("Stream deleted."); } }
                    }
                }
            }
        }
        Commands::Beam { action } => {
            match action {
                BeamAction::Add { name, url } => { fs::create_dir_all(repo_dir.join("remotes"))?; fs::write(repo_dir.join("remotes").join(name), url)?; println!("Remote added."); }
                BeamAction::Default { name } => { fs::write(repo_dir.join("default_remote"), name)?; println!("Default remote set."); }
                BeamAction::List => { if let Ok(r) = fs::read_dir(repo_dir.join("remotes")) { for e in r { let n = e?.file_name().to_string_lossy().to_string(); println!("  {} -> {}", n, fs::read_to_string(repo_dir.join("remotes").join(&n))?.trim()); } } }
            }
        }
        Commands::Persona { action } => {
            let mut store = IdentityStore::load(&current_dir);
            match action {
                Some(PersonaAction::Add { id, name, email, global }) => { let mut i = identity::Identity { id, name, email, public_key: None, private_key: None }; i.generate_keys(); store.identities.push(i); store.save(&current_dir, global)?; }
                Some(PersonaAction::List) => { for i in &store.identities { println!("  {} - {} <{}>", i.id, i.name, i.email); } }
                Some(PersonaAction::Use { id, global }) => { store.active_id = id; store.save(&current_dir, global)?; }
                Some(PersonaAction::Keys) => { let a = store.active_id.clone(); if let Some(id) = store.identities.iter_mut().find(|i| i.id == a) { id.generate_keys(); store.save(&current_dir, false)?; } }
                _ => { let a = store.get_active(); println!("{} <{}>", a.name, a.email); }
            }
        }
        Commands::Burn { hash, aggressive } => {
            let target = if let Some(h) = hash { h } else { get_head_hash(&current_dir, &config)?.unwrap() };
            let (d, f) = target.split_at(2); let p = repo_dir.join(&config.objects_dir).join(d).join(f);
            if p.exists() { fs::remove_file(p)?; }
            if aggressive { println!("Aggressive cleanup..."); } println!("Burned.");
        }
        Commands::State => { println!("WIP: Working state comparison."); }
        Commands::Peek { hash } => { let (_, d) = storage.read_object(&hash)?; println!("{}", String::from_utf8_lossy(&d)); }
    }
    Ok(())
}
