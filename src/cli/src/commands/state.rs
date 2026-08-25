use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use std::path::Path;

pub fn execute(current_dir: &Path) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let ws = kitsu_core::state::compute_state(
        current_dir,
        repo.config(),
        repo.storage(),
        repo.exclude(),
    )?;

    println!("{}", "Kitsu Working State".bold());

    if !ws.staged_added.is_empty()
        || !ws.staged_modified.is_empty()
        || !ws.staged_deleted.is_empty()
    {
        println!("\nChanges to be frozen:");
        println!("  (use \"kitsu rollback\" to unstage)");
        for p in &ws.staged_added {
            println!("\t{}", format!("new file:   {}", p).green());
        }
        for p in &ws.staged_modified {
            println!("\t{}", format!("modified:   {}", p).green());
        }
        for p in &ws.staged_deleted {
            println!("\t{}", format!("deleted:    {}", p).green());
        }
    }

    if !ws.unstaged_modified.is_empty() || !ws.unstaged_deleted.is_empty() {
        println!("\nChanges not staged for freeze:");
        println!("  (use \"kitsu track <file>...\" to update what will be frozen)");
        for p in &ws.unstaged_modified {
            println!("\t{}", format!("modified:   {}", p).red());
        }
        for p in &ws.unstaged_deleted {
            println!("\t{}", format!("deleted:    {}", p).red());
        }
    }

    if !ws.untracked.is_empty() {
        println!("\nUntracked files:");
        println!("  (use \"kitsu track <file>...\" to include in what will be frozen)");
        for p in &ws.untracked {
            println!("\t{}", p.red());
        }
    }

    if ws.is_clean() {
        println!("\n{}", "nothing to freeze, working tree clean".green());
    }

    Ok(())
}
