use ed25519_dalek::{Signer, Verifier, VerifyingKey};

use crate::error::PktapError;

/// Sign `message` with the provided `SigningKey`, returning the 64-byte signature.
///
/// This function is used in tests and for signature verification on received records.
/// Per D-03: production signing uses Android Keystore (non-extractable Ed25519 key);
/// the Rust `SigningKey` is only used in unit tests and for in-Rust signature operations.
pub(crate) fn sign_bytes(signing_key: &ed25519_dalek::SigningKey, message: &[u8]) -> Vec<u8> {
    let signature = signing_key.sign(message);
    signature.to_bytes().to_vec()
}

/// Verify a 64-byte Ed25519 `signature` over `message` using `verifying_key_bytes`.
///
/// Returns `Ok(())` if the signature is valid.
/// Returns `Err(PktapError::InvalidKey)` if the verifying key bytes are malformed or
/// represent the identity/neutral element (all-zero bytes).
/// Returns `Err(PktapError::RecordInvalid)` if the signature bytes are invalid length
/// or the signature verification fails.
///
/// Uses constant-time comparison internally (ed25519-dalek 2.x uses `subtle::ConstantTimeEq`
/// for tag comparison) — T-01-07 threat mitigation.
pub(crate) fn verify_signature(
    verifying_key_bytes: &[u8; 32],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<(), PktapError> {
    // Explicitly reject all-zero bytes — ed25519-dalek 2.x accepts the identity point,
    // but it is a degenerate key that must never be used for verification.
    if verifying_key_bytes == &[0u8; 32] {
        return Err(PktapError::InvalidKey);
    }

    // Parse verifying key.
    let verifying_key =
        VerifyingKey::from_bytes(verifying_key_bytes).map_err(|_| PktapError::InvalidKey)?;

    // Parse signature — must be exactly 64 bytes.
    if signature_bytes.len() != 64 {
        return Err(PktapError::RecordInvalid);
    }
    let sig_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| PktapError::RecordInvalid)?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    // Verify — returns RecordInvalid on any verification failure.
    verifying_key
        .verify(message, &signature)
        .map_err(|_| PktapError::RecordInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use hex_literal::hex;

    // -----------------------------------------------------------------------
    // Round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sign_verify_round_trip() {
        // Generate a keypair from a fixed seed for determinism.
        let seed = [0x42u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();

        let message = b"hello pktap signing";
        let sig = sign_bytes(&signing_key, message);

        assert_eq!(sig.len(), 64, "Ed25519 signature must be 64 bytes");

        let result = verify_signature(&verifying_key_bytes, message, &sig);
        assert!(
            result.is_ok(),
            "Valid signature must verify successfully: {:?}",
            result
        );
    }

    #[test]
    fn test_sign_verify_empty_message() {
        // Edge case: sign/verify an empty message.
        let seed = [0x01u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();

        let sig = sign_bytes(&signing_key, b"");
        let result = verify_signature(&verifying_key_bytes, b"", &sig);
        assert!(result.is_ok(), "Empty message sign/verify must succeed");
    }

    // -----------------------------------------------------------------------
    // Tampered message rejection
    // -----------------------------------------------------------------------

    #[test]
    fn test_tampered_message_rejected() {
        // Signing a message then verifying a different message must fail.
        let seed = [0x11u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();

        let message = b"original message";
        let sig = sign_bytes(&signing_key, message);

        let tampered = b"tampered message";
        let result = verify_signature(&verifying_key_bytes, tampered, &sig);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Tampered message must return RecordInvalid"
        );
    }

    #[test]
    fn test_wrong_public_key_rejected() {
        // Valid signature, valid message, but wrong verifying key must fail.
        let seed_a = [0x22u8; 32];
        let seed_b = [0x33u8; 32];
        let signing_key_a = SigningKey::from_bytes(&seed_a);
        let signing_key_b = SigningKey::from_bytes(&seed_b);

        let message = b"key mismatch test";
        let sig = sign_bytes(&signing_key_a, message);

        // Verify with B's key — must fail.
        let wrong_key_bytes: [u8; 32] = signing_key_b.verifying_key().to_bytes();
        let result = verify_signature(&wrong_key_bytes, message, &sig);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Wrong public key must return RecordInvalid"
        );
    }

    #[test]
    fn test_invalid_signature_length_rejected() {
        // Signature bytes of wrong length must return RecordInvalid (not panic).
        let seed = [0x44u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();

        let short_sig = vec![0u8; 32]; // only 32 bytes instead of 64
        let result = verify_signature(&verifying_key_bytes, b"test", &short_sig);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Short signature must return RecordInvalid"
        );
    }

    #[test]
    fn test_invalid_verifying_key_rejected() {
        // All-zero verifying key bytes (invalid Edwards point) must return InvalidKey.
        let result = verify_signature(&[0u8; 32], b"test", &[0u8; 64]);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "All-zero verifying key must return InvalidKey"
        );
    }

    // -----------------------------------------------------------------------
    // RFC 8032 section 5.1 Known Answer Test — Test Vector 1 (empty message)
    //
    // Private key seed: 9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60
    // Public key:       d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a
    // Message:          (empty)
    // Signature (64B):  e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155
    //                   5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b
    //
    // Note: ed25519-dalek 2.x derives the public key from the seed via SHA-512 expansion.
    // The public key above was verified against the actual library output.
    // -----------------------------------------------------------------------

    #[test]
    fn test_kat_rfc8032_test1_empty_message() {
        // RFC 8032 §5.1 Test vector 1 — empty message.
        let private_seed: [u8; 32] =
            hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let expected_pub: [u8; 32] =
            hex!("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let expected_sig: [u8; 64] = hex!(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155"
            "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );

        let signing_key = SigningKey::from_bytes(&private_seed);

        // Verify the public key matches the expected value.
        let actual_pub: [u8; 32] = signing_key.verifying_key().to_bytes();
        assert_eq!(
            actual_pub, expected_pub,
            "Public key derived from RFC 8032 test seed must match expected value"
        );

        // Sign the empty message.
        let sig_bytes = sign_bytes(&signing_key, b"");
        assert_eq!(
            sig_bytes.as_slice(),
            &expected_sig,
            "Signature of empty message must match RFC 8032 §5.1 Test 1 vector"
        );

        // Verify round-trip.
        let result = verify_signature(&expected_pub, b"", &sig_bytes);
        assert!(
            result.is_ok(),
            "RFC 8032 Test 1 signature must verify correctly"
        );
    }
}
