# Phase 1: Rust Crypto Core - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

All cryptographic primitives in Rust, unit-tested in isolation, with zeroize memory safety. This phase delivers the `pktap-core` Rust library with a finalized API surface covering: Ed25519/X25519 key conversion, ECDH key agreement, HKDF key derivation, XChaCha20-Poly1305 encryption/decryption, Ed25519 signing/verification, DNS TXT record construction, and memory zeroing. No Android code, no DHT networking — pure Rust crypto library.

</domain>

<decisions>
## Implementation Decisions

### API Surface Design
- **D-01:** Composites only across FFI — only expose high-level operations (`ecdhAndEncrypt`, `decryptAndVerify`). No lower-level primitives cross the FFI boundary. Internal Rust modules may use fine-grained functions, but the UniFFI-exported surface is composite-only.
- **D-02:** Rust accepts raw Ed25519 public key bytes (32 bytes) from Kotlin. Android Keystore generates the master keypair (hardware-backed, non-extractable). Rust never generates the master key.
- **D-03:** Split signing/encryption model — Android Keystore owns Ed25519 signing (private key never leaves hardware). For ECDH, Rust derives an X25519 key from the HKDF seed (passed from Kotlin as encrypted bytes, decrypted via Keystore AES key). This means the composite `ecdhAndEncrypt` handles encryption only; signing is a separate Keystore operation on the Kotlin side.
- **D-04:** Composite functions return opaque byte blobs (`Vec<u8>`) — nonce + ciphertext + tag in a single buffer, ready to be signed externally and published. Kotlin never parses crypto internals.

### Record Wire Format
- **D-05:** Contact fields serialized as JSON inside the encrypted payload. Simple `{"name":"Alice","email":"a@b.com"}` encoding. Matches `kotlinx-serialization` on the Android side. Well within the ~600 byte budget for text-only fields.
- **D-06:** Encrypted record byte layout: `version(1) + nonce(24) + ciphertext(var) + tag(16)`. Version byte first for future-proofing. Total fixed overhead: 41 bytes. Signature is applied separately by Keystore via Pkarr's signing mechanism.

### Error Handling
- **D-07:** Typed error enum via UniFFI — define a `PktapError` enum (`InvalidKey`, `RecordInvalid`, `RecordTooLarge`, `SerializationFailed`, etc.) that UniFFI maps to a Kotlin sealed class. Each variant carries a safe message.
- **D-08:** Coalesce crypto failures externally — `DecryptionFailed` and `SignatureInvalid` map to a single `RecordInvalid` error across FFI to prevent oracle attacks. Internal Rust code may distinguish them for logging/debugging, but the FFI surface exposes only the coalesced variant.

### Testing Approach
- **D-09:** Known Answer Tests (KATs) from RFC 7748 (X25519), RFC 8032 (Ed25519), and IETF draft vectors for XChaCha20-Poly1305. Plus round-trip tests for every composite function.
- **D-10:** Include a pipeline integration test within this phase: generate keys -> ECDH -> encrypt -> construct record -> parse record -> decrypt. Proves the pieces fit together before Phase 2 adds DHT.

### Claude's Discretion
- Internal Rust module structure and file organization
- Exact `Cargo.toml` dependency version pins (within the ranges specified in CLAUDE.md)
- HKDF domain separator string format (requirement says "pktap-v1")
- Zeroize implementation details (derive vs manual `Drop`)
- Whether to use `#[cfg(test)]` inline tests or a separate `tests/` directory

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Specifications
- `.planning/PROJECT.md` — Core constraints (crypto in Rust, memory safety, no server), key decisions (Rust-heavy architecture, BIP-39 mnemonic)
- `.planning/REQUIREMENTS.md` §Crypto Core — CRYPTO-01 through CRYPTO-07 acceptance criteria, KEY-06 zeroize requirement
- `CLAUDE.md` §Technology Stack — Recommended Rust crate versions with confidence levels, Rust workspace layout

### Protocol Design
- `.planning/REQUIREMENTS.md` §DHT Integration — DHT-07 defines the ~858 usable byte budget (1000 byte BEP-44 limit minus wire format + AEAD overhead) — record construction must respect this
- `pktap_prd.md` — Original PRD with detailed protocol specification

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project with no existing Rust code

### Established Patterns
- None yet — this phase establishes the foundational patterns for the Rust workspace

### Integration Points
- Phase 2 (Pkarr DHT) will consume the record construction and byte-blob output from this phase
- Phase 3 (UniFFI Bridge) will generate Kotlin bindings from the UniFFI-exported functions defined here
- Phase 4 (Android Keystore) will call composite functions, passing HKDF seed bytes and receiving opaque encrypted blobs
- The signing split (D-03) means Phase 4/5 must handle Ed25519 signing on the Kotlin side via Keystore, then pass the signature back for Pkarr record assembly

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches within the decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-rust-crypto-core*
*Context gathered: 2026-04-05*
