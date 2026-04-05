# Phase 1: Rust Crypto Core - Research

**Researched:** 2026-04-05
**Domain:** Rust cryptography — RustCrypto ecosystem, UniFFI proc-macros, zeroize memory safety
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Composites only across FFI — only expose high-level operations (`ecdhAndEncrypt`, `decryptAndVerify`). No lower-level primitives cross the FFI boundary. Internal Rust modules may use fine-grained functions, but the UniFFI-exported surface is composite-only.
- **D-02:** Rust accepts raw Ed25519 public key bytes (32 bytes) from Kotlin. Android Keystore generates the master keypair (hardware-backed, non-extractable). Rust never generates the master key.
- **D-03:** Split signing/encryption model — Android Keystore owns Ed25519 signing (private key never leaves hardware). For ECDH, Rust derives an X25519 key from the HKDF seed (passed from Kotlin as encrypted bytes, decrypted via Keystore AES key). This means the composite `ecdhAndEncrypt` handles encryption only; signing is a separate Keystore operation on the Kotlin side.
- **D-04:** Composite functions return opaque byte blobs (`Vec<u8>`) — nonce + ciphertext + tag in a single buffer, ready to be signed externally and published. Kotlin never parses crypto internals.
- **D-05:** Contact fields serialized as JSON inside the encrypted payload. Simple `{"name":"Alice","email":"a@b.com"}` encoding. Matches `kotlinx-serialization` on the Android side. Well within the ~600 byte budget for text-only fields.
- **D-06:** Encrypted record byte layout: `version(1) + nonce(24) + ciphertext(var) + tag(16)`. Version byte first for future-proofing. Total fixed overhead: 41 bytes. Signature is applied separately by Keystore via Pkarr's signing mechanism.
- **D-07:** Typed error enum via UniFFI — define a `PktapError` enum (`InvalidKey`, `RecordInvalid`, `RecordTooLarge`, `SerializationFailed`, etc.) that UniFFI maps to a Kotlin sealed class. Each variant carries a safe message.
- **D-08:** Coalesce crypto failures externally — `DecryptionFailed` and `SignatureInvalid` map to a single `RecordInvalid` error across FFI to prevent oracle attacks. Internal Rust code may distinguish them for logging/debugging, but the FFI surface exposes only the coalesced variant.
- **D-09:** Known Answer Tests (KATs) from RFC 7748 (X25519), RFC 8032 (Ed25519), and IETF draft vectors for XChaCha20-Poly1305. Plus round-trip tests for every composite function.
- **D-10:** Include a pipeline integration test within this phase: generate keys -> ECDH -> encrypt -> construct record -> parse record -> decrypt. Proves the pieces fit together before Phase 2 adds DHT.

### Claude's Discretion

- Internal Rust module structure and file organization
- Exact `Cargo.toml` dependency version pins (within the ranges specified in CLAUDE.md)
- HKDF domain separator string format (requirement says "pktap-v1")
- Zeroize implementation details (derive vs manual `Drop`)
- Whether to use `#[cfg(test)]` inline tests or a separate `tests/` directory

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CRYPTO-01 | Rust module performs Ed25519 to X25519 key conversion with input validation (reject malformed keys) | `VerifyingKey::from_bytes()` validates Edwards point; `to_montgomery()` on curve25519-dalek EdwardsPoint; `SigningKey::to_scalar_bytes()` for private key. Two-stage validation: point validity + low-order check post-ECDH. |
| CRYPTO-02 | Rust module performs X25519 ECDH key agreement and derives encryption key via HKDF with domain separator "pktap-v1" | `StaticSecret::diffie_hellman()` in x25519-dalek 2.0.1; `Hkdf::<Sha256>::new()` + `expand()` in hkdf 0.12.4. Verified API. |
| CRYPTO-03 | Rust module encrypts contact field payload with XChaCha20-Poly1305 (random 24-byte nonce, AEAD) | `XChaCha20Poly1305` type in chacha20poly1305 0.10.1; `XNonce` is 24 bytes; `OsRng` for nonce generation; `AeadCore` + `Aead` traits. |
| CRYPTO-04 | Rust module signs encrypted payload with Ed25519 key | Per D-03: signing is a Keystore operation on Kotlin side. Rust defines the `ecdhAndEncrypt` function that returns the byte blob for external signing. `SigningKey::sign()` is available in Rust for testing only. |
| CRYPTO-05 | Rust module decrypts and verifies signature on received encrypted records | `decryptAndVerify` composite function: `VerifyingKey::verify()` then `XChaCha20Poly1305::decrypt()`; internal Rust may distinguish failures, FFI coalesces to `RecordInvalid` per D-08. |
| CRYPTO-06 | Rust module constructs DNS TXT records for both encrypted and public mode with `_pktap.` namespace prefix | Phase 1 produces the byte blob in D-06 format and a `PktapRecord` struct with the DNS key name. DNS wire-format encoding (via simple-dns) deferred to Phase 2 where pkarr is introduced. |
| CRYPTO-07 | All crypto operations are composed inside Rust (e.g., ecdhAndEncrypt as one FFI call) — secret material never crosses FFI boundary | Composite functions return `Vec<u8>` opaque blobs per D-04. No intermediate `SharedSecret` or derived key bytes in FFI signatures. |
| KEY-06 | All secret material (seed, derived keys, shared secrets) is zeroed from memory after use | `ZeroizeOnDrop` derive (requires `zeroize` with `derive` feature) on all secret-holding structs. `SharedSecret.as_bytes()` used then immediately dropped. |
</phase_requirements>

