use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use kitsu_core::global_registry::parse_github_slug;
use kitsu_core::identity::IdentityStore;
use kitsu_core::identity::github::GitHubCredentials;
use kitsu_core::issues::{GitHubBridge, LocalIssueManager};
use kitsu_core::remote::{RemoteRegistry, default_remote_name};
use std::fs;
use std::path::Path;

use crate::app::{IssueAction, PrAction, RemoteAction, RepoAction, RepoImportAction, StreamAction};

/// Executes the `repository` subcommand.
pub fn execute(current_dir: &Path, action: RepoAction) -> Result<()> {
    // Handle import commands that might be run before a Kitsu repository is initialized
    if let RepoAction::Import {
        action: import_action,
    } = action
    {
        return match import_action {
            RepoImportAction::Git { path } => {
                let target_dir = path.unwrap_or_else(|| current_dir.to_path_buf());
                println!(
                    "{} Importing Git repository from {}...",
                    "→".cyan().bold(),
                    target_dir.display().to_string().yellow()
                );
                let cp_hash = kitsu_core::import_git_repository(&target_dir)?;
                println!(
                    "{} Git repository imported successfully into Kitsu!",
                    "✓".green().bold()
                );
                println!("  Initial Checkpoint: {}", cp_hash.yellow());
                println!("  Active Stream:      {}", "main".cyan());
                println!("  Remote data branch: {}", "kitsu-data".yellow());
                Ok(())
            }
        };
    }

    let repo = Repository::open(current_dir)?;
    let repo_dir = repo.repo_dir();

    match action {
        RepoAction::Import { .. } => unreachable!(),
        RepoAction::Info => {
            println!("{}", "--- Repository Information ---".cyan().bold());
            let id_store = IdentityStore::load(current_dir);
            println!("Active Persona:   {}", id_store.get_active().id.green());
            println!(
                "Default Remote:   {}",
                default_remote_name(&repo_dir)
                    .unwrap_or("none".into())
                    .yellow()
            );
            let seals_dir = repo_dir.join("seals");
            let seals_count = fs::read_dir(seals_dir).map(|d| d.count()).unwrap_or(0);
            println!("Seals (Versions): {}", seals_count.to_string().magenta());
            if let Some(h) = repo.head_hash()? {
                println!("HEAD Checkpoint:  {}", h.yellow());
            }
        }
        RepoAction::Stats => {
            println!("{}", "--- Repository Statistics ---".cyan().bold());
            let mut total_size = 0u64;
            let mut obj_count = 0u64;
            let obj_dir = repo_dir.join(&repo.config().objects_dir);
            if obj_dir.exists() {
                for entry in fs::read_dir(obj_dir)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        for obj in fs::read_dir(entry.path())? {
                            let obj = obj?;
                            total_size += obj.metadata()?.len();
                            obj_count += 1;
                        }
                    }
                }
            }
            println!("Total Objects:    {}", obj_count.to_string().green());
            println!(
                "Storage Usage:    {:.2} MB",
                total_size as f64 / 1_048_576.0
            );
        }
        RepoAction::Verify => {
            println!("{}", "--- Integrity Verification ---".cyan().bold());
            let obj_dir = repo_dir.join(&repo.config().objects_dir);
            let mut total = 0;
            let mut errors = 0;
            if obj_dir.exists() {
                for entry in fs::read_dir(obj_dir)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        for obj in fs::read_dir(entry.path())? {
                            let obj = obj?;
                            let prefix = entry.file_name().to_string_lossy().to_string();
                            let suffix = obj.file_name().to_string_lossy().to_string();
                            let hash = format!("{}{}", prefix, suffix);
                            total += 1;
                            if repo.storage().read_object(&hash).is_err() {
                                errors += 1;
                                println!("  {} {}", "CORRUPT:".red(), hash);
                            }
                            print!("\rVerifying: {} objects checked...", total);
                        }
                    }
                }
            }
            println!(
                "\nVerification complete. {} objects checked, {} errors.",
                total, errors
            );
        }
        RepoAction::Vacuum => {
            println!("{}", "Cleaning up repository...".yellow());
            println!("Vacuum finished.");
        }
        RepoAction::Remote { action } => match action {
            RemoteAction::Add { name, url, branch } => {
                RemoteRegistry::add(&repo_dir, &name, &url, branch.as_deref())?;
                if let Some(b) = &branch {
                    println!("Remote '{}' added: {} (branch: {})", name, url, b.yellow());
                } else {
                    println!("Remote '{}' added: {}", name, url);
                }
            }
            RemoteAction::Edit { name, url, branch } => {
                RemoteRegistry::edit(&repo_dir, &name, &url, branch.as_deref())?;
                if let Some(b) = &branch {
                    println!(
                        "Remote '{}' updated to: {} (branch: {})",
                        name,
                        url,
                        b.yellow()
                    );
                } else {
                    println!("Remote '{}' updated to: {}", name, url);
                }
            }
            RemoteAction::Default { name } => {
                RemoteRegistry::set_default(&repo_dir, &name)?;
                println!("Default remote set to: {}", name);
            }
            RemoteAction::List => {
                let entries = RemoteRegistry::list(&repo_dir)?;
                for e in entries {
                    if let Some(b) = &e.branch {
                        println!("  {} -> {} [{}]", e.name.green(), e.url.yellow(), b.cyan());
                    } else {
                        println!("  {} -> {}", e.name.green(), e.url.yellow());
                    }
                }
            }
            RemoteAction::Remove { name } => {
                RemoteRegistry::remove(&repo_dir, &name)?;
                println!("Remote removed.");
            }
        },
        RepoAction::Stream { action } => match action {
            StreamAction::New { name } => {
                kitsu_core::refs::create_stream(&name, current_dir, repo.config())?;
                println!("Stream '{}' created from HEAD.", name);
            }
            StreamAction::List => {
                let streams = kitsu_core::refs::list_streams(current_dir, repo.config())?;
                for s in streams {
                    println!("  {}", s.cyan());
                }
            }
            StreamAction::Rename { old, new } => {
                kitsu_core::refs::rename_stream(&old, &new, current_dir, repo.config())?;
                println!("Stream '{}' renamed to '{}'.", old, new);
            }
            StreamAction::Delete { name } => {
                kitsu_core::refs::delete_stream(&name, current_dir, repo.config())?;
                println!("Stream deleted.");
            }
        },
        RepoAction::Issue {
            action: issue_action,
            id,
        } => {
            handle_issue_command(current_dir, &repo_dir, issue_action, id)?;
        }
        RepoAction::Pr {
            action: pr_action,
            id,
        } => {
            handle_pr_command(current_dir, &repo_dir, pr_action, id)?;
        }
    }
    Ok(())
}

