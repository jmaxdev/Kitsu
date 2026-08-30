use anyhow::Result;
use colored::*;
use dialoguer::Input;
use kitsu_core::identity::github::{
    GITHUB_CLIENT_ID, GitHubCredentials, fetch_user_profile_and_email,
};
use kitsu_core::identity::{Identity, IdentityStore};
use kitsu_core::server::{DEFAULT_SERVER_PORT, ensure_server_started};

use std::path::Path;
use std::time::Duration;

use crate::app::{GithubSubAction, PersonaAction};

/// Executes the `persona` subcommand.
pub fn execute(current_dir: &Path, action: Option<PersonaAction>) -> Result<()> {
    let mut store = IdentityStore::load(current_dir);
    match action {
        Some(PersonaAction::Add {
            id,
            name,
            email,
            global,
        }) => {
            let mut i = Identity {
                id: id.clone(),
                name,
                email,
                public_key: None,
                private_key: None,
            };
            i.generate_keys();
            store.identities.push(i);
            store.save(current_dir, global)?;
            println!("{} Persona '{}' added.", "✓".green().bold(), id);
        }
        Some(PersonaAction::List) => {
            println!("{}", "=== Registered Personas ===".cyan().bold());
            for i in &store.identities {
                if i.id == store.active_id {
                    println!(
                        "  {} {} - {} <{}>",
                        "*".green().bold(),
                        i.id.green().bold(),
                        i.name,
                        i.email
                    );
                } else {
                    println!("    {} - {} <{}>", i.id.yellow(), i.name, i.email);
                }
            }
        }
        Some(PersonaAction::Use { id, global }) => {
            if store.identities.iter().any(|i| i.id == id) {
                store.active_id = id.clone();
                store.save(current_dir, global)?;
                println!("{} Active persona set to '{}'.", "✓".green().bold(), id);
            } else {
                println!("{} Persona '{}' not found.", "ERROR:".red().bold(), id);
            }
        }
        Some(PersonaAction::Edit {
            id,
            name,
            email,
            global,
        }) => {
            if let Some(identity) = store.identities.iter_mut().find(|i| i.id == id) {
                if let Some(n) = name {
                    identity.name = n;
                }
                if let Some(e) = email {
                    identity.email = e;
                }
                store.save(current_dir, global)?;
                println!("{} Persona '{}' updated.", "✓".green().bold(), id);
            } else {
                println!("{} Persona '{}' not found.", "ERROR:".red().bold(), id);
            }
        }
        Some(PersonaAction::Remove { id, global }) | Some(PersonaAction::Delete { id, global }) => {
            if store.remove(&id)? {
                store.save(current_dir, global)?;
                println!(
                    "{} Persona '{}' removed. Active persona is now '{}'.",
                    "✓".green().bold(),
                    id,
                    store.active_id.green()
                );
            } else {
                println!("{} Persona '{}' not found.", "ERROR:".red().bold(), id);
            }
        }
        Some(PersonaAction::Github {
            username,
            id,
            global,
            action,
        }) => {
            match action {
                Some(GithubSubAction::Auth { token, global: g }) => {
                    handle_github_auth(current_dir, &mut store, token, g)?;
                }
                None => {
                    if let Some(ref u) = username {
                        if u == "auth" {
                            handle_github_auth(current_dir, &mut store, None, global)?;
                        } else {
                            // Query GitHub public profile for accurate numeric ID
                            handle_github_username_import(
                                current_dir,
                                &mut store,
                                u,
                                id.as_deref(),
                                global,
                            )?;
                        }
                    } else {
                        handle_github_auth(current_dir, &mut store, None, global)?;
                    }
                }
            }
        }
        Some(PersonaAction::Keys) => {
            let a = store.active_id.clone();
            if let Some(id) = store.identities.iter_mut().find(|i| i.id == a) {
                id.generate_keys();
                store.save(current_dir, false)?;
                println!(
                    "{} Keys regenerated for persona '{}'.",
                    "✓".green().bold(),
                    a
                );
            }
        }
        None => {
            let a = store.get_active();
            println!(
                "Active Persona: {} <{}> [{}]",
                a.name.green(),
                a.email.yellow(),
                a.id.cyan()
            );
        }
    }
    Ok(())
}

