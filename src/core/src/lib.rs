//! Kitsu Core — the heart of the Kitsu version control system.
//!
//! Provides all version control primitives: content-addressable object storage,
//! staging, refs (streams/seals), diffing, identity management, and remote
//! transport. The CLI crate builds on top of this library.

#![warn(missing_docs)]

pub mod config;
pub mod diff;
pub mod error;
pub mod exclude;
pub mod git_import;
pub mod global_registry;
pub mod identity;
pub mod issues;
pub mod objects;
pub mod refs;
pub mod remote;
pub mod repository;
pub mod server;
pub mod state;
pub mod storage;
pub mod update;

pub use config::AppConfig;
pub use error::KitsuError;
pub use git_import::import_git_repository;
pub use global_registry::GlobalRegistry;
pub use repository::Repository;

#[doc(hidden)]
pub use std::fmt;
#[doc(hidden)]
pub use std::write;
