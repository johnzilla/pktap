use sha2::{Digest, Sha256};

use crate::error::PktapError;

/// Maximum plaintext length accepted by the cipher layer.
///
/// Conservative limit for Phase 1 — the full record budget analysis in Phase 2 will
/// validate that this fits within the Pkarr 1000-byte limit after encryption overhead
/// (version + nonce + tag = 41 bytes) plus DNS TXT encoding and Ed25519 signature.
/// At 750 bytes the encrypted record is ~791 bytes, safely within the 1000-byte limit.
pub(crate) const MAX_PLAINTEXT_LEN: usize = 750;

/// Validate that `plaintext` does not exceed `MAX_PLAINTEXT_LEN`.
///
/// Returns `Err(PktapError::RecordTooLarge)` if the plaintext is too long.
pub(crate) fn validate_plaintext_size(plaintext: &[u8]) -> Result<(), PktapError> {
    if plaintext.len() > MAX_PLAINTEXT_LEN {
        return Err(PktapError::RecordTooLarge);
    }
    Ok(())
}

/// Derive the shared DNS record name for a private contact exchange between two parties.
///
/// The name is deterministic and symmetric: `shared_record_name(A, B) == shared_record_name(B, A)`.
///
/// Algorithm:
/// 1. Sort the two 32-byte Ed25519 public keys lexicographically.
/// 2. Compute SHA-256 over the concatenation of the sorted keys.
/// 3. Hex-encode the 32-byte hash.
/// 4. Return `"_pktap._share.<hex>"`.
///
/// This ensures both peers independently derive the same DHT address without coordination.
pub(crate) fn shared_record_name(pub_key_a: &[u8; 32], pub_key_b: &[u8; 32]) -> String {
    // Canonical sort: smaller key comes first (lexicographic byte order).
    let (first, second) = if pub_key_a <= pub_key_b {
        (pub_key_a, pub_key_b)
    } else {
        (pub_key_b, pub_key_a)
    };

    let mut hasher = Sha256::new();
    hasher.update(first);
    hasher.update(second);
    let hash = hasher.finalize();

    let hex_hash: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    format!("_pktap._share.{}", hex_hash)
}

