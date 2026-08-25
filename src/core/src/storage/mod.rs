//! Content-addressable object storage and staging area.
//!
//! The storage layer handles persisting, reading, and hashing Kitsu objects
//! using SHA-256 content addressing with zlib compression. The staging
//! index tracks which files are queued for the next checkpoint.

mod backend;
mod index;

pub use backend::{ObjectType, Storage};
pub use index::{Stage, StageEntry};
