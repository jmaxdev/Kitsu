//! Local issue tracking and GitHub Issues / Pull Requests integration.
//!
//! Provides a local filesystem issue tracker (.kitsu/issues/<id>.toml)
//! and a unified bridge to GitHub REST APIs for remote issue and PR management.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A comment on an issue.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IssueComment {
    /// Incremental comment ID.
    pub id: u64,
    /// Comment author display name or email.
    pub author: String,
    /// Comment markdown or plain text body.
    pub body: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// A local issue stored in `.kitsu/issues/<id>.toml`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LocalIssue {
    /// Numerical sequential identifier (1, 2, 3...).
    pub id: u64,
    /// Issue title summary.
    pub title: String,
    /// Issue description body.
    pub body: String,
    /// Issue state: `"open"` or `"closed"`.
    pub state: String,
    /// Issue author.
    pub author: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 closure timestamp if closed.
    pub closed_at: Option<String>,
    /// Optional closing resolution comment.
    pub close_comment: Option<String>,
    /// Thread of comments on this issue.
    #[serde(default)]
    pub comments: Vec<IssueComment>,
}

/// Manager for local repository issues.
pub struct LocalIssueManager;

impl LocalIssueManager {
    /// Returns the issues directory path for a repository (`.kitsu/issues/`).
    pub fn issues_dir(repo_dir: &Path) -> std::path::PathBuf {
        repo_dir.join("issues")
    }

    /// Creates a new local issue with the next sequential ID.
    ///
    /// # Errors
    /// Returns an error if directory creation or file writing fails.
    pub fn create(repo_dir: &Path, title: &str, body: &str, author: &str) -> Result<LocalIssue> {
        let dir = Self::issues_dir(repo_dir);
        fs::create_dir_all(&dir)?;

        let mut max_id = 0u64;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id_str) = name.strip_suffix(".toml")
                && let Ok(id) = id_str.parse::<u64>()
                && id > max_id
            {
                max_id = id;
            }
        }

        let new_id = max_id + 1;
        let now = Utc::now().to_rfc3339();
        let issue = LocalIssue {
            id: new_id,
            title: title.to_string(),
            body: body.to_string(),
            state: "open".to_string(),
            author: author.to_string(),
            created_at: now,
            closed_at: None,
            close_comment: None,
            comments: Vec::new(),
        };

        let content = toml::to_string(&issue)?;
        fs::write(dir.join(format!("{}.toml", new_id)), content)?;
        Ok(issue)
    }

    /// Retrieves a single issue by ID.
    ///
    /// # Errors
    /// Returns an error if the issue file does not exist or fails to parse.
    pub fn get(repo_dir: &Path, id: u64) -> Result<LocalIssue> {
        let path = Self::issues_dir(repo_dir).join(format!("{}.toml", id));
        if !path.exists() {
            return Err(anyhow::anyhow!("Issue #{} not found", id));
        }
        let content = fs::read_to_string(path)?;
        let issue: LocalIssue = toml::from_str(&content)?;
        Ok(issue)
    }

    /// Lists all local issues for a repository, sorted by ID.
    ///
    /// # Errors
    /// Returns an error if reading the issues directory fails.
    pub fn list(repo_dir: &Path) -> Result<Vec<LocalIssue>> {
        let dir = Self::issues_dir(repo_dir);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut issues = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id_str) = name.strip_suffix(".toml")
                && let Ok(id) = id_str.parse::<u64>()
                && let Ok(issue) = Self::get(repo_dir, id)
            {
                issues.push(issue);
            }
        }

        issues.sort_by_key(|i| i.id);
        Ok(issues)
    }

    /// Closes an issue with an optional comment message.
    ///
    /// # Errors
    /// Returns an error if the issue is not found or file writing fails.
    pub fn close(repo_dir: &Path, id: u64, message: Option<&str>) -> Result<LocalIssue> {
        let mut issue = Self::get(repo_dir, id)?;
        issue.state = "closed".to_string();
        issue.closed_at = Some(Utc::now().to_rfc3339());
        if let Some(msg) = message {
            issue.close_comment = Some(msg.to_string());
        }
        let path = Self::issues_dir(repo_dir).join(format!("{}.toml", id));
        let content = toml::to_string(&issue)?;
        fs::write(path, content)?;
        Ok(issue)
    }

    /// Reopens a previously closed issue.
    ///
    /// # Errors
    /// Returns an error if the issue is not found or file writing fails.
    pub fn reopen(repo_dir: &Path, id: u64) -> Result<LocalIssue> {
        let mut issue = Self::get(repo_dir, id)?;
        issue.state = "open".to_string();
        issue.closed_at = None;
        let path = Self::issues_dir(repo_dir).join(format!("{}.toml", id));
        let content = toml::to_string(&issue)?;
        fs::write(path, content)?;
        Ok(issue)
    }

    /// Deletes an issue completely from local storage.
    ///
    /// # Errors
    /// Returns an error if deleting the issue file fails.
    pub fn delete(repo_dir: &Path, id: u64) -> Result<bool> {
        let path = Self::issues_dir(repo_dir).join(format!("{}.toml", id));
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// -------------------------------------------------------------------------------------------------
// GitHub Remote Bridge
// -------------------------------------------------------------------------------------------------

/// GitHub Issue structure returned by REST API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHubIssue {
    /// GitHub issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body markdown.
    pub body: Option<String>,
    /// Issue state ("open" or "closed").
    pub state: String,
    /// User who created the issue.
    pub user: GitHubUserMini,
    /// Browser HTML URL.
    pub html_url: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 closure timestamp if closed.
    pub closed_at: Option<String>,
    /// Number of comments.
    #[serde(default)]
    pub comments: u64,
    /// Present if this issue is actually a pull request.
    pub pull_request: Option<serde_json::Value>,
}

