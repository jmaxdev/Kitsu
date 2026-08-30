//! Persistent local HTTP server for Kitsu.
//!
//! Listens on 127.0.0.1:5911, protecting all `/api/v1/*` endpoints with a local
//! Bearer token stored in `~/.kitsu/server_token`. Provides REST endpoints for
//! repository inventory, local/remote issue and PR management, and GitHub OAuth callback.

use anyhow::Result;
use rand::RngCore;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::global_registry::GlobalRegistry;
use crate::identity::github::{
    GITHUB_CLIENT_ID, GitHubCredentials, exchange_oauth_code, fetch_user_profile_and_email,
    get_client_secret,
};

use crate::identity::{Identity, IdentityStore};
use crate::issues::{GitHubBridge, LocalIssueManager};

/// Default port for the local Kitsu server.
pub const DEFAULT_SERVER_PORT: u16 = 5911;

/// Manages the local API security token.
pub struct ServerToken;

impl ServerToken {
    /// Returns the path to the server token file (`~/.kitsu/server_token`).
    pub fn file_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let kitsu_dir = home.join(".kitsu");
        if !kitsu_dir.exists() {
            fs::create_dir_all(&kitsu_dir)?;
        }
        Ok(kitsu_dir.join("server_token"))
    }

    /// Loads the existing token or creates a secure random 32-byte hex token.
    pub fn get_or_create() -> Result<String> {
        let path = Self::file_path()?;
        if path.exists() {
            let token = fs::read_to_string(&path)?.trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);
        fs::write(path, &token)?;
        Ok(token)
    }

    /// Reads the token without generating a new one.
    pub fn read() -> Result<String> {
        let path = Self::file_path()?;
        if path.exists() {
            Ok(fs::read_to_string(path)?.trim().to_string())
        } else {
            Self::get_or_create()
        }
    }
}

/// Checks if the server is actively listening and responsive on the specified port.
pub fn is_server_running(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/api/v1/health", port);
    ureq::get(&url)
        .timeout(Duration::from_millis(400))
        .call()
        .map(|r| r.status() == 200)
        .unwrap_or(false)
}

/// Spawns the server daemon as a detached background process if it is not already running.
pub fn ensure_server_started() -> Result<()> {
    if is_server_running(DEFAULT_SERVER_PORT) {
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("server").arg("daemon");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    let child = cmd.spawn()?;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let pid_path = home.join(".kitsu/server.pid");
    fs::write(pid_path, format!("{}", child.id()))?;

    // Brief wait to ensure socket bound
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(50));
        if is_server_running(DEFAULT_SERVER_PORT) {
            break;
        }
    }
    Ok(())
}

/// Stops the background server by sending an authenticated shutdown request.
pub fn stop_server(port: u16) -> Result<bool> {
    if !is_server_running(port) {
        return Ok(false);
    }

    let token = ServerToken::read()?;
    let url = format!("http://127.0.0.1:{}/api/v1/shutdown", port);
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", token))
        .timeout(Duration::from_secs(2))
        .call();

    // Check PID file as backup
    let home = dirs::home_dir().unwrap();
    let pid_path = home.join(".kitsu/server.pid");
    if pid_path.exists() {
        let _ = fs::remove_file(pid_path);
    }

    Ok(resp.is_ok())
}

