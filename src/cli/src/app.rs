use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kitsu — a modern version control system.
#[derive(Parser)]
#[command(
    name = "kitsu",
    about = "A modern version control system written in Rust",
    author,
    version
)]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level Kitsu commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Kitsu repository.
    Ignite,
    /// Clone a repository from a remote URL.
    Copy {
        /// Remote URL to clone from.
        url: String,
        /// Local directory name (defaults to repo name).
        directory: Option<PathBuf>,
    },
    /// Stage files for the next checkpoint.
    Track {
        /// Files to stage.
        files: Vec<PathBuf>,
    },
    /// Create a new checkpoint from staged changes.
    Freeze {
        /// Checkpoint message.
        #[arg(short = 'm')]
        message: String,
        /// Sign the checkpoint with the active persona's Ed25519 key.
        #[arg(short = 'S', long)]
        sign: bool,
    },
    /// Show the checkpoint history.
    Timeline,
    /// Show differences between checkpoints or working tree.
    Diff {
        /// Old checkpoint/stream/seal reference.
        old: Option<String>,
        /// New checkpoint/stream/seal reference.
        new: Option<String>,
    },
    /// Roll back to a previous checkpoint.
    Rollback {
        /// Target checkpoint (hash, stream, seal, ~N, #N). Defaults to parent.
        target: Option<String>,
    },
    /// Create or list version seals (tags).
    Seal {
        /// Explicit semver version string (e.g., "1.0.0", "0.0.4-alpha", "0.0.4-rc.1").
        version: Option<String>,
        /// Auto-bump the version (major, minor, patch, alpha, beta, rc, alpha.0, etc.).
        #[arg(short = 'b', long)]
        bump: Option<String>,
        /// List all existing seals.
        #[arg(short = 'l', long)]
        list: bool,
    },
    /// Switch to a different checkpoint or stream.
    Switch {
        /// Target stream name, seal, or checkpoint hash.
        target: String,
    },
    /// Export objects to a portable archive.
    Export {
        /// Target reference to export.
        target: String,
        /// Output file path.
        output: PathBuf,
    },
    /// Import objects from a portable archive.
    Import {
        /// Input archive file path.
        input: PathBuf,
    },
    /// Push objects to a remote registry.
    Push {
        /// Remote name (defaults to the configured default).
        remote: Option<String>,
        /// Target stream/seal to push.
        target: Option<String>,
        /// Custom remote data branch (defaults to configured branch or "kitsu-data").
        #[arg(short = 'b', long)]
        branch: Option<String>,
    },
    /// Pull objects from a remote registry.
    Pull {
        /// Remote name (defaults to the configured default).
        remote: Option<String>,
        /// Target reference to pull.
        target: Option<String>,
        /// Custom remote data branch (defaults to configured branch or "kitsu-data").
        #[arg(short = 'b', long)]
        branch: Option<String>,
    },
    /// Show contents of a checkpoint's file tree.
    Contents {
        /// Checkpoint reference (defaults to HEAD).
        target: Option<String>,
    },
    /// Compute the SHA-256 hash of a file.
    Hash {
        /// File to hash.
        file: PathBuf,
    },
    /// Repository management and inspection commands.
    Repository {
        /// Repository subcommand.
        #[command(subcommand)]
        action: RepoAction,
    },
    /// Manage identity personas.
    Persona {
        /// Persona subcommand. Omit to show the active persona.
        #[command(subcommand)]
        action: Option<PersonaAction>,
    },
    /// Delete objects from the store.
    Burn {
        /// Object hash to delete (defaults to HEAD).
        hash: Option<String>,
        /// Run aggressive cleanup.
        #[arg(short = 'a', long)]
        aggressive: bool,
    },
    /// Show working tree status.
    State,
    /// Inspect raw object content by hash.
    Peek {
        /// Object hash.
        hash: String,
    },
    /// Update Kitsu to the latest version.
    Update {
        /// Specific version or tag to install (e.g., "0.0.4-alpha").
        #[arg(short = 't', long)]
        tag: Option<String>,
        /// Check for available updates without installing.
        #[arg(short = 'c', long)]
        check: bool,
    },
}

/// Persona management subcommands.
#[derive(Subcommand)]
pub enum PersonaAction {
    /// Add a new persona.
    Add {
        /// Short identifier (e.g., "work").
        id: String,
        /// Display name.
        name: String,
        /// Email address.
        email: String,
        /// Store globally instead of per-repository.
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// List all personas.
    List,
    /// Set the active persona.
    Use {
        /// Persona ID to activate.
        id: String,
        /// Set globally.
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Edit an existing persona's fields.
    Edit {
        /// Persona ID to edit.
        id: String,
        /// New display name.
        #[arg(short = 'n', long)]
        name: Option<String>,
        /// New email address.
        #[arg(short = 'e', long)]
        email: Option<String>,
        /// Save globally.
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Import persona from a GitHub username.
    Github {
        /// GitHub username.
        username: String,
        /// Persona ID (defaults to the username).
        id: Option<String>,
        /// Store globally.
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Regenerate signing keys for the active persona.
    Keys,
}

/// Repository management subcommands.
#[derive(Subcommand)]
pub enum RepoAction {
    /// Show repository metadata.
    Info,
    /// Show storage statistics.
    Stats,
    /// Verify object store integrity.
    Verify,
    /// Clean up unreachable objects.
    Vacuum,
    /// Manage remote registries.
    Remote {
        /// Remote subcommand.
        #[command(subcommand)]
        action: RemoteAction,
    },
    /// Manage streams (branches).
    Stream {
        /// Stream subcommand.
        #[command(subcommand)]
        action: StreamAction,
    },
}

/// Remote registry management subcommands.
#[derive(Subcommand)]
pub enum RemoteAction {
    /// Add a new remote.
    Add {
        /// Remote name.
        name: String,
        /// Remote URL or local filesystem path.
        url: String,
        /// Custom remote data branch name (defaults to "kitsu-data" for Git remotes).
        #[arg(short = 'b', long)]
        branch: Option<String>,
    },
    /// Edit an existing remote's URL and branch.
    Edit {
        /// Remote name.
        name: String,
        /// New URL or local filesystem path.
        url: String,
        /// Custom remote data branch name.
        #[arg(short = 'b', long)]
        branch: Option<String>,
    },
    /// Set the default remote.
    Default {
        /// Remote name.
        name: String,
    },
    /// List all remotes.
    List,
    /// Remove a remote.
    Remove {
        /// Remote name.
        name: String,
    },
}

/// Stream (branch) management subcommands.
#[derive(Subcommand)]
pub enum StreamAction {
    /// Create a new stream from HEAD.
    New {
        /// Stream name.
        name: String,
    },
    /// List all streams.
    List,
    /// Rename a stream.
    Rename {
        /// Current name.
        old: String,
        /// New name.
        new: String,
    },
    /// Delete a stream.
    Delete {
        /// Stream name.
        name: String,
    },
}
