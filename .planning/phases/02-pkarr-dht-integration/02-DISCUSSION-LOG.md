# Phase 2: Pkarr DHT Integration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-05
**Phase:** 02-pkarr-dht-integration
**Areas discussed:** Pkarr API usage pattern, Offline queue strategy, Record lifecycle & TTL, Integration test approach

---

## Pkarr API Usage Pattern

| Option | Description | Selected |
|--------|-------------|----------|
| pkarr 2.3.x | Stable, wraps mainline BEP-44, compatible with ed25519-dalek 2.x | ✓ |
| pkarr latest (5.x+) | Newer API but pulls dalek 3.0.0-pre — conflicts with Phase 1 | |
| Skip pkarr, use mainline directly | More control but must reimplement pkarr's DNS format | |

**User's choice:** pkarr 2.3.x
**Notes:** Research confirmed no version conflicts with Phase 1 deps.

| Option | Description | Selected |
|--------|-------------|----------|
| Pkarr signs in Rust | Use pkarr's built-in signing with HKDF-derived key for transport-layer DHT signing | ✓ |
| Pass signature from Kotlin | Have Keystore sign BEP-44 records, pass to Rust | |
| You decide | Let Claude determine | |

**User's choice:** Pkarr signs in Rust
**Notes:** DHT record signing is transport-layer, separate from identity signing (Keystore).

---

## Offline Queue Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| In-memory queue | VecDeque, lost on kill, simplest for Phase 2 | ✓ |
| File-backed queue | JSON on disk, survives restart | |
| You decide | Let Claude pick | |

**User's choice:** In-memory queue
**Notes:** Android layer can persist to Room/SQLite in Phase 6.

| Option | Description | Selected |
|--------|-------------|----------|
| Publish-attempt-based | Try publish, on error enqueue + exponential backoff | ✓ |
| Periodic flush with backoff | Background task flushes queue periodically | |
| Caller-driven retry | No auto retry, caller decides | |

**User's choice:** Publish-attempt-based
**Notes:** The publish attempt IS the connectivity check. Backoff 1s→2s→4s...→60s cap.

---

## Record Lifecycle & TTL

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 2 provides mechanism, Phase 6 triggers | Build republish() and TTL tracking, Android triggers | ✓ |
| Phase 2 runs background timer | Tokio task auto-republishes | |
| No republish in Phase 2 | Entirely Phase 6+ concern | |

**User's choice:** Phase 2 provides mechanism, Phase 6 triggers
**Notes:** Phase 2 does NOT run background tasks. Exposes republish() and get_records_expiring_before().

| Option | Description | Selected |
|--------|-------------|----------|
| Unix timestamp at publish time | SystemTime::now() as u64 seq, increment if same second | ✓ |
| Persistent counter in a file | Store last-used seq on disk | |
| You decide | Let Claude pick | |

**User's choice:** Unix timestamp at publish time
**Notes:** Simple, monotonic, no persistent state needed.

---

## Integration Test Approach

| Option | Description | Selected |
|--------|-------------|----------|
| Local DHT node | Spin up local bootstrap node via pkarr/mainline test helpers | ✓ |
| Mock DHT client trait | Define trait, mock for tests | |
| Live DHT network | Test against real Mainline DHT | |
| Both: local node + trait mock | Most thorough but most code | |

**User's choice:** Local DHT node
**Notes:** No external network needed. Deterministic and fast.

---

## Claude's Discretion

- DhtClient module structure
- Async runtime choice for pkarr
- Exact retry backoff details
- TTL configurability
- DHT-specific error variant naming

## Deferred Ideas

None — discussion stayed within phase scope.