---

## Summary

Phase 1 is a pure Rust cryptography library with no Android, no network, and no external services. All work is `cargo test` verifiable on the development machine.

The RustCrypto ecosystem (ed25519-dalek, x25519-dalek, chacha20poly1305, hkdf, sha2, zeroize) is the correct stack and is well-maintained. However, **several version numbers in CLAUDE.md require updating**: crates have had significant releases since that research was written. The dalek crates (ed25519-dalek 2.2.0, x25519-dalek 2.0.1, curve25519-dalek 4.1.3) remain on their stable 2.x/4.x series and are fully compatible with each other. The notable changes are: `hkdf` has a new stable 0.13 but it requires `sha2 0.11`, which conflicts with `ed25519-dalek 2.2.0`'s `sha2 ^0.10` requirement — therefore **use hkdf 0.12.4 + sha2 0.10.9**. UniFFI is at 0.31.0 (not 0.28) and `rand` is at 0.10 which is incompatible with the dalek crates — **use rand_core 0.6.4 directly**.

The pkarr version conflict (Phase 1 success criterion 5) applies only within the Phase 1 workspace scope: ed25519-dalek 2.2.0 and x25519-dalek 2.0.1 share curve25519-dalek ^4 cleanly. The broader conflict with pkarr 5.x (which pulls ed25519-dalek 3.0.0-pre and curve25519-dalek 5.0.0-pre) is a Phase 2 decision. For Phase 2, the recommended approach is to keep pkarr 2.3.1 (still maintained, stable, uses ed25519-dalek ^2.1.1) rather than pkarr 5.x.

**Primary recommendation:** Initialize the Cargo workspace now (Phase 1 creates the workspace root and `pktap-core` crate), annotate composite functions with `#[uniffi::export]` from the start (so Phase 3 has nothing to add), and use `ZeroizeOnDrop` derives on all secret-holding newtypes.

---

## Project Constraints (from CLAUDE.md)

| Directive | Source | Impact on Phase 1 |
|-----------|--------|-------------------|
| All cryptographic operations in Rust via UniFFI — no JVM crypto for protocol operations | CLAUDE.md Constraints | Phase 1 defines all crypto primitives; UniFFI annotations applied from the start |
| All secret material zeroed after use (zeroize crate in Rust) | CLAUDE.md Constraints | `ZeroizeOnDrop` derive on all secret newtypes required |
| No server — DHT only | CLAUDE.md Constraints | Phase 1 is pure local computation; no network calls |
| Crypto in Rust: pkarr 2.3.x, ed25519-dalek 2.1.x, x25519-dalek 2.0.x, curve25519-dalek 4.1.x, chacha20poly1305 0.10.x, hkdf 0.12.x, zeroize 1.7.x | CLAUDE.md Stack | See Standard Stack below for verified current versions |

---

## Standard Stack

### Core (Phase 1 dependencies)

| Library | Verified Version | Purpose | Source |
|---------|-----------------|---------|--------|
| ed25519-dalek | **2.2.0** | Ed25519 sign/verify, Ed25519→X25519 key conversion via `to_scalar_bytes()` + `to_montgomery()` | [VERIFIED: crates.io, updated 2025-07-09] |
| x25519-dalek | **2.0.1** | X25519 ECDH via `StaticSecret::diffie_hellman()` | [VERIFIED: crates.io, updated 2024-02-07] |
| curve25519-dalek | **4.1.3** | Shared backend for ed25519-dalek + x25519-dalek; `EdwardsPoint::to_montgomery()` for public key conversion | [VERIFIED: crates.io, updated 2024-06-18] |
| chacha20poly1305 | **0.10.1** | `XChaCha20Poly1305` AEAD with 24-byte `XNonce` | [VERIFIED: crates.io, updated 2022-08-10] |
| hkdf | **0.12.4** | HKDF key derivation from ECDH shared secret | [VERIFIED: crates.io, updated 2023-12-13] |
| sha2 | **0.10.9** | SHA-256 for HKDF; SHA-512 used internally by ed25519-dalek | [VERIFIED: crates.io, updated 2025-04-30] |
| zeroize | **1.8.2** | `Zeroize` + `ZeroizeOnDrop` derives for secret newtypes | [VERIFIED: crates.io, updated 2025-09-29] |
| serde | **1.0.228** | Serialize/deserialize contact field structs to JSON | [VERIFIED: crates.io, updated 2025-09-27] |
| serde_json | **1.0.149** | JSON encoding of contact fields inside encrypted payload (D-05) | [VERIFIED: crates.io, updated 2026-01-06] |
| rand_core | **0.6.4** | `OsRng` for cryptographically secure nonce generation | [VERIFIED: crates.io] |
| uniffi | **0.31.0** | `#[uniffi::export]` proc-macro annotations on composite functions | [VERIFIED: crates.io, updated 2026-01-14] |

**CLAUDE.md version discrepancies (update required):**

| Crate | CLAUDE.md Said | Verified Current | Action |
|-------|---------------|-----------------|--------|
| ed25519-dalek | 2.1.x | **2.2.0** | Use 2.2.0 |
| sha2 | 0.10.x | **0.10.9** | Confirm 0.10.x, NOT 0.11 (would conflict with hkdf 0.12) |
| hkdf | 0.12.x | **0.12.4** | Use 0.12.4, NOT 0.13 (requires sha2 0.11 — conflicts) |
| zeroize | 1.7.x | **1.8.2** | Use 1.8.2 |
| rand | 0.8.x | **0.8.5 (last 0.8)** | Do NOT use rand 0.10 — incompatible with ed25519-dalek 2.x; use rand_core 0.6.4 directly |
| uniffi | 0.28.x | **0.31.0** | Use 0.31.0; Phase 1 proc-macro API is unchanged |
| pkarr | 2.3.x | 2.3.1 (still maintained) or 5.0.4 (current) | Phase 2 decision; Phase 1 does not use pkarr |

