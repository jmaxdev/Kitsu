use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;

/// Generates a fresh Ed25519 keypair using OS-level entropy.
///
/// Returns `(private_key_bytes, public_key_bytes)` suitable for
/// storage in an [`super::Identity`].
pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (
        signing_key.to_bytes().to_vec(),
        verifying_key.to_bytes().to_vec(),
    )
}

/// Signs arbitrary data with a private Ed25519 key.
///
/// # Errors
/// Returns an error if `private_key_bytes` is not exactly 32 bytes
/// or cannot be interpreted as a valid Ed25519 secret key.
pub fn sign_data(private_key_bytes: &[u8], data: &[u8]) -> anyhow::Result<String> {
    let bytes: [u8; 32] = private_key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let signing_key = SigningKey::from_bytes(&bytes);
    let signature = signing_key.sign(data);
    Ok(hex::encode(signature.to_bytes()))
}

/// Verifies an Ed25519 signature against public key and data.
///
/// # Errors
/// Returns an error if the key or signature bytes are malformed.
#[allow(dead_code)]
pub fn verify_signature(
    public_key_bytes: &[u8],
    data: &[u8],
    signature_hex: &str,
) -> anyhow::Result<bool> {
    let sig_bytes = hex::decode(signature_hex)?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
    let signature = Signature::from_bytes(&sig_arr);
    let pub_arr: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
    let verifying_key = VerifyingKey::from_bytes(&pub_arr)?;
    Ok(verifying_key.verify_strict(data, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_sign_and_verify_roundtrip() {
        let (priv_key, pub_key) = generate_keypair();
        assert_eq!(priv_key.len(), 32);
        assert_eq!(pub_key.len(), 32);

        let data = b"kitsu checkpoint payload to sign";
        let sig = sign_data(&priv_key, data).unwrap();
        assert_eq!(sig.len(), 128); // 64 bytes in hex

        let is_valid = verify_signature(&pub_key, data, &sig).unwrap();
        assert!(is_valid);

        let is_invalid = verify_signature(&pub_key, b"tampered data", &sig).unwrap();
        assert!(!is_invalid);
    }
}
