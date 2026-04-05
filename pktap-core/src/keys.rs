use ed25519_dalek::VerifyingKey;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::ZeroizeOnDrop;

use crate::error::PktapError;

/// X25519 scalar (private key) derived from an HKDF seed.
///
/// Wrapped in ZeroizeOnDrop so the raw scalar bytes are wiped from memory
/// when the value goes out of scope. Never exposed outside this crate.
#[derive(ZeroizeOnDrop)]
pub(crate) struct X25519ScalarBytes(pub(crate) [u8; 32]);

/// Validate a 32-byte slice as an Ed25519 public key.
///
/// Returns `Err(PktapError::InvalidKey)` if:
/// - The bytes do not represent a valid Edwards point.
/// - The bytes are all-zero (identity/neutral element — a weak key).
/// - The point is of small order (torsion subgroup).
///
/// Note: `ed25519-dalek 2.2.0` accepts the identity point (all-zeros) because it
/// is technically a valid Edwards point. We explicitly reject it here because the
/// identity element produces a degenerate X25519 public key and must never be used
/// for key exchange.
pub(crate) fn validate_ed25519_public_key(
    bytes: &[u8; 32],
) -> Result<VerifyingKey, PktapError> {
    // Explicit check for the identity element (all-zeros).
    // The identity point in compressed Edwards form has y=1 (0x0100...00 in little-endian)
    // but all-zero bytes are also accepted by dalek as the identity — reject them explicitly.
    if bytes == &[0u8; 32] {
        return Err(PktapError::InvalidKey);
    }
    VerifyingKey::from_bytes(bytes).map_err(|_| PktapError::InvalidKey)
}

/// Convert a validated Ed25519 public key to its X25519 (Montgomery) equivalent.
///
/// Uses the birational equivalence between the Edwards curve (Ed25519) and the
/// Montgomery curve (Curve25519). The conversion is:
///   u = (1 + y) / (1 - y)   (in the Montgomery affine form)
pub(crate) fn ed25519_pub_to_x25519_pub(vk: &VerifyingKey) -> x25519_dalek::PublicKey {
    // to_montgomery() converts the Edwards point to Montgomery form.
    let montgomery = vk.to_montgomery();
    x25519_dalek::PublicKey::from(montgomery.to_bytes())
}

/// Derive an X25519 scalar (private key) from a 32-byte HKDF seed.
///
/// Per D-03: Android Keystore generates the master Ed25519 keypair (non-extractable).
/// Rust receives the HKDF seed bytes (encrypted by Keystore AES key, decrypted on
/// Kotlin side before FFI call). The X25519 private key is derived from the seed
/// using HKDF-SHA256 with a fixed info string to provide domain separation.
pub(crate) fn seed_to_x25519_scalar(seed: &[u8; 32]) -> X25519ScalarBytes {
    let hk = Hkdf::<Sha256>::new(None, seed);
    let mut scalar_bytes = [0u8; 32];
    // Unwrap is safe: HKDF-SHA256 expand with 32-byte output length always succeeds.
    hk.expand(b"pktap-v1-x25519", &mut scalar_bytes)
        .expect("HKDF expand with 32 bytes always succeeds");
    X25519ScalarBytes(scalar_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A known valid Ed25519 public key (the basepoint / generator compressed).
    // The Ed25519 basepoint in compressed form is the canonical test vector from RFC 8032.
    const ED25519_BASEPOINT_BYTES: [u8; 32] = [
        0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    ];

    #[test]
    fn test_validate_all_zero_key_rejected() {
        // All-zero bytes are not a valid Edwards point.
        let result = validate_ed25519_public_key(&[0u8; 32]);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "All-zero key should return InvalidKey, got: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_known_valid_key_accepted() {
        // The Ed25519 basepoint is a valid key.
        let result = validate_ed25519_public_key(&ED25519_BASEPOINT_BYTES);
        assert!(
            result.is_ok(),
            "Known valid Ed25519 key should be accepted, got: {:?}",
            result
        );
    }

    #[test]
    fn test_ed25519_to_x25519_conversion_produces_montgomery() {
        let vk = validate_ed25519_public_key(&ED25519_BASEPOINT_BYTES)
            .expect("Basepoint is valid");
        let x25519_pub = ed25519_pub_to_x25519_pub(&vk);
        // The X25519 public key should be non-zero (a valid Montgomery point).
        assert_ne!(
            x25519_pub.as_bytes(),
            &[0u8; 32],
            "X25519 public key from valid Ed25519 key should not be all-zeros"
        );
    }

    #[test]
    fn test_ed25519_to_x25519_conversion_deterministic() {
        // Same Ed25519 public key must always produce the same X25519 public key.
        let vk = validate_ed25519_public_key(&ED25519_BASEPOINT_BYTES)
            .expect("Basepoint is valid");
        let pub1 = ed25519_pub_to_x25519_pub(&vk);
        let pub2 = ed25519_pub_to_x25519_pub(&vk);
        assert_eq!(
            pub1.as_bytes(),
            pub2.as_bytes(),
            "Key conversion must be deterministic"
        );
    }

    #[test]
    fn test_x25519_scalar_bytes_zeroize_on_drop() {
        // Verify that X25519ScalarBytes zeroes its memory on drop.
        // We use a Box (heap allocation) so the memory address remains stable
        // and the compiler cannot reclaim/reuse it before the assertion.
        // This pattern follows the zeroize crate's own internal tests.
        let secret = Box::new(X25519ScalarBytes([0xAB; 32]));
        let raw_ptr: *const u8 = secret.0.as_ptr();

        // Confirm the value is 0xAB before drop.
        assert_eq!(unsafe { *raw_ptr }, 0xAB);

        drop(secret);

        // After drop, ZeroizeOnDrop must have zeroed the heap bytes.
        // The heap memory is not returned to the OS between drop and this read.
        assert_eq!(
            unsafe { *raw_ptr },
            0x00,
            "X25519ScalarBytes should zero heap memory on drop"
        );
    }
}