### Version Compatibility Matrix

| Crate | Requires | Compatible With |
|-------|----------|----------------|
| ed25519-dalek 2.2.0 | curve25519-dalek ^4 | curve25519-dalek 4.1.3 |
| x25519-dalek 2.0.1 | curve25519-dalek ^4 | curve25519-dalek 4.1.3 |
| ed25519-dalek 2.2.0 | sha2 ^0.10 | sha2 0.10.9 |
| hkdf 0.12.4 | hmac ^0.12 | hmac 0.12.1 |
| ed25519-dalek 2.2.0 | rand_core ^0.6.4 | rand_core 0.6.4 |
| chacha20poly1305 0.10.1 | zeroize ^1.5 | zeroize 1.8.2 |

[VERIFIED: All version requirements confirmed via crates.io API against actual published dependency lists]

**Why NOT hkdf 0.13.0:** hkdf 0.13.0 requires sha2 ^0.11. ed25519-dalek 2.2.0 requires sha2 ^0.10. Using hkdf 0.13.0 would pull two versions of sha2 into the build. Use hkdf 0.12.4 to keep a single sha2 0.10.9 instance. [VERIFIED: crates.io dependency lists]

**Why NOT rand 0.10.0:** rand 0.10.0 uses rand_core 0.10. ed25519-dalek 2.2.0 requires rand_core ^0.6.4. These are incompatible. Use rand_core 0.6.4 directly with `OsRng` from its `getrandom` feature. [VERIFIED: crates.io dependency lists]

### Installation

```toml
# workspace Cargo.toml
[workspace]
members = ["pktap-core", "uniffi-bindgen"]
resolver = "2"

[workspace.dependencies]
# Pin shared deps to prevent version skew
curve25519-dalek = { version = "4.1.3", features = ["zeroize"] }
zeroize = { version = "1.8.2", features = ["derive"] }

# pktap-core/Cargo.toml
[package]
name = "pktap-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
ed25519-dalek = { version = "2.2.0", features = ["zeroize"] }
x25519-dalek = { version = "2.0.1", features = ["static_secrets", "zeroize"] }
curve25519-dalek = { workspace = true }
chacha20poly1305 = { version = "0.10.1", features = ["std"] }
hkdf = "0.12.4"
sha2 = "0.10.9"
zeroize = { workspace = true }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
rand_core = { version = "0.6.4", features = ["getrandom"] }
uniffi = "0.31.0"
thiserror = "2"
```

---

## Architecture Patterns

### Recommended Project Structure

```
pktap/                            # workspace root
├── Cargo.toml                    # workspace manifest + pinned workspace.dependencies
├── pktap-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # uniffi::setup_scaffolding!(), re-exports, PktapError
│       ├── keys.rs               # Ed25519->X25519 conversion, key validation
│       ├── ecdh.rs               # X25519 ECDH + HKDF key derivation
│       ├── cipher.rs             # XChaCha20-Poly1305 encrypt/decrypt
│       ├── record.rs             # Byte-blob record construction (D-06), DNS name derivation
│       ├── signing.rs            # Ed25519 sign/verify (for testing; production signing in Keystore)
│       └── ffi.rs                # Composite FFI functions: ecdhAndEncrypt, decryptAndVerify
└── uniffi-bindgen/
    ├── Cargo.toml
    └── src/main.rs               # cargo run --bin uniffi-bindgen generate (used in Phase 3)
```

### Pattern 1: Secret Newtype with ZeroizeOnDrop

**What:** Wrap every `[u8; N]` that holds secret material in a newtype deriving `ZeroizeOnDrop`. Never pass raw `[u8; 32]` for secrets.

**When to use:** All derived keys, shared secrets, scalar bytes before conversion.

**Example:**
```rust
// Source: zeroize 1.8.2 docs.rs + ed25519-dalek 2.2.0 docs.rs
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
struct DerivedKey([u8; 32]);

#[derive(ZeroizeOnDrop)]
struct X25519ScalarBytes([u8; 32]);
```

### Pattern 2: Ed25519 to X25519 Key Conversion

**What:** Convert Ed25519 identity key pair to X25519 for ECDH. Uses the Birational equivalence between Edwards and Montgomery curves.

**When to use:** `ecdhAndEncrypt` — called with the HKDF seed derived X25519 private key and the peer's Ed25519 public key.

**Example:**
```rust
// Source: ed25519-dalek 2.2.0 SigningKey::to_scalar_bytes() docs
//         curve25519-dalek 4.1.3 EdwardsPoint::to_montgomery() docs
//         x25519-dalek 2.0.1 StaticSecret::from([u8;32]) docs
use ed25519_dalek::{SigningKey, VerifyingKey};
use x25519_dalek::{StaticSecret, PublicKey};
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
struct X25519ScalarBytes([u8; 32]);

fn ed25519_signing_key_to_x25519(sk: &SigningKey) -> X25519ScalarBytes {
    // to_scalar_bytes() returns the lower 32 bytes of SHA-512(seed),
    // which is the raw scalar compatible with StaticSecret::from([u8;32])
    X25519ScalarBytes(sk.to_scalar_bytes())
}

fn ed25519_verifying_key_to_x25519_public(vk: &VerifyingKey) -> PublicKey {
    // to_montgomery() converts Edwards point to Montgomery point
    let montgomery = vk.to_montgomery();
    PublicKey::from(montgomery.to_bytes())
}

fn validate_ed25519_public_key(bytes: &[u8; 32]) -> Result<VerifyingKey, PktapError> {
    VerifyingKey::from_bytes(bytes).map_err(|_| PktapError::InvalidKey)
}
```