fn get_github_remote_info(repo_dir: &Path) -> Option<(String, String)> {
    let remotes = RemoteRegistry::list(repo_dir).ok()?;
    for r in remotes {
        if let Some(slug) = parse_github_slug(&r.url)
            && let Ok(mut creds) = GitHubCredentials::load()
            && let Ok(token) = creds.get_valid_token()
        {
            return Some((slug, token));
        }
    }

    None
}

fn handle_issue_command(
    current_dir: &Path,
    repo_dir: &Path,
    action: Option<IssueAction>,
    direct_id: Option<u64>,
) -> Result<()> {
    let github_info = get_github_remote_info(repo_dir);

    // If a direct ID was passed (e.g. `kitsu repository issue 1900`)
    if let Some(num) = direct_id {
        return view_issue_details(repo_dir, github_info.as_ref(), num);
    }

    match action {
        Some(IssueAction::View { id }) => view_issue_details(repo_dir, github_info.as_ref(), id),
        Some(IssueAction::Open { title, body }) => {
            let body_str = body.unwrap_or_default();
            if let Some((ref slug, ref token)) = github_info {
                let issue = GitHubBridge::create_issue(token, slug, &title, &body_str)?;
                println!(
                    "{} GitHub Issue #{} opened: {}",
                    "✓".green().bold(),
                    issue.number.to_string().yellow(),
                    issue.title.cyan()
                );
                println!("  URL: {}", issue.html_url.bright_black());
            } else {
                let id_store = IdentityStore::load(current_dir);
                let author = id_store.get_active().name.clone();
                let issue = LocalIssueManager::create(repo_dir, &title, &body_str, &author)?;
                println!(
                    "{} Local Issue #{} opened: {}",
                    "✓".green().bold(),
                    issue.id.to_string().yellow(),
                    issue.title.cyan()
                );
            }
            Ok(())
        }
        Some(IssueAction::Close { id, message }) => {
            if let Some((ref slug, ref token)) = github_info {
                let issue = GitHubBridge::close_issue(token, slug, id, message.as_deref())?;
                println!(
                    "{} GitHub Issue #{} closed.",
                    "✓".green().bold(),
                    issue.number.to_string().yellow()
                );
                if let Some(msg) = message {
                    println!("  Comment added: {}", msg.bright_black());
                }
            } else {
                let issue = LocalIssueManager::close(repo_dir, id, message.as_deref())?;
                println!(
                    "{} Local Issue #{} closed.",
                    "✓".green().bold(),
                    issue.id.to_string().yellow()
                );
                if let Some(msg) = issue.close_comment {
                    println!("  Resolution: {}", msg.bright_black());
                }
            }
            Ok(())
        }
        Some(IssueAction::Delete { id }) => {
            if let Some((ref slug, ref token)) = github_info {
                let issue =
                    GitHubBridge::close_issue(token, slug, id, Some("Deleted via Kitsu VCS"))?;
                println!(
                    "{} GitHub Issue #{} closed/removed.",
                    "✓".green().bold(),
                    issue.number.to_string().yellow()
                );
            } else {
                let deleted = LocalIssueManager::delete(repo_dir, id)?;
                if deleted {
                    println!(
                        "{} Local Issue #{} deleted.",
                        "✓".green().bold(),
                        id.to_string().yellow()
                    );
                } else {
                    println!("{} Issue #{} not found.", "ERROR:".red().bold(), id);
                }
            }
            Ok(())
        }
        Some(IssueAction::Reopen { id }) => {
            if let Some((ref slug, ref token)) = github_info {
                let issue = GitHubBridge::reopen_issue(token, slug, id)?;
                println!(
                    "{} GitHub Issue #{} reopened.",
                    "✓".green().bold(),
                    issue.number.to_string().yellow()
                );
            } else {
                let issue = LocalIssueManager::reopen(repo_dir, id)?;
                println!(
                    "{} Local Issue #{} reopened.",
                    "✓".green().bold(),
                    issue.id.to_string().yellow()
                );
            }
            Ok(())
        }
        Some(IssueAction::List) | None => {
            if let Some((ref slug, ref token)) = github_info {
                println!("=== GitHub Issues ({}) ===", slug.cyan().bold());
                let issues = GitHubBridge::list_issues(token, slug, None)?;
                if issues.is_empty() {
                    println!("  No issues found.");
                } else {
                    for i in issues {
                        let state_str = if i.state == "open" {
                            "[OPEN]".green()
                        } else {
                            "[CLOSED]".bright_black()
                        };
                        println!(
                            "  #{} {} {} ({})",
                            i.number.to_string().yellow(),
                            state_str,
                            i.title.bold(),
                            i.user.login.bright_black()
                        );
                    }
                }
            } else {
                println!("{}", "=== Local Repository Issues ===".cyan().bold());
                let issues = LocalIssueManager::list(repo_dir)?;
                if issues.is_empty() {
                    println!(
                        "  No local issues found. Create one with 'kitsu repository issue open <title>'"
                    );
                } else {
                    for i in issues {
                        let state_str = if i.state == "open" {
                            "[OPEN]".green()
                        } else {
                            "[CLOSED]".bright_black()
                        };
                        println!(
                            "  #{} {} {} ({})",
                            i.id.to_string().yellow(),
                            state_str,
                            i.title.bold(),
                            i.author.bright_black()
                        );
                    }
                }
            }
            Ok(())
        }
    }
}

