# SECURITY.md — pktap Phase 1 (01-rust-crypto-core)

**Audit date:** 2026-04-05
**ASVS Level:** 1
**Phase:** 01 — rust-crypto-core (Plans 01-01, 01-02, 01-03)
**Auditor:** gsd-secure-phase
**Status:** SECURED — 13/13 threats closed

---

## Threat Verification

| Threat ID | Category | Disposition | Component | Evidence |
|-----------|----------|-------------|-----------|----------|
| T-01-01 | Tampering | mitigate | keys::validate_ed25519_public_key | `keys.rs:30` — explicit all-zero rejection; `keys.rs:33` — `VerifyingKey::from_bytes()` rejects invalid Edwards points with `PktapError::InvalidKey` |
| T-01-02 | Tampering | mitigate | ecdh::ecdh_derive_key | `ecdh.rs:51` — `if shared.as_bytes() == &[0u8; 32]` rejects low-order/small-subgroup shared secrets with `PktapError::InvalidKey` |
| T-01-03 | Information Disclosure | mitigate | ecdh::DerivedKey, keys::X25519ScalarBytes | `keys.rs:10` — `#[derive(ZeroizeOnDrop)]` on `X25519ScalarBytes`; `ecdh.rs:13` — `#[derive(ZeroizeOnDrop)]` on `DerivedKey` |
| T-01-04 | Elevation of Privilege | mitigate | Secret material lifetime | `keys.rs:10–11` — `X25519ScalarBytes` wraps `[u8; 32]`; `ecdh.rs:13–14` — `DerivedKey` wraps `[u8; 32]`; `SharedSecret` (x25519-dalek) consumed inline at `ecdh.rs:47`, never stored or returned |
| T-01-05 | Information Disclosure | mitigate | cipher::encrypt_record | `cipher.rs:5,39` — `OsRng` imported and used via `XChaCha20Poly1305::generate_nonce(&mut OsRng)` to produce a fresh 24-byte XNonce per call |
| T-01-06 | Tampering | mitigate | cipher::decrypt_record | `cipher.rs:86` — `.map_err(\|_\| PktapError::RecordInvalid)` on AEAD `decrypt()`; tampered-ciphertext and tampered-nonce tests in `cipher.rs` test suite confirm rejection |
| T-01-07 | Tampering | signing::verify_signature | mitigate | `signing.rs:52` — `verifying_key.verify(message, &signature).map_err(\|_\| PktapError::RecordInvalid)`; ed25519-dalek 2.x uses `subtle::ConstantTimeEq` internally |
| T-01-08 | Repudiation | accept | record::shared_record_name | Accepted risk — see Accepted Risks Log below |
| T-01-09 | Information Disclosure | mitigate | ffi::decrypt_and_verify | `ffi.rs:126,131,135,141` — ALL error paths in `decrypt_and_verify` map to `PktapError::RecordInvalid`; seed/key length failures, signature failures, ECDH failures, AEAD failures, and UTF-8 failures are all coalesced (D-08) |
| T-01-10 | Tampering | mitigate | ffi::ecdh_and_encrypt input | `ffi.rs:40–45` — `our_seed_bytes.len() != 32` and `peer_ed25519_public.len() != 32` validated before any crypto; reject with `PktapError::InvalidKey` |
| T-01-11 | Elevation of Privilege | mitigate | ffi::ecdh_and_encrypt seed lifetime | `ffi.rs:67` — `seed.zeroize()` after key derivation in `ecdh_and_encrypt`; `ffi.rs:138` — same in `decrypt_and_verify`; `DerivedKey` drops with `ZeroizeOnDrop` at end of function scope |
| T-01-12 | Spoofing | mitigate | ffi::decrypt_and_verify signature | `ffi.rs:123–126` — `signing::verify_signature` called at Step 1, before `ecdh_derive_key` (Step 2) and `decrypt_record` (Step 3); forged records rejected before any key material is derived |
| T-01-13 | Denial of Service | mitigate | ffi::ecdh_and_encrypt payload size | `ffi.rs:58` — `record::validate_plaintext_size(contact_fields_json.as_bytes())?` called before encryption; `record.rs:17–19` — enforces `MAX_PLAINTEXT_LEN = 750` with `PktapError::RecordTooLarge` |

---

## Accepted Risks Log

| Threat ID | Category | Component | Risk Description | Rationale | Accepted by |
|-----------|----------|-----------|------------------|-----------|-------------|
| T-01-08 | Repudiation | record::shared_record_name | The deterministic DNS name derived from `SHA-256(sort(A_pk, B_pk))` provides no non-repudiation guarantee. Either party can derive the DHT address, and there is no per-record proof of who initiated the exchange at the record-naming layer. | Non-repudiation is not required at this layer. DHT records are signed separately by Pkarr using the publisher's Ed25519 key, which provides the integrity guarantee needed. The shared name is a coordination address, not an authentication token. Accepted by design in Plan 01-02 threat model. | Plan 01-02 threat model (disposition: accept) |

---

## Unregistered Threat Flags

No unregistered flags. SUMMARY.md `## Threat Flags` sections for Plans 01-01, 01-02, and 01-03 report no new attack surface beyond the registered threat register. Plan 01-03 SUMMARY.md explicitly states: "No new security surface beyond what was specified in the plan's threat model."

---

## Implementation Notes

### Notable Deviations with Security Impact

**seed_to_x25519_scalar: HKDF removed (Plan 01-03)**
The original plan specified HKDF expansion in `seed_to_x25519_scalar`. During Plan 01-03 implementation the executor discovered this broke ECDH symmetry — the derived X25519 scalar was decoupled from the Ed25519 public key. The fix (direct pass-through) is the canonical Curve25519 approach: `signing_key.to_scalar_bytes()` as seed, clamped by `StaticSecret::from()`, produces the X25519 private key whose public counterpart equals `verifying_key.to_montgomery()`. This is architecturally correct and does not weaken any registered threat mitigation.

**All-zero Ed25519 identity point: explicit rejection added**
`ed25519-dalek 2.2.0` accepts the identity element (all-zeros) as a valid Edwards point. Both `keys::validate_ed25519_public_key` (`keys.rs:30`) and `signing::verify_signature` (`signing.rs:32–34`) add an explicit early return for all-zero bytes. This strengthens T-01-01 and T-01-07 beyond what the library provides by default.

### T-01-03 / T-01-04 — ZeroizeOnDrop Test Coverage Note
The compile-time trait bound pattern (`fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}`) is used for `DerivedKey` in `ecdh.rs:175` — if the derive is removed the code will not compile. The `X25519ScalarBytes` test in `keys.rs:137–157` uses a Box/heap pattern; this is noted in SUMMARY 01-01 as having inherent unreliability due to allocator reuse, but the compile-time check provides the authoritative guarantee.

---

## Scope

This audit covers Phase 1 (01-rust-crypto-core) implementation only:
- `pktap-core/src/keys.rs`
- `pktap-core/src/ecdh.rs`
- `pktap-core/src/cipher.rs`
- `pktap-core/src/signing.rs`
- `pktap-core/src/record.rs`
- `pktap-core/src/ffi.rs`
- `pktap-core/src/error.rs`

Subsequent phases (DHT client, Android integration, NFC HCE, key storage) will introduce new trust boundaries and require separate audit cycles.
