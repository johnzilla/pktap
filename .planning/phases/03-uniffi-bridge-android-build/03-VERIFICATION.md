---
phase: 03-uniffi-bridge-android-build
verified: 2026-04-05T18:00:00Z
status: passed
score: 8/8 must-haves verified
---

# Phase 3: UniFFI Bridge + Android Build Verification Report

**Phase Goal:** The Rust pktap-core builds as an .aar, Kotlin bindings are generated and importable, and a hello-world FFI call proves the pipeline before any real crypto is wired through it
**Verified:** 2026-04-05
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `./gradlew assembleDebug` succeeds with the Rust .aar bundled — no manual steps required | VERIFIED | Confirmed by execution context: BUILD SUCCESSFUL. `preBuild` depends on `generateUniFFIBindings` which depends on both `buildRustLibrary` and `buildHostLibrary` — fully automated chain. |
| 2 | A Kotlin Android test calls a Rust function via UniFFI bindings and gets back the expected result | VERIFIED | 3 instrumented tests pass on Android emulator: `pktapPingReturnsPktapOk`, `ecdhAndEncryptZeroesSeedBytes`, `decryptAndVerifyZeroesSeedBytes`. Confirmed by execution context. |
| 3 | ByteArray secrets are zeroed (`.fill(0)`) immediately after use in the bridge layer | VERIFIED | `PktapBridge.kt` lines 50 and 83 contain `seedBytes.fill(0)` in `finally` blocks for `ecdhAndEncrypt` and `decryptAndVerify`. Two instrumented tests (`ecdhAndEncryptZeroesSeedBytes`, `decryptAndVerifyZeroesSeedBytes`) verify this contract at runtime. |
| 4 | cargo-ndk cross-compiles pktap-core to arm64-v8a and x86_64 .so files | VERIFIED | `libpktap_core.so` exists in both `android/rust-bridge/src/main/jniLibs/arm64-v8a/` and `android/rust-bridge/src/main/jniLibs/x86_64/`. `buildRustLibrary` task targets `-t arm64-v8a -t x86_64`. |
| 5 | uniffi-bindgen generates Kotlin bindings from the compiled .so | VERIFIED | `android/rust-bridge/src/main/java/uniffi/pktap_core/pktap_core.kt` exists (1239 lines). Contains `pktapPing`, `ecdhAndEncrypt`, `decryptAndVerify`, `deriveSharedRecordName` as generated package-level functions. |
| 6 | Generated .kt files appear in rust-bridge/src/main/java/ | VERIFIED | `uniffi/pktap_core/pktap_core.kt` confirmed present under `rust-bridge/src/main/java/`. |
| 7 | PktapBridge.kt wraps all FFI calls and no caller outside it imports generated UniFFI bindings directly | VERIFIED | `PktapBridge.kt` is the sole importer of `uniffi.pktap_core.*`. Grep over `android/app/` finds no direct UniFFI imports. `PktapBridgeInstrumentedTest.kt` correctly uses `PktapBridge.ping()`, not the generated functions directly. |
| 8 | The :app module depends on :rust-bridge and builds successfully | VERIFIED | `android/app/build.gradle.kts` line 25: `implementation(project(":rust-bridge"))`. `android/settings.gradle.kts` includes both `:app` and `:rust-bridge`. Build confirmed successful. |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `android/settings.gradle.kts` | Multi-module declaration (:app, :rust-bridge) | VERIFIED | Lines 16-17: `include(":app")` and `include(":rust-bridge")` |
| `android/rust-bridge/build.gradle.kts` | Gradle exec tasks for cargo-ndk and uniffi-bindgen (D-03, D-04) | VERIFIED | Contains `buildRustLibrary`, `buildHostLibrary`, `generateUniFFIBindings`, and `preBuild` dependency wiring |
| `android/gradle/libs.versions.toml` | Version catalog including jna | VERIFIED | `jna = "5.17.0"` at line 5; AGP 8.7.3, Kotlin 2.0.21 also present |
| `android/app/build.gradle.kts` | :app module depending on :rust-bridge | VERIFIED | `implementation(project(":rust-bridge"))` |
| `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt` | D-06 ByteArray zeroing wrapper over UniFFI-generated bindings | VERIFIED | `object PktapBridge` singleton; `fill(0)` in `finally` blocks for both secret-handling methods; imports from `uniffi.pktap_core` |
| `android/rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt` | FFI-02 hello-world instrumented test | VERIFIED | Three tests with `@RunWith(AndroidJUnit4::class)`; `assertEquals("pktap-ok", result)` and zeroing assertions |
| `android/rust-bridge/src/main/java/uniffi/pktap_core/pktap_core.kt` | Generated UniFFI bindings | VERIFIED | 1239-line generated file with all 4 exported functions (`pktapPing`, `ecdhAndEncrypt`, `decryptAndVerify`, `deriveSharedRecordName`) using `kotlin.ByteArray` directly |
| `pktap-core/src/ffi.rs` | `pktap_ping()` with `#[uniffi::export]` | VERIFIED | Lines 12-15: `#[uniffi::export]` on `pub fn pktap_ping() -> String` returning `"pktap-ok"` |
| `android/rust-bridge/src/main/jniLibs/arm64-v8a/libpktap_core.so` | Cross-compiled arm64 .so | VERIFIED | File present |
| `android/rust-bridge/src/main/jniLibs/x86_64/libpktap_core.so` | Cross-compiled x86_64 .so | VERIFIED | File present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `android/rust-bridge/build.gradle.kts` | `pktap-core/src/` | `cargo ndk` exec task | WIRED | `buildRustLibrary` task: `commandLine("cargo", "ndk", "-t", "arm64-v8a", "-t", "x86_64", ...)` with `inputs.dir(cargoDir.resolve("pktap-core/src"))` |
| `android/rust-bridge/build.gradle.kts` | `uniffi-bindgen` | `uniffi-bindgen generate` exec task | WIRED | `generateUniFFIBindings` task: `commandLine("cargo", "run", "--bin", "uniffi-bindgen", "generate", "--library", soPath, ...)` targeting host `.so` |
| `android/app/build.gradle.kts` | `android/rust-bridge` | project dependency | WIRED | `implementation(project(":rust-bridge"))` on line 25 |
| `PktapBridgeInstrumentedTest.kt` | generated UniFFI bindings | import via `PktapBridge.ping()` | WIRED | Test calls `PktapBridge.ping()` which delegates to `ffiPktapPing()` aliased from `uniffi.pktap_core.\`pktapPing\`` |
| `PktapBridge.kt` | generated UniFFI bindings | backtick-aliased imports | WIRED | All 4 FFI functions imported: `pktapPing`, `ecdhAndEncrypt`, `decryptAndVerify`, `deriveSharedRecordName` |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces a build pipeline and FFI bridge layer, not UI components rendering dynamic data. The data flow is: Rust function invoked → return value flows back through JNA → through generated bindings → through PktapBridge → to test assertions. This is a function call pipeline, not a data rendering pipeline. Verified at Level 3 (wired) is sufficient.

