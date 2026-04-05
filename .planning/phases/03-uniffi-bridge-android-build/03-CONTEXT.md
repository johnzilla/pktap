# Phase 3: UniFFI Bridge + Android Build - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the Android project scaffold and Rust-to-Kotlin FFI pipeline. This phase creates the multi-module Android project, integrates cargo-ndk for cross-compilation, generates Kotlin bindings via UniFFI, and proves the pipeline with a hello-world FFI call. No real crypto is wired through yet — this is build infrastructure only.

</domain>

<decisions>
## Implementation Decisions

### Android Project Scaffold
- **D-01:** Multi-module structure: `:app` + `:rust-bridge` library module. `:rust-bridge` holds the .so native libraries and generated Kotlin UniFFI bindings. `:app` depends on `:rust-bridge`. Clean separation — Rust build only runs when rust-bridge is built, not on every app source change.
- **D-02:** Build for arm64-v8a + x86_64 ABI targets. arm64-v8a covers 95%+ real devices, x86_64 covers the Android emulator. Skip armeabi-v7a (32-bit) to speed up builds — can add later if needed.

### Rust-to-Android Build Pipeline
- **D-03:** Custom Gradle exec task (`buildRustLibrary`) in `rust-bridge/build.gradle.kts` that runs `cargo ndk -t arm64-v8a -t x86_64 build`. Outputs .so files to `jniLibs/`. Runs before `preBuild`. No third-party Gradle plugin — simple, explicit, full control.

### UniFFI Binding Generation
- **D-04:** Regenerate Kotlin bindings on every Rust build. The `buildRustLibrary` task also runs `uniffi-bindgen generate` after cargo-ndk completes. Generated .kt files always in sync with Rust API.
- **D-05:** Generated Kotlin bindings committed to git at `rust-bridge/src/main/java/`. Reviewable in PRs. Accepted risk of merge conflicts on concurrent Rust API changes.

### ByteArray Zeroing
- **D-06:** Bridge wrapper functions in a dedicated `PktapBridge.kt` that wraps raw UniFFI calls. Each wrapper calls the FFI function, copies needed data, then immediately calls `byteArray.fill(0)` on the raw result. Callers never see the raw FFI ByteArray — they get the processed result. This satisfies FFI-03.

### Claude's Discretion
- Exact Gradle version catalog vs buildSrc for dependency management
- Android project package name (e.g., `com.pktap.app`)
- NDK version pin (r26 vs r27)
- Whether `buildRustLibrary` uses `--release` or `--debug` profile
- UniFFI scaffolding macro placement in lib.rs
- Hello-world test function design (what it returns, how it's tested)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Specifications
- `.planning/PROJECT.md` — Core constraints (crypto in Rust via UniFFI, no JVM crypto)
- `.planning/REQUIREMENTS.md` §UniFFI Bridge — FFI-01, FFI-02, FFI-03 acceptance criteria
- `CLAUDE.md` §Technology Stack — UniFFI 0.31.x, cargo-ndk 3.5.x, AGP 8.5.x, Kotlin 2.0.x, NDK r26/r27, Gradle 8.8+

### Phase 1 Code (the Rust API being bridged)
- `pktap-core/src/ffi.rs` — Three `#[uniffi::export]` functions: `ecdh_and_encrypt`, `decrypt_and_verify`, `derive_shared_record_name`
- `pktap-core/src/lib.rs` — `uniffi::setup_scaffolding!()` macro
- `pktap-core/Cargo.toml` — UniFFI dependency, crate-type includes `cdylib`

### Prior Phase Decisions
- `.planning/phases/01-rust-crypto-core/01-CONTEXT.md` — D-01 (composites only across FFI), D-04 (opaque byte blob returns), D-07 (typed PktapError via UniFFI)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `pktap-core/src/ffi.rs` — Already has `#[uniffi::export]` on three composite functions. UniFFI scaffolding macro in lib.rs.
- `uniffi-bindgen/` — Workspace member with `uniffi-bindgen` binary stub (created in Phase 1). Ready for Kotlin binding generation.
- `pktap-core/Cargo.toml` — Already has `crate-type = ["cdylib", "staticlib"]` for native library output.

### Established Patterns
- UniFFI proc-macro API (not UDL files) — per Phase 1 and CLAUDE.md
- `PktapError` enum with UniFFI `#[derive(uniffi::Error)]` — maps to Kotlin sealed class

### Integration Points
- Phase 4 (Android Keystore) will use `:rust-bridge` module to call crypto functions
- Phase 5 (NFC) will call FFI composites through the bridge layer
- Phase 6 (App Integration) will wire UI to bridge functions

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches within the decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-uniffi-bridge-android-build*
*Context gathered: 2026-04-05*
