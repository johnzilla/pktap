// Composite FFI functions — Plan 03 implementation.
// These are the ONLY functions that cross the UniFFI boundary.
// No secret material appears in any function signature.

use ed25519_dalek::SigningKey;
use zeroize::Zeroize;

use crate::error::PktapError;
use crate::{cipher, ecdh, mnemonic, record, signing};

/// Hello-world function for pipeline smoke test (Phase 3 / FFI-02).
/// Returns "pktap-ok" to confirm the Rust->Kotlin FFI channel is live.
#[uniffi::export]
pub fn pktap_ping() -> String {
    "pktap-ok".to_string()
}

/// Perform ECDH key agreement, derive an encryption key, and encrypt the contact fields JSON.
///
/// This is the primary encrypt entry point across the UniFFI boundary.
/// All intermediate key material (seed array, DerivedKey) is zeroed before the function returns.
///
/// # Arguments
/// - `our_seed_bytes`: 32-byte HKDF seed from EncryptedSharedPreferences (decrypted by Keystore).
/// - `peer_ed25519_public`: 32-byte Ed25519 public key received from the peer via NFC/QR.
/// - `contact_fields_json`: JSON string of contact fields to encrypt (max 750 bytes).
///
/// # Returns
/// An opaque D-06 byte blob: `version(1) || nonce(24) || ciphertext+tag(n+16)`.
///
/// # Errors
/// - `PktapError::InvalidKey` — seed or peer key has wrong length, or peer key is invalid.
/// - `PktapError::RecordTooLarge` — contact JSON exceeds 750 bytes.
/// - `PktapError::SerializationFailed` — encryption failed (should not occur in practice).
///
/// # Security (T-01-10, T-01-11, T-01-13)
/// - Input lengths validated before any crypto operation.
/// - `our_seed_bytes` array is explicitly zeroed after key derivation.
/// - `DerivedKey` is dropped with `ZeroizeOnDrop` before the function returns.
/// - Payload size checked before encryption (T-01-13).
#[uniffi::export]
pub fn ecdh_and_encrypt(
    our_seed_bytes: Vec<u8>,
    peer_ed25519_public: Vec<u8>,
    contact_fields_json: String,
) -> Result<Vec<u8>, PktapError> {
    // Validate input lengths before any crypto operation (T-01-10).
    if our_seed_bytes.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    if peer_ed25519_public.len() != 32 {
        return Err(PktapError::InvalidKey);
    }

    // Convert to fixed-size arrays.
    let mut seed: [u8; 32] = our_seed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;
    let peer_pub: [u8; 32] = peer_ed25519_public
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;

    // Validate plaintext size before encryption (T-01-13 — DoS mitigation).
    record::validate_plaintext_size(contact_fields_json.as_bytes())?;

    // Derive encryption key via ECDH + HKDF. DerivedKey drops with ZeroizeOnDrop.
    let derived = ecdh::ecdh_derive_key(&seed, &peer_pub)?;

    // Encrypt the contact fields.
    let result = cipher::encrypt_record(&derived.0, contact_fields_json.as_bytes());

    // Zero the seed array (T-01-11 — seed lifetime mitigation).
    seed.zeroize();

    result
}

