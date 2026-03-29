// tests/security_hmac_test.rs — Integration tests for HMAC (Story 6.x)

use bmux::security::hmac::{generate_key, sign_message, verify_message, HmacError};

#[test]
fn integration_hmac_full_sign_verify_cycle() {
    let key = generate_key().expect("key generation must succeed");
    let message = b"bmux-ipc-message-payload";

    let tag = sign_message(&key, message).expect("signing must succeed");
    assert!(
        verify_message(&key, message, &tag).is_ok(),
        "fresh sign-then-verify must succeed"
    );
}

#[test]
fn integration_hmac_rejects_different_key() {
    let key1 = generate_key().unwrap();
    let key2 = generate_key().unwrap();
    assert_ne!(key1, key2, "two random keys must differ");

    let tag = sign_message(&key1, b"payload").unwrap();
    assert!(
        verify_message(&key2, b"payload", &tag).is_err(),
        "verification with wrong key must fail"
    );
}

#[test]
fn integration_hmac_rejects_bit_flip_in_message() {
    let key = generate_key().unwrap();
    let mut msg = b"original message".to_vec();
    let tag = sign_message(&key, &msg).unwrap();

    // Flip one bit in the message
    msg[0] ^= 0x01;
    assert!(
        verify_message(&key, &msg, &tag).is_err(),
        "verification with bit-flipped message must fail"
    );
}

#[test]
fn integration_hmac_rejects_bit_flip_in_tag() {
    let key = generate_key().unwrap();
    let msg = b"message";
    let mut tag = sign_message(&key, msg).unwrap();

    // Flip one bit in the tag
    tag[0] ^= 0x01;
    assert!(
        verify_message(&key, msg, &tag).is_err(),
        "verification with bit-flipped tag must fail"
    );
}

#[test]
fn integration_hmac_short_key_is_rejected() {
    let short_key = vec![0u8; 31]; // 31 bytes is below 32-byte minimum
    let result = sign_message(&short_key, b"test");
    assert!(
        matches!(result, Err(HmacError::InvalidKeyLength(31))),
        "31-byte key must return InvalidKeyLength(31)"
    );
}
