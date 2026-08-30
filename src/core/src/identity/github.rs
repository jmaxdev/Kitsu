//! GitHub OAuth and user details integration.
//!
//! Provides OAuth token exchange, token refresh, and user profile
//! extraction with accurate noreply/verified email resolution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Public OAuth Client ID for Kitsu.
pub const GITHUB_CLIENT_ID: &str = "Ov23li1HhEd83bO82al6";

/// Default Client Secret fallback (can be overridden at build time via `CLIENT_SECRET`).
pub const GITHUB_CLIENT_SECRET: &str = match option_env!("CLIENT_SECRET") {
    Some(secret) => secret,
    None => match option_env!("CLIENT_SECRET") {
        Some(secret) => secret,
        None => "",
    },
};

/// Returns the effective client secret checking runtime environment variable `CLIENT_SECRET` first.
pub fn get_client_secret() -> String {
    std::env::var("CLIENT_SECRET").unwrap_or_else(|_| GITHUB_CLIENT_SECRET.to_string())
}

/// Stored credentials for an authenticated GitHub account.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GitHubCredentials {
    /// OAuth access token.
    pub access_token: String,
    /// Optional OAuth refresh token for token renewal.
    pub refresh_token: Option<String>,
    /// Token type (typically "bearer").
    pub token_type: String,
    /// Expiration timestamp in seconds since UNIX epoch.
    pub expires_at: Option<i64>,
    /// GitHub login username.
    pub username: String,
    /// GitHub numeric user ID.
    pub user_id: u64,
    /// Display name configured on GitHub.
    pub name: String,
    /// Canonical commit email address (e.g. `251231531+jmaxdev@users.noreply.github.com`).
    pub email: String,
}

/// GitHub user profile returned by `/user`.
#[derive(Deserialize, Debug)]
pub struct GitHubUserProfile {
    /// Unique GitHub user identifier.
    pub id: u64,
    /// GitHub username / handle.
    pub login: String,
    /// Full display name.
    pub name: Option<String>,
    /// Publicly visible email address, if any.
    pub email: Option<String>,
}

/// GitHub email record returned by `/user/emails`.
#[derive(Deserialize, Debug)]
pub struct GitHubEmailRecord {
    /// Email address string.
    pub email: String,
    /// Whether this is the primary email.
    pub primary: bool,
    /// Whether this email address is verified.
    pub verified: bool,
    /// Visibility (`"public"` or `"private"`).
    pub visibility: Option<String>,
}

/// OAuth token response from GitHub token endpoint.
#[derive(Deserialize, Debug)]
pub struct GitHubTokenResponse {
    /// OAuth access token string.
    pub access_token: String,
    /// Token type (e.g. "bearer").
    pub token_type: String,
    /// Optional OAuth refresh token.
    pub refresh_token: Option<String>,
    /// Number of seconds until access token expires.
    pub expires_in: Option<i64>,
    /// Number of seconds until refresh token expires.
    pub refresh_token_expires_in: Option<i64>,
}

impl GitHubCredentials {
    /// Returns the global path where GitHub credentials are saved.
    pub fn file_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let kitsu_dir = home.join(".kitsu");
        if !kitsu_dir.exists() {
            fs::create_dir_all(&kitsu_dir)?;
        }
        Ok(kitsu_dir.join("github_credentials.toml"))
    }

    /// Loads the stored GitHub credentials from disk.
    ///
    /// # Errors
    /// Returns an error if the credentials file does not exist or fails to parse.
    pub fn load() -> Result<Self> {
        let path = Self::file_path()?;
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "No GitHub credentials found. Run 'kitsu persona github auth' to authenticate."
            ));
        }
        let content = fs::read_to_string(path)?;
        let creds: Self = toml::from_str(&content)?;
        Ok(creds)
    }

    /// Persists GitHub credentials to disk.
    ///
    /// # Errors
    /// Returns an error if serializing or writing fails.
    pub fn save(&self) -> Result<()> {
        let path = Self::file_path()?;
        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Deletes stored GitHub credentials from disk.
    pub fn clear() -> Result<()> {
        let path = Self::file_path()?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Returns a valid access token, refreshing it if expired and refresh_token is available.
    pub fn get_valid_token(&mut self) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        if let Some(exp) = self.expires_at
            && now >= exp - 60
            && let Some(ref refresh) = self.refresh_token
            && let Ok(new_token) =
                refresh_access_token(refresh, GITHUB_CLIENT_ID, &get_client_secret())
        {
            self.access_token = new_token.access_token.clone();
            if let Some(r) = new_token.refresh_token {
                self.refresh_token = Some(r);
            }
            if let Some(exp_in) = new_token.expires_in {
                self.expires_at = Some(now + exp_in);
            }
            let _ = self.save();
        }

        Ok(self.access_token.clone())
    }
}

/// Exchanges an OAuth temporary authorization code for an access token.
///
/// # Errors
/// Returns an error if the HTTP request fails or GitHub returns an error.
pub fn exchange_oauth_code(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<GitHubTokenResponse> {
    let response: GitHubTokenResponse = ureq::post("https://github.com/login/oauth/access_token")
        .set("Accept", "application/json")
        .set("User-Agent", "Kitsu-VCS")
        .send_json(serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))?
        .into_json()?;

    Ok(response)
}

/// Refreshes an expired OAuth access token using a refresh token.
///
/// # Errors
/// Returns an error if the HTTP request fails or the refresh token is invalid.
pub fn refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<GitHubTokenResponse> {
    let response: GitHubTokenResponse = ureq::post("https://github.com/login/oauth/access_token")
        .set("Accept", "application/json")
        .set("User-Agent", "Kitsu-VCS")
        .send_json(serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }))?
        .into_json()?;

    Ok(response)
}

/// Fetches user profile and resolves the canonical valid commit email address.
///
/// GitHub noreply emails follow the format `{id}+{login}@users.noreply.github.com`.
/// If the user has a verified primary private/public email, it will use that or the noreply email.
///
/// # Errors
/// Returns an error if network request fails or authentication is denied.
pub fn fetch_user_profile_and_email(token: &str) -> Result<(GitHubUserProfile, String)> {
    let profile: GitHubUserProfile = ureq::get("https://api.github.com/user")
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github.v3+json")
        .set("User-Agent", "Kitsu-VCS")
        .call()?
        .into_json()?;

    // Standard valid noreply email for GitHub users
    let noreply_email = format!("{}+{}@users.noreply.github.com", profile.id, profile.login);

    // Try fetching /user/emails to check for primary verified email or explicit noreply
    let email_res = ureq::get("https://api.github.com/user/emails")
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github.v3+json")
        .set("User-Agent", "Kitsu-VCS")
        .call();

    let chosen_email = if let Ok(resp) = email_res
        && let Ok(emails) = resp.into_json::<Vec<GitHubEmailRecord>>()
    {
        let primary_verified = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email.clone());
        primary_verified.unwrap_or(noreply_email)
    } else {
        profile.email.clone().unwrap_or(noreply_email)
    };

    Ok((profile, chosen_email))
}
