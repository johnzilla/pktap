---
phase: 03-uniffi-bridge-android-build
plan: "01"
subsystem: android-build
status: checkpoint-pending
tags: [android, gradle, rust, uniffi, cargo-ndk, build-pipeline]
dependency_graph:
  requires: []
  provides: [android-project-scaffold, rust-bridge-module, pktap_ping-ffi-function]
  affects: [03-02-PLAN.md]
tech_stack:
  added:
    - AGP 8.7.3
    - Kotlin 2.0.21
    - Gradle 8.11.1
    - JNA 5.17.0@aar
    - Jetpack Compose BOM 2024.09.03
    - Material3 (via BOM)
  patterns:
    - cargo-ndk Gradle exec task (D-03)
    - uniffi-bindgen generate --library (D-04)
    - preBuild dependency chain for zero-step Gradle build
    - JNA @aar classifier for Android UniFFI runtime
key_files:
  created:
    - android/settings.gradle.kts
    - android/build.gradle.kts
    - android/gradle.properties
    - android/gradle/libs.versions.toml
    - android/app/build.gradle.kts
    - android/app/src/main/AndroidManifest.xml
    - android/app/src/main/java/com/pktap/app/MainActivity.kt
    - android/.gitignore
    - android/gradlew
    - android/gradle/wrapper/gradle-wrapper.properties
    - android/gradle/wrapper/gradle-wrapper.jar
    - android/rust-bridge/build.gradle.kts
    - android/rust-bridge/src/main/AndroidManifest.xml
  modified:
    - pktap-core/src/ffi.rs (added pktap_ping())
decisions:
  - "JNA dependency uses @aar classifier inline string instead of version catalog — version catalog does not support classifier syntax in libs.versions.toml"
  - "cargoDir = rootProject.file('../') — android/ is rootProject, '../' reaches pktap/ workspace root where Cargo.toml lives"
  - "cargo-ndk targets arm64-v8a + x86_64 only (D-02) — skips armeabi-v7a to speed builds"
  - "uniffi-bindgen uses --library flag against x86_64 .so — all ABIs produce identical bindings, x86_64 used for emulator compatibility"
metrics:
  duration: "~4 minutes (Tasks 1-2 only; Task 3 pending human verification)"
  completed_date: "2026-04-05"
  tasks_completed: 2
  tasks_total: 3
  files_created: 13
  files_modified: 1
---

# Phase 3 Plan 1: Android Project Scaffold + Rust Build Pipeline Summary

**One-liner:** Android multi-module project (:app + :rust-bridge) with cargo-ndk exec task cross-compiling pktap-core to arm64-v8a/x86_64 and uniffi-bindgen generating Kotlin bindings automatically on preBuild.

## Status

**Awaiting human verification (Task 3 checkpoint).**

Tasks 1 and 2 are complete and committed. Task 3 requires the user to install Android SDK, NDK r27, cargo-ndk, and Rust Android targets, then run `./gradlew assembleDebug` from `android/`.

## Tasks Completed

### Task 1: Android project scaffold

Created the complete `android/` project tree:

- `settings.gradle.kts` — includes `:app` and `:rust-bridge` (D-01)
- `build.gradle.kts` — root plugins-only file
- `gradle.properties` — AndroidX, parallel builds, 2GB heap
- `gradle/libs.versions.toml` — version catalog: AGP 8.7.3, Kotlin 2.0.21, JNA 5.17.0, Compose BOM 2024.09.03
- `app/build.gradle.kts` — depends on `:rust-bridge`, Compose + Material3
- `app/src/main/AndroidManifest.xml` — minimal launcher activity
- `app/src/main/java/com/pktap/app/MainActivity.kt` — minimal Compose activity
- `.gitignore` — excludes .so, .apk, .aab, build/, .gradle/
- Gradle wrapper 8.11.1

### Task 2: rust-bridge module + pktap_ping

Created `:rust-bridge` Android library module:

- `rust-bridge/build.gradle.kts` — complete build pipeline (D-03, D-04):
  - `buildRustLibrary` exec task: `cargo ndk -t arm64-v8a -t x86_64 -o jniLibs/ build`
  - `generateUniFFIBindings` exec task: `cargo run --bin uniffi-bindgen generate --library x86_64/libpktap_core.so --language kotlin`
  - `preBuild` wired to `generateUniFFIBindings` — zero-step `./gradlew assembleDebug`
  - JNA 5.17.0@aar dependency (required for UniFFI Kotlin runtime on Android)
- `rust-bridge/src/main/AndroidManifest.xml` — minimal library manifest

Added to `pktap-core/src/ffi.rs`:
- `pktap_ping() -> String` with `#[uniffi::export]` — returns "pktap-ok" for FFI-02 pipeline smoke test

**Rust tests:** `cargo test --lib -p pktap-core` — 67 passed, 0 failed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] JNA @aar classifier applied inline**
- **Found during:** Task 2
- **Issue:** Version catalog (`libs.versions.toml`) does not support Maven artifact classifiers. The `@aar` classifier is required for JNA on Android (otherwise Gradle resolves the plain .jar which cannot be loaded on Android).
- **Fix:** Used inline string `"net.java.dev.jna:jna:${libs.versions.jna.get()}@aar"` in `rust-bridge/build.gradle.kts` instead of `libs.jna`. The plan anticipated this and documented it explicitly.
- **Files modified:** `android/rust-bridge/build.gradle.kts`
- **Commit:** 6c3effc

## Known Stubs

None — no UI data flowing through stubs.

## Threat Flags

None — no new network endpoints or auth paths introduced. `.so` exclusion in `.gitignore` mitigates T-03-02 (Information Disclosure).

## Self-Check

(Pending Task 3 completion — partial summary)

### Files verified present:

- android/settings.gradle.kts: FOUND
- android/rust-bridge/build.gradle.kts: FOUND
- android/gradle/libs.versions.toml: FOUND
- pktap-core/src/ffi.rs (pktap_ping): FOUND

### Commits:
- 97bb81c: feat(03-01): create Android project scaffold with Gradle build files
- 6c3effc: feat(03-01): create rust-bridge module and add pktap_ping to ffi.rs
