---
phase: 3
slug: uniffi-bridge-android-build
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-04-05
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Android Instrumented Tests (androidTest/) + cargo test |
| **Config file** | rust-bridge/build.gradle.kts, app/build.gradle.kts |
| **Quick run command** | `./gradlew :rust-bridge:connectedDebugAndroidTest` |
| **Full suite command** | `./gradlew assembleDebug && ./gradlew :rust-bridge:connectedDebugAndroidTest && cargo test --all` |
| **Estimated runtime** | ~60 seconds (includes Rust compile + emulator test) |

---

## Sampling Rate

- **After every task commit:** Run `./gradlew assembleDebug` (build check)
- **After every plan wave:** Run full suite command
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

Tests created inline by TDD tasks during execution. No separate Wave 0 test scaffold.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Created By | Status |
|---------|------|------|-------------|-----------|-------------------|------------|--------|
| 03-01-01 | 01 | 1 | FFI-03 | build | `./gradlew assembleDebug` | Plan 01 Task 1 | ⬜ pending |
| 03-01-02 | 01 | 1 | FFI-01, FFI-02 | instrumented | `./gradlew :rust-bridge:connectedDebugAndroidTest` | Plan 01 Task 2 | ⬜ pending |
| 03-01-03 | 01 | 1 | FFI-03 | instrumented | `./gradlew :rust-bridge:connectedDebugAndroidTest` (zeroing test) | Plan 01 Task 2 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 plan needed. Environment setup (Android SDK, NDK, cargo-ndk) is a prerequisite task within the plans. TDD tasks create tests inline.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Build runs on CI without manual steps | FFI-03 SC1 | Requires CI pipeline setup | Run `./gradlew assembleDebug` on a clean checkout |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No separate Wave 0 needed — prerequisites in task actions
- [x] No watch-mode flags
- [ ] Feedback latency < 90s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