**Security note:** `to_scalar_bytes()` does NOT require the `hazmat` feature. It is a public API on `SigningKey` in ed25519-dalek 2.2.0. [VERIFIED: docs.rs source view]

Per D-03: the Kotlin side passes the HKDF seed bytes (32 bytes) to Rust, not the actual Ed25519 signing key. Rust derives the X25519 private key from the seed — not from the Keystore-backed Ed25519 key. The above pattern illustrates the curve math; the actual private key input to Rust is the seed.

### Pattern 3: ECDH + HKDF Key Derivation

**What:** X25519 ECDH then HKDF-SHA256 to produce a 32-byte encryption key.

**When to use:** Inside `ecdhAndEncrypt`.

**Example:**
```rust
// Source: x25519-dalek 2.0.1 docs, hkdf 0.12.4 docs (RFC 5869 API)
use x25519_dalek::{StaticSecret, PublicKey, SharedSecret};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
struct EncryptionKey([u8; 32]);

fn ecdh_and_derive(
    our_x25519_scalar: &[u8; 32],   // from HKDF seed, passed from Kotlin
    peer_x25519_public: &[u8; 32],  // converted from peer's Ed25519 VerifyingKey
) -> Result<EncryptionKey, PktapError> {
    let our_secret = StaticSecret::from(*our_x25519_scalar);
    let peer_public = PublicKey::from(*peer_x25519_public);
    let shared = our_secret.diffie_hellman(&peer_public);
    
    // Reject low-order points (produce all-zero shared secret)
    if shared.as_bytes() == &[0u8; 32] {
        return Err(PktapError::InvalidKey);
    }
    
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"pktap-v1", &mut okm).map_err(|_| PktapError::SerializationFailed)?;
    Ok(EncryptionKey(okm))
}
```

### Pattern 4: XChaCha20-Poly1305 Encrypt

**What:** AEAD encrypt with a random 24-byte nonce. Output is the D-06 record byte blob.

**When to use:** Inside `ecdhAndEncrypt`.

**Example:**
```rust
// Source: chacha20poly1305 0.10.1 docs (aead::Aead + AeadCore traits)
use chacha20poly1305::{XChaCha20Poly1305, XNonce, KeyInit, AeadCore, aead::Aead};
use rand_core::OsRng;

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, PktapError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);  // 24 bytes
    let ciphertext = cipher.encrypt(&nonce, plaintext)
        .map_err(|_| PktapError::SerializationFailed)?;
    
    // D-06 wire layout: version(1) + nonce(24) + ciphertext+tag(var)
    let mut record = Vec::with_capacity(1 + 24 + ciphertext.len());
    record.push(0x01);                   // version byte
    record.extend_from_slice(&nonce);
    record.extend_from_slice(&ciphertext); // ciphertext already includes the 16-byte Poly1305 tag
    Ok(record)
}
```

### Pattern 5: Composite FFI Functions with UniFFI

**What:** `uniffi::setup_scaffolding!()` at crate root; `#[uniffi::export]` on composite functions; `#[derive(uniffi::Error)]` on `PktapError`.

**When to use:** lib.rs and ffi.rs.

**Example:**
```rust
// Source: UniFFI 0.31.0 proc-macro docs (mozilla.github.io/uniffi-rs)
// lib.rs
uniffi::setup_scaffolding!();

pub mod ffi;
pub use ffi::*;

// error.rs
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PktapError {
    #[error("Invalid key bytes")]
    InvalidKey,
    #[error("Record invalid or decryption failed")]
    RecordInvalid,
    #[error("Record payload too large")]
    RecordTooLarge,
    #[error("Serialization failed")]
    SerializationFailed,
}

// ffi.rs
#[uniffi::export]
pub fn ecdh_and_encrypt(
    our_seed_bytes: Vec<u8>,         // 32-byte HKDF seed from Kotlin
    peer_ed25519_public: Vec<u8>,    // 32-byte Ed25519 public key from peer
    contact_fields_json: String,     // JSON contact data to encrypt
) -> Result<Vec<u8>, PktapError> {
    // ... all crypto happens here, no secret material in return type
}

#[uniffi::export]
pub fn decrypt_and_verify(
    our_seed_bytes: Vec<u8>,         // 32-byte HKDF seed from Kotlin
    peer_ed25519_public: Vec<u8>,    // 32-byte Ed25519 public key of sender
    peer_ed25519_signature: Vec<u8>, // 64-byte signature to verify
    record_bytes: Vec<u8>,           // D-06 format encrypted record
) -> Result<String, PktapError> {   // Returns JSON contact fields on success
    // ... verify sig (using Keystore pubkey), then ECDH+HKDF, then decrypt
}
```

**D-08 coalescing:** Inside `decrypt_and_verify`, verification failure and decryption failure both surface as `PktapError::RecordInvalid` to the FFI caller.

### Anti-Patterns to Avoid