/// Derive the public profile DNS record name for a single public key.
///
/// Returns `"_pktap._profile.<hex>"` where `<hex>` is the hex-encoded 32-byte public key.
///
/// Note: Phase 2 will switch the encoding to z-base-32 when Pkarr integration is
/// added — z-base-32 is the canonical Pkarr key encoding. Hex is used in Phase 1
/// for simplicity since DHT publishing is not yet implemented.
pub(crate) fn public_profile_name(pub_key: &[u8; 32]) -> String {
    let hex_key: String = pub_key.iter().map(|b| format!("{:02x}", b)).collect();
    format!("_pktap._profile.{}", hex_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // -----------------------------------------------------------------------
    // shared_record_name tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_shared_record_name_symmetric() {
        // shared_record_name(A, B) must equal shared_record_name(B, A).
        let key_a = [0x01u8; 32];
        let key_b = [0x02u8; 32];

        let name_ab = shared_record_name(&key_a, &key_b);
        let name_ba = shared_record_name(&key_b, &key_a);

        assert_eq!(
            name_ab, name_ba,
            "shared_record_name must be symmetric: same name regardless of key order"
        );
    }

    #[test]
    fn test_shared_record_name_prefix() {
        // Output must start with the _pktap._share. prefix.
        let key_a = [0xAAu8; 32];
        let key_b = [0xBBu8; 32];
        let name = shared_record_name(&key_a, &key_b);
        assert!(
            name.starts_with("_pktap._share."),
            "shared_record_name must start with '_pktap._share.', got: {}",
            name
        );
    }

    #[test]
    fn test_shared_record_name_deterministic() {
        // Same inputs must always produce the same output.
        let key_a = [0x11u8; 32];
        let key_b = [0x22u8; 32];

        let name1 = shared_record_name(&key_a, &key_b);
        let name2 = shared_record_name(&key_a, &key_b);
        assert_eq!(name1, name2, "shared_record_name must be deterministic");
    }

    #[test]
    fn test_shared_record_name_known_value() {
        // Verify a known-input/output pair to catch any regression in the hash computation.
        // Inputs: key_a = [0x01; 32], key_b = [0x02; 32]
        // Sorted: [0x01; 32] < [0x02; 32] so hash = SHA-256([0x01; 32] || [0x02; 32])
        let key_a = [0x01u8; 32];
        let key_b = [0x02u8; 32];

        // Pre-compute expected SHA-256([0x01;32] || [0x02;32]):
        let mut hasher = sha2::Sha256::new();
        hasher.update(&[0x01u8; 32]);
        hasher.update(&[0x02u8; 32]);
        let expected_hash = hasher.finalize();
        let expected_hex: String = expected_hash.iter().map(|b| format!("{:02x}", b)).collect();
        let expected_name = format!("_pktap._share.{}", expected_hex);

        let actual_name = shared_record_name(&key_a, &key_b);
        assert_eq!(
            actual_name, expected_name,
            "shared_record_name known-value test failed"
        );
    }

    #[test]
    fn test_shared_record_name_equal_keys() {
        // Edge case: both keys are identical — should still produce a valid name.
        let key = [0x55u8; 32];
        let name = shared_record_name(&key, &key);
        assert!(
            name.starts_with("_pktap._share."),
            "shared_record_name with equal keys must still return a valid name"
        );
        // Both orderings produce the same name trivially.
        let name2 = shared_record_name(&key, &key);
        assert_eq!(name, name2);
    }

    #[test]
    fn test_shared_record_name_hex_length() {
        // The hex-encoded SHA-256 hash is always 64 characters.
        // Total name length = "_pktap._share.".len() + 64 = 14 + 64 = 78.
        let key_a = [0xCAu8; 32];
        let key_b = [0xFEu8; 32];
        let name = shared_record_name(&key_a, &key_b);
        assert_eq!(
            name.len(),
            "_pktap._share.".len() + 64,
            "shared_record_name must be exactly 78 characters long"
        );
    }

    // -----------------------------------------------------------------------
    // public_profile_name tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_public_profile_name_prefix() {
        // Output must start with the _pktap._profile. prefix.
        let key = [0x77u8; 32];
        let name = public_profile_name(&key);
        assert!(
            name.starts_with("_pktap._profile."),
            "public_profile_name must start with '_pktap._profile.', got: {}",
            name
        );
    }

    #[test]
    fn test_public_profile_name_deterministic() {
        let key = [0x88u8; 32];
        let name1 = public_profile_name(&key);
        let name2 = public_profile_name(&key);
        assert_eq!(name1, name2, "public_profile_name must be deterministic");
    }

    #[test]
    fn test_public_profile_name_hex_length() {
        // The hex-encoded 32-byte key is always 64 characters.
        // Total name length = "_pktap._profile.".len() + 64 = 16 + 64 = 80.
        let key = [0x99u8; 32];
        let name = public_profile_name(&key);
        assert_eq!(
            name.len(),
            "_pktap._profile.".len() + 64,
            "public_profile_name must be exactly 80 characters long"
        );
    }

    #[test]
    fn test_public_profile_name_known_value() {
        // Verify a known-input/output pair.
        // key = [0x00; 32] → hex = "0000...0000" (64 zeros)
        let key = [0x00u8; 32];
        let name = public_profile_name(&key);
        let expected = format!("_pktap._profile.{}", "00".repeat(32));
        assert_eq!(name, expected, "public_profile_name known-value test failed");
    }

    // -----------------------------------------------------------------------
    // validate_plaintext_size tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_plaintext_size_at_limit() {
        // Exactly MAX_PLAINTEXT_LEN bytes must be accepted.
        let plaintext = vec![0u8; MAX_PLAINTEXT_LEN];
        let result = validate_plaintext_size(&plaintext);
        assert!(
            result.is_ok(),
            "Plaintext of exactly MAX_PLAINTEXT_LEN must be accepted"
        );
    }

    #[test]
    fn test_validate_plaintext_size_over_limit() {
        // MAX_PLAINTEXT_LEN + 1 bytes must be rejected.
        let plaintext = vec![0u8; MAX_PLAINTEXT_LEN + 1];
        let result = validate_plaintext_size(&plaintext);
        assert!(
            matches!(result, Err(PktapError::RecordTooLarge)),
            "Plaintext exceeding MAX_PLAINTEXT_LEN must return RecordTooLarge"
        );
    }

    #[test]
    fn test_validate_plaintext_size_empty() {
        // Empty plaintext is always valid.
        let result = validate_plaintext_size(&[]);
        assert!(result.is_ok(), "Empty plaintext must be accepted");
    }

    #[test]
    fn test_validate_plaintext_size_one_byte() {
        // Single byte is valid.
        let result = validate_plaintext_size(&[0x42]);
        assert!(result.is_ok(), "Single-byte plaintext must be accepted");
    }
}
