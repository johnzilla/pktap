---
phase: 4
slug: android-keystore-module
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-04-05
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Android Instrumented Tests (androidTest/) + cargo test |
| **Config file** | app/build.gradle.kts, rust-bridge/build.gradle.kts |
| **Quick run command** | `./gradlew :app:connectedDebugAndroidTest` |
| **Full suite command** | `./gradlew assembleDebug && ./gradlew :app:connectedDebugAndroidTest && cargo test -p pktap-core` |
| **Estimated runtime** | ~90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `./gradlew assembleDebug` (build check)
- **After every plan wave:** Run full suite command
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

Tests created inline by TDD tasks during execution.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Created By | Status |
|---------|------|------|-------------|-----------|-------------------|------------|--------|
| 04-01-01 | 01 | 1 | KEY-01, KEY-02 | instrumented | `./gradlew :app:connectedDebugAndroidTest --tests *KeystoreTest*` | Plan 01 (TDD) | ⬜ pending |
| 04-01-02 | 01 | 1 | KEY-03 | instrumented | `./gradlew :app:connectedDebugAndroidTest --tests *SeedStorageTest*` | Plan 01 (TDD) | ⬜ pending |
| 04-01-03 | 01 | 1 | KEY-05 | instrumented | `./gradlew :app:connectedDebugAndroidTest --tests *StrongBoxFallbackTest*` | Plan 01 (TDD) | ⬜ pending |
| 04-02-01 | 02 | 2 | KEY-04 | instrumented | `./gradlew :app:connectedDebugAndroidTest --tests *MnemonicTest*` | Plan 02 (TDD) | ⬜ pending |
| 04-02-02 | 02 | 2 | — | unit (Rust) | `cargo test -p pktap-core ffi::tests::test_derive` | Plan 02 (TDD) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 needed. TDD tasks create tests inline during execution.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| BIP-39 words never written to logcat | KEY-04 | Requires logcat inspection | Run `adb logcat` during first launch, search for mnemonic words |
| StrongBox path on real hardware | KEY-01 | Requires StrongBox-capable device | Test on Pixel 6+ or Samsung Galaxy S22+ |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No separate Wave 0 needed
- [x] No watch-mode flags
- [ ] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
