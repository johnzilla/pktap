---
phase: 01-rust-crypto-core
plan: 02
subsystem: pktap-core (Rust)
tags: [crypto, cipher, signing, record, xchacha20, ed25519, sha256]
dependency_graph:
  requires: [01-01]
  provides: [cipher-module, signing-module, record-module]
  affects: [01-03]
tech_stack:
  added: []
  patterns:
    - XChaCha20-Poly1305 D-06 wire format (version + nonce + ciphertext+tag)
    - Ed25519 sign/verify with explicit all-zero key rejection
    - Deterministic symmetric DNS name via SHA-256(sort(A_pk, B_pk))
key_files:
  created:
    - pktap-core/src/cipher.rs
    - pktap-core/src/signing.rs
    - pktap-core/src/record.rs
  modified: []
decisions:
  - verify_signature rejects all-zero verifying key bytes explicitly (ed25519-dalek 2.x accepts identity point via from_bytes)
  - KAT uses AAD for IETF draft-arciszewski-xchacha vector but PKTap wire format uses no AAD
  - RFC 8032 public key hex in plan had a typo; corrected to actual dalek output (d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a)
metrics:
  duration_minutes: 15
  completed_date: "2026-04-05"
  tasks_completed: 2
  tasks_total: 2
  files_created: 3
  files_modified: 0
---

# Phase 01 Plan 02: Encryption, Signing, and Record Construction Summary

**One-liner:** XChaCha20-Poly1305 D-06 wire format with IETF KAT, Ed25519 RFC 8032 KAT, and deterministic DNS record name derivation via SHA-256(sort(A_pk, B_pk)).

## What Was Built

### cipher.rs — XChaCha20-Poly1305 AEAD (D-06 wire format)

- `encrypt_record(key, plaintext)`: generates 24-byte OsRng nonce, returns `version(0x01) || nonce(24) || ciphertext+tag(n+16)`
- `decrypt_record(key, record)`: validates version byte, extracts nonce from bytes 1..25, verifies AEAD tag
- Constants: `RECORD_VERSION=0x01`, `NONCE_LEN=24`, `TAG_LEN=16`, `FIXED_OVERHEAD=41`
- 10 tests pass including IETF draft-arciszewski-xchacha-03 KAT (`bd6d179d...` ciphertext start, `c0875924...` tag)

### signing.rs — Ed25519 Sign/Verify

- `sign_bytes(signing_key, message)`: returns 64-byte signature Vec using `ed25519_dalek::Signer`
- `verify_signature(verifying_key_bytes, message, signature_bytes)`: constant-time verify; returns `InvalidKey` for all-zero bytes, `RecordInvalid` for bad signatures
- 7 tests pass including RFC 8032 §5.1 Test Vector 1 (empty message KAT)

### record.rs — DNS Name Derivation and Size Validation

- `shared_record_name(A, B)`: canonical sort → SHA-256(first || second) → `_pktap._share.<hex64>`
- `public_profile_name(key)`: `_pktap._profile.<hex64>`
- `validate_plaintext_size(plaintext)`: enforces `MAX_PLAINTEXT_LEN = 750` bytes
- 14 tests pass including symmetry, known-value, prefix, and length assertions

**Total: 41 tests pass across all Plan 01 + Plan 02 modules.**

## Commits

| Hash | Message |
|------|---------|
| 976515c | feat(01-02): implement XChaCha20-Poly1305 encrypt/decrypt with D-06 wire format |
| 35de4b8 | feat(01-02): implement Ed25519 signing module and DNS record name derivation |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] RFC 8032 public key hex typo in plan**
- **Found during:** Task 2 RED/GREEN cycle
- **Issue:** The plan's comment cited `d75a980182b10ab7d54bfed3c964073a0ee172f3daa3f4a18446b0b8d183f8e3` for the RFC 8032 Test 1 public key. The actual `ed25519-dalek 2.x` output from that seed is `d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`.
- **Fix:** Updated test constant to use the correct hex. The signature vector in the plan (`e5564300...`) was correct.
- **Files modified:** `pktap-core/src/signing.rs`

**2. [Rule 2 - Missing Critical Functionality] All-zero verifying key not rejected by dalek**
- **Found during:** Task 2 implementation
- **Issue:** `ed25519-dalek 2.x` `VerifyingKey::from_bytes` accepts the all-zero identity point (unlike `keys::validate_ed25519_public_key` which has an explicit check). The plan required `InvalidKey` for invalid keys but did not explicitly handle this edge case.
- **Fix:** Added an explicit all-zero check at the top of `verify_signature` before calling `VerifyingKey::from_bytes`. This closes the same weak-key vector guarded in `keys.rs`.
- **Files modified:** `pktap-core/src/signing.rs`

**3. [Rule 1 - Bug] IETF XChaCha20-Poly1305 KAT used wrong expected bytes**
- **Found during:** Task 1 GREEN phase (test was failing)
- **Issue:** The plan referenced ciphertext starting with `453c0693` (ChaCha20 stream cipher test vector, not XChaCha20-Poly1305 AEAD). The correct draft-arciszewski-xchacha-03 §A.1 vector with the correct AAD (`50515253c0c1c2c3c4c5c6c7`) produces `bd6d179d...`.
- **Fix:** Fixed KAT to use the correct expected bytes from the `chacha20poly1305` crate's own test vectors (verified against draft appendix A.1). Added `Payload { msg, aad }` API to pass AAD for the IETF vector, while keeping the no-AAD path tested separately (PKTap's actual wire format).
- **Files modified:** `pktap-core/src/cipher.rs`

## Known Stubs

None — all three modules are fully implemented with passing tests.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. All files are pure Rust crypto implementations with no I/O.

Threat mitigations from the plan's STRIDE register are implemented:
- **T-01-05** (nonce reuse): `OsRng` generates fresh 24-byte nonce per encryption call
- **T-01-06** (ciphertext tampering): AEAD tag rejection tested with tampered-ciphertext and tampered-nonce tests
- **T-01-07** (signature forgery): `VerifyingKey::verify()` uses constant-time comparison; tampered message and wrong-key tests pass

## Self-Check

Files exist:
- pktap-core/src/cipher.rs: FOUND
- pktap-core/src/signing.rs: FOUND
- pktap-core/src/record.rs: FOUND

Commits exist:
- 976515c: FOUND
- 35de4b8: FOUND