/// Minimal user info in GitHub API responses.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHubUserMini {
    /// Username / handle.
    pub login: String,
}

/// GitHub Pull Request branch ref details.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHubPrBranch {
    /// Target ref name.
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// GitHub Pull Request structure returned by REST API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHubPr {
    /// GitHub PR number.
    pub number: u64,
    /// Pull request title.
    pub title: String,
    /// Pull request description.
    pub body: Option<String>,
    /// PR state ("open" or "closed").
    pub state: String,
    /// PR author.
    pub user: GitHubUserMini,
    /// Browser HTML URL.
    pub html_url: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Head branch details.
    pub head: GitHubPrBranch,
    /// Base branch details.
    pub base: GitHubPrBranch,
    /// Whether this PR was merged.
    #[serde(default)]
    pub merged: bool,
}

/// Bridge for calling GitHub Issues & PRs REST APIs.
pub struct GitHubBridge;

impl GitHubBridge {
    /// Lists issues from a GitHub repository (`owner/repo`).
    pub fn list_issues(
        token: &str,
        owner_repo: &str,
        state: Option<&str>,
    ) -> Result<Vec<GitHubIssue>> {
        let state_param = state.unwrap_or("all");
        let url = format!(
            "https://api.github.com/repos/{}/issues?state={}&per_page=50",
            owner_repo, state_param
        );
        let issues: Vec<GitHubIssue> = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .call()?
            .into_json()?;

        // Filter out items that are pull requests
        Ok(issues
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .collect())
    }

