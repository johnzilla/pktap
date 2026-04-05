# Phase 3: UniFFI Bridge + Android Build - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-05
**Phase:** 03-uniffi-bridge-android-build
**Areas discussed:** Android project scaffold, Rust-to-Android build pipeline, UniFFI binding generation, ByteArray zeroing strategy

---

## Android Project Scaffold

| Option | Description | Selected |
|--------|-------------|----------|
| Multi-module: app + rust-bridge | Separate :rust-bridge library module, :app depends on it | ✓ |
| Single app module | Everything in one module | |
| You decide | | |

**User's choice:** Multi-module
**Notes:** Clean separation — Rust build isolated from app changes.

| Option | Description | Selected |
|--------|-------------|----------|
| arm64-v8a + x86_64 | Real devices + emulator, skip 32-bit | ✓ |
| arm64-v8a + armeabi-v7a + x86_64 | Full coverage including 32-bit | |
| arm64-v8a only | Fastest builds, no emulator | |

**User's choice:** arm64-v8a + x86_64

---

## Rust-to-Android Build Pipeline

| Option | Description | Selected |
|--------|-------------|----------|
| Custom Gradle exec task | buildRustLibrary task runs cargo ndk, outputs to jniLibs | ✓ |
| Mozilla rust-android-gradle plugin | Declarative config, more magic | |
| You decide | | |

**User's choice:** Custom Gradle exec task

---

## UniFFI Binding Generation

| Option | Description | Selected |
|--------|-------------|----------|
| On every Rust build | buildRustLibrary also runs uniffi-bindgen | ✓ |
| Manual regeneration | Separate task, risk of stale bindings | |
| You decide | | |

**User's choice:** On every Rust build

| Option | Description | Selected |
|--------|-------------|----------|
| rust-bridge/src/main/java/ (committed) | Reviewable in PRs, merge conflict risk | ✓ |
| rust-bridge/build/generated/ (build output) | Clean git, slower first build | |
| You decide | | |

**User's choice:** Committed to git

---

## ByteArray Zeroing Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Bridge wrapper functions | PktapBridge.kt wraps FFI, zeros raw ByteArrays | ✓ |
| Inline extension function | ByteArray.useAndZero {} extension | |
| You decide | | |

**User's choice:** Bridge wrapper functions

---

## Claude's Discretion

- Gradle dependency management approach
- Package name
- NDK version pin
- Build profile (debug/release)
- Hello-world test function design

## Deferred Ideas

None.