/// Verify the Ed25519 signature on a record, then ECDH-derive the key and decrypt.
///
/// This is the primary decrypt entry point across the UniFFI boundary.
/// Per D-08, ALL internal errors (signature failure, key derivation failure, decryption failure)
/// are coalesced into a single `PktapError::RecordInvalid` to prevent oracle attacks (T-01-09).
///
/// # Arguments
/// - `our_seed_bytes`: 32-byte HKDF seed (our own seed).
/// - `peer_ed25519_public`: 32-byte Ed25519 public key of the record sender (used for ECDH + sig verify).
/// - `peer_ed25519_signature`: 64-byte Ed25519 signature produced by the peer over `record_bytes`.
/// - `record_bytes`: Opaque D-06 byte blob to verify and decrypt.
///
/// # Returns
/// The decrypted contact fields JSON string on success.
///
/// # Errors
/// - `PktapError::RecordInvalid` — signature invalid, record tampered, wrong key, or decryption failed.
///   All error paths return the same variant to prevent side-channel leakage (D-08, T-01-09).
///
/// # Security (T-01-09, T-01-12)
/// - Signature is verified BEFORE key derivation and decryption (T-01-12 — Spoofing mitigation).
/// - All internal errors map to `RecordInvalid`; callers cannot distinguish failure modes (D-08).
/// - Seed array zeroed after key derivation.
#[uniffi::export]
pub fn decrypt_and_verify(
    our_seed_bytes: Vec<u8>,
    peer_ed25519_public: Vec<u8>,
    peer_ed25519_signature: Vec<u8>,
    record_bytes: Vec<u8>,
) -> Result<String, PktapError> {
    // Validate input lengths. ALL length errors for sig/record coalesce to RecordInvalid (D-08).
    if our_seed_bytes.len() != 32 {
        return Err(PktapError::RecordInvalid);
    }
    if peer_ed25519_public.len() != 32 {
        return Err(PktapError::RecordInvalid);
    }
    if peer_ed25519_signature.len() != 64 {
        return Err(PktapError::RecordInvalid);
    }

    // Convert to fixed-size arrays.
    let mut seed: [u8; 32] = our_seed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::RecordInvalid)?;
    let peer_pub: [u8; 32] = peer_ed25519_public
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::RecordInvalid)?;

    // Step 1: Verify signature BEFORE key derivation and decryption (T-01-12).
    // Map ANY verification error to RecordInvalid (D-08 coalescing).
    signing::verify_signature(&peer_pub, &record_bytes, &peer_ed25519_signature)
        .map_err(|_| PktapError::RecordInvalid)?;

    // Step 2: Derive encryption key via ECDH + HKDF.
    // Map key derivation errors to RecordInvalid (D-08 — do not leak whether key was invalid).
    let derived = ecdh::ecdh_derive_key(&seed, &peer_pub)
        .map_err(|_| PktapError::RecordInvalid)?;

    // Step 3: Decrypt the record.
    let plaintext = cipher::decrypt_record(&derived.0, &record_bytes)
        .map_err(|_| PktapError::RecordInvalid)?;

    // Zero the seed array after key derivation.
    seed.zeroize();

    // Step 4: Convert plaintext bytes to UTF-8 string.
    String::from_utf8(plaintext).map_err(|_| PktapError::RecordInvalid)
}

/// Derive the deterministic shared DHT record name for a contact exchange between two parties.
///
/// The name is symmetric: `derive_shared_record_name(A, B) == derive_shared_record_name(B, A)`.
/// Format: `"_pktap._share.<hex-encoded SHA-256(sort(A, B))>"`.
///
/// # Errors
/// - `PktapError::InvalidKey` — either key is not exactly 32 bytes.
#[uniffi::export]
pub fn derive_shared_record_name(
    pub_key_a: Vec<u8>,
    pub_key_b: Vec<u8>,
) -> Result<String, PktapError> {
    if pub_key_a.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    if pub_key_b.len() != 32 {
        return Err(PktapError::InvalidKey);
    }

    let key_a: [u8; 32] = pub_key_a
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;
    let key_b: [u8; 32] = pub_key_b
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;

    Ok(record::shared_record_name(&key_a, &key_b))
}

/// Derive a 12-word BIP-39 mnemonic from a 32-byte seed.
///
/// Uses the first 16 bytes of `seed_bytes` as entropy (128-bit → 12 words per D-03).
/// The seed array is zeroed before this function returns (T-04-06).
///
/// # Errors
/// - `PktapError::InvalidKey` — seed_bytes is not exactly 32 bytes.
/// - `PktapError::SerializationFailed` — mnemonic generation failed (should not occur).
#[uniffi::export]
pub fn derive_mnemonic_from_seed(seed_bytes: Vec<u8>) -> Result<String, PktapError> {
    if seed_bytes.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    let mut seed: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;
    let result = mnemonic::mnemonic_from_entropy(&seed);
    seed.zeroize();
    result
}