### Behavioral Spot-Checks

| Behavior | Evidence | Status |
|----------|----------|--------|
| `./gradlew assembleDebug` succeeds | Confirmed by execution context (BUILD SUCCESSFUL, 3 instrumented tests passed on emulator) | PASS |
| `pktapPingReturnsPktapOk` returns "pktap-ok" | Instrumented test passed on Android emulator per execution context | PASS |
| `ecdhAndEncryptZeroesSeedBytes` — zeroing contract holds | Instrumented test passed | PASS |
| `decryptAndVerifyZeroesSeedBytes` — zeroing contract holds | Instrumented test passed | PASS |
| 67 Rust unit tests pass (no regressions) | Confirmed by execution context | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| FFI-01 | 03-01-PLAN.md | Rust crypto core exposed to Kotlin via UniFFI proc-macro API (no UDL files) | SATISFIED | `pktap-core/src/ffi.rs` uses `#[uniffi::export]` proc-macro on all 4 functions. `uniffi::setup_scaffolding!()` in `lib.rs`. No `.udl` files exist. Generated `pktap_core.kt` confirms proc-macro path was used. |
| FFI-02 | 03-01-PLAN.md, 03-02-PLAN.md | UniFFI bindings verified with a hello-world FFI call before building crypto operations on top | SATISFIED | `pktapPingReturnsPktapOk` instrumented test passes — calls `PktapBridge.ping()` which calls `ffiPktapPing()` (generated binding) which calls Rust `pktap_ping()` returning `"pktap-ok"`. Full round-trip proven. |
| FFI-03 | 03-01-PLAN.md, 03-02-PLAN.md | Build pipeline uses cargo-ndk + custom Gradle exec task for UniFFI bindgen | SATISFIED | `buildRustLibrary` (cargo-ndk) and `generateUniFFIBindings` (uniffi-bindgen) exec tasks exist in `rust-bridge/build.gradle.kts`. Both wired into `preBuild`. `./gradlew assembleDebug` succeeds without manual steps. |

All three Phase 3 requirements (FFI-01, FFI-02, FFI-03) are satisfied. No orphaned requirements found — REQUIREMENTS.md maps exactly FFI-01, FFI-02, FFI-03 to Phase 3 and all are accounted for.

### Anti-Patterns Found

| File | Pattern | Severity | Assessment |
|------|---------|----------|------------|
| `android/app/src/main/java/com/pktap/app/MainActivity.kt` | `setContent { Text("PKTap") }` — minimal placeholder UI | Info | Intentional for this phase. The goal is pipeline verification, not UI. MainActivity is a stub by design here — real UI comes in Phase 6. Not a blocker. |

No TODO/FIXME/placeholder anti-patterns found in the bridge layer or build files. No empty implementations in security-critical paths.

### Human Verification Required

None. All success criteria are verifiable programmatically and confirmed by the execution context (build passes, instrumented tests pass on emulator, Rust tests pass). The ROADMAP SC2 uses the phrase "Android unit test" but the plans correctly implement this as an instrumented test (`androidTest/`) because `.so` files cannot load in a JVM-only test runner. This is the correct implementation — the intent (prove FFI round-trip) is fully satisfied.

### Notable Build Fix: Host .so for UniFFI Bindgen

The execution context documents a key build fix: the original plan used the cross-compiled x86_64 Android `.so` for binding generation, but cross-compiled `.so` files lack the UniFFI metadata section. The fix adds a `buildHostLibrary` task (`cargo build -p pktap-core` for the host platform) and `generateUniFFIBindings` now uses `target/debug/libpktap_core.so` (the host build) as the `--library` input. This is architecturally correct and documented in `build.gradle.kts` comments. The `--no-format` flag is also added to avoid requiring `ktfmt` at build time.

The `jvmToolchain(21)` fix (from the default JDK 25 which Kotlin 2.0.x cannot parse) is correctly applied in both `rust-bridge/build.gradle.kts` and `app/build.gradle.kts`.

### Gaps Summary

No gaps. All phase must-haves are verified, all three requirements are satisfied, the build pipeline is fully wired, and the three instrumented tests prove the end-to-end FFI channel.

---

_Verified: 2026-04-05T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
