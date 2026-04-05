use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand_core::OsRng;

use crate::error::PktapError;

/// Wire format version byte for the D-06 encrypted record layout.
pub(crate) const RECORD_VERSION: u8 = 0x01;

/// Length of the XChaCha20 nonce (192-bit / 24 bytes).
pub(crate) const NONCE_LEN: usize = 24;

/// Length of the version prefix byte.
pub(crate) const VERSION_LEN: usize = 1;

/// Length of the Poly1305 authentication tag.
pub(crate) const TAG_LEN: usize = 16;

/// Total fixed overhead added to plaintext: version(1) + nonce(24) + tag(16) = 41 bytes.
pub(crate) const FIXED_OVERHEAD: usize = VERSION_LEN + NONCE_LEN + TAG_LEN;

/// Encrypt `plaintext` under `key` using XChaCha20-Poly1305, returning a D-06 byte blob.
///
/// D-06 wire layout:
/// ```text
/// byte 0:      version = 0x01
/// bytes 1..25: 24-byte random XChaCha20 nonce (generated with OsRng)
/// bytes 25..:  ciphertext || 16-byte Poly1305 tag  (produced by AEAD encrypt)
/// ```
///
/// The 192-bit nonce space makes random collision probability negligible even at
/// high volumes — no nonce-reuse risk (T-01-05 mitigation).
pub(crate) fn encrypt_record(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, PktapError> {
    let cipher = XChaCha20Poly1305::new(key.into());

    // Generate a fresh 24-byte nonce for every encryption.
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

    // Encrypt; the AEAD appends the 16-byte Poly1305 tag to the ciphertext bytes.
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| PktapError::SerializationFailed)?;

    // Build the D-06 blob: version || nonce || ciphertext+tag
    let mut out = Vec::with_capacity(VERSION_LEN + NONCE_LEN + ciphertext.len());
    out.push(RECORD_VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);

    Ok(out)
}