fn handle_github_auth(
    current_dir: &Path,
    store: &mut IdentityStore,
    token: Option<String>,
    global: bool,
) -> Result<()> {
    if let Some(t) = token {
        let (profile, valid_email) = fetch_user_profile_and_email(&t)?;
        let creds = GitHubCredentials {
            access_token: t,
            refresh_token: None,
            token_type: "bearer".into(),
            expires_at: None,
            username: profile.login.clone(),
            user_id: profile.id,
            name: profile
                .name
                .clone()
                .unwrap_or_else(|| profile.login.clone()),
            email: valid_email.clone(),
        };
        creds.save()?;
        update_or_create_github_persona(
            store,
            current_dir,
            &profile.login,
            profile.name.as_deref(),
            &valid_email,
            global,
        )?;
        println!(
            "{} Authenticated with GitHub as {} <{}>",
            "✓".green().bold(),
            profile.login.green().bold(),
            valid_email.yellow()
        );
        return Ok(());
    }

    println!("{}", "=== Kitsu GitHub Authentication ===".cyan().bold());
    let _ = ensure_server_started();

    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri=http://localhost:{}/api/v1/github/auth&scope=read:user,user:email,repo",
        GITHUB_CLIENT_ID, DEFAULT_SERVER_PORT
    );

    println!("Opening browser for GitHub authorization...");
    println!("  URL: {}\n", auth_url.cyan());

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", &auth_url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&auth_url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&auth_url)
            .spawn();
    }

    println!(
        "Waiting for OAuth confirmation in browser (or press Enter to paste a Personal Access Token)..."
    );

    // Poll for credentials saved by server for up to 15 seconds
    let initial_creds = GitHubCredentials::load().ok();
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(500));
        if let Ok(current_creds) = GitHubCredentials::load()
            && initial_creds.as_ref() != Some(&current_creds)
        {
            *store = IdentityStore::load(current_dir);
            println!(
                "\n{} Authenticated with GitHub as {} <{}>",
                "✓".green().bold(),
                current_creds.username.green().bold(),
                current_creds.email.yellow()
            );
            return Ok(());
        }
    }

    // Fallback: prompt for PAT
    println!("\n{}", "No browser response detected yet.".yellow());
    let pat: String = Input::new()
        .with_prompt("Enter GitHub Personal Access Token (or leave empty to cancel)")
        .allow_empty(true)
        .interact_text()?;

    if !pat.trim().is_empty() {
        let (profile, valid_email) = fetch_user_profile_and_email(pat.trim())?;
        let creds = GitHubCredentials {
            access_token: pat.trim().to_string(),
            refresh_token: None,
            token_type: "bearer".into(),
            expires_at: None,
            username: profile.login.clone(),
            user_id: profile.id,
            name: profile
                .name
                .clone()
                .unwrap_or_else(|| profile.login.clone()),
            email: valid_email.clone(),
        };
        creds.save()?;
        update_or_create_github_persona(
            store,
            current_dir,
            &profile.login,
            profile.name.as_deref(),
            &valid_email,
            global,
        )?;
        println!(
            "{} Authenticated with GitHub as {} <{}>",
            "✓".green().bold(),
            profile.login.green().bold(),
            valid_email.yellow()
        );
    }

    Ok(())
}

fn handle_github_username_import(
    current_dir: &Path,
    store: &mut IdentityStore,
    username: &str,
    custom_id: Option<&str>,
    global: bool,
) -> Result<()> {
    let persona_id = custom_id.unwrap_or(username).to_string();

    // Query public GitHub API to get accurate numeric ID
    let (display_name, valid_email) =
        match ureq::get(&format!("https://api.github.com/users/{}", username))
            .set("User-Agent", "Kitsu-VCS")
            .call()
        {
            Ok(resp) => {
                if let Ok(val) = resp.into_json::<serde_json::Value>() {
                    let id = val.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    let name = val
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(username)
                        .to_string();
                    let email = if id > 0 {
                        format!("{}+{}@users.noreply.github.com", id, username)
                    } else {
                        format!("{}@users.noreply.github.com", username)
                    };
                    (name, email)
                } else {
                    (
                        username.to_string(),
                        format!("{}@users.noreply.github.com", username),
                    )
                }
            }
            Err(_) => (
                username.to_string(),
                format!("{}@users.noreply.github.com", username),
            ),
        };

    update_or_create_github_persona(
        store,
        current_dir,
        &persona_id,
        Some(&display_name),
        &valid_email,
        global,
    )?;
    println!(
        "{} Persona '{}' created from GitHub user '{}' with valid email <{}>.",
        "✓".green().bold(),
        persona_id.green(),
        username,
        valid_email.yellow()
    );
    Ok(())
}

fn update_or_create_github_persona(
    store: &mut IdentityStore,
    current_dir: &Path,
    persona_id: &str,
    name: Option<&str>,
    email: &str,
    global: bool,
) -> Result<()> {
    let display_name = name.unwrap_or(persona_id).to_string();
    if let Some(existing) = store.identities.iter_mut().find(|i| i.id == persona_id) {
        existing.name = display_name;
        existing.email = email.to_string();
    } else {
        let mut new_persona = Identity {
            id: persona_id.to_string(),
            name: display_name,
            email: email.to_string(),
            public_key: None,
            private_key: None,
        };
        new_persona.generate_keys();
        store.identities.push(new_persona);
    }
    store.active_id = persona_id.to_string();
    store.save(current_dir, global)?;
    Ok(())
}
