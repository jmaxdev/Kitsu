//! Structured error types for all Kitsu operations.

use thiserror::Error;

/// Typed error variants for all Kitsu operations.
///
/// Provides structured error context instead of opaque string messages,
/// enabling callers to match on specific failure modes and present
/// actionable diagnostics.
#[derive(Error, Debug)]
pub enum KitsuError {
    /// Referenced object hash does not exist in the object store.
    #[error("object not found: {hash}")]
    ObjectNotFound {
        /// The SHA-256 hash that was looked up.
        hash: String,
    },

    /// Stored object data does not conform to the expected wire format.
    #[error("invalid object format")]
    InvalidObjectFormat,

    /// Object data failed integrity validation.
    #[error("corrupt object: {hash}")]
    CorruptObject {
        /// The SHA-256 hash of the corrupt object.
        hash: String,
    },

    /// No HEAD reference exists (empty repository).
    #[error("no HEAD reference found")]
    NoHead,

    /// The current checkpoint has no parent (already at root).
    #[error("no parent checkpoint")]
    NoParent,

    /// Named stream (branch) does not exist.
    #[error("stream not found: {name}")]
    StreamNotFound {
        /// The stream name that was looked up.
        name: String,
    },

    /// Named seal (tag) does not exist.
    #[error("seal not found: {name}")]
    SealNotFound {
        /// The seal name that was looked up.
        name: String,
    },

    /// SSH or credential-based authentication failed.
    #[error("authentication failed for user: {user}")]
    AuthenticationFailed {
        /// The username used in the failed attempt.
        user: String,
    },

    /// A remote transport operation failed.
    #[error("remote error: {0}")]
    RemoteError(String),

    /// No identity with the given ID exists in the store.
    #[error("identity not found: {id}")]
    IdentityNotFound {
        /// The persona ID that was looked up.
        id: String,
    },

    /// Signing was requested but the active identity has no private key.
    #[error("no private key available for signing")]
    NoPrivateKey,

    /// Numeric index exceeded the history depth.
    #[error("index out of bounds: {index} (max: {max})")]
    IndexOutOfBounds {
        /// The requested index.
        index: usize,
        /// The maximum valid index.
        max: usize,
    },

    /// No Kitsu repository found at the given path.
    #[error("repository not found at {path}")]
    RepositoryNotFound {
        /// The path that was inspected.
        path: String,
    },

    /// A repository already exists at the target path.
    #[error("repository already exists at {path}")]
    RepositoryAlreadyExists {
        /// The conflicting path.
        path: String,
    },

    /// Object type header contains an unrecognized type string.
    #[error("unknown object type: {0}")]
    UnknownObjectType(String),
}
