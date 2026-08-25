//! Content-addressable object types for the Kitsu object model.
//!
//! Kitsu stores three kinds of objects:
//! - [`Chunk`] — raw file content (analogous to git blob)
//! - [`Map`] — directory listing (analogous to git tree)
//! - [`Checkpoint`] — versioned snapshot (analogous to git commit)

mod checkpoint;
mod chunk;
mod map;

pub use checkpoint::Checkpoint;
pub use chunk::Chunk;
pub use map::{Map, MapEntry};