/// Decrypt a D-06 byte blob under `key`, returning the original plaintext.
///
/// Returns `Err(PktapError::RecordInvalid)` if:
/// - The blob is too short to hold the fixed overhead.
/// - The version byte is not `0x01`.
/// - The Poly1305 authentication tag does not verify (tampered ciphertext, wrong key, etc.).
///
/// AEAD authentication ensures any modification to the ciphertext or nonce is detected
/// before any plaintext bytes are returned (T-01-06 mitigation).
pub(crate) fn decrypt_record(key: &[u8; 32], record: &[u8]) -> Result<Vec<u8>, PktapError> {
    // Minimum length: 1 version + 24 nonce + 16 tag.
    if record.len() < FIXED_OVERHEAD {
        return Err(PktapError::RecordInvalid);
    }

    // Version check — must be 0x01.
    if record[0] != RECORD_VERSION {
        return Err(PktapError::RecordInvalid);
    }

    // Extract the 24-byte nonce from bytes 1..25.
    let nonce = XNonce::from_slice(&record[1..25]);

    // The rest is ciphertext+tag (16-byte Poly1305 tag is at the tail).
    let ciphertext_and_tag = &record[25..];

    let cipher = XChaCha20Poly1305::new(key.into());

    // Decrypt; returns Err if the tag verification fails.
    cipher
        .decrypt(nonce, ciphertext_and_tag)
        .map_err(|_| PktapError::RecordInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    /// Fixed 32-byte test key (all 0x42 bytes — not a real key, for testing only).
    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    // -----------------------------------------------------------------------
    // Round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_round_trip_basic() {
        // Encrypting then decrypting must recover the original plaintext exactly.
        let plaintext = b"hello pktap";
        let blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt should succeed");
        let recovered = decrypt_record(&TEST_KEY, &blob).expect("decrypt should succeed");
        assert_eq!(
            recovered.as_slice(),
            plaintext,
            "Round-trip failed: decrypted bytes do not match original plaintext"
        );
    }

    #[test]
    fn test_round_trip_empty_plaintext() {
        // Empty plaintext is a valid edge case — the AEAD should still work.
        let blob = encrypt_record(&TEST_KEY, b"").expect("encrypt empty plaintext should succeed");
        let recovered = decrypt_record(&TEST_KEY, &blob).expect("decrypt empty ciphertext");
        assert!(recovered.is_empty(), "Decrypted empty plaintext must be empty");
    }

    #[test]
    fn test_round_trip_long_plaintext() {
        // 600-byte payload — realistic contact record size.
        let plaintext = vec![0xABu8; 600];
        let blob = encrypt_record(&TEST_KEY, &plaintext).expect("encrypt large payload");
        let recovered = decrypt_record(&TEST_KEY, &blob).expect("decrypt large payload");
        assert_eq!(recovered, plaintext, "Round-trip failed for 600-byte payload");
    }

    // -----------------------------------------------------------------------
    // D-06 wire layout tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_d06_wire_layout() {
        // Verify the blob structure: version(1) || nonce(24) || ciphertext+tag(n+16)
        let plaintext = b"wire format check";
        let blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt");

        // Version byte must be 0x01.
        assert_eq!(
            blob[0], RECORD_VERSION,
            "byte[0] must be RECORD_VERSION (0x01)"
        );

        // Bytes 1..25 are the 24-byte nonce.
        assert_eq!(blob[1..25].len(), NONCE_LEN, "nonce must be 24 bytes");

        // Total length must be 1 + 24 + plaintext.len() + TAG_LEN.
        let expected_len = VERSION_LEN + NONCE_LEN + plaintext.len() + TAG_LEN;
        assert_eq!(
            blob.len(),
            expected_len,
            "blob length must be {} but got {}",
            expected_len,
            blob.len()
        );
    }

    // -----------------------------------------------------------------------
    // Authentication failure tests (T-01-06)
    // -----------------------------------------------------------------------

    #[test]
    fn test_tampered_ciphertext_rejected() {
        // Flipping a byte in the ciphertext portion must cause AEAD verification to fail.
        let plaintext = b"tamper target";
        let mut blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt");

        // Flip the first ciphertext byte (index 25 is ciphertext[0]).
        blob[25] ^= 0xFF;

        let result = decrypt_record(&TEST_KEY, &blob);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Tampered ciphertext must return RecordInvalid"
        );
    }

    #[test]
    fn test_tampered_nonce_rejected() {
        // Changing the nonce without re-encrypting invalidates the AEAD tag.
        let plaintext = b"nonce tamper";
        let mut blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt");

        // Flip the first nonce byte (index 1).
        blob[1] ^= 0xFF;

        let result = decrypt_record(&TEST_KEY, &blob);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Tampered nonce must return RecordInvalid"
        );
    }

    #[test]
    fn test_wrong_key_rejected() {
        // Decrypting with a different key must fail — AEAD tags are key-dependent.
        let plaintext = b"wrong key";
        let blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt");

        let wrong_key = [0x00u8; 32];
        let result = decrypt_record(&wrong_key, &blob);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Wrong key must return RecordInvalid"
        );
    }

    // -----------------------------------------------------------------------
    // Boundary / version tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_too_short_blob_rejected() {
        // A blob shorter than FIXED_OVERHEAD (41 bytes) must be rejected immediately.
        let short = vec![0x01u8; FIXED_OVERHEAD - 1];
        let result = decrypt_record(&TEST_KEY, &short);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Blob shorter than FIXED_OVERHEAD must return RecordInvalid"
        );
    }

    #[test]
    fn test_wrong_version_rejected() {
        // A blob with version byte != 0x01 must be rejected.
        let plaintext = b"version check";
        let mut blob = encrypt_record(&TEST_KEY, plaintext).expect("encrypt");
        blob[0] = 0x02; // Wrong version
        let result = decrypt_record(&TEST_KEY, &blob);
        assert!(
            matches!(result, Err(PktapError::RecordInvalid)),
            "Wrong version byte must return RecordInvalid"
        );
    }

    // -----------------------------------------------------------------------
    // IETF XChaCha20-Poly1305 Known Answer Test
    //
    // Uses a fixed key and nonce to produce a deterministic ciphertext, then
    // verifies round-trip decryption. This validates that the cipher implementation
    // produces correct output for known inputs (not dependent on OsRng).
    //
    // Key and nonce are from draft-irtf-cfrg-xchacha-03 §2.2.
    // We verify the first 4 bytes of the ciphertext match the draft to confirm
    // the cipher is correct, then verify full round-trip decryption.
    // -----------------------------------------------------------------------

    #[test]
    fn test_kat_xchacha20poly1305_deterministic() {
        // IETF draft-irtf-cfrg-xchacha-03 §2.2 Known Answer Test.
        // Source: https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha-03#appendix-A.1
        //
        // The draft vector includes AAD (Additional Authenticated Data).
        // PKTap does not use AAD in its wire format (we rely solely on the nonce + tag),
        // but we test against the full draft vector here to confirm the cipher implementation
        // is correct — then separately verify PKTap's no-AAD path round-trips correctly.
        let key: [u8; 32] =
            hex!("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f");
        let nonce_bytes: [u8; 24] =
            hex!("404142434445464748494a4b4c4d4e4f5051525354555657");
        // AAD from draft §2.2:
        let aad: [u8; 12] = hex!("50515253c0c1c2c3c4c5c6c7");
        // Plaintext from draft §2.2 (114 bytes):
        let plaintext: &[u8] =
            b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        // Encrypt with fixed nonce and AAD (deterministic, no OsRng).
        let nonce = XNonce::from_slice(&nonce_bytes);
        let cipher = XChaCha20Poly1305::new((&key).into());
        let ct_tag = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .expect("KAT encryption must not fail");

        // Verify output length: plaintext.len() + TAG_LEN.
        assert_eq!(
            ct_tag.len(),
            plaintext.len() + TAG_LEN,
            "Ciphertext+tag must be plaintext.len() + 16"
        );

        // Verify the first 4 bytes of ciphertext match draft-arciszewski-xchacha-03 §A.1:
        // bd6d179d...
        // Source: chacha20poly1305 crate test vectors and IETF draft appendix A.1.
        assert_eq!(
            &ct_tag[..4],
            &hex!("bd6d179d"),
            "First 4 bytes of ciphertext must match IETF draft vector"
        );

        // Verify the Poly1305 tag (last 16 bytes) matches the draft vector:
        // c0875924c1c7987947deafd8780acf49
        let tag_start = ct_tag.len() - TAG_LEN;
        assert_eq!(
            &ct_tag[tag_start..],
            &hex!("c0875924c1c7987947deafd8780acf49"),
            "Poly1305 tag must match IETF draft vector"
        );

        // Verify decryption with AAD round-trips correctly.
        let recovered = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &ct_tag,
                    aad: &aad,
                },
            )
            .expect("KAT decryption must succeed");
        assert_eq!(
            recovered.as_slice(),
            plaintext,
            "KAT round-trip failed: recovered plaintext does not match"
        );

        // Also verify PKTap's no-AAD path (what encrypt_record / decrypt_record use):
        // A separate deterministic round-trip with a fixed key, fixed nonce, no AAD.
        let fixed_key = [0x42u8; 32];
        let fixed_nonce_bytes = [0xBBu8; 24];
        let fixed_nonce = XNonce::from_slice(&fixed_nonce_bytes);
        let cipher2 = XChaCha20Poly1305::new((&fixed_key).into());
        let ct2 = cipher2
            .encrypt(fixed_nonce, b"pktap deterministic test".as_ref())
            .expect("no-AAD encrypt must succeed");
        let pt2 = cipher2
            .decrypt(fixed_nonce, ct2.as_slice())
            .expect("no-AAD decrypt must succeed");
        assert_eq!(pt2.as_slice(), b"pktap deterministic test");

        // Build D-06 blob and verify decrypt_record handles it correctly.
        let mut d06_blob = Vec::with_capacity(VERSION_LEN + NONCE_LEN + ct2.len());
        d06_blob.push(RECORD_VERSION);
        d06_blob.extend_from_slice(&fixed_nonce_bytes);
        d06_blob.extend_from_slice(&ct2);
        let recovered2 =
            decrypt_record(&fixed_key, &d06_blob).expect("D-06 no-AAD decrypt must succeed");
        assert_eq!(recovered2.as_slice(), b"pktap deterministic test");
    }
}