/// Runs the HTTP server loop on `127.0.0.1:port`.
pub fn run_server(port: u16) -> Result<()> {
    let token = ServerToken::get_or_create()?;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let pid_path = home.join(".kitsu/server.pid");
    fs::write(pid_path, format!("{}", std::process::id()))?;

    while r_clone.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let r_inner = Arc::clone(&r_clone);
                let tok = token.clone();
                std::thread::spawn(move || {
                    let _ = handle_connection(stream, &tok, r_inner, port);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    expected_token: &str,
    running: Arc<AtomicBool>,
    port: u16,
) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = [0u8; 8192];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return Ok(()),
    };

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request_str.lines();
    let first_line = match lines.next() {
        Some(l) => l,
        None => return Ok(()),
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let full_path = parts[1];

    let (path, query) = match full_path.find('?') {
        Some(idx) => (&full_path[..idx], &full_path[idx + 1..]),
        None => (full_path, ""),
    };

    // Extract authorization header or X-Kitsu-Token
    let mut auth_token = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("authorization:") {
            let val = line["authorization:".len()..].trim();
            if let Some(token) = val.strip_prefix("Bearer ") {
                auth_token = Some(token.trim().to_string());
            } else {
                auth_token = Some(val.to_string());
            }
        } else if lower.starts_with("x-kitsu-token:") {
            auth_token = Some(line["x-kitsu-token:".len()..].trim().to_string());
        }
    }

    // Unauthenticated public endpoints
    if path == "/api/v1/health" {
        return send_json(&mut stream, 200, &json!({"status": "ok"}));
    }

    if path == "/api/v1/github/auth" {
        return handle_github_oauth_callback(&mut stream, query, port);
    }

    // Protected endpoint verification
    let is_authenticated = match auth_token {
        Some(ref t) => t == expected_token,
        None => false,
    };

    if !is_authenticated {
        return send_json(
            &mut stream,
            401,
            &json!({"error": "Unauthorized. Bearer token required in Authorization header."}),
        );
    }

    // Authenticated API routes
    match (method, path) {
        ("GET", "/api/v1/status") => send_json(
            &mut stream,
            200,
            &json!({
                "status": "running",
                "version": env!("CARGO_PKG_VERSION"),
                "port": port,
                "pid": std::process::id()
            }),
        ),
        ("POST", "/api/v1/shutdown") => {
            send_json(&mut stream, 200, &json!({"status": "shutting_down"}))?;
            running.store(false, Ordering::SeqCst);
            Ok(())
        }
        ("GET", "/api/v1/repositories") => {
            let repos = GlobalRegistry::list().unwrap_or_default();
            let mut details_list = Vec::new();
            for meta in &repos {
                if let Ok(details) = GlobalRegistry::get_details(meta) {
                    details_list.push(details);
                } else {
                    details_list.push(crate::global_registry::RepositoryFullDetails {
                        meta: meta.clone(),
                        active_persona: "unknown".into(),
                        current_stream: None,
                        head_hash: None,
                        seals_count: 0,
                        total_objects: 0,
                        storage_bytes: 0,
                        local_issues_count: 0,
                    });
                }
            }
            send_json(&mut stream, 200, &json!({ "repositories": details_list }))
        }
        ("GET", p) if p.starts_with("/api/v1/repositories/") && p.ends_with("/issues") => {
            let repo_identifier = extract_repo_id_from_path(p, "/issues");
            handle_get_repository_issues(&mut stream, &repo_identifier)
        }
        ("GET", p) if p.starts_with("/api/v1/repositories/") && p.ends_with("/prs") => {
            let repo_identifier = extract_repo_id_from_path(p, "/prs");
            handle_get_repository_prs(&mut stream, &repo_identifier)
        }
        _ => send_json(&mut stream, 404, &json!({"error": "Endpoint not found"})),
    }
}

fn extract_repo_id_from_path(full_path: &str, suffix: &str) -> String {
    let prefix = "/api/v1/repositories/";
    if full_path.starts_with(prefix) && full_path.ends_with(suffix) {
        full_path[prefix.len()..full_path.len() - suffix.len()].to_string()
    } else {
        String::new()
    }
}

fn handle_get_repository_issues(stream: &mut TcpStream, repo_id: &str) -> Result<()> {
    let repos = GlobalRegistry::list().unwrap_or_default();
    let target_repo = repos.iter().find(|r| {
        r.name == repo_id
            || r.path == repo_id
            || r.path.ends_with(repo_id)
            || r.github_repo.as_deref() == Some(repo_id)
    });

    match target_repo {
        Some(repo) => {
            let repo_path = PathBuf::from(&repo.path);
            let repo_dir = repo_path.join(".kitsu");

            // If connected to GitHub and credentials exist, query GitHub
            if repo.is_github
                && let Some(ref slug) = repo.github_repo
                && let Ok(mut creds) = GitHubCredentials::load()
                && let Ok(token) = creds.get_valid_token()
            {
                match GitHubBridge::list_issues(&token, slug, None) {
                    Ok(issues) => send_json(
                        stream,
                        200,
                        &json!({ "source": "github", "issues": issues }),
                    ),
                    Err(e) => send_json(
                        stream,
                        500,
                        &json!({ "error": format!("GitHub error: {}", e) }),
                    ),
                }
            } else {
                let local_issues = LocalIssueManager::list(&repo_dir).unwrap_or_default();
                send_json(
                    stream,
                    200,
                    &json!({ "source": "local", "issues": local_issues }),
                )
            }
        }
        None => send_json(
            stream,
            404,
            &json!({ "error": format!("Repository '{}' not found", repo_id) }),
        ),
    }
}

