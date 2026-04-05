---
phase: 03-uniffi-bridge-android-build
plan: "02"
subsystem: android-bridge
status: checkpoint-pending
tags: [android, kotlin, uniffi, ffi, d-06, zeroing, instrumented-test]
dependency_graph:
  requires: [03-01]
  provides: [PktapBridge-wrapper, FFI-02-hello-world-test, D-06-zeroing-pattern]
  affects: [04-android-contacts-ui]
tech_stack:
  added: []
  patterns:
    - PktapBridge singleton as sole FFI entry point (D-06)
    - ByteArray zeroing in finally blocks for secret material
    - Instrumented test in androidTest/ for .so-dependent tests
key_files:
  created:
    - android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt
    - android/rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt
  modified: []
decisions:
  - "Vec<u8> maps directly to kotlin.ByteArray in UniFFI 0.31 (not List<UByte> as originally assumed in research A1) — no conversion needed, simplifies bridge code"
  - "UniFFI 0.31 generates functions with backtick-quoted camelCase names at package level (not inside an object) — import as top-level functions with alias"
  - "Kotlin compiler in Gradle plugin cannot parse Java 25 version string — JDK 21 required via JAVA_HOME=/usr/lib/jvm/java-21-openjdk for all Gradle tasks"
  - "local.properties is git-ignored per android/.gitignore — developers must set sdk.dir manually or via ANDROID_HOME"
metrics:
  duration: "~8 minutes"
  completed_date: "2026-04-05"
  tasks_completed: 2
  tasks_total: 3
  files_created: 2
  files_modified: 0
---

# Phase 3 Plan 2: PktapBridge Wrapper + Instrumented Tests Summary

**One-liner:** PktapBridge singleton wrapping all UniFFI-generated Rust FFI calls with ByteArray zeroing in finally blocks (D-06), plus three instrumented tests proving the Rust-to-Kotlin pipeline and zeroing contract.

## Status

**Awaiting human verification (Task 3 checkpoint).**

Tasks 1 and 2 are complete and committed. Task 3 requires running instrumented tests on an Android emulator.

## Tasks Completed

### Task 1: PktapBridge.kt wrapper (af73f08)

Created `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt`:

- Singleton `object PktapBridge` wraps all 4 FFI functions
- `ping()` delegates to `ffiPktapPing()` — FFI-02 health check
- `ecdhAndEncrypt()` zeros `seedBytes` in `finally` block (D-06)
- `decryptAndVerify()` zeros `seedBytes` in `finally` block (D-06)
- `deriveSharedRecordName()` — no secrets, no zeroing needed
- Import paths use backtick-aliased top-level functions from `uniffi.pktap_core`

Key discovery: `Vec<u8>` maps to `kotlin.ByteArray` directly (assumption A1 in research was wrong — no `List<UByte>` conversion needed). The generated bindings use backtick-quoted camelCase names as package-level functions.

Compilation verified: `./gradlew :rust-bridge:compileDebugKotlin BUILD SUCCESSFUL` (requires JAVA_HOME=JDK21 + ANDROID_NDK_HOME).

### Task 2: Instrumented tests (aa7fffa)

Created `android/rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt`:

1. `pktapPingReturnsPktapOk` — calls `PktapBridge.ping()`, asserts `"pktap-ok"` (FFI-02)
2. `ecdhAndEncryptZeroesSeedBytes` — verifies `seedBytes.all { it == 0.toByte() }` after call (D-06)
3. `decryptAndVerifyZeroesSeedBytes` — verifies zeroing for decrypt path (D-06)

Tests are in `androidTest/` (not `test/`) because the `.so` can only be loaded on an Android runtime.

## Deviations from Plan

### Auto-corrected Assumptions

**1. [Rule 1 - Bug] Vec<u8> maps to ByteArray, not List<UByte>**
- **Found during:** Task 1, reading generated bindings
- **Issue:** Research assumption A1 stated `Vec<u8>` maps to `List<UByte>`. Actual generated code uses `kotlin.ByteArray` directly.
- **Fix:** Removed all `map { it.toUByte() }` and `toUByteArray().toByteArray()` conversions. Bridge code is simpler.
- **Files modified:** android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt
- **Commit:** af73f08

**2. [Rule 3 - Blocking] Kotlin compiler cannot parse Java 25 version string**
- **Found during:** Task 1 verification
- **Issue:** Running `./gradlew :rust-bridge:compileDebugKotlin` with JDK 25 (system default) throws `java.lang.IllegalArgumentException: 25.0.2` in `JavaVersion.parse()`. Pre-existing environment issue.
- **Fix:** Build requires `JAVA_HOME=/usr/lib/jvm/java-21-openjdk`. JDK 21 is installed and available.
- **Impact:** All Gradle tasks must be run with `JAVA_HOME` pointing to JDK 21.
- **Commit:** N/A (environment workaround, not code change)

## Known Stubs

None — PktapBridge delegates directly to generated FFI without stubs.

## Threat Surface Scan

No new threat surface introduced beyond what is documented in the plan's threat model (T-03-05, T-03-06, T-03-07).

## Self-Check: PARTIAL

Tasks 1 and 2 complete. Task 3 (instrumented test execution) pending human verification on Android emulator.

- [x] PktapBridge.kt exists: `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt`
- [x] Instrumented test exists: `android/rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt`
- [x] Commit af73f08 exists (Task 1)
- [x] Commit aa7fffa exists (Task 2)
- [ ] Instrumented tests PASSED on Android emulator (pending Task 3)
