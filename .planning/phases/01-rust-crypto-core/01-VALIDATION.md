---
phase: 1
slug: rust-crypto-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-05
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + `cargo test` |
| **Config file** | none — Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test --all` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test --all`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | CRYPTO-01 | — | Malformed key rejected with typed error, not panic | unit | `cargo test test_key_conversion` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | CRYPTO-02 | — | ECDH shared secret derived via HKDF with domain separator | unit | `cargo test test_ecdh_hkdf` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | CRYPTO-03 | — | XChaCha20-Poly1305 encrypt/decrypt round-trips | unit | `cargo test test_aead` | ❌ W0 | ⬜ pending |
| 01-01-04 | 01 | 1 | CRYPTO-04 | — | Ed25519 sign/verify round-trips (mock signing for Phase 1) | unit | `cargo test test_signing` | ❌ W0 | ⬜ pending |
| 01-01-05 | 01 | 1 | CRYPTO-05 | — | Decrypt+verify rejects tampered ciphertext/signature | unit | `cargo test test_decrypt_verify` | ❌ W0 | ⬜ pending |
| 01-01-06 | 01 | 1 | CRYPTO-06 | — | DNS TXT record constructed with _pktap. prefix | unit | `cargo test test_dns_record` | ❌ W0 | ⬜ pending |
| 01-01-07 | 01 | 1 | CRYPTO-07 | — | Composite functions never expose intermediate secrets | unit | `cargo test test_composite` | ❌ W0 | ⬜ pending |
| 01-01-08 | 01 | 1 | KEY-06 | — | Secret material wrapped in ZeroizeOnDrop | unit | `cargo test test_zeroize` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `pktap-core/src/lib.rs` — crate root with module declarations
- [ ] `pktap-core/Cargo.toml` — dependencies with correct version pins
- [ ] Test stubs in each module for the requirements above

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ZeroizeOnDrop actually zeroes memory | KEY-06 | Cannot verify memory zeroing in safe Rust tests | Code review: verify `#[derive(ZeroizeOnDrop)]` on all secret-holding types |
| curve25519-dalek version resolves without conflict | SC-5 | Dependency resolution is a build-time property | Run `cargo tree -d` and verify no duplicate curve25519-dalek versions |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
