//! Remote transport and registry management.
//!
//! Supports three transport backends:
//! - **Git bridge** — for GitHub/GitLab via the `git2` crate with configurable data branch
//! - **Local filesystem** — for local directories, backup drives, and offline repositories
//! - **SSH/SFTP** — for sovereign (self-hosted) remote registries

mod git_bridge;
mod local_bridge;
mod registry;
mod transport;

pub use git_bridge::GitBridge;
pub use local_bridge::LocalBridge;
pub use registry::{RemoteEntry, RemoteRegistry, default_remote_name, is_git_url, is_local_path};
pub use transport::SshTransport;
