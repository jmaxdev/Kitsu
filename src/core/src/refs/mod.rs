//! Reference management: HEAD resolution, streams (branches), and seals (tags).

mod head;
mod seal;
mod stream;

pub use head::{get_head_hash, resolve_target};
pub use seal::{bump_version, create_seal, list_seals};
pub use stream::{create_stream, delete_stream, list_streams, rename_stream};