- **Returning SharedSecret bytes across FFI:** The shared secret must never appear in a function return value — it must be consumed internally and dropped before the function returns.
- **Using rand 0.10 with ed25519-dalek 2.x:** rand 0.10 uses rand_core 0.10, which is incompatible with ed25519-dalek 2.x's rand_core ^0.6.4. Use rand_core 0.6.4 directly.
- **Using hkdf 0.13 with sha2 0.10:** hkdf 0.13 requires sha2 ^0.11. This creates a dual sha2 version in the build and a maintenance headache. Use hkdf 0.12.4.
- **Raw `[u8; 32]` for secrets:** Never store secret material in plain arrays. Wrap in a `ZeroizeOnDrop` newtype so it clears on scope exit.
- **`thread_rng()` for nonce generation:** Use `OsRng` from rand_core. `thread_rng()` is not appropriate for cryptographic nonce generation.
- **Not checking for all-zero shared secret:** x25519-dalek does NOT automatically reject low-order points. After `diffie_hellman()`, check `shared.as_bytes() != &[0u8; 32]` and return `PktapError::InvalidKey` if true.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XChaCha20-Poly1305 AEAD | Custom stream cipher + MAC | chacha20poly1305 0.10.1 | Nonce size, key schedule, and authentication tag are non-trivial; timing attacks in custom code |
| HKDF key derivation | Raw HMAC loops | hkdf 0.12.4 | RFC 5869 two-phase extract-then-expand has subtle edge cases around salt and OKM length |
| Ed25519 signature verification | Custom point arithmetic | ed25519-dalek 2.2.0 | Cofactor handling, batch verification, and timing-constant comparison are architecture-specific |
| Ed25519→X25519 conversion | SHA-512 + clamping by hand | `to_scalar_bytes()` + `to_montgomery()` | The conversion is specified precisely — any implementation error silently produces wrong keys |
| Memory zeroing on drop | `memset` or assignment of zeros | zeroize 1.8.2 with `ZeroizeOnDrop` | Compiler can eliminate "dead" stores; zeroize uses platform-specific volatile writes |
| Error oracle prevention | Custom error mapping | Single `RecordInvalid` variant per D-08 | Timing and branch pattern in error reporting leaks decryption vs verification failure |

---

## Common Pitfalls

### Pitfall 1: rand_core Version Mismatch

**What goes wrong:** Adding `rand = "0.10"` to Cargo.toml for `OsRng` breaks `ed25519-dalek 2.2.0` which requires `rand_core ^0.6.4`. Cargo may fail to resolve or silently build with duplicate `rand_core` versions causing `CryptoRng` trait bound failures.

**Why it happens:** rand 0.10 bumped rand_core to 0.10 (breaking change). The dalek 2.x crates haven't yet adopted rand_core 0.10.

**How to avoid:** Use `rand_core = { version = "0.6.4", features = ["getrandom"] }` directly. Do not add `rand` as a dependency at all in Phase 1.

**Warning signs:** `error[E0277]: the trait bound OsRng: CryptoRng is not satisfied` or `mismatched types: expected rand_core::RngCore, found rand::RngCore`.

### Pitfall 2: hkdf 0.13 + sha2 0.11 Conflict

**What goes wrong:** Cargo resolves `sha2` to two versions (0.10.9 for ed25519-dalek, 0.11.0 for hkdf 0.13) in the dependency tree. This is allowed by Cargo but wastes compile time and can cause confusion when debugging.

**Why it happens:** hkdf 0.13 is the newest stable but requires sha2 0.11. ed25519-dalek 2.2.0 pins sha2 ^0.10.

**How to avoid:** Use `hkdf = "0.12.4"` which aligns with sha2 0.10.x.

**Warning signs:** `cargo tree` shows two sha2 entries.

### Pitfall 3: Low-Order Point Attack (All-Zero Shared Secret)

**What goes wrong:** A malicious peer sends a low-order X25519 point. `StaticSecret::diffie_hellman()` returns an all-zero `SharedSecret`. HKDF then derives a deterministic key from all-zeros — an attacker who knows this can decrypt any message.

**Why it happens:** x25519-dalek 2.0.1 does not automatically validate for low-order points on `diffie_hellman()`.

**How to avoid:** After every `diffie_hellman()` call, check `shared.as_bytes() == &[0u8; 32]` and return `PktapError::InvalidKey` if true. This must be done BEFORE calling `hkdf`.

**Warning signs:** Success criterion 2 — "malformed peer public key input is rejected" — catches this in tests.

### Pitfall 4: curve25519-dalek Workspace Version Conflict

**What goes wrong:** If pkarr 5.x is added to the workspace (Phase 2), it transitively requires `curve25519-dalek = 5.0.0-pre.6` via `ed25519-dalek 3.0.0-pre.6`. This conflicts with Phase 1's `curve25519-dalek 4.1.3`. Cargo will attempt to build both, which may fail due to the pre-release version specifier `=5.0.0-pre.6` (exact pin).

**Why it happens:** pkarr 5.x has moved to the unstable dalek 3.x pre-release series.

**How to avoid:** For Phase 2, use `pkarr = "2.3.1"` (which requires ed25519-dalek ^2.1.1 and compatible curve25519-dalek 4.x). Do not add pkarr 5.x to the workspace until the dalek 3.x series reaches stable. [VERIFIED: crates.io dependency lists for pkarr 2.3.1 and 5.0.4]

**Warning signs:** `error: failed to select a version for 'curve25519-dalek'` or pre-release version in `cargo tree`.

### Pitfall 5: UniFFI crate-type Missing

**What goes wrong:** `pktap-core` cannot produce the `.so` / `.a` files needed for Android if `[lib] crate-type` is not set correctly. `cargo test` still works (it uses `rlib`), so this goes unnoticed until Phase 3 tries to build the `.aar`.

