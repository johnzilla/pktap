---
phase: 5
slug: nfc-hce-module
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-04-05
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | JUnit local tests (test/) + Android Instrumented Tests (androidTest/) |
| **Config file** | app/build.gradle.kts |
| **Quick run command** | `./gradlew :app:testDebugUnitTest` |
| **Full suite command** | `./gradlew assembleDebug && ./gradlew :app:testDebugUnitTest && cargo test -p pktap-core` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `./gradlew :app:testDebugUnitTest`
- **After every plan wave:** Run full suite command
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 45 seconds

---

## Per-Task Verification Map

Tests created inline by TDD tasks.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Created By | Status |
|---------|------|------|-------------|-----------|-------------------|------------|--------|
| 05-01-01 | 01 | 1 | NFC-04 | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests *NfcPayloadTest*` | Plan 01 (TDD) | ⬜ pending |
| 05-01-02 | 01 | 1 | NFC-02, NFC-03 | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests *ApduServiceTest*` | Plan 01 (TDD) | ⬜ pending |
| 05-02-01 | 02 | 2 | NFC-05 | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests *ReaderModeTest*` | Plan 02 (TDD) | ⬜ pending |
| 05-02-02 | 02 | 2 | NFC-06 | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests *PostTapTest*` | Plan 02 (TDD) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 needed. TDD tasks create test infrastructure inline.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Two-device NFC tap exchange | NFC-01 | Requires two physical Android devices with NFC | Install on both, tap, verify both show peer's key |
| Samsung AID routing | NFC-05 | Requires Samsung device | Test on Samsung Galaxy, verify no manual AID config needed |
| APDU handler timing < 300ms | NFC-03 | Timing requires real NFC hardware | Profile processCommandApdu() on physical device |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No separate Wave 0 needed
- [x] No watch-mode flags
- [ ] Feedback latency < 45s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
