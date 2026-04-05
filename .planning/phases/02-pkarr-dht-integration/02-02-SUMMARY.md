---
phase: 02-pkarr-dht-integration
plan: 02
subsystem: dht
tags: [pkarr, mainline-dht, rust, offline-queue, exponential-backoff, ttl-tracking, republish, zeroize]

# Dependency graph
requires:
  - phase: 02-pkarr-dht-integration
    plan: 01
    provides: DhtClient, publish_encrypted, publish_public, resolve_encrypted, resolve_public, PktapError DHT variants

provides:
  - DhtClient.queue: Mutex<VecDeque<PendingPublish>> — offline queue with exponential backoff
  - DhtClient.flush_queue() -> usize — retry due items, remove successes, apply backoff on failure
  - DhtClient.queue_len() -> usize — pending queue item count
  - DhtClient.tracked: Mutex<Vec<TrackedRecord>> — TTL tracking for published records
  - DhtClient.get_records_expiring_before(unix_timestamp: u64) -> Vec<String>
  - DhtClient.republish(record_name: &str) -> Result<(), PktapError>
  - DhtClient.tracked_count() -> usize
  - TrackedRecord struct with Zeroize/Drop zeroing seed and data bytes
  - 7 new tests (4 offline queue + 3 TTL/republish); total 13 DHT tests

affects: [06-android-crypto-integration, ffi-dht-wrappers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - PendingPublish stored in VecDeque<PendingPublish> behind Mutex for offline queue
    - Backoff formula: delay = min(2^attempt_count, 60) seconds; capped to prevent unbounded wait
    - publish_encrypted/publish_public return DhtPublishQueued (not DhtPublishFailed) when offline
    - TrackedRecord upserted by record_name on each successful publish; published_at updated on republish
    - Manual Drop impl on TrackedRecord zeroes seed ([u8;32]) and data (Vec<u8>) using zeroize crate
    - republish() clones data out of tracked lock before calling publish to avoid holding lock during network I/O

key-files:
  created: []
  modified:
    - pktap-core/src/dht.rs — PendingPublish, TrackedRecord, queue/tracked fields, flush_queue, queue_len, get_records_expiring_before, republish, tracked_count, track_record, 7 new tests

key-decisions:
  - "DhtPublishQueued returned on offline publish: callers need visibility into whether publish was immediate or deferred; DhtPublishFailed is reserved for unrecoverable errors"
  - "Manual Drop for TrackedRecord instead of ZeroizeOnDrop derive: Vec<u8> and [u8;32] both implement Zeroize; manual Drop makes the zeroing explicit and auditable per T-02-08"
  - "track_record() upserts by record_name: republish semantics require finding the original seed; upsert avoids duplicate entries when publish_encrypted is called multiple times for the same record"
  - "Clone data out of tracked Mutex before publish in republish(): avoids holding the lock during blocking network I/O — prevents deadlock if flush_queue is called concurrently"

# Metrics
duration: 7min
completed: 2026-04-05
---

# Phase 2 Plan 02: Offline Queue, TTL Tracking, and Republish API Summary

**Offline publish queue with exponential backoff (VecDeque + Mutex), TrackedRecord TTL tracking with Zeroize drop, and republish API — 13 DHT tests + 67 total tests green**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-04-05T15:53:24Z
- **Completed:** 2026-04-05T16:00:51Z
- **Tasks:** 2
- **Files modified:** 1 (pktap-core/src/dht.rs)

## Accomplishments

- Offline publish queue: `VecDeque<PendingPublish>` behind `Mutex` added to `DhtClient`
- `publish_encrypted` and `publish_public` now return `DhtPublishQueued` on network failure and enqueue the `SignedPacket` for retry
- `flush_queue()` iterates due items, publishes, removes successes; failed items get backoff: `delay = min(2^attempt_count, 60)` seconds
- `queue_len()` exposes queue depth for Phase 6 "pending sync" indicator
- `TrackedRecord` struct stores seed, record_name, data, ttl_secs, published_at, is_public
- `Drop` impl on `TrackedRecord` zeroes `seed` and `data` bytes (satisfies threat T-02-08)
- `get_records_expiring_before(unix_timestamp)` returns record names whose `published_at + ttl_secs < unix_timestamp` — Phase 6 WorkManager contract
- `republish(record_name)` re-publishes with a fresh BEP-44 microsecond timestamp, updates `published_at` on success
- `tracked_count()` for test inspection
- 13 DHT tests pass; full suite: 67 tests green, zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Offline queue with exponential backoff** - `811535d` (feat)
2. **Task 2: TTL tracking, public mode, republish API** - `c72a6d1` (feat)

## Files Created/Modified

- `pktap-core/src/dht.rs` — PendingPublish struct, TrackedRecord struct (with Drop/Zeroize), DhtClient extended with queue + tracked fields, flush_queue, queue_len, get_records_expiring_before, republish, tracked_count, track_record, 7 new tests

## Decisions Made

- `DhtPublishQueued` returned on offline publish so callers distinguish deferred vs. failed
- Manual `Drop` on `TrackedRecord` instead of `ZeroizeOnDrop` derive — makes zeroing auditable
- `track_record()` upserts by `record_name` to avoid duplicates on repeated publishes
- Clone data out of `tracked` lock before network I/O in `republish()` to prevent deadlock

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Cast precedence in filter expression**
- **Found during:** Task 2 (compile step)
- **Issue:** `r.ttl_secs as u64 < unix_timestamp` was parsed as `r.ttl_secs as (u64 < unix_timestamp)` by Rust (generic syntax ambiguity)
- **Fix:** Added parentheses: `(r.ttl_secs as u64) < unix_timestamp`
- **Files modified:** pktap-core/src/dht.rs
- **Commit:** c72a6d1 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - compile error)
**Impact on plan:** One-character fix; no scope creep.

## Known Stubs

None — all methods are fully implemented with real pkarr DHT operations.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced beyond what the plan's threat model covers. All threats mitigated as specified:

| Threat | Status |
|--------|--------|
| T-02-08: TrackedRecord.seed in memory | Mitigated — manual Drop zeroes seed and data |
| T-02-09: Unbounded offline queue | Accepted — queue bounded by process lifetime; no max size needed for Phase 2 |
| T-02-10: Public mode spoofing | Accepted — BEP-44 signature verifies publisher |
| T-02-11: Republish with stale data | Mitigated — pkarr auto-generates new microsecond timestamp; NotMostRecent returns DhtOutdatedRecord |

## Self-Check: PASSED

- pktap-core/src/dht.rs — FOUND
- commit 811535d (Task 1) — FOUND
- commit c72a6d1 (Task 2) — FOUND

## Next Phase Readiness

- Phase 3 FFI wrappers can expose `flush_queue`, `queue_len`, `get_records_expiring_before`, `republish` via UniFFI
- Phase 6 Android WorkManager can call `get_records_expiring_before(now + ttl_buffer)` then `republish(name)` for each result
