---
phase: 01-rust-crypto-core
plan: 03
subsystem: crypto
tags: [rust, uniffi, ed25519, x25519, xchacha20poly1305, ecdh, ffi]

# Dependency graph
requires:
  - phase: 01-rust-crypto-core plan 01
    provides: keys, ecdh, error modules; seed_to_x25519_scalar; HKDF derivation
  - phase: 01-rust-crypto-core plan 02
    provides: cipher (encrypt/decrypt), signing (sign/verify), record (validate/names)

provides:
  - "ecdh_and_encrypt: composite FFI function — ECDH + HKDF + XChaCha20 encryption in one call"
  - "decrypt_and_verify: composite FFI function — sig verify + ECDH + decrypt with D-08 error coalescing"
  - "derive_shared_record_name: FFI wrapper for symmetric DHT record name derivation"
  - "D-10 pipeline integration test covering full key-gen -> encrypt -> sign -> verify -> decrypt flow"

affects:
  - "Phase 2: DHT client will call ecdh_and_encrypt and decrypt_and_verify as its crypto layer"
  - "Phase 3: UniFFI Kotlin bindings generation targets these exported functions"
  - "Phase 5: NFC HCE — Android layer calls these functions after receiving peer Ed25519 public key"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-08 error coalescing: all decrypt_and_verify error paths return RecordInvalid to prevent oracle attacks"
    - "D-10 pipeline integration pattern: generate keys -> encrypt -> sign -> verify -> decrypt"
    - "Seed = signing_key.to_scalar_bytes(): ECDH seed must be Ed25519 scalar bytes for symmetry"
    - "Verify-before-decrypt: signature checked before ECDH derivation (T-01-12)"
    - "ZeroizeOnDrop via DerivedKey + explicit seed.zeroize() after key derivation (T-01-11)"

key-files:
  created:
    - "pktap-core/src/ffi.rs — composite UniFFI export functions with full test coverage"
  modified:
    - "pktap-core/src/keys.rs — removed incorrect HKDF layer from seed_to_x25519_scalar (symmetry fix)"

key-decisions:
  - "seed_to_x25519_scalar uses direct pass-through (not HKDF) so that passing signing_key.to_scalar_bytes() as seed produces the X25519 scalar whose public key equals signing_key.verifying_key().to_montgomery()"
  - "peer_ed25519_public is used for both ECDH (via to_montgomery()) and signature verification — this is the canonical Ed25519/X25519 birational equivalence path"
  - "D-08 enforced: ALL decrypt_and_verify error paths (sig fail, ECDH fail, decrypt fail, UTF-8 fail) map to single RecordInvalid"

patterns-established:
  - "FFI boundary: only composite functions cross UniFFI — no raw keys, shared secrets, or intermediate material"
  - "Error coalescing at FFI: decrypt path returns RecordInvalid for all failures to prevent side-channel oracle"
  - "Test consistency: seed = signing_key.to_scalar_bytes() for any test requiring ECDH round-trip"

requirements-completed: [CRYPTO-05, CRYPTO-07, KEY-06]

# Metrics
duration: 35min
completed: 2026-04-05
---

# Phase 01 Plan 03: Composite FFI Functions Summary

**ecdh_and_encrypt and decrypt_and_verify UniFFI exports wiring all Phase 1 crypto modules into a verified end-to-end pipeline, with D-08 oracle-resistant error coalescing and D-10 bidirectional integration test**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-04-05T13:00:00Z
- **Completed:** 2026-04-05T13:35:26Z
- **Tasks:** 1 (TDD: RED → GREEN → fixed deviations)
- **Files modified:** 2

## Accomplishments

- Implemented `ecdh_and_encrypt` — single FFI call that performs ECDH key agreement (X25519), HKDF-SHA256 derivation, and XChaCha20-Poly1305 encryption; returns opaque D-06 byte blob; seed bytes zeroed after use
- Implemented `decrypt_and_verify` — verifies Ed25519 signature BEFORE key derivation, then ECDH + decrypt; ALL error paths coalesced to `RecordInvalid` per D-08 to prevent decryption/verification oracle attacks (T-01-09)
- Implemented `derive_shared_record_name` — FFI wrapper for deterministic shared DHT address computation
- D-10 pipeline integration test passes in both directions (Alice→Bob and Bob→Alice) with freshly generated keypairs
- Fixed critical ECDH symmetry bug in `seed_to_x25519_scalar`: removed incorrect HKDF layer that broke the binding between Ed25519 public key and ECDH private key
- All 54 tests across all modules pass; zero duplicate `curve25519-dalek` versions

## Task Commits

1. **Task 1: Composite FFI functions with UniFFI export** - `d311e17` (feat)

## Files Created/Modified