**Why it happens:** The default crate-type is `rlib` which does not produce C-compatible symbols.

**How to avoid:** Set in `pktap-core/Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib", "staticlib"]
```

**Warning signs:** Phase 3 build fails with "no artifacts produced" or linker error.

### Pitfall 6: ZeroizeOnDrop Requires `derive` Feature

**What goes wrong:** `#[derive(ZeroizeOnDrop)]` fails with "cannot find derive macro" if the `derive` feature is not enabled on the zeroize crate.

**Why it happens:** The derive macros are in a separate `zeroize_derive` subcrate, gated behind the `derive` feature flag. [VERIFIED: docs.rs — `Available on crate feature zeroize_derive only`]

**How to avoid:** Always add `zeroize = { version = "1.8.2", features = ["derive"] }`.

**Warning signs:** `error[E0277]: cannot find derive macro ZeroizeOnDrop in this scope`.

---

## Code Examples

### Contact Field JSON Serialization (D-05)

```rust
// Source: serde 1.0.228 + serde_json 1.0.149 docs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ContactFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

fn serialize_fields(fields: &ContactFields) -> Result<Vec<u8>, PktapError> {
    serde_json::to_vec(fields).map_err(|_| PktapError::SerializationFailed)
}
```

### D-06 Record Byte Layout

```
[version: 1 byte] [nonce: 24 bytes] [ciphertext+tag: var bytes]
  0x01              random 24B        XChaCha20-Poly1305 output
                                      (tag is last 16 bytes per AEAD spec)
```

Fixed overhead: **41 bytes**. At the ~858 usable byte budget (Phase 2 enforcement), this leaves ~817 bytes for the encrypted payload. JSON contact fields for typical 3-4 field profiles will be well under 500 bytes.

### DNS Record Name Derivation (CRYPTO-06)

```rust
// Phase 1 produces the name string only; DNS wire-format encoding is Phase 2 (pkarr)
use sha2::{Sha256, Digest};

fn shared_record_name(pub_key_a: &[u8; 32], pub_key_b: &[u8; 32]) -> String {
    let mut keys = [pub_key_a, pub_key_b];
    keys.sort();  // canonical sort — same address regardless of who calls
    let mut hasher = Sha256::new();
    hasher.update(keys[0]);
    hasher.update(keys[1]);
    let hash = hex::encode(hasher.finalize());
    format!("_pktap._share.{}", hash)
}

fn public_profile_name(derived_pub_key: &[u8; 32]) -> String {
    let b32 = z32::encode(derived_pub_key);  // Pkarr uses z-base-32
    format!("_pktap._profile.{}", b32)
}
```

Note: `z32` crate (used by pkarr) is for Phase 2. Phase 1 can use hex encoding for the shared record name and defer z-base-32 to Phase 2.

### Known Answer Test Vectors (D-09)

RFC 7748 §6.1 X25519 vector:
```rust
// Alice's private key:
let alice_priv: [u8; 32] = hex!("77076d0a7318a57d3c16c17251b26645 2f875f54f6e4e8d9d8a27b5ad0c5ed19");
// Bob's public key:
let bob_pub: [u8; 32]   = hex!("de9edb7d7b7dc1b4d35b61c2ece43537 3f8343c85b78674dadfc7e146f882b4f");
// Expected shared secret:
let expected: [u8; 32]  = hex!("4a5d9d5ba4ce2de1728e3bf480350f25 e07e21c947d19e3376f09b3c1e161742");
```

RFC 8032 §5.1.7 Ed25519 vector (abbreviated — use all vectors from the RFC):
```rust
let sk_bytes: [u8; 32] = hex!("9d61b19deffd5a60ba844af492ec2cc44449..."); // 32 bytes
let msg: &[u8] = b"";
// expected signature: 64 bytes per RFC
```

IETF XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha): Use test vectors from the IETF draft at https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha-03.

RFC 5869 Appendix A HKDF vectors: https://datatracker.ietf.org/doc/html/rfc5869#appendix-A

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| UDL files for UniFFI interface definitions | `#[uniffi::export]` proc-macro (no UDL) | UniFFI 0.22+ (proc-macro API), stable by 0.27 | Eliminates duplicate type definitions; Rust types ARE the interface |
| `#[zeroize(drop)]` attribute | `#[derive(ZeroizeOnDrop)]` | zeroize 1.3.0 | Old attribute form deprecated |
| `kapt` for annotation processing | `ksp` | AGP 8.x era | Faster builds; kapt on deprecation path |
| `rand::rngs::OsRng` (via rand 0.8) | `rand_core::OsRng` (direct, via rand_core 0.6) | Explicit since rand_core became standalone | Avoids the rand 0.10 incompatibility; lighter dependency |

**Deprecated/outdated:**
- `rand 0.8` `OsRng`: Still works, but unnecessary; use `rand_core 0.6` directly to avoid pulling in the full rand 0.8 crate.
- UniFFI UDL files: Still supported but no longer needed for pure proc-macro crates.
- `#[zeroize(drop)]`: Deprecated; use `#[derive(ZeroizeOnDrop)]`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `to_scalar_bytes()` on `SigningKey` does not require the `hazmat` feature flag | Standard Stack / Pattern 2 | If it does require `hazmat`, all code calling it must add `features = ["hazmat"]` to ed25519-dalek; low risk as source inspection showed no cfg gate | 
| A2 | pkarr 2.3.1 will remain unmaintained but installable through Phase 2 | Pitfall 4 | If pkarr yanks 2.x, Phase 2 must upgrade to pkarr 5.x and adopt the pre-release dalek 3.x series |
| A3 | CRYPTO-06 "constructs DNS TXT records" refers to producing the record name string and byte blob, not full DNS wire-format encoding | Phase Requirements | If full DNS wire-format is required in Phase 1, add `simple-dns = "0.11.2"` to pktap-core dependencies |

