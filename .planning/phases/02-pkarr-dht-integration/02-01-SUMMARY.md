---
phase: 02-pkarr-dht-integration
plan: 01
subsystem: dht
tags: [pkarr, mainline-dht, bep44, rust, uniffi, ed25519, dns-txt]

# Dependency graph
requires:
  - phase: 01-rust-crypto-core
    provides: PktapError, record module (shared_record_name, public_profile_name), HKDF seed bytes, ecdh_and_encrypt/decrypt_and_verify FFI

provides:
  - DhtClient struct with publish_encrypted/resolve_encrypted for private records
  - DhtClient::publish_public/resolve_public for public profile records
  - PktapError extended with DhtPublishFailed, DhtResolveFailed, DhtOutdatedRecord, DhtPublishQueued
  - pkarr 2.3.1 integrated without version conflicts with Phase 1 dalek stack
  - 6 integration/unit tests covering publish/resolve round-trip, size validation, TTL, sequence rejection

affects: [03-android-nfc, 06-android-crypto-integration, ffi-dht-wrappers]

# Tech tracking
tech-stack:
  added:
    - pkarr 2.3.1 (resolved from ^2.3.0) with dht feature — PkarrClient, Keypair, SignedPacket, PublicKey
    - mainline 2.0.1 (transitive via pkarr) — Testnet for local DHT integration tests
    - simple-dns 0.9.3 (transitive via pkarr) — DNS packet construction, TXT/CharacterString
  patterns:
    - DhtClient wraps PkarrClient; all pkarr errors mapped to PktapError at the boundary
    - Keypair::from_secret_key(&[u8;32]) derives Ed25519 keypair from HKDF seed (DHT address = keypair.public_key())
    - CharacterString 255-byte chunking for binary ciphertext in TXT records
    - Binary TXT rdata extraction via mini-packet serialization (workaround for pub(crate) WireFormat in simple-dns)
    - Testnet::new(10) with DhtSettings bootstrap override for all integration tests

key-files:
  created:
    - pktap-core/src/dht.rs — DhtClient, constants (PRIVATE_RECORD_TTL=86400, PUBLIC_RECORD_TTL=604800, MAX_CIPHERTEXT_LEN=850), build_signed_packet, publish_packet, resolve_bytes helpers, 6 tests
  modified:
    - pktap-core/Cargo.toml — added pkarr = { version = "2.3.0", features = ["dht"] }
    - pktap-core/src/error.rs — added DhtPublishFailed, DhtResolveFailed, DhtOutdatedRecord, DhtPublishQueued variants
    - pktap-core/src/lib.rs — added pub mod dht
    - Cargo.lock — 49 new packages locked

key-decisions:
  - "Use pkarr 2.3.x (not 5.x+): 2.3.x is compatible with ed25519-dalek 2.x and curve25519-dalek 4.x from Phase 1; 5.x pulls dalek 3.0.0-pre which conflicts"
  - "Keypair::from_secret_key(&[u8;32]) as DHT signing key: ECDH is symmetric so both peers derive identical keypair from shared HKDF secret; both can publish/resolve same address"
  - "Binary TXT extraction via mini-packet serialization: simple-dns WireFormat trait is pub(crate); built fresh Packet with one answer, serialized with build_bytes_vec(), parsed TXT rdata wire bytes manually"
  - "Testnet::new(10) for all integration tests: no external network required, deterministic"
  - "Sequence rejection test: publish sp_a, sleep 2ms, publish sp_b (newer), re-publish sp_a triggers cache check returning NotMostRecent -> DhtOutdatedRecord"

patterns-established:
  - "Pattern: DhtClient::with_bootstrap(Vec<String>) constructor for testnet override"
  - "Pattern: publish_packet() helper centralizes pkarr::Error -> PktapError mapping"
  - "Pattern: build_signed_packet() returns (Keypair, SignedPacket) to expose public key to caller"

requirements-completed: [DHT-01, DHT-02, DHT-05, DHT-06, DHT-07]

# Metrics
duration: 9min
completed: 2026-04-05
---

# Phase 2 Plan 01: Pkarr DHT Integration Summary

