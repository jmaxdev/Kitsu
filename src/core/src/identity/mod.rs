//! Identity management: personas and Ed25519 cryptographic operations.

mod crypto;
mod persona;

pub use crypto::{generate_keypair, sign_data, verify_signature};
pub use persona::{Identity, IdentityStore};
