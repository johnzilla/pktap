# Phase 2: Pkarr DHT Integration - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

DHT publish/resolve in pure Rust using the pkarr crate. This phase adds a `DhtClient` module to `pktap-core` that can publish signed encrypted records to the Mainline DHT and resolve them back. Includes deterministic address derivation, BEP-44 sequence number management, size budget enforcement (~858 usable bytes), TTL tracking, and offline queuing with retry. No Android code — pure Rust library extending Phase 1's crypto core.

</domain>

<decisions>
## Implementation Decisions

### Pkarr API Usage
- **D-01:** Use pkarr 2.3.x (not 5.x+). Research confirmed 2.3.x is compatible with ed25519-dalek 2.x and curve25519-dalek 4.x from Phase 1. Pkarr 5.x pulls dalek 3.0.0-pre which conflicts with the Phase 1 crypto stack.
- **D-02:** Pkarr signs DHT records in Rust using the HKDF-derived key. This is transport-layer signing (BEP-44 requires it), separate from the contact payload signature handled by Android Keystore (Phase 1 D-03). The HKDF-derived key is acceptable for DHT record signing because it's a transport concern, not an identity assertion.

### Offline Queue Strategy
- **D-03:** In-memory queue (VecDeque) for pending publishes. Lost on process kill — Android layer (Phase 6) can persist to Room/SQLite if needed. Keeps Phase 2 scope minimal.
- **D-04:** Publish-attempt-based retry with exponential backoff (1s, 2s, 4s... capped at 60s). No separate connectivity detection — the publish attempt IS the connectivity check. On network error, enqueue and schedule retry.

### Record Lifecycle & TTL
- **D-05:** Phase 2 provides the republish mechanism and TTL tracking. Phase 6 (Android WorkManager) triggers republishing on schedule. Phase 2 does NOT run background tasks or spawn Tokio timers — it exposes `republish(record_key)` and `get_records_expiring_before(timestamp)`.
- **D-06:** BEP-44 sequence numbers use `SystemTime::now()` as u64 unix timestamp at publish time. If two publishes happen in the same second, increment by 1. No persistent counter needed.

### Integration Testing
- **D-07:** Use a local DHT bootstrap node in the test process via pkarr/mainline test helpers. No external network required. Tests are deterministic and fast. Publish and resolve against the local node.

### Claude's Discretion
- DhtClient module structure (single file vs split)
- Async runtime choice (tokio features needed for pkarr)
- Exact retry backoff implementation details
- Whether to expose TTL as a configurable parameter or hardcode 24h/7d defaults
- Error variant naming for DHT-specific failures (DhtPublishFailed, DhtResolveFailed, etc.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Specifications
- `.planning/PROJECT.md` — Core constraints (no server, DHT only), Pkarr protocol description
- `.planning/REQUIREMENTS.md` §DHT Integration — DHT-01 through DHT-08 acceptance criteria
- `CLAUDE.md` §Technology Stack — pkarr 2.3.x version pin, recommended Rust crate versions

### Phase 1 Code (reuse these)
- `pktap-core/src/record.rs` — `shared_record_name()`, `public_profile_name()`, `validate_plaintext_size()` — Phase 2 builds on these for DHT address derivation
- `pktap-core/src/ffi.rs` — Composite functions that produce the encrypted byte blobs Phase 2 will publish
- `pktap-core/src/error.rs` — `PktapError` enum to extend with DHT error variants

### Prior Phase Decisions
- `.planning/phases/01-rust-crypto-core/01-CONTEXT.md` — D-03 (split signing model), D-06 (byte layout), D-07 (typed error enum)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `record::shared_record_name(pk_a, pk_b)` — returns the `_pktap._share.<hash>` DNS name, directly usable as the DHT address
- `record::public_profile_name(pk)` — returns the `_pktap._profile.<hash>` DNS name for public mode
- `record::validate_plaintext_size(payload)` — enforces the 750-byte plaintext limit before encryption
- `ffi::ecdh_and_encrypt()` — produces the opaque encrypted byte blob that gets published to DHT
- `error::PktapError` — typed error enum via UniFFI, extend with DHT variants

### Established Patterns
- ZeroizeOnDrop newtypes for all secret material
- TDD approach with KATs and round-trip tests
- Module-per-concern structure (keys.rs, ecdh.rs, cipher.rs, signing.rs, record.rs, ffi.rs)

### Integration Points
- Phase 2 adds `dht.rs` (or `dht/` directory) to `pktap-core`
- New error variants added to `PktapError` for DHT failures
- `ffi.rs` composite functions may gain DHT-aware wrappers (encrypt + publish as one call)
- Phase 3 (UniFFI Bridge) will export the DHT functions alongside the crypto composites

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

*Phase: 02-pkarr-dht-integration*
*Context gathered: 2026-04-05*