**DhtClient wrapping pkarr 2.3.1 with publish/resolve for encrypted records via Mainline DHT, size validation, stale-publish rejection, and 6 passing integration/unit tests against local testnet**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-04-05T11:41:16Z
- **Completed:** 2026-04-05T11:50:34Z
- **Tasks:** 2
- **Files modified:** 5 (plus Cargo.lock)

## Accomplishments
- pkarr 2.3.1 integrated without version conflicts with Phase 1 crypto stack (ed25519-dalek 2.2.0, curve25519-dalek 4.1.3)
- DhtClient with publish_encrypted/resolve_encrypted (PRIVATE_RECORD_TTL=86400) and publish_public/resolve_public (PUBLIC_RECORD_TTL=604800)
- MAX_CIPHERTEXT_LEN=850 pre-validated before packet build; oversized payloads rejected with RecordTooLarge
- Stale publish correctly returns DhtOutdatedRecord via pkarr's cache check (NotMostRecent mapping)
- 60 total tests green (54 Phase 1 + 6 new DHT)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add pkarr dependency and extend PktapError** - `7467868` (feat)
2. **Task 2: Implement DhtClient with publish/resolve and integration tests** - `872950f` (feat)

## Files Created/Modified
- `pktap-core/src/dht.rs` — DhtClient, TTL/size constants, packet helpers, 6 tests (created)
- `pktap-core/src/error.rs` — DhtPublishFailed, DhtResolveFailed, DhtOutdatedRecord, DhtPublishQueued added
- `pktap-core/src/lib.rs` — pub mod dht declared
- `pktap-core/Cargo.toml` — pkarr 2.3.0 dependency added
- `Cargo.lock` — 49 new packages locked

## Decisions Made
- Used pkarr 2.3.x not 5.x+ to maintain dalek version compatibility with Phase 1
- Binary TXT rdata extraction required a workaround: `simple-dns`'s `WireFormat` trait is `pub(crate)` so `TXT.write_to()` is inaccessible from outside the crate. Solved by building a fresh single-answer DNS packet, calling `Packet::build_bytes_vec()`, and parsing the TXT rdata from the known wire-format byte offsets.
- Sequence rejection test uses pkarr's client-side cache check: publish A, sleep 2ms, publish B (newer timestamp captured in cache), re-publish A triggers `NotMostRecent` -> `DhtOutdatedRecord`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Binary TXT extraction via mini-packet serialization instead of WireFormat**
- **Found during:** Task 2 (DhtClient implementation)
- **Issue:** Research doc Pattern 4 referenced `txt.strings` and `WireFormat::write_to()` for extracting binary bytes from resolved TXT records. Both are `pub(crate)` in simple-dns 0.9.3 and inaccessible from pktap-core.
- **Fix:** Build a single-answer `Packet` containing the resolved `ResourceRecord`, call `Packet::build_bytes_vec()` (which is public), then parse the serialized bytes manually: skip 12-byte DNS header, scan past variable-length name, skip 8 bytes type/class/TTL, read 2-byte rdlength, parse TXT wire chunks (length-prefixed).
- **Files modified:** pktap-core/src/dht.rs
- **Verification:** test_publish_resolve_round_trip passes - 200 bytes published, same 200 bytes resolved
- **Committed in:** 872950f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - implementation bug/API constraint)
**Impact on plan:** Essential for correct binary ciphertext retrieval. Wire format parsing is deterministic and well-specified (RFC 1035). No scope creep.

## Issues Encountered
- Cargo test was running against the main repo (`/home/john/vault/projects/github.com/pktap/`) rather than the worktree when invoked with `cd /home/john/vault/projects/github.com/pktap && cargo test`. Fixed by running all cargo commands without `cd` (working directory is already the worktree). No code changes needed.

## Known Stubs
None - DhtClient methods are fully wired with real pkarr 2.3.1 DHT operations.

## Next Phase Readiness
- Phase 2 Plan 02 can proceed: DhtClient is available at `pktap_core::dht::DhtClient`
- FFI wrappers for publish/resolve (Phase 3) can call `DhtClient::with_bootstrap`/`DhtClient::new`
- Offline queue (D-03, D-04, D-05 from RESEARCH.md) is deferred to Phase 6 as planned; `DhtPublishQueued` error variant is reserved for that use

---
*Phase: 02-pkarr-dht-integration*
*Completed: 2026-04-05*