**Most claims in this research are VERIFIED via crates.io API calls or official docs.rs documentation.**

---

## Open Questions

1. **Signing in `decryptAndVerify` (D-03 implication)**
   - What we know: D-03 says signing happens in Keystore (Kotlin side). But `CRYPTO-05` says "Rust module decrypts and verifies signature on received encrypted records."
   - What's unclear: Does `decryptAndVerify` receive the peer's Ed25519 public key as a parameter for verification? If so, the verification is done in Rust but the signing is done in Kotlin. This is consistent and the approach in Pattern 5's signature is correct.
   - Recommendation: `decrypt_and_verify` takes `peer_ed25519_public` + `peer_ed25519_signature` + `record_bytes` and performs verification then decryption in Rust. This matches D-03 and CRYPTO-05.

2. **Record size validation (D-07 vs. Phase 2)**
   - What we know: Phase 2 enforces the ~858 byte budget. Phase 1's `ecdhAndEncrypt` produces the byte blob.
   - What's unclear: Should Phase 1 enforce a maximum `contact_fields_json` input size, or only Phase 2 enforces the complete record budget?
   - Recommendation: Phase 1 enforces a `MAX_PLAINTEXT_LEN` constant (e.g., 750 bytes) on the JSON input to `ecdhAndEncrypt`, returning `RecordTooLarge`. Phase 2 validates the full DNS record size.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | All Rust compilation | ✓ | 1.93.0 / 1.93.0 | — |
| rustup | Toolchain management | ✓ | 1.28.2 | — |
| Android targets (aarch64-linux-android etc.) | Phase 3 cross-compile | Available but not installed | — | `rustup target add` in Phase 3 |
| cargo-ndk | Phase 3 cross-compile | ✗ Not installed | — | `cargo install cargo-ndk` in Phase 3 |
| Java / JVM | Phase 3+ (Gradle, Android build) | ✓ | OpenJDK 25.0.2 | — |
| Gradle | Phase 3+ | ✗ Not installed | — | `gradle wrapper` bootstrap in Phase 3 |
| Android SDK | Phase 3+ | ✗ Not found at standard paths | — | Install in Phase 3 |

**Phase 1 has no missing dependencies:** `cargo test` runs entirely with the installed stable Rust toolchain. All crates are fetched from crates.io.