- `pktap-core/src/ffi.rs` — three `#[uniffi::export]` composite functions; 13 tests covering all behavior and D-08/D-10 requirements
- `pktap-core/src/keys.rs` — `seed_to_x25519_scalar` simplified to identity pass-through (HKDF removed); StaticSecret::from() handles clamping

## Decisions Made

- **Direct scalar pass-through in seed_to_x25519_scalar**: The original HKDF transform broke ECDH symmetry because it decoupled the X25519 private scalar from the Ed25519 public key. Removing HKDF restores the canonical Curve25519 binding: `signing_key.to_scalar_bytes()` as seed → `StaticSecret::from()` clamps it → produces the X25519 private key whose public key equals `verifying_key.to_montgomery()`.
- **peer_ed25519_public serves dual role**: The same 32-byte key is used for signature verification (via `VerifyingKey::from_bytes`) and ECDH (via `to_montgomery()`). This is the correct and intended architecture — one key exchanged over NFC, used for both operations.
- **Verify-before-decrypt ordering**: Signature verification happens before ECDH derivation and decryption. Forged records are rejected before any key material is derived (T-01-12 mitigation).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ECDH symmetry: removed HKDF from seed_to_x25519_scalar**
- **Found during:** Task 1 (round-trip test failures in GREEN phase)
- **Issue:** `seed_to_x25519_scalar` applied `HKDF(seed, "pktap-v1-x25519")` which produced a different scalar than the one corresponding to the Ed25519 public key's Montgomery form. This meant `ecdh_derive_key(seed_a, pub_b) != ecdh_derive_key(seed_b, pub_a)` when using `signing_key.to_scalar_bytes()` as the seed, making the round-trip impossible.
- **Fix:** Replaced HKDF expansion with identity pass-through — `X25519ScalarBytes(*seed)`. The `StaticSecret::from([u8; 32])` call in `ecdh_derive_key` handles Curve25519 clamping automatically.
- **Files modified:** `pktap-core/src/keys.rs`
- **Verification:** All 54 tests pass including D-10 bidirectional pipeline integration test
- **Committed in:** `d311e17` (included in Task 1 commit)

**2. [Rule 1 - Bug] Fixed SigningKey::generate API mismatch**
- **Found during:** Task 1 (RED phase compile error)
- **Issue:** Plan specified `SigningKey::generate(&mut OsRng)` but ed25519-dalek 2.2.0 does not expose this method
- **Fix:** Used `OsRng.fill_bytes(&mut seed_bytes)` then `SigningKey::from_bytes(&seed_bytes)`
- **Files modified:** `pktap-core/src/ffi.rs` (test only)
- **Verification:** Compiles and passes
- **Committed in:** `d311e17`

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bug fixes)
**Impact on plan:** Both fixes necessary for correctness. The HKDF removal is architecturally significant — it aligns the implementation with the canonical Curve25519 key derivation path documented in ed25519-dalek's own API (`to_scalar_bytes()` / `to_montgomery()` pairing).

## Issues Encountered

- The plan's D-10 test specification used `SigningKey::generate(&mut OsRng)` which does not exist in ed25519-dalek 2.2.0. Auto-fixed via `OsRng.fill_bytes`.
- The plan's D-10 test data suggestion (using `signing_key.to_scalar_bytes()` as seed with arbitrary Ed25519 keys) only works correctly after fixing `seed_to_x25519_scalar` to use direct pass-through — the original HKDF version would have made the test fail even with the correct seed derivation approach.

## Known Stubs

None — all three exported functions are fully implemented with no placeholder values or TODO paths.

## Threat Flags

No new security surface beyond what was specified in the plan's threat model. All T-01-09 through T-01-13 mitigations are implemented and tested.

## Next Phase Readiness

- Phase 1 crypto core is complete: all 54 tests pass, no warnings from plan-added code
- Phase 2 (DHT client) can import `ffi::ecdh_and_encrypt` and `ffi::decrypt_and_verify` directly
- Phase 3 (UniFFI Kotlin bindings) has `#[uniffi::export]` on all three public functions
- **Note for Phase 2**: The HKDF removal from `seed_to_x25519_scalar` means the production seed passed from Kotlin must be `signing_key.to_scalar_bytes()` (the clamped Ed25519 scalar bytes) — document this in the Kotlin FFI call site when EncryptedSharedPreferences integration is added

## Self-Check: PASSED

- `pktap-core/src/ffi.rs` — FOUND
- `pktap-core/src/keys.rs` — FOUND
- `.planning/phases/01-rust-crypto-core/01-03-SUMMARY.md` — FOUND
- Commit `d311e17` — FOUND

---
*Phase: 01-rust-crypto-core*
*Completed: 2026-04-05*
