// BIP-39 mnemonic generation from a 32-byte HKDF seed.
// Uses only the first 16 bytes as entropy (128 bits → 12 words per D-03).

use crate::error::PktapError;

/// Generate a 12-word BIP-39 mnemonic from a 32-byte seed.
///
/// Uses the first 16 bytes of `seed` as entropy (128-bit → 12 words).
/// The remaining bytes are not used for mnemonic derivation.
///
/// # Errors
/// Returns `PktapError::SerializationFailed` if `bip39::Mnemonic::from_entropy` fails
/// (should not occur in practice for valid 16-byte entropy).
pub(crate) fn mnemonic_from_entropy(seed: &[u8; 32]) -> Result<String, PktapError> {
    let entropy = &seed[..16];
    let mnemonic = bip39::Mnemonic::from_entropy(entropy)
        .map_err(|_| PktapError::SerializationFailed)?;
    Ok(mnemonic.words().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_from_entropy_returns_12_words() {
        let seed = [0x01u8; 32];
        let result = mnemonic_from_entropy(&seed);
        assert!(result.is_ok(), "Should succeed for valid seed: {:?}", result);
        let mnemonic = result.unwrap();
        let words: Vec<&str> = mnemonic.split(' ').collect();
        assert_eq!(words.len(), 12, "Expected 12 words, got {}", words.len());
    }

    #[test]
    fn test_mnemonic_from_entropy_is_deterministic() {
        let seed = [0xBBu8; 32];
        let m1 = mnemonic_from_entropy(&seed).unwrap();
        let m2 = mnemonic_from_entropy(&seed).unwrap();
        assert_eq!(m1, m2);
    }
}
