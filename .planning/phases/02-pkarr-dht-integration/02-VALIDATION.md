---
phase: 2
slug: pkarr-dht-integration
status: draft
nyquist_compliant: false
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

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | DHT-01, DHT-02 | — | Publish/resolve round-trip with deterministic address | integration | `cargo test -p pktap-core dht::tests::test_publish_resolve` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 1 | DHT-07 | — | Record exceeding byte budget rejected before publish | unit | `cargo test -p pktap-core dht::tests::test_size_validation` | ❌ W0 | ⬜ pending |
| 02-01-03 | 01 | 1 | DHT-06 | — | Monotonic sequence numbers, stale publish rejected | unit | `cargo test -p pktap-core dht::tests::test_sequence_numbers` | ❌ W0 | ⬜ pending |
| 02-01-04 | 01 | 1 | DHT-05 | — | TTL tracking and expiry query | unit | `cargo test -p pktap-core dht::tests::test_ttl` | ❌ W0 | ⬜ pending |
| 02-02-01 | 02 | 2 | DHT-08 | — | Offline queue enqueue/retry/flush | unit | `cargo test -p pktap-core dht::tests::test_offline_queue` | ❌ W0 | ⬜ pending |
| 02-02-02 | 02 | 2 | DHT-03, DHT-04 | — | Public mode publish/resolve | integration | `cargo test -p pktap-core dht::tests::test_public_mode` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `pktap-core/src/dht.rs` — module with test stubs
- [ ] pkarr 2.3.0 added to Cargo.toml dev-dependencies (or dependencies)

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| DHT resolve from a different machine | DHT-02 | Requires separate network node | Publish from one machine, resolve from another using same derived address |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