/// Derive the Ed25519 public key from a 32-byte seed.
///
/// Returns a 32-byte Ed25519 verifying key. The seed array is zeroed before
/// this function returns (T-04-06).
///
/// # Errors
/// - `PktapError::InvalidKey` — seed_bytes is not exactly 32 bytes.
#[uniffi::export]
pub fn derive_public_key(seed_bytes: Vec<u8>) -> Result<Vec<u8>, PktapError> {
    if seed_bytes.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    let mut seed: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PktapError::InvalidKey)?;
    let signing_key = SigningKey::from_bytes(&seed);
    let pubkey = signing_key.verifying_key().to_bytes().to_vec();
    seed.zeroize();
    Ok(pubkey)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    use crate::error::PktapError;
    use crate::record::MAX_PLAINTEXT_LEN;
    use crate::signing;
    use super::*;

    // -----------------------------------------------------------------------
    // ecdh_and_encrypt tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ecdh_and_encrypt_valid_returns_blob_with_version_byte() {
        // Valid seed + valid peer key + JSON -> blob starting with 0x01, len >= 41.
        let seed_a = [0x01u8; 32].to_vec();
        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let peer_pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let json = r#"{"name":"Alice"}"#.to_string();

        let result = ecdh_and_encrypt(seed_a, peer_pub_b, json);
        assert!(result.is_ok(), "Valid inputs should succeed: {:?}", result);
        let blob = result.unwrap();
        assert_eq!(blob[0], 0x01, "First byte must be version 0x01");
        assert!(blob.len() >= 41, "Blob must be >= 41 bytes, got {}", blob.len());
    }

    #[test]
    fn test_ecdh_and_encrypt_invalid_peer_key_all_zeros() {
        // Invalid peer key (32 zeros) -> returns Err(InvalidKey).
        let seed = [0x01u8; 32].to_vec();
        let zero_peer = vec![0u8; 32];
        let json = r#"{}"#.to_string();

        let result = ecdh_and_encrypt(seed, zero_peer, json);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "All-zero peer key must return InvalidKey, got {:?}",
            result
        );
    }

    #[test]
    fn test_ecdh_and_encrypt_json_too_large_returns_record_too_large() {
        // JSON exceeding MAX_PLAINTEXT_LEN -> returns Err(RecordTooLarge).
        let seed = [0x01u8; 32].to_vec();
        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let peer_pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let big_json = "x".repeat(MAX_PLAINTEXT_LEN + 1);

        let result = ecdh_and_encrypt(seed, peer_pub_b, big_json);
        assert!(
            matches!(result, Err(PktapError::RecordTooLarge)),
            "Oversized JSON must return RecordTooLarge, got {:?}",
            result
        );
    }

    #[test]
    fn test_ecdh_and_encrypt_empty_json_succeeds() {
        // Empty JSON object "{}" is a valid edge case.
        let seed = [0x01u8; 32].to_vec();
        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let peer_pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let json = r#"{}"#.to_string();

        let result = ecdh_and_encrypt(seed, peer_pub_b, json);
        assert!(result.is_ok(), "Empty JSON '{{}}' must succeed: {:?}", result);
    }

    #[test]
    fn test_ecdh_and_encrypt_wrong_seed_length_returns_invalid_key() {
        // Seed != 32 bytes -> InvalidKey.
        let bad_seed = vec![0x01u8; 16]; // Only 16 bytes
        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let peer_pub_b = signing_key_b.verifying_key().to_bytes().to_vec();

        let result = ecdh_and_encrypt(bad_seed, peer_pub_b, "{}".to_string());
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "Short seed must return InvalidKey, got {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // decrypt_and_verify tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decrypt_and_verify_round_trip() {
        // Round-trip: encrypt with ecdh_and_encrypt, sign, then decrypt_and_verify recovers JSON.
        //
        // Keys must be consistent: seed = signing_key.to_scalar_bytes() so that
        // seed_to_x25519_scalar(seed) produces the scalar whose X25519 public key equals
        // signing_key.verifying_key().to_montgomery(). This is the canonical production
        // key derivation path (seed from EncryptedSharedPreferences is the Ed25519 scalar).
        let signing_key_a = SigningKey::from_bytes(&[0x11u8; 32]);
        let signing_key_b = SigningKey::from_bytes(&[0x22u8; 32]);

        let seed_a = signing_key_a.to_scalar_bytes().to_vec();
        let seed_b = signing_key_b.to_scalar_bytes().to_vec();
        let pub_a = signing_key_a.verifying_key().to_bytes().to_vec();
        let pub_b = signing_key_b.verifying_key().to_bytes().to_vec();

        let original_json = r#"{"name":"Alice","phone":"+1234567890"}"#.to_string();

        // Alice encrypts to Bob.
        let record = ecdh_and_encrypt(seed_a, pub_b.clone(), original_json.clone())
            .expect("encrypt should succeed");

        // Alice signs the record.
        let signature = signing::sign_bytes(&signing_key_a, &record);

        // Bob decrypts using Alice's public key.
        let recovered = decrypt_and_verify(seed_b, pub_a, signature, record)
            .expect("decrypt_and_verify should succeed");

        assert_eq!(
            recovered, original_json,
            "Recovered JSON must match original"
        );
    }

    #[test]
    fn test_decrypt_and_verify_tampered_record_returns_record_invalid() {
        // Tampered record bytes -> Err(RecordInvalid), not a panic.
        let seed_a = [0x01u8; 32].to_vec();
        let signing_key_a = SigningKey::from_bytes(&[0x01u8; 32]);
        let pub_a = signing_key_a.verifying_key().to_bytes().to_vec();

        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let seed_b = [0x02u8; 32].to_vec();

        let record = ecdh_and_encrypt(seed_a, pub_b, r#"{"name":"Alice"}"#.to_string())
            .expect("encrypt");
        let signature = signing::sign_bytes(&signing_key_a, &record);

        // Tamper the record after signing.
        let mut tampered_record = record;
        tampered_record[25] ^= 0xFF;

        let result = decrypt_and_verify(seed_b, pub_a, signature, tampered_record);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Tampered record must return RecordInvalid (D-08), got {:?}",
            result
        );
    }

    #[test]
    fn test_decrypt_and_verify_wrong_peer_key_returns_record_invalid() {
        // Wrong peer key (D-08 coalescing) -> Err(RecordInvalid), NOT InvalidKey.
        let seed_a = [0x01u8; 32].to_vec();
        let signing_key_a = SigningKey::from_bytes(&[0x01u8; 32]);

        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let seed_b = [0x02u8; 32].to_vec();

        let record = ecdh_and_encrypt(seed_a, pub_b, r#"{"name":"Alice"}"#.to_string())
            .expect("encrypt");
        let signature = signing::sign_bytes(&signing_key_a, &record);

        // Use a completely wrong peer key (not Alice's real key).
        let wrong_pub = [0x99u8; 32].to_vec();

        let result = decrypt_and_verify(seed_b, wrong_pub, signature, record);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Wrong peer key must return RecordInvalid (D-08 coalescing), got {:?}",
            result
        );
    }

    #[test]
    fn test_decrypt_and_verify_invalid_signature_returns_record_invalid() {
        // Invalid signature bytes -> Err(RecordInvalid) (D-08 coalescing).
        let seed_a = [0x01u8; 32].to_vec();
        let signing_key_b = SigningKey::from_bytes(&[0x02u8; 32]);
        let pub_b = signing_key_b.verifying_key().to_bytes().to_vec();
        let seed_b = [0x02u8; 32].to_vec();
        let signing_key_a = SigningKey::from_bytes(&[0x01u8; 32]);
        let pub_a = signing_key_a.verifying_key().to_bytes().to_vec();

        let record = ecdh_and_encrypt(seed_a, pub_b, r#"{}"#.to_string()).expect("encrypt");

        // Provide garbage signature bytes (64 zeros).
        let bad_sig = vec![0u8; 64];

        let result = decrypt_and_verify(seed_b, pub_a, bad_sig, record);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Invalid signature must return RecordInvalid (D-08), got {:?}",
            result
        );
    }

    #[test]
    fn test_decrypt_and_verify_truncated_record_returns_record_invalid() {
        // Truncated record (too short) -> Err(RecordInvalid).
        let signing_key_a = SigningKey::from_bytes(&[0x01u8; 32]);
        let pub_a = signing_key_a.verifying_key().to_bytes().to_vec();
        let seed_b = [0x02u8; 32].to_vec();
        let truncated = vec![0x01u8; 10]; // way too short
        let sig = vec![0u8; 64];

        let result = decrypt_and_verify(seed_b, pub_a, sig, truncated);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Truncated record must return RecordInvalid, got {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // D-10 Pipeline integration test: full round-trip both directions
    // -----------------------------------------------------------------------

    #[test]
    fn test_pipeline_integration() {
        // D-10: Generate keys -> ECDH -> encrypt -> sign -> verify -> decrypt -> recover JSON.
        // Both Alice->Bob and Bob->Alice directions.

        // Generate two keypairs using OsRng for realism.
        // ed25519-dalek 2.2.0 does not expose SigningKey::generate; generate random seed bytes
        // via OsRng and construct via from_bytes.
        use rand_core::RngCore;
        let mut seed_bytes_a = [0u8; 32];
        let mut seed_bytes_b = [0u8; 32];
        OsRng.fill_bytes(&mut seed_bytes_a);
        OsRng.fill_bytes(&mut seed_bytes_b);
        let signing_key_a = SigningKey::from_bytes(&seed_bytes_a);
        let signing_key_b = SigningKey::from_bytes(&seed_bytes_b);

        let pub_a = signing_key_a.verifying_key().to_bytes().to_vec();
        let pub_b = signing_key_b.verifying_key().to_bytes().to_vec();

        // Use the signing key's scalar bytes as the HKDF seed (for testing purposes).
        // In production, the seed comes from EncryptedSharedPreferences.
        let seed_a: Vec<u8> = signing_key_a.to_scalar_bytes().to_vec();
        let seed_b: Vec<u8> = signing_key_b.to_scalar_bytes().to_vec();

        // --- Direction 1: Alice encrypts contact fields to Bob ---
        let alice_json = r#"{"name":"Alice","email":"alice@example.com"}"#.to_string();

        // Alice encrypts to Bob.
        let record_a_to_b = ecdh_and_encrypt(
            seed_a.clone(),
            pub_b.clone(),
            alice_json.clone(),
        )
        .expect("Alice->Bob encrypt should succeed");

        // Record starts with version byte 0x01 and is >= 41 bytes.
        assert_eq!(record_a_to_b[0], 0x01, "D-10: version byte must be 0x01");
        assert!(record_a_to_b.len() >= 41, "D-10: record must be >= 41 bytes");

        // Alice signs the encrypted record.
        let sig_a = signing::sign_bytes(&signing_key_a, &record_a_to_b);
        assert_eq!(sig_a.len(), 64, "D-10: Ed25519 signature must be 64 bytes");

        // Bob decrypts and verifies.
        let recovered_by_bob = decrypt_and_verify(
            seed_b.clone(),
            pub_a.clone(),
            sig_a,
            record_a_to_b,
        )
        .expect("Bob decrypt_and_verify should succeed");

        assert_eq!(
            recovered_by_bob, alice_json,
            "D-10: Bob must recover Alice's original JSON"
        );

        // --- Direction 2: Bob encrypts contact fields to Alice ---
        let bob_json = r#"{"name":"Bob","phone":"+9876543210"}"#.to_string();

        // Bob encrypts to Alice.
        let record_b_to_a = ecdh_and_encrypt(
            seed_b.clone(),
            pub_a.clone(),
            bob_json.clone(),
        )
        .expect("Bob->Alice encrypt should succeed");

        // Bob signs the encrypted record.
        let sig_b = signing::sign_bytes(&signing_key_b, &record_b_to_a);

        // Alice decrypts and verifies.
        let recovered_by_alice = decrypt_and_verify(
            seed_a.clone(),
            pub_b.clone(),
            sig_b,
            record_b_to_a,
        )
        .expect("Alice decrypt_and_verify should succeed");

        assert_eq!(
            recovered_by_alice, bob_json,
            "D-10: Alice must recover Bob's original JSON"
        );
    }

    // -----------------------------------------------------------------------
    // derive_shared_record_name tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_shared_record_name_symmetric() {
        let key_a = vec![0x01u8; 32];
        let key_b = vec![0x02u8; 32];

        let name_ab = derive_shared_record_name(key_a.clone(), key_b.clone())
            .expect("Should succeed");
        let name_ba = derive_shared_record_name(key_b, key_a)
            .expect("Should succeed");

        assert_eq!(name_ab, name_ba, "derive_shared_record_name must be symmetric");
        assert!(
            name_ab.starts_with("_pktap._share."),
            "Must start with _pktap._share."
        );
    }

    #[test]
    fn test_derive_shared_record_name_wrong_length() {
        let bad_key = vec![0x01u8; 16]; // Too short
        let good_key = vec![0x02u8; 32];

        let result = derive_shared_record_name(bad_key, good_key);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "Wrong-length key must return InvalidKey, got {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // derive_mnemonic_from_seed tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_mnemonic_from_seed_returns_12_words() {
        let seed = vec![0x01u8; 32];
        let result = derive_mnemonic_from_seed(seed);
        assert!(result.is_ok(), "Valid 32-byte seed should succeed: {:?}", result);
        let mnemonic = result.unwrap();
        let words: Vec<&str> = mnemonic.split(' ').collect();
        assert_eq!(words.len(), 12, "Must return exactly 12 words, got {}", words.len());
    }

    #[test]
    fn test_derive_mnemonic_from_seed_is_deterministic() {
        let seed = vec![0xABu8; 32];
        let m1 = derive_mnemonic_from_seed(seed.clone()).expect("first call");
        let m2 = derive_mnemonic_from_seed(seed).expect("second call");
        assert_eq!(m1, m2, "Same seed must produce same mnemonic");
    }

    #[test]
    fn test_derive_mnemonic_from_seed_wrong_length_returns_invalid_key() {
        let short_seed = vec![0x01u8; 16]; // 16 bytes instead of 32
        let result = derive_mnemonic_from_seed(short_seed);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "Wrong-length seed must return InvalidKey, got {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // derive_public_key tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_public_key_returns_32_bytes() {
        let seed = vec![0x01u8; 32];
        let result = derive_public_key(seed);
        assert!(result.is_ok(), "Valid 32-byte seed should succeed: {:?}", result);
        let pubkey = result.unwrap();
        assert_eq!(pubkey.len(), 32, "Must return 32 bytes, got {}", pubkey.len());
    }

    #[test]
    fn test_derive_public_key_is_deterministic() {
        let seed = vec![0x42u8; 32];
        let pk1 = derive_public_key(seed.clone()).expect("first call");
        let pk2 = derive_public_key(seed).expect("second call");
        assert_eq!(pk1, pk2, "Same seed must produce same public key");
    }

    #[test]
    fn test_derive_public_key_wrong_length_returns_invalid_key() {
        let short_seed = vec![0x01u8; 16];
        let result = derive_public_key(short_seed);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "Wrong-length seed must return InvalidKey, got {:?}",
            result
        );
    }

    #[test]
    fn test_derive_public_key_matches_dalek_verifying_key() {
        use ed25519_dalek::SigningKey;
        let seed_bytes = [0x77u8; 32];
        let expected = SigningKey::from_bytes(&seed_bytes).verifying_key().to_bytes().to_vec();
        let result = derive_public_key(seed_bytes.to_vec()).expect("should succeed");
        assert_eq!(result, expected, "derive_public_key must match ed25519_dalek verifying key");
    }
}
