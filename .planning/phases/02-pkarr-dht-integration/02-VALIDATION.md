---
phase: 2
slug: pkarr-dht-integration
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-04-05
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + `cargo test` |
| **Config file** | none — Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test -p pktap-core dht` |
| **Full suite command** | `cargo test --all` |
| **Estimated runtime** | ~10 seconds (DHT bootstrap + publish/resolve) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p pktap-core dht`
- **After every plan wave:** Run `cargo test --all`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

Tests are created inline during task execution via TDD (`tdd="true"` on each task). There is no separate Wave 0 test scaffold plan — each task writes its tests as part of the RED-GREEN-REFACTOR cycle.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Created By | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|------------|--------|
| 02-01-01 | 01 | 1 | DHT-01, DHT-02 | — | Publish/resolve round-trip with deterministic address | integration | `cargo test -p pktap-core dht::tests::test_publish_resolve` | Plan 01 Task 2 (TDD) | ⬜ pending |
| 02-01-02 | 01 | 1 | DHT-07 | — | Record exceeding byte budget rejected before publish | unit | `cargo test -p pktap-core dht::tests::test_size_validation` | Plan 01 Task 2 (TDD) | ⬜ pending |
| 02-01-03 | 01 | 1 | DHT-06 | — | Monotonic sequence numbers verified, stale publish rejected with DhtOutdatedRecord | unit + integration | `cargo test -p pktap-core dht::tests::test_sequence` | Plan 01 Task 2 (TDD) | ⬜ pending |
| 02-01-04 | 01 | 1 | DHT-05 | — | TTL tracking and expiry query | unit | `cargo test -p pktap-core dht::tests::test_ttl` | Plan 01 Task 2 (TDD) | ⬜ pending |
| 02-02-01 | 02 | 2 | DHT-08 | — | Offline queue enqueue/retry/flush | unit | `cargo test -p pktap-core dht::tests::test_offline_queue` | Plan 02 Task 1 (TDD) | ⬜ pending |
| 02-02-02 | 02 | 2 | DHT-03, DHT-04 | — | Public mode publish/resolve | integration | `cargo test -p pktap-core dht::tests::test_public_mode` | Plan 02 Task 2 (TDD) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 plan is needed. All tests are created inline by TDD tasks during Wave 1 (Plan 01) and Wave 2 (Plan 02). The `tdd="true"` attribute on each task ensures tests are written before implementation (RED phase) and verified after (GREEN phase).

Existing infrastructure covers all phase requirements:
- `cargo test` framework already configured from Phase 1
- `pkarr` and `mainline` test helpers (Testnet) available via dev-dependencies added in Plan 01 Task 1

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| DHT resolve from a different machine | DHT-02 | Requires separate network node | Publish from one machine, resolve from another using same derived address |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No separate Wave 0 needed — TDD tasks create tests inline
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