    /// Fetches a single issue by number.
    pub fn get_issue(token: &str, owner_repo: &str, number: u64) -> Result<GitHubIssue> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            owner_repo, number
        );
        let issue: GitHubIssue = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .call()?
            .into_json()?;
        Ok(issue)
    }

    /// Opens a new issue on GitHub.
    pub fn create_issue(
        token: &str,
        owner_repo: &str,
        title: &str,
        body: &str,
    ) -> Result<GitHubIssue> {
        let url = format!("https://api.github.com/repos/{}/issues", owner_repo);
        let issue: GitHubIssue = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .send_json(serde_json::json!({
                "title": title,
                "body": body,
            }))?
            .into_json()?;
        Ok(issue)
    }

    /// Closes a GitHub issue, optionally leaving a comment message.
    pub fn close_issue(
        token: &str,
        owner_repo: &str,
        number: u64,
        message: Option<&str>,
    ) -> Result<GitHubIssue> {
        if let Some(msg) = message {
            let comment_url = format!(
                "https://api.github.com/repos/{}/issues/{}/comments",
                owner_repo, number
            );
            let _ = ureq::post(&comment_url)
                .set("Authorization", &format!("Bearer {}", token))
                .set("Accept", "application/vnd.github.v3+json")
                .set("User-Agent", "Kitsu-VCS")
                .send_json(serde_json::json!({
                    "body": msg,
                }));
        }

        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            owner_repo, number
        );
        let issue: GitHubIssue = ureq::patch(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .send_json(serde_json::json!({
                "state": "closed",
            }))?
            .into_json()?;
        Ok(issue)
    }

    /// Reopens a closed GitHub issue.
    pub fn reopen_issue(token: &str, owner_repo: &str, number: u64) -> Result<GitHubIssue> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            owner_repo, number
        );
        let issue: GitHubIssue = ureq::patch(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .send_json(serde_json::json!({
                "state": "open",
            }))?
            .into_json()?;
        Ok(issue)
    }

    /// Lists pull requests from a GitHub repository.
    pub fn list_prs(token: &str, owner_repo: &str, state: Option<&str>) -> Result<Vec<GitHubPr>> {
        let state_param = state.unwrap_or("all");
        let url = format!(
            "https://api.github.com/repos/{}/pulls?state={}&per_page=50",
            owner_repo, state_param
        );
        let prs: Vec<GitHubPr> = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .call()?
            .into_json()?;
        Ok(prs)
    }

    /// Fetches a single pull request by number.
    pub fn get_pr(token: &str, owner_repo: &str, number: u64) -> Result<GitHubPr> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls/{}",
            owner_repo, number
        );
        let pr: GitHubPr = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .call()?
            .into_json()?;
        Ok(pr)
    }

    /// Opens a new pull request on GitHub.
    pub fn create_pr(
        token: &str,
        owner_repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<GitHubPr> {
        let url = format!("https://api.github.com/repos/{}/pulls", owner_repo);
        let pr: GitHubPr = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .send_json(serde_json::json!({
                "title": title,
                "body": body,
                "head": head,
                "base": base,
            }))?
            .into_json()?;
        Ok(pr)
    }

    /// Closes a pull request on GitHub.
    pub fn close_pr(token: &str, owner_repo: &str, number: u64) -> Result<GitHubPr> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls/{}",
            owner_repo, number
        );
        let pr: GitHubPr = ureq::patch(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "Kitsu-VCS")
            .send_json(serde_json::json!({
                "state": "closed",
            }))?
            .into_json()?;
        Ok(pr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_local_issue_lifecycle() {
        let dir = tempdir().unwrap();
        let repo_dir = dir.path().join(".kitsu");
        fs::create_dir_all(&repo_dir).unwrap();

        let i1 = LocalIssueManager::create(&repo_dir, "First bug", "Bug details", "UserA").unwrap();
        assert_eq!(i1.id, 1);
        assert_eq!(i1.state, "open");
        assert_eq!(i1.title, "First bug");

        let i2 = LocalIssueManager::create(&repo_dir, "Second bug", "More info", "UserB").unwrap();
        assert_eq!(i2.id, 2);

        let list = LocalIssueManager::list(&repo_dir).unwrap();
        assert_eq!(list.len(), 2);

        let closed = LocalIssueManager::close(&repo_dir, 1, Some("Fixed in rev 123")).unwrap();
        assert_eq!(closed.state, "closed");
        assert_eq!(closed.close_comment.as_deref(), Some("Fixed in rev 123"));

        let reopened = LocalIssueManager::reopen(&repo_dir, 1).unwrap();
        assert_eq!(reopened.state, "open");

        let deleted = LocalIssueManager::delete(&repo_dir, 2).unwrap();
        assert!(deleted);
        let list2 = LocalIssueManager::list(&repo_dir).unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].id, 1);
    }
}
