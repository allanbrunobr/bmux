// src/security/hmac.rs — HMAC-SHA256 IPC message authentication
//
// Used by the message bus (wt4) to sign and verify all IPC messages,
// preventing tampering and injection attacks.
//
// Integration contract: wt4's MessageBus calls `sign_message` before sending
// and `verify_message` on receipt. wt5 provides this module.

use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use thiserror::Error;

// ─── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum HmacError {
    #[error("HMAC verification failed")]
    Verification,

    #[error("Invalid key length: {0} bytes (minimum 32 required)")]
    InvalidKeyLength(usize),

    #[error("Key generation failed")]
    KeyGeneration,
}

// ─── Public API ────────────────────────────────────────────────────────────────

/// Generate a cryptographically-random 32-byte HMAC key.
pub fn generate_key() -> Result<Vec<u8>, HmacError> {
    let rng = SystemRandom::new();
    let mut key_bytes = vec![0u8; 32];
    rng.fill(&mut key_bytes).map_err(|_| HmacError::KeyGeneration)?;
    Ok(key_bytes)
}

/// Sign `message` with `key_bytes` using HMAC-SHA256.
///
/// Returns the 32-byte tag. The tag must be transmitted alongside the message
/// and verified by the recipient with [`verify_message`].
pub fn sign_message(key_bytes: &[u8], message: &[u8]) -> Result<Vec<u8>, HmacError> {
    if key_bytes.len() < 32 {
        return Err(HmacError::InvalidKeyLength(key_bytes.len()));
    }
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
    let tag = hmac::sign(&key, message);
    Ok(tag.as_ref().to_vec())
}

/// Verify that `tag` is a valid HMAC-SHA256 signature of `message` under `key_bytes`.
///
/// Returns `Ok(())` on success, `Err(HmacError::Verification)` on mismatch.
/// Uses constant-time comparison to prevent timing attacks.
pub fn verify_message(
    key_bytes: &[u8],
    message: &[u8],
    tag: &[u8],
) -> Result<(), HmacError> {
    if key_bytes.len() < 32 {
        return Err(HmacError::InvalidKeyLength(key_bytes.len()));
    }
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
    hmac::verify(&key, message, tag).map_err(|_| HmacError::Verification)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key() -> Vec<u8> {
        vec![0x42u8; 32]
    }

    #[test]
    fn test_generate_key_returns_32_bytes() {
        let key = generate_key().expect("key generation should succeed");
        assert_eq!(key.len(), 32, "generated key must be 32 bytes");
    }

    #[test]
    fn test_generate_key_is_random() {
        let k1 = generate_key().unwrap();
        let k2 = generate_key().unwrap();
        // Astronomically unlikely to collide with a secure PRNG
        assert_ne!(k1, k2, "two generated keys should differ");
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let key = sample_key();
        let message = b"hello from the message bus";
        let tag = sign_message(&key, message).expect("sign should succeed");
        assert!(
            verify_message(&key, message, &tag).is_ok(),
            "verify should succeed for a freshly signed message"
        );
    }

    #[test]
    fn test_verify_fails_with_wrong_key() {
        let key1 = sample_key();
        let key2 = vec![0xFFu8; 32];
        let message = b"important payload";
        let tag = sign_message(&key1, message).unwrap();
        assert!(
            verify_message(&key2, message, &tag).is_err(),
            "verify with wrong key must fail"
        );
    }

    #[test]
    fn test_verify_fails_with_tampered_message() {
        let key = sample_key();
        let message = b"original message";
        let tag = sign_message(&key, message).unwrap();

        let tampered = b"tampered message";
        assert!(
            verify_message(&key, tampered, &tag).is_err(),
            "verify with tampered message must fail"
        );
    }

    #[test]
    fn test_verify_fails_with_truncated_tag() {
        let key = sample_key();
        let message = b"test";
        let tag = sign_message(&key, message).unwrap();
        let truncated = &tag[..16];
        assert!(
            verify_message(&key, message, truncated).is_err(),
            "verify with truncated tag must fail"
        );
    }

    #[test]
    fn test_short_key_returns_error() {
        let short_key = vec![0u8; 16]; // 16 bytes < required 32
        let result = sign_message(&short_key, b"test");
        assert!(
            matches!(result, Err(HmacError::InvalidKeyLength(16))),
            "short key should return InvalidKeyLength"
        );
    }

    #[test]
    fn test_sign_empty_message() {
        let key = sample_key();
        let tag = sign_message(&key, b"").expect("signing empty message should succeed");
        assert!(
            verify_message(&key, b"", &tag).is_ok(),
            "verifying empty-message tag should succeed"
        );
    }

    #[test]
    fn test_sign_large_message() {
        let key = generate_key().unwrap();
        let large_msg = vec![0xABu8; 64 * 1024]; // 64 KB
        let tag = sign_message(&key, &large_msg).expect("signing large message should succeed");
        assert!(
            verify_message(&key, &large_msg, &tag).is_ok(),
            "verifying large message tag should succeed"
        );
    }
}
