# Phase 1: Rust Crypto Core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-05
**Phase:** 01-rust-crypto-core
**Areas discussed:** API surface design, Record wire format, Error handling strategy, Testing approach

---

## API Surface Design

| Option | Description | Selected |
|--------|-------------|----------|
| Composites only | Only expose high-level operations (ecdhAndEncrypt, decryptAndVerify). No lower-level primitives cross FFI. | ✓ |
| Composites + read-only helpers | Composites for all crypto, plus non-secret helpers like computeDhtAddress, constructDnsTxt, validateRecordSize. | |
| Full layered API | Expose composites AND individual primitives behind an "advanced" namespace. | |

**User's choice:** Composites only
**Notes:** Simplest and safest approach. Internal Rust modules can still use fine-grained functions.

| Option | Description | Selected |
|--------|-------------|----------|
| Accept bytes from Kotlin | Android Keystore generates Ed25519 keypair. Kotlin passes 32-byte public key to Rust. | ✓ |
| Generate in Rust, store via Kotlin | Rust generates keypair, returns to Kotlin for Keystore storage. | |
| Dual: Keystore for master, Rust for ephemeral | Keystore generates master, Rust generates ephemeral keys. | |

**User's choice:** Accept bytes from Kotlin
**Notes:** Private key stays in hardware-backed Keystore.

| Option | Description | Selected |
|--------|-------------|----------|
| HKDF seed derives signing key in Rust | Pass HKDF seed to Rust, derive Ed25519 keypair from seed. | |
| Keystore signs, Rust encrypts separately | Keystore does Ed25519 signing. Rust derives X25519 from HKDF seed for ECDH. Split responsibility. | ✓ |
| You decide | Let Claude figure out the best approach. | |

**User's choice:** Keystore signs, Rust encrypts separately
**Notes:** Key architectural decision — signing stays in hardware, encryption key derived from seed in Rust. This means the composite FFI function handles encryption only; signing is a Kotlin-side Keystore operation.

| Option | Description | Selected |
|--------|-------------|----------|
| Opaque byte blob | Single Vec<u8> with nonce + ciphertext + tag. Kotlin never parses internals. | ✓ |
| Structured record object | UniFFI struct with named fields. Kotlin can inspect parts. | |
| Both: struct internally, bytes for FFI | Struct in Rust, serialized to bytes at FFI boundary. | |

**User's choice:** Opaque byte blob
**Notes:** Simplest FFI surface. Ready to publish directly.

---

## Record Wire Format

| Option | Description | Selected |
|--------|-------------|----------|
| JSON | Simple {"name":"Alice"} encoding. Human-debuggable, matches kotlinx-serialization. | ✓ |
| MessagePack | Binary JSON-like, ~20-30% smaller. Adds rmp-serde dependency. | |
| Custom binary format | Hand-rolled TLV encoding. Smallest but hardest to debug/extend. | |

**User's choice:** JSON
**Notes:** Well within the ~600 byte budget for text-only fields.

| Option | Description | Selected |
|--------|-------------|----------|
| version(1) + nonce(24) + ciphertext(var) + tag(16) | Version byte first for future-proofing. 41 bytes fixed overhead. | ✓ |
| nonce(24) + ciphertext(var) + tag(16) (no version) | Skip version, record name implies v1. Saves 1 byte. | |
| You decide | Let Claude pick based on Pkarr conventions. | |

**User's choice:** version(1) + nonce(24) + ciphertext(var) + tag(16)
**Notes:** Version byte enables future format changes without breaking resolution.

---

## Error Handling Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Typed enum via UniFFI | PktapError enum mapped to Kotlin sealed class. Safe messages per variant. | ✓ |
| String error messages | Result<T, String> across FFI. Simpler but no pattern matching. | |
| Error code + message | Numeric codes plus human-readable string. C-style. | |

**User's choice:** Typed enum via UniFFI
**Notes:** Enables Kotlin pattern matching for targeted UI error messages.

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, coalesce crypto failures | DecryptionFailed and SignatureInvalid → single RecordInvalid externally. Prevents oracle attacks. | ✓ |
| No, keep them separate | Separate errors across FFI. Useful for debugging. Accept theoretical oracle risk. | |
| Separate in debug, coalesced in release | Feature flag: debug gets detailed, release coalesces. | |

**User's choice:** Coalesce crypto failures
**Notes:** Security-first approach. Internal Rust logging can still distinguish for debugging.

---

## Testing Approach

| Option | Description | Selected |
|--------|-------------|----------|
| KATs + round-trips | Known Answer Tests from RFCs plus round-trip tests for composites. | ✓ |
| KATs + round-trips + fuzzing | All above plus cargo-fuzz/proptest for malformed inputs. | |
| Round-trips only | Skip external vectors. Faster but less confidence. | |

**User's choice:** KATs + round-trips
**Notes:** Solid confidence without over-engineering. RFC 7748, RFC 8032, IETF XChaCha20-Poly1305 vectors.

| Option | Description | Selected |
|--------|-------------|----------|
| Unit tests only | Test each primitive in isolation. DHT is Phase 2. | |
| Unit + pipeline test | Also test full chain: keys → ECDH → encrypt → construct → parse → decrypt. | ✓ |
| You decide | Let Claude determine test boundaries. | |

**User's choice:** Unit + pipeline test
**Notes:** Proves pieces fit together before Phase 2 adds DHT integration.

---

## Claude's Discretion

- Internal Rust module structure and file organization
- Exact Cargo.toml dependency version pins
- HKDF domain separator string format
- Zeroize implementation details
- Test file organization

## Deferred Ideas

None — discussion stayed within phase scope.