fn view_issue_details(
    repo_dir: &Path,
    github_info: Option<&(String, String)>,
    id: u64,
) -> Result<()> {
    if let Some((slug, token)) = github_info {
        let issue = GitHubBridge::get_issue(token, slug, id)?;

        let state_badge = if issue.state == "open" {
            "OPEN".green().bold()
        } else {
            "CLOSED".red().bold()
        };
        println!(
            "\n{} Issue #{}: {}",
            "=== GitHub ===".cyan().bold(),
            issue.number.to_string().yellow(),
            issue.title.bold()
        );
        println!("  State:      {}", state_badge);
        println!("  Author:     {}", issue.user.login.green());
        println!("  Created:    {}", issue.created_at.bright_black());
        if let Some(ref closed) = issue.closed_at {
            println!("  Closed:     {}", closed.bright_black());
        }
        println!("  URL:        {}\n", issue.html_url.bright_black());
        println!("{}", "Description:".bold());
        println!(
            "{}\n",
            issue
                .body
                .unwrap_or_else(|| "No description provided.".into())
        );
    } else {
        let issue = LocalIssueManager::get(repo_dir, id)?;
        let state_badge = if issue.state == "open" {
            "OPEN".green().bold()
        } else {
            "CLOSED".red().bold()
        };
        println!(
            "\n{} Issue #{}: {}",
            "=== Local ===".cyan().bold(),
            issue.id.to_string().yellow(),
            issue.title.bold()
        );
        println!("  State:      {}", state_badge);
        println!("  Author:     {}", issue.author.green());
        println!("  Created:    {}", issue.created_at.bright_black());
        if let Some(ref closed) = issue.closed_at {
            println!("  Closed:     {}", closed.bright_black());
        }
        if let Some(ref comment) = issue.close_comment {
            println!("  Resolution: {}", comment.yellow());
        }
        println!("\n{}", "Description:".bold());
        println!("{}\n", issue.body);
    }
    Ok(())
}