**Missing dependencies with no Phase 1 fallback:** None — all missing items (Android targets, cargo-ndk, Gradle, Android SDK) are Phase 3+ concerns.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (no external test crate needed for Phase 1) |
| Config file | `Cargo.toml` — `[dev-dependencies]` section; `[features]` if needed for test utilities |
| Quick run command | `cargo test -p pktap-core` |
| Full suite command | `cargo test -p pktap-core -- --include-ignored` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CRYPTO-01 | Ed25519 public key validation rejects 32 bytes of zeros | unit | `cargo test -p pktap-core keys::tests::test_invalid_key_rejected` | ❌ Wave 0 |
| CRYPTO-01 | Ed25519→X25519 conversion produces correct public key | unit | `cargo test -p pktap-core keys::tests::test_ed25519_to_x25519` | ❌ Wave 0 |
| CRYPTO-01 | Low-order point input returns InvalidKey error | unit | `cargo test -p pktap-core ecdh::tests::test_low_order_point` | ❌ Wave 0 |
| CRYPTO-02 | X25519 ECDH matches RFC 7748 §6.1 KAT | unit | `cargo test -p pktap-core ecdh::tests::test_kat_rfc7748` | ❌ Wave 0 |
| CRYPTO-02 | HKDF-SHA256 with "pktap-v1" produces expected output | unit | `cargo test -p pktap-core ecdh::tests::test_hkdf_derivation` | ❌ Wave 0 |
| CRYPTO-03 | XChaCha20-Poly1305 encrypt/decrypt round-trip | unit | `cargo test -p pktap-core cipher::tests::test_encrypt_decrypt_roundtrip` | ❌ Wave 0 |
| CRYPTO-03 | XChaCha20-Poly1305 matches IETF draft KAT | unit | `cargo test -p pktap-core cipher::tests::test_kat_xchacha20` | ❌ Wave 0 |
| CRYPTO-04 | Ed25519 sign produces valid signature | unit | `cargo test -p pktap-core signing::tests::test_sign_verify` | ❌ Wave 0 |
| CRYPTO-04 | RFC 8032 §5.1.7 Ed25519 KAT | unit | `cargo test -p pktap-core signing::tests::test_kat_rfc8032` | ❌ Wave 0 |
| CRYPTO-05 | Tampered ciphertext returns RecordInvalid | unit | `cargo test -p pktap-core ffi::tests::test_decrypt_tampered` | ❌ Wave 0 |
| CRYPTO-05 | Wrong peer key returns RecordInvalid (not panic) | unit | `cargo test -p pktap-core ffi::tests::test_decrypt_wrong_key` | ❌ Wave 0 |
| CRYPTO-06 | DNS record name derivation is deterministic and symmetric | unit | `cargo test -p pktap-core record::tests::test_record_name` | ❌ Wave 0 |
| CRYPTO-07 | `ecdh_and_encrypt` + `decrypt_and_verify` pipeline integration | integration | `cargo test -p pktap-core -- --test pipeline` | ❌ Wave 0 |
| KEY-06 | Secret newtypes implement ZeroizeOnDrop | unit | `cargo test -p pktap-core -- zeroize` (compile-time verified) | ❌ Wave 0 |
| KEY-06 | ZeroizeOnDrop actually zeroes bytes on drop | unit | `cargo test -p pktap-core keys::tests::test_zeroize_on_drop` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p pktap-core`
- **Per wave merge:** `cargo test -p pktap-core -- --include-ignored`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

All source files and tests are new. Wave 0 must create:

- [ ] `pktap-core/src/lib.rs` — crate root with `uniffi::setup_scaffolding!()`
- [ ] `pktap-core/src/error.rs` — `PktapError` enum
- [ ] `pktap-core/src/keys.rs` — Ed25519→X25519 conversion + validation
- [ ] `pktap-core/src/ecdh.rs` — ECDH + HKDF
- [ ] `pktap-core/src/cipher.rs` — XChaCha20-Poly1305
- [ ] `pktap-core/src/signing.rs` — Ed25519 sign/verify
- [ ] `pktap-core/src/record.rs` — record byte layout (D-06), DNS name derivation
- [ ] `pktap-core/src/ffi.rs` — composite FFI functions
- [ ] `Cargo.toml` (workspace root) — workspace manifest
- [ ] `pktap-core/Cargo.toml` — crate manifest with verified deps
- [ ] `uniffi-bindgen/Cargo.toml` + `uniffi-bindgen/src/main.rs` — bindgen binary stub

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A — no user accounts |
| V3 Session Management | No | N/A — no sessions |
| V4 Access Control | No | N/A — local library |
| V5 Input Validation | Yes | `VerifyingKey::from_bytes()` validates Ed25519 points; size checks on all byte slice inputs |
| V6 Cryptography | Yes | RustCrypto ecosystem — ed25519-dalek, chacha20poly1305, hkdf, sha2; never hand-rolled |

### Known Threat Patterns for Rust Crypto Core

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Invalid/low-order public key (contributory attack) | Tampering | Validate `VerifyingKey::from_bytes()` + check non-zero shared secret post-ECDH |
| Nonce reuse (XChaCha20-Poly1305) | Information Disclosure | Always use `OsRng` for nonce generation; 192-bit nonce space makes collision negligible |
| Oracle attack via error differentiation | Information Disclosure | D-08: coalesce `DecryptionFailed` + `SignatureInvalid` to single `RecordInvalid` across FFI |
| Secret material in memory after use | Elevation of Privilege | `ZeroizeOnDrop` on all secret newtypes; check with drop-order test |
| Timing attack on signature verification | Elevation of Privilege | ed25519-dalek 2.x uses constant-time operations; do not add early-exit branches in verification paths |

---

## Sources

### Primary (HIGH confidence)

- [crates.io API](https://crates.io/api/v1/crates/) — All crate versions verified via live API calls in this session
- [docs.rs/ed25519-dalek/2.2.0](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.SigningKey.html) — `to_scalar_bytes()`, `to_montgomery()` API confirmed
- [docs.rs/x25519-dalek/2.0.1](https://docs.rs/x25519-dalek/2.0.1/x25519_dalek/struct.StaticSecret.html) — `StaticSecret::from([u8;32])`, `diffie_hellman()` API confirmed
- [docs.rs/chacha20poly1305 latest](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/) — `XChaCha20Poly1305`, `XNonce` (24-byte), `AeadCore` trait confirmed
- [docs.rs/hkdf/0.12.4](https://docs.rs/hkdf/0.12.4/hkdf/struct.Hkdf.html) — `Hkdf::new()` + `expand()` API confirmed
- [docs.rs/zeroize/1.8.2](https://docs.rs/zeroize/1.8.2/zeroize/) — `ZeroizeOnDrop` requires `derive` feature confirmed
- [mozilla.github.io/uniffi-rs](https://mozilla.github.io/uniffi-rs/latest/proc_macro/index.html) — `#[uniffi::export]`, `setup_scaffolding!()`, error derive confirmed
- [UniFFI CHANGELOG](https://raw.githubusercontent.com/mozilla/uniffi-rs/main/CHANGELOG.md) — Breaking changes in 0.29–0.31 reviewed
- [crates.io pkarr versions + deps](https://crates.io/api/v1/crates/pkarr/) — pkarr 2.3.1 (ed25519-dalek ^2.1.1), pkarr 5.0.4 (ed25519-dalek ^3.0.0-pre.1) dependency confirmed

### Secondary (MEDIUM confidence)

- [github.com/pubky/pkarr](https://github.com/pubky/pkarr) — pkarr 5.x async API, `SignedPacket::builder()` pattern
- [docs.rs/curve25519-dalek/4.1.3 EdwardsPoint](https://docs.rs/curve25519-dalek/4.1.3/curve25519_dalek/edwards/struct.EdwardsPoint.html) — `to_montgomery()` method confirmed

### Tertiary (LOW confidence)

- IETF draft-irtf-cfrg-xchacha-03 test vectors — referenced but not fetched in this session; known to exist

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified against live crates.io API with full dependency chain analysis
- Architecture: HIGH — API surface verified against docs.rs source; patterns match crate documentation
- Pitfalls: HIGH — all identified conflicts are VERIFIED by reading actual `[dependencies]` from crates.io, not inferred

**Research date:** 2026-04-05
**Valid until:** 2026-10-05 (stable RustCrypto crates move slowly; UniFFI has active releases — recheck if >6 months old)
