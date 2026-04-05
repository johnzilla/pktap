use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::ZeroizeOnDrop;

use crate::error::PktapError;
use crate::keys;

/// A 32-byte symmetric encryption key derived via ECDH + HKDF.
///
/// Wrapped in ZeroizeOnDrop so the key material is wiped from memory when
/// the value goes out of scope. Never exposed outside this crate.
#[derive(ZeroizeOnDrop)]
pub(crate) struct DerivedKey(pub(crate) [u8; 32]);

/// Perform X25519 ECDH key agreement and derive a 32-byte symmetric key via HKDF-SHA256.
///
/// # Arguments
/// - `our_seed`: 32-byte HKDF seed passed from Kotlin (decrypted from EncryptedSharedPreferences
///   by Keystore AES key). The X25519 scalar is derived from this seed internally.
/// - `peer_ed25519_pub`: 32-byte Ed25519 public key received from the peer via NFC/QR.
///   Rust converts it to X25519 internally.
///
/// # Returns
/// A `DerivedKey` containing 32 bytes suitable for use as a XChaCha20-Poly1305 key,
/// or `Err(PktapError::InvalidKey)` if:
/// - The peer's Ed25519 public key is malformed.
/// - The X25519 ECDH produces an all-zero shared secret (low-order point attack).
///
/// # Security notes
/// - The low-order point check (all-zero shared secret) prevents small-subgroup attacks.
/// - The HKDF "pktap-v1" domain separator ensures the derived key is bound to this protocol.
/// - Both parties with opposite roles produce the same derived key (ECDH is symmetric).
pub(crate) fn ecdh_derive_key(
    our_seed: &[u8; 32],
    peer_ed25519_pub: &[u8; 32],
) -> Result<DerivedKey, PktapError> {
    // Step 1: Validate and convert the peer's Ed25519 public key to X25519.
    let peer_vk = keys::validate_ed25519_public_key(peer_ed25519_pub)?;
    let peer_x25519_pub = keys::ed25519_pub_to_x25519_pub(&peer_vk);

    // Step 2: Derive our X25519 scalar from the HKDF seed.
    let our_scalar = keys::seed_to_x25519_scalar(our_seed);

    // Step 3: Perform X25519 ECDH.
    let our_secret = StaticSecret::from(our_scalar.0);
    let shared = our_secret.diffie_hellman(&peer_x25519_pub);

    // Step 4: Reject low-order points (T-01-02 threat mitigation).
    // An all-zero shared secret indicates the peer used a point in the small-order subgroup.
    if shared.as_bytes() == &[0u8; 32] {
        return Err(PktapError::InvalidKey);
    }

    // Step 5: Derive a 32-byte key via HKDF-SHA256 with domain separator.
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut okm = [0u8; 32];
    // expand() only fails if the output length exceeds 255 * hash_len — 32 bytes is always safe.
    hk.expand(b"pktap-v1", &mut okm)
        .map_err(|_| PktapError::SerializationFailed)?;

    Ok(DerivedKey(okm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // RFC 7748 section 6.1 — X25519 Known Answer Test vectors.
    // Alice's private scalar (little-endian, already clamped per RFC 7748 §5):
    const RFC7748_ALICE_PRIV: [u8; 32] =
        hex!("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    // Bob's public key:
    const RFC7748_BOB_PUB: [u8; 32] =
        hex!("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
    // Expected shared secret:
    const RFC7748_SHARED: [u8; 32] =
        hex!("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");

    #[test]
    fn test_kat_rfc7748() {
        // Test the raw X25519 ECDH operation against RFC 7748 § 6.1 vectors.
        // This validates that x25519-dalek computes the correct Montgomery ladder result.
        let alice_secret = StaticSecret::from(RFC7748_ALICE_PRIV);
        let bob_public = PublicKey::from(RFC7748_BOB_PUB);
        let shared = alice_secret.diffie_hellman(&bob_public);
        assert_eq!(
            shared.as_bytes(),
            &RFC7748_SHARED,
            "RFC 7748 §6.1 KAT failed: X25519 shared secret does not match"
        );
    }

    #[test]
    fn test_low_order_point_rejected() {
        // All-zero peer pubkey is the identity element — rejected by validate_ed25519_public_key.
        // This covers the key-validation path of the low-order point defense.
        let seed = [0x42u8; 32];
        let all_zero_peer = [0u8; 32];
        let result = ecdh_derive_key(&seed, &all_zero_peer);
        assert!(
            matches!(result, Err(PktapError::InvalidKey)),
            "All-zero peer pubkey (low-order / identity point) should return InvalidKey"
        );
    }

    #[test]
    fn test_hkdf_derivation_deterministic() {
        // HKDF-SHA256 with info "pktap-v1" must produce the same 32-byte key for the same input.
        let fixed_ikm = [0x37u8; 32];
        let hk = Hkdf::<Sha256>::new(None, &fixed_ikm);
        let mut okm1 = [0u8; 32];
        let mut okm2 = [0u8; 32];
        hk.expand(b"pktap-v1", &mut okm1).unwrap();
        hk.expand(b"pktap-v1", &mut okm2).unwrap();
        assert_eq!(okm1, okm2, "HKDF must be deterministic for same input");

        // Different inputs must produce different outputs.
        let other_ikm = [0x38u8; 32];
        let hk2 = Hkdf::<Sha256>::new(None, &other_ikm);
        let mut okm3 = [0u8; 32];
        hk2.expand(b"pktap-v1", &mut okm3).unwrap();
        assert_ne!(okm1, okm3, "Different HKDF inputs must produce different outputs");
    }

    #[test]
    fn test_ecdh_symmetry() {
        // Both parties must derive the same key.
        // We test X25519 symmetry directly: DH(scalar_a, pub_b) == DH(scalar_b, pub_a).
        // seed_to_x25519_scalar derives deterministic scalars, and PublicKey::from(&StaticSecret)
        // gives the corresponding X25519 public key.
        let seed_a = [0x01u8; 32];
        let seed_b = [0x02u8; 32];

        // Derive X25519 scalars from seeds (same path as ecdh_derive_key uses internally).
        let scalar_a = keys::seed_to_x25519_scalar(&seed_a);
        let scalar_b = keys::seed_to_x25519_scalar(&seed_b);

        // Compute the X25519 public keys for each party.
        let static_a = StaticSecret::from(scalar_a.0);
        let static_b = StaticSecret::from(scalar_b.0);
        let xpub_a = PublicKey::from(&static_a);
        let xpub_b = PublicKey::from(&static_b);

        // Perform ECDH both ways.
        let shared_a_sees =
            StaticSecret::from(keys::seed_to_x25519_scalar(&seed_a).0).diffie_hellman(&xpub_b);
        let shared_b_sees =
            StaticSecret::from(keys::seed_to_x25519_scalar(&seed_b).0).diffie_hellman(&xpub_a);

        assert_eq!(
            shared_a_sees.as_bytes(),
            shared_b_sees.as_bytes(),
            "ECDH must be symmetric: DH(a, B) == DH(b, A)"
        );

        // Verify HKDF derives the same key from identical shared secrets.
        let mut key1 = [0u8; 32];
        let mut key2 = [0u8; 32];
        Hkdf::<Sha256>::new(None, shared_a_sees.as_bytes())
            .expand(b"pktap-v1", &mut key1)
            .unwrap();
        Hkdf::<Sha256>::new(None, shared_b_sees.as_bytes())
            .expand(b"pktap-v1", &mut key2)
            .unwrap();
        assert_eq!(key1, key2, "Both sides must derive the same encryption key");
    }

    #[test]
    fn test_zeroize_derived_key() {
        // Verify that DerivedKey implements ZeroizeOnDrop by asserting the trait bound.
        // The actual zeroing is tested via compile-time trait constraint — if ZeroizeOnDrop
        // is not derived, this function cannot be called.
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<DerivedKey>();

        // Additionally, verify initial value is preserved before drop (sanity check).
        let key = DerivedKey([0xCDu8; 32]);
        assert_eq!(key.0[0], 0xCD, "DerivedKey should hold initial value before drop");
        // key is dropped here — ZeroizeOnDrop runs the zeroize impl.
    }
}