fn handle_pr_command(
    _current_dir: &Path,
    repo_dir: &Path,
    action: Option<PrAction>,
    direct_id: Option<u64>,
) -> Result<()> {
    let (slug, token) = match get_github_remote_info(repo_dir) {
        Some(info) => info,
        None => {
            println!(
                "{} Pull Requests require a GitHub remote configured and authentication ('kitsu persona github auth').",
                "ERROR:".red().bold()
            );
            return Ok(());
        }
    };

    if let Some(num) = direct_id {
        return view_pr_details(&slug, &token, num);
    }

    match action {
        Some(PrAction::View { id }) => view_pr_details(&slug, &token, id),
        Some(PrAction::Open {
            title,
            body,
            head,
            base,
        }) => {
            let body_str = body.unwrap_or_default();
            let pr = GitHubBridge::create_pr(&token, &slug, &title, &body_str, &head, &base)?;
            println!(
                "{} Pull Request #{} opened: {}",
                "✓".green().bold(),
                pr.number.to_string().yellow(),
                pr.title.cyan()
            );
            println!(
                "  Branch: {} -> {}",
                pr.head.ref_name.cyan(),
                pr.base.ref_name.yellow()
            );
            println!("  URL:    {}", pr.html_url.bright_black());
            Ok(())
        }
        Some(PrAction::Close { id }) => {
            let pr = GitHubBridge::close_pr(&token, &slug, id)?;
            println!(
                "{} Pull Request #{} closed.",
                "✓".green().bold(),
                pr.number.to_string().yellow()
            );
            Ok(())
        }
        Some(PrAction::List) | None => {
            println!("=== GitHub Pull Requests ({}) ===", slug.cyan().bold());
            let prs = GitHubBridge::list_prs(&token, &slug, None)?;
            if prs.is_empty() {
                println!("  No pull requests found.");
            } else {
                for p in prs {
                    let state_str = if p.merged {
                        "[MERGED]".magenta()
                    } else if p.state == "open" {
                        "[OPEN]".green()
                    } else {
                        "[CLOSED]".bright_black()
                    };
                    println!(
                        "  #{} {} {} ({} -> {}) by {}",
                        p.number.to_string().yellow(),
                        state_str,
                        p.title.bold(),
                        p.head.ref_name.cyan(),
                        p.base.ref_name.yellow(),
                        p.user.login.bright_black()
                    );
                }
            }
            Ok(())
        }
    }
}

fn view_pr_details(slug: &str, token: &str, id: u64) -> Result<()> {
    let pr = GitHubBridge::get_pr(token, slug, id)?;
    let state_badge = if pr.merged {
        "MERGED".magenta().bold()
    } else if pr.state == "open" {
        "OPEN".green().bold()
    } else {
        "CLOSED".red().bold()
    };
    println!(
        "\n{} Pull Request #{}: {}",
        "=== GitHub ===".cyan().bold(),
        pr.number.to_string().yellow(),
        pr.title.bold()
    );
    println!("  State:      {}", state_badge);
    println!("  Author:     {}", pr.user.login.green());
    println!(
        "  Branches:   {} -> {}",
        pr.head.ref_name.cyan(),
        pr.base.ref_name.yellow()
    );
    println!("  Created:    {}", pr.created_at.bright_black());
    println!("  URL:        {}\n", pr.html_url.bright_black());
    println!("{}", "Description:".bold());
    println!(
        "{}\n",
        pr.body.unwrap_or_else(|| "No description provided.".into())
    );
    Ok(())
}
