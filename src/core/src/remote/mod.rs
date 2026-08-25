//! Remote transport and registry management.
//!
//! Supports two transport backends:
//! - **SSH/SFTP** — for sovereign (self-hosted) registries
//! - **Git bridge** — for GitHub/GitLab via the `git2` crate

mod git_bridge;
mod registry;
mod transport;

pub use git_bridge::GitBridge;
pub use registry::{RemoteRegistry, default_remote_name, is_git_url};
pub use transport::SshTransport;