fn handle_get_repository_prs(stream: &mut TcpStream, repo_id: &str) -> Result<()> {
    let repos = GlobalRegistry::list().unwrap_or_default();
    let target_repo = repos.iter().find(|r| {
        r.name == repo_id
            || r.path == repo_id
            || r.path.ends_with(repo_id)
            || r.github_repo.as_deref() == Some(repo_id)
    });

    match target_repo {
        Some(repo) if repo.is_github => {
            if let Some(ref slug) = repo.github_repo
                && let Ok(mut creds) = GitHubCredentials::load()
                && let Ok(token) = creds.get_valid_token()
            {
                match GitHubBridge::list_prs(&token, slug, None) {
                    Ok(prs) => send_json(
                        stream,
                        200,
                        &json!({ "source": "github", "pull_requests": prs }),
                    ),
                    Err(e) => send_json(
                        stream,
                        500,
                        &json!({ "error": format!("GitHub error: {}", e) }),
                    ),
                }
            } else {
                send_json(
                    stream,
                    400,
                    &json!({ "error": "Not authenticated with GitHub" }),
                )
            }
        }
        Some(_) => send_json(
            stream,
            400,
            &json!({ "error": "Repository does not have a GitHub remote" }),
        ),
        None => send_json(
            stream,
            404,
            &json!({ "error": format!("Repository '{}' not found", repo_id) }),
        ),
    }
}

fn handle_github_oauth_callback(stream: &mut TcpStream, query: &str, port: u16) -> Result<()> {
    let mut code = None;
    for param in query.split('&') {
        if let Some(c) = param.strip_prefix("code=") {
            code = Some(c.to_string());
        }
    }

    let code = match code {
        Some(c) => c,
        None => {
            let html = "<html><body><h2>Authentication Failed</h2><p>No authorization code received.</p></body></html>";
            return send_html(stream, 400, html);
        }
    };

    let redirect_uri = format!("http://localhost:{}/api/v1/github/auth", port);
    match exchange_oauth_code(&code, GITHUB_CLIENT_ID, &get_client_secret(), &redirect_uri) {
        Ok(token_resp) => {
            match fetch_user_profile_and_email(&token_resp.access_token) {
                Ok((profile, valid_email)) => {
                    let creds = GitHubCredentials {
                        access_token: token_resp.access_token,
                        refresh_token: token_resp.refresh_token,
                        token_type: token_resp.token_type,
                        expires_at: token_resp
                            .expires_in
                            .map(|exp| chrono::Utc::now().timestamp() + exp),
                        username: profile.login.clone(),
                        user_id: profile.id,
                        name: profile
                            .name
                            .clone()
                            .unwrap_or_else(|| profile.login.clone()),
                        email: valid_email.clone(),
                    };
                    let _ = creds.save();

                    // Update or create global persona
                    let mut store = IdentityStore::load(Path::new("."));
                    let persona_id = profile.login.clone();
                    if let Some(existing) = store.identities.iter_mut().find(|i| i.id == persona_id)
                    {
                        existing.name = profile.name.unwrap_or_else(|| profile.login.clone());
                        existing.email = valid_email.clone();
                    } else {
                        let mut new_persona = Identity {
                            id: persona_id.clone(),
                            name: profile.name.unwrap_or_else(|| profile.login.clone()),
                            email: valid_email.clone(),
                            public_key: None,
                            private_key: None,
                        };
                        new_persona.generate_keys();
                        store.identities.push(new_persona);
                    }
                    store.active_id = persona_id;
                    let _ = store.save(Path::new("."), true);

                    let html = format!(
                        "<!DOCTYPE html><html><head><title>Kitsu GitHub Auth</title><style>body{{font-family:sans-serif;background:#0d1117;color:#c9d1d9;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;}}.card{{background:#161b22;padding:2rem;border-radius:12px;border:1px solid #30363d;text-align:center;box-shadow:0 8px 24px rgba(0,0,0,0.5);}}h1{{color:#58a6ff;margin-top:0;}}code{{background:#21262d;padding:4px 8px;border-radius:6px;color:#7ee787;}}</style></head><body><div class=\"card\"><h1>&#10004; Authenticated with GitHub</h1><p>Welcome, <strong>{}</strong>!</p><p>Valid commit email: <code>{}</code></p><p>You can close this window and return to your terminal.</p></div></body></html>",
                        profile.login, valid_email
                    );
                    send_html(stream, 200, &html)
                }
                Err(e) => {
                    let html = format!(
                        "<html><body><h2>Failed to get profile</h2><p>{}</p></body></html>",
                        e
                    );
                    send_html(stream, 500, &html)
                }
            }
        }
        Err(e) => {
            let html = format!(
                "<html><body><h2>OAuth Token Exchange Failed</h2><p>{}</p></body></html>",
                e
            );
            send_html(stream, 500, &html)
        }
    }
}

fn send_json(stream: &mut TcpStream, status_code: u16, payload: &serde_json::Value) -> Result<()> {
    let body = serde_json::to_string(payload)?;
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status_code,
        status_text,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn send_html(stream: &mut TcpStream, status_code: u16, html: &str) -> Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        status_text,
        html.len(),
        html
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
