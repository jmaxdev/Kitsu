# Identity and Cryptography

Kitsu incorporates first-class identity persona management and cryptographic checkpoint signing using the Ed25519 digital signature algorithm.

---

## 1. Persona System

A **Persona** represents an author identity with associated cryptographic keys.

### Data Model
```rust
pub struct Identity {
    pub id: String,                  // Short identifier, e.g., "work", "personal"
    pub name: String,                // Display name, e.g., "Alice Smith"
    pub email: String,               // Contact email, e.g., "alice@example.com"
    pub public_key: Option<Vec<u8>>, // 32-byte Ed25519 public key
    pub private_key: Option<Vec<u8>>,// 32-byte Ed25519 secret key
}

pub struct IdentityStore {
    pub identities: Vec<Identity>,
    pub active_id: String,
}
```

---

## 2. Configuration Resolution Order

When resolving the active identity, Kitsu inspects configurations in the following hierarchy:

1. **Local Repository Config**: `.kitsu/identity.toml` in the repository root.
2. **Global User Config**: `~/.kitsu_identity.toml` in the user's home directory.
3. **Default Fallback**: Automatically generates a `"default"` persona with a new Ed25519 keypair if no configuration exists.

### TOML Format Example
```toml
active_id = "work"

[[identities]]
id = "work"
name = "Jane Developer"
email = "jane@company.com"
public_key = [142, 210, 45, ...]
private_key = [12, 88, 204, ...]

[[identities]]
id = "personal"
name = "Example"
email = "12345+example@users.noreply.github.com"
public_key = [33, 91, 108, ...]
private_key = [201, 4, 19, ...]
```

---

## 3. Persona Lifecycle & Deletion

- **Adding Personas**: `kitsu persona add <id> <name> <email> [-g/--global]`
- **Switching Personas**: `kitsu persona use <id> [-g/--global]`
- **Editing Personas**: `kitsu persona edit <id> [-n <name>] [-e <email>] [-g/--global]`
- **Removing Personas**: `kitsu persona remove <id> [-g/--global]` (or `delete`)
  - **Automatic Fallback**: If the active persona (`active_id`) is removed, Kitsu automatically sets the next available persona as active, or regenerates a new `"default"` persona if no identities remain.

---

## 4. GitHub OAuth Authentication & Verified Noreply Email

Kitsu provides native GitHub integration via `kitsu persona github auth`:
- **OAuth Callback Flow**: Initiates a browser authentication request redirecting to `http://localhost:5911/api/v1/github/auth`.
- **Token Management**: Persists access and refresh tokens in `~/.kitsu/github_credentials.toml`. Automatically refreshes expired tokens using `refresh_token`.
- **Accurate Noreply Email**: Queries GitHub's `/user` and `/user/emails` endpoints to resolve:
  - Official GitHub Privacy Noreply Email: `{id}+{login}@users.noreply.github.com` (e.g. `251231531+jmaxdev@users.noreply.github.com`).
  - Or the user's primary verified email address.
- **Fallback Authentication**: Interactive prompt for GitHub Personal Access Tokens (PAT).

---

## 5. Cryptographic Operations

### Keypair Generation
- Uses the `ed25519-dalek` crate with entropy sourced from the operating system via `rand_core::OsRng`.
- Generates a 32-byte secret signing key (`SigningKey`) and a corresponding 32-byte public verifying key (`VerifyingKey`).

### Checkpoint Signing Workflow
When `kitsu freeze --sign` is executed:
1. The checkpoint header and message are serialized into canonical wire bytes.
2. The active persona's 32-byte private key signs the serialized bytes using Ed25519.
3. The resulting 64-byte signature is encoded as a 128-character hexadecimal string.
4. The `signature <hex>` header line is appended to the checkpoint before hashing and persisting to storage.

### Signature Verification
To verify a checkpoint signature:
```rust
let is_valid = kitsu_core::identity::verify_signature(
    &public_key_bytes,
    &serialized_checkpoint_payload,
    &signature_hex,
)?;
```
- During `kitsu timeline`, signatures are validated and visually flagged as `VALID` or `NONE`.

