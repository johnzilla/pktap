# Phase 3: UniFFI Bridge + Android Build - Research

**Researched:** 2026-04-05
**Domain:** UniFFI bindings pipeline, Android multi-module Gradle, cargo-ndk cross-compilation
**Confidence:** HIGH (core stack verified against registry/official docs), MEDIUM (version currency for AGP/NDK)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Multi-module structure: `:app` + `:rust-bridge` library module. `:rust-bridge` holds the .so native libraries and generated Kotlin UniFFI bindings. `:app` depends on `:rust-bridge`. Clean separation — Rust build only runs when rust-bridge is built, not on every app source change.
- **D-02:** Build for arm64-v8a + x86_64 ABI targets. arm64-v8a covers 95%+ real devices, x86_64 covers the Android emulator. Skip armeabi-v7a (32-bit) to speed up builds — can add later if needed.
- **D-03:** Custom Gradle exec task (`buildRustLibrary`) in `rust-bridge/build.gradle.kts` that runs `cargo ndk -t arm64-v8a -t x86_64 build`. Outputs .so files to `jniLibs/`. Runs before `preBuild`. No third-party Gradle plugin — simple, explicit, full control.
- **D-04:** Regenerate Kotlin bindings on every Rust build. The `buildRustLibrary` task also runs `uniffi-bindgen generate` after cargo-ndk completes. Generated .kt files always in sync with Rust API.
- **D-05:** Generated Kotlin bindings committed to git at `rust-bridge/src/main/java/`. Reviewable in PRs. Accepted risk of merge conflicts on concurrent Rust API changes.
- **D-06:** Bridge wrapper functions in a dedicated `PktapBridge.kt` that wraps raw UniFFI calls. Each wrapper calls the FFI function, copies needed data, then immediately calls `byteArray.fill(0)` on the raw result. Callers never see the raw FFI ByteArray — they get the processed result. This satisfies FFI-03.

### Claude's Discretion

- Exact Gradle version catalog vs buildSrc for dependency management
- Android project package name (e.g., `com.pktap.app`)
- NDK version pin (r26 vs r27)
- Whether `buildRustLibrary` uses `--release` or `--debug` profile
- UniFFI scaffolding macro placement in lib.rs
- Hello-world test function design (what it returns, how it's tested)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FFI-01 | Rust crypto core exposed to Kotlin via UniFFI proc-macro API (no UDL files) | `#[uniffi::export]` and `uniffi::setup_scaffolding!()` already in ffi.rs and lib.rs from Phase 1; bindgen generates Kotlin sealed class from `PktapError` via `#[derive(uniffi::Error)]` |
| FFI-02 | UniFFI bindings verified with a hello-world FFI call before building crypto operations on top | Add a `pktap_hello()` function to ffi.rs for proof-of-pipeline; Android instrumented test calls it and verifies return value |
| FFI-03 | Build pipeline uses cargo-ndk + custom Gradle exec task for UniFFI bindgen | D-03/D-04 locked; exec task pattern verified against official UniFFI docs and the uniffi-starter reference project |
</phase_requirements>

---

## Summary

Phase 3 creates the Android project scaffold and wires it to the Phase 1 Rust library via UniFFI. The Rust side (`pktap-core`) is already fully prepared: `uniffi::setup_scaffolding!()` is in `lib.rs`, three `#[uniffi::export]` functions exist in `ffi.rs`, `PktapError` has `#[derive(uniffi::Error)]`, and the `uniffi-bindgen` binary is in the workspace. The Android side needs to be created from scratch.

The pipeline has two mechanical parts: (1) cross-compiling the Rust crate to Android .so files via `cargo ndk`, and (2) generating the Kotlin bindings via `cargo run --bin uniffi-bindgen generate --library <path-to-.so>`. Both are driven by a custom Gradle `Exec` task in `rust-bridge/build.gradle.kts`. The generated `.kt` files plus a handwritten `PktapBridge.kt` wrapper (which calls `.fill(0)` on any returned ByteArray secrets) live in the `:rust-bridge` module. The `:app` module depends on `:rust-bridge` and never calls generated bindings directly.

A critical environment note: local unit tests (JVM) cannot load Android-targeted `.so` files. The hello-world FFI-02 test must be an **instrumented test** running on the x86_64 emulator — not a JUnit local test. This is the largest planning gotcha in this phase.

**Primary recommendation:** Use the `--library` flag on `uniffi-bindgen generate` (not the old `--udl` flag) so binding generation derives from the actual compiled `.so`, which accounts for all proc-macro attributes without requiring a separate UDL file.

---

## Standard Stack

### Core Build Tooling
| Tool | Version | Purpose | Confidence |
|------|---------|---------|------------|
| uniffi (Rust crate) | 0.31.0 | Proc-macro attributes + scaffolding; already in Cargo.toml | HIGH [VERIFIED: cargo metadata in repo] |
| uniffi-bindgen (workspace binary) | 0.31.0 | Kotlin binding generation CLI; already in uniffi-bindgen/ | HIGH [VERIFIED: uniffi-bindgen/Cargo.toml] |
| cargo-ndk | 3.5.7 (or 4.x — see note) | Cross-compile Rust .so for Android ABI targets | MEDIUM [VERIFIED: crates.io, latest 4.1.2] |
| Android NDK | r27 (LTS) | Native toolchain for cross-compilation | MEDIUM [CITED: developer.android.com/ndk] |
| Android Gradle Plugin (AGP) | 8.7.x | Android build system; minimum 8.5 for Kotlin 2.0 | HIGH [CITED: developer.android.com/build/kotlin-support] |
| Kotlin | 2.0.21 | Android language | HIGH [CITED: kotlinlang.org] |
| Gradle | 8.11+ | Build orchestration; required by AGP 8.7+ | HIGH [CITED: docs.gradle.org/compatibility] |
| KSP | 2.0.21-1.0.28 | Symbol processing (Room, Hilt) — not needed this phase but required for correct version catalog | MEDIUM [ASSUMED — version pairing] |

**cargo-ndk version note:** The project CLAUDE.md specifies 3.5.x, which is 3.5.7. cargo-ndk 4.x was released July 2025 and introduces breaking changes (16 KB page-size alignment defaults from NDK r28). Stick with 3.5.7 to match the project's locked recommendation.

### Runtime Dependencies (Android)
| Library | Version | Purpose | Confidence |
|---------|---------|---------|------------|
| net.java.dev.jna:jna | 5.17.0@aar | Native method dispatch — UniFFI requires JNA on Android | HIGH [CITED: deepwiki uniffi-starter; VERIFIED: Maven Central] |
| kotlinx-coroutines-android | 1.8.x | Dispatch FFI calls on `Dispatchers.IO`; already in CLAUDE.md stack | HIGH [ASSUMED — consistent with CLAUDE.md] |

**Installation:**
```bash
# Install cargo-ndk
cargo install cargo-ndk --version "~3.5"

# Add Android Rust targets
rustup target add aarch64-linux-android x86_64-linux-android

# NDK via Android Studio SDK Manager: NDK r27 (from SDK Manager > SDK Tools > NDK)
```

### Version Verification

At time of research (2026-04-05):
- `uniffi` crate: 0.31.0 [VERIFIED: docs.rs/crate/uniffi/latest shows 0.31.0]
- `cargo-ndk`: 4.1.2 latest, 3.5.7 latest 3.x [VERIFIED: crates.io/crates/cargo-ndk/3.5.5 series]
- AGP: 8.13.0 latest stable (Sep 2025); 8.7.x is current minimum-safe for Kotlin 2.0.x [CITED: developer.android.com/build/releases]
- Kotlin: 2.3.20 is the absolute latest; 2.0.21 is the version in CLAUDE.md; both work with AGP 8.7+ [CITED: kotlinlang.org/docs/releases]
- JNA: 5.17.0 (Mar 2025); earlier 5.15.0 is also fine; the `@aar` classifier is required [VERIFIED: mvnrepository.com]

---

## Architecture Patterns

### Recommended Project Structure
```
pktap/                              # Workspace root
├── Cargo.toml                      # Existing workspace (pktap-core, uniffi-bindgen)
├── pktap-core/                     # Phase 1 Rust library (existing)
│   └── src/ffi.rs                  # #[uniffi::export] functions
├── uniffi-bindgen/                 # Phase 1 bindgen binary (existing)
├── android/                        # Android project root (NEW this phase)
│   ├── settings.gradle.kts         # Declares :app and :rust-bridge
│   ├── build.gradle.kts            # Root build — plugin versions only
│   ├── gradle/
│   │   └── libs.versions.toml      # Version catalog
│   ├── app/
│   │   ├── build.gradle.kts        # com.android.application
│   │   └── src/main/
│   │       └── AndroidManifest.xml
│   └── rust-bridge/
│       ├── build.gradle.kts        # com.android.library + buildRustLibrary task
│       └── src/
│           ├── main/
│           │   ├── java/com/pktap/bridge/
│           │   │   ├── PktapBridge.kt          # D-06 wrapper (handwritten)
│           │   │   └── uniffi/pktap_core/      # Generated .kt files (D-05, committed)
│           │   └── jniLibs/
│           │       ├── arm64-v8a/
│           │       │   └── libpktap_core.so    # Built by cargo-ndk, NOT committed
│           │       └── x86_64/
│           │           └── libpktap_core.so
│           └── androidTest/java/com/pktap/bridge/
│               └── PktapBridgeInstrumentedTest.kt  # FFI-02 hello-world test
```

**Note on .so files in git:** The `.so` files should be in `.gitignore` — they are rebuilt by `buildRustLibrary` on every build. Only the generated `.kt` files (D-05) are committed.

### Pattern 1: Custom Gradle Exec Task (D-03, D-04)

The `buildRustLibrary` task in `rust-bridge/build.gradle.kts` chains two exec calls: cargo-ndk (compiles Rust to .so), then uniffi-bindgen (generates .kt from .so). It wires into `preBuild` so `./gradlew assembleDebug` requires no manual steps.

```kotlin
// rust-bridge/build.gradle.kts — Source: verified against uniffi-starter DeepWiki + official docs

val cargoDir = rootProject.file("../")  // Workspace root (Cargo.toml)
val jniLibsDir = file("src/main/jniLibs")
val bindingsOutDir = file("src/main/java")

val buildRustLibrary by tasks.registering(Exec::class) {
    description = "Cross-compiles pktap-core for arm64-v8a and x86_64 via cargo-ndk"
    workingDir(cargoDir)

    // D-03: Use --debug for development; can be toggled to --release
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", jniLibsDir.absolutePath,
        "build"   // append "--release" for release builds
    )
}

val generateUniFFIBindings by tasks.registering(Exec::class) {
    description = "Generates Kotlin bindings from the compiled .so via uniffi-bindgen"
    dependsOn(buildRustLibrary)
    workingDir(cargoDir)

    // Use --library with the x86_64 .so (sufficient for binding generation)
    val soPath = file("${jniLibsDir}/x86_64/libpktap_core.so")

    commandLine(
        "cargo", "run", "--bin", "uniffi-bindgen",
        "generate",
        "--library", soPath.absolutePath,
        "--language", "kotlin",
        "--out-dir", bindingsOutDir.absolutePath
    )
}

tasks.named("preBuild") {
    dependsOn(generateUniFFIBindings)
}
```

**Key insight:** The `--library` flag reads the compiled `.so` and extracts UniFFI metadata from proc-macro attributes automatically. No `.udl` file is needed. Using the x86_64 .so for binding generation is sufficient — all ABIs produce identical Kotlin bindings.

### Pattern 2: JNA Dependency (mandatory)

UniFFI-generated Kotlin code dispatches native calls through JNA. Without this dependency the generated code will not compile.

```kotlin
// rust-bridge/build.gradle.kts — Source: CITED from official UniFFI docs
dependencies {
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
}
```

### Pattern 3: PktapBridge.kt Wrapper (D-06 — ByteArray Zeroing)

The generated UniFFI bindings return raw `ByteArray` from Rust. Any `ByteArray` that contained secret material must be zeroed immediately after use. `PktapBridge.kt` is the only class that imports the generated bindings — all callers use `PktapBridge` exclusively.

```kotlin
// rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt
// Source: Pattern derived from D-06 decision; Kotlin stdlib ByteArray.fill is documented

package com.pktap.bridge

import com.pktap.bridge.uniffi.pktap_core.ecdhAndEncrypt
import com.pktap.bridge.uniffi.pktap_core.decryptAndVerify
import com.pktap.bridge.uniffi.pktap_core.deriveSharedRecordName
import com.pktap.bridge.uniffi.pktap_core.PktapError

object PktapBridge {

    /**
     * Encrypts contact fields JSON. The raw FFI ByteArray is zeroed after copying.
     * Callers receive a copy of the encrypted blob — never the raw FFI result.
     * 
     * seedBytes is zeroed by the Rust side (zeroize crate); also zero here defensively.
     */
    fun ecdhAndEncrypt(
        seedBytes: ByteArray,
        peerEd25519Public: ByteArray,
        contactFieldsJson: String
    ): Result<ByteArray> = runCatching {
        val rawResult = ecdhAndEncrypt(
            ourSeedBytes = seedBytes.toList().map { it },  // Vec<u8> in Kotlin is List<UByte>
            peerEd25519Public = peerEd25519Public.toList().map { it },
            contactFieldsJson = contactFieldsJson
        )
        val copy = rawResult.toByteArray()
        // rawResult is a List<UByte> from UniFFI — no ByteArray to zero on the Kotlin List side;
        // zero seedBytes defensively since Rust already zeroizes but the JVM copy persists
        seedBytes.fill(0)
        copy
    }

    fun deriveSharedRecordName(pubKeyA: ByteArray, pubKeyB: ByteArray): Result<String> =
        runCatching {
            deriveSharedRecordName(
                pubKeyA = pubKeyA.toList().map { it },
                pubKeyB = pubKeyB.toList().map { it }
            )
        }
}
```

**Implementation note on UniFFI type mapping:** UniFFI 0.31 maps Rust `Vec<u8>` to Kotlin `List<UByte>` (not `ByteArray`). The wrapper must convert `ByteArray` → `List<UByte>` for FFI input, and `List<UByte>` → `ByteArray` for output. This is the correct zeroing surface — the raw `List<UByte>` returned from FFI is a JVM object and its contents are not zeroed by Rust; the caller must copy to a `ByteArray`, zero anything sensitive from the input side, and use the copy. [ASSUMED — type mapping from training; verify against generated .kt files after bindgen runs]

### Pattern 4: Hello-World Function for FFI-02

Add a minimal exported Rust function to validate the pipeline before real crypto flows through it. Place it in `ffi.rs` alongside the real exports.

```rust
// pktap-core/src/ffi.rs — addition for Phase 3 FFI-02 validation

/// Hello-world function for pipeline smoke test (Phase 3 / FFI-02).
/// Returns "pktap-ok" to confirm the Rust->Kotlin FFI channel is live.
/// This function is safe to remove after Phase 3 is verified, or keep as a health check.
#[uniffi::export]
pub fn pktap_ping() -> String {
    "pktap-ok".to_string()
}
```

The instrumented test:
```kotlin
// rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt
@RunWith(AndroidJUnit4::class)
class PktapBridgeInstrumentedTest {
    @Test
    fun pktapPingReturnsPktapOk() {
        // Calls the generated binding directly (not through PktapBridge wrapper)
        // to prove the raw FFI channel works before the wrapper is tested
        val result = pktapPing()
        assertEquals("pktap-ok", result)
    }
}
```

### Pattern 5: Version Catalog Structure (libs.versions.toml)

```toml
# android/gradle/libs.versions.toml — Source: CITED kotlinlang.org, android developer docs
[versions]
agp = "8.7.3"
kotlin = "2.0.21"
ksp = "2.0.21-1.0.28"
coreKtx = "1.15.0"
jna = "5.17.0"
coroutines = "1.8.1"
junit = "4.13.2"
androidxTestRunner = "1.6.2"

[libraries]
androidx-core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "coreKtx" }
jna = { group = "net.java.dev.jna", name = "jna", version.ref = "jna" }
coroutines-android = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-android", version.ref = "coroutines" }
junit = { group = "junit", name = "junit", version.ref = "junit" }
androidx-test-runner = { group = "androidx.test", name = "runner", version.ref = "androidxTestRunner" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
android-library = { id = "com.android.library", version.ref = "agp" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
compose-compiler = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
ksp = { id = "com.google.devtools.ksp", version.ref = "ksp" }
```

### Anti-Patterns to Avoid

- **Using `--udl` flag with uniffi-bindgen:** The UDL file approach is the old UniFFI workflow. With proc-macro API (`#[uniffi::export]`), always use `--library <path-to.so>`. [CITED: official uniffi docs; VERIFIED: ffi.rs and lib.rs already use proc-macro style]
- **Generating bindings from the UDL file path:** Even if you have a `.udl`, `--library` is more reliable for proc-macro crates because it uses the compiled artifact, not a separate spec file.
- **Running JUnit local tests that call FFI:** Standard `test/` (local JVM) tests cannot load `.so` files compiled for Android. FFI verification must happen in `androidTest/` (instrumented tests on emulator/device). [VERIFIED: robolectric GitHub issues confirm Robolectric does not support Android-compiled .so]
- **Calling generated UniFFI bindings directly from `:app`:** All callers must go through `PktapBridge.kt`. This enforces the zeroing contract and keeps the FFI surface auditable.
- **Committing `.so` files to git:** The `.so` outputs are large, architecture-specific, and rebuilt on demand. Only the generated `.kt` files are committed (D-05).
- **Setting ANDROID_HOME in code:** `ANDROID_HOME` must be set in the developer environment (shell profile or `local.properties`) before Gradle runs. The Gradle task for cargo-ndk requires NDK path to be discoverable via this env var.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Native method dispatch | Custom JNI glue code | JNA via UniFFI (automatic) | UniFFI generates all JNA wiring; manual JNI is error-prone and not type-safe |
| Kotlin bindings from Rust | Manual type mapping | `uniffi-bindgen generate --library` | Proc-macro attributes produce 100% correct bindings; hand-mapping would desync |
| ABI-specific .so packaging | Custom copy tasks | `cargo ndk -o jniLibs/` | cargo-ndk outputs exactly the `jniLibs/<ABI>/libname.so` layout Android expects |
| ByteArray zeroing in Rust | Kotlin-side memory wipe of Rust's output | Rust `zeroize` crate (Phase 1) + Kotlin `fill(0)` on input | Rust already zeros secret intermediates; Kotlin zeros any input ByteArray that contained secrets before the JVM GC touches it |
| Binding generation at runtime | Dynamic binding discovery | Compile-time exec task (D-04) | Runtime binding generation adds complexity; compile-time is deterministic and reviewable |

**Key insight:** The entire `java/com/pktap/bridge/uniffi/` directory is generated code. Never edit those files — they are overwritten on every build.

---

## Common Pitfalls

### Pitfall 1: Local Unit Tests Fail with UnsatisfiedLinkError
**What goes wrong:** A developer writes an FFI test in `test/` (standard JUnit) and gets `UnsatisfiedLinkError` because the `.so` was compiled for Android, not the host JVM.
**Why it happens:** `./gradlew test` runs JVM tests against the host CPU. Android `.so` files are ARM64 or x86_64 Android ABI — they cannot be loaded by a desktop JVM.
**How to avoid:** ALL tests that call UniFFI-generated functions must live in `androidTest/` (instrumented). Run with `./gradlew connectedAndroidTest` on an emulator.
**Warning signs:** `UnsatisfiedLinkError` in test output; test discovery shows 0 tests when androidTest has content.

### Pitfall 2: Binding Generation Fails When .so Doesn't Exist Yet
**What goes wrong:** The `generateUniFFIBindings` Exec task runs before `buildRustLibrary` completes (or on a clean checkout where no `.so` exists), causing `uniffi-bindgen` to fail with "file not found".
**Why it happens:** Task dependency is missing or the `.so` path is wrong.
**How to avoid:** `generateUniFFIBindings.dependsOn(buildRustLibrary)` must be explicit. The `.so` path in the `--library` argument must exactly match the cargo-ndk output path (`jniLibs/x86_64/libpktap_core.so`).
**Warning signs:** `No such file or directory` in Gradle output for uniffi-bindgen.

### Pitfall 3: Incorrect Rust Library Name
**What goes wrong:** cargo-ndk outputs `libpktap_core.so` (Rust package name with underscores), but the Gradle task references `libpktap-core.so` (with hyphen).
**Why it happens:** Rust normalizes hyphens to underscores in the compiled library filename. The Cargo.toml `name = "pktap-core"` produces `libpktap_core.so`.
**How to avoid:** Always use underscores in the `.so` filename in Gradle task arguments.
**Warning signs:** uniffi-bindgen "library not found" error even though build succeeded.

### Pitfall 4: UniFFI Vec<u8> ↔ ByteArray Type Mismatch in Wrapper
**What goes wrong:** `PktapBridge.kt` tries to pass a Kotlin `ByteArray` directly to a UniFFI function that expects `List<UByte>` (or vice versa), causing a type error.
**Why it happens:** UniFFI 0.31 maps Rust `Vec<u8>` to Kotlin `List<UByte>`, not `ByteArray`. The conversion is explicit.
**How to avoid:** Convert `ByteArray` to `List<UByte>` with `byteArray.map { it.toUByte() }` before passing to FFI. Convert `List<UByte>` to `ByteArray` with `.toUByteArray().toByteArray()` after receiving from FFI.
**Warning signs:** Kotlin type mismatch compilation errors in PktapBridge.kt.
[ASSUMED — verify against actual generated .kt files after first successful bindgen run]

### Pitfall 5: ANDROID_HOME / NDK Not Found
**What goes wrong:** `buildRustLibrary` Gradle task fails with "NDK not found" or cargo-ndk reports "ANDROID_NDK_HOME is not set".
**Why it happens:** `ANDROID_HOME` environment variable not set in the shell that launches Android Studio / Gradle daemon, or NDK r27 not installed.
**How to avoid:** Set `ANDROID_HOME=/path/to/Android/Sdk` in shell profile. Install NDK r27 via SDK Manager (`Tools > SDK Manager > SDK Tools > NDK (Side by side)`). Add `sdk.dir` to `android/local.properties` (do not commit this file).
**Warning signs:** `cargo ndk: error: NDK not found` in Gradle output.

### Pitfall 6: Generated Bindings Not on Source Path
**What goes wrong:** Android Studio shows errors for generated UniFFI types as if they don't exist, even though the files were generated.
**Why it happens:** The generated `.kt` output directory (e.g., `src/main/java/`) may not be included in the source set for the variant used during IDE sync.
**How to avoid:** Since D-05 commits the generated files to `src/main/java/`, they are a stable source tree and IDE always finds them. The exec task regenerates on build, not just on sync.
**Warning signs:** Red squiggles in `PktapBridge.kt` imports during IDE usage only (build succeeds).

---

## Code Examples

### Complete rust-bridge/build.gradle.kts
```kotlin
// Source: pattern synthesized from CITED uniffi-starter (deepwiki.com/ianthetechie/uniffi-starter)
// and CITED official UniFFI Gradle doc (mozilla.github.io/uniffi-rs/latest/kotlin/gradle.html)
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.pktap.bridge"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

val cargoDir = rootProject.file("../")   // Workspace root: rootProject=android/, "../" reaches pktap/
val jniLibsDir = file("src/main/jniLibs")
val bindingsOutDir = file("src/main/java")

val buildRustLibrary by tasks.registering(Exec::class) {
    group = "rust"
    description = "Compiles pktap-core to Android .so via cargo-ndk (arm64-v8a, x86_64)"
    workingDir(cargoDir)
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", jniLibsDir.absolutePath,
        "build"
    )
    inputs.dir(cargoDir.resolve("pktap-core/src"))
    inputs.file(cargoDir.resolve("pktap-core/Cargo.toml"))
    outputs.dir(jniLibsDir)
}

val generateUniFFIBindings by tasks.registering(Exec::class) {
    group = "rust"
    description = "Generates Kotlin bindings from compiled .so via uniffi-bindgen"
    dependsOn(buildRustLibrary)
    workingDir(cargoDir)
    val soPath = file("${jniLibsDir}/x86_64/libpktap_core.so")
    commandLine(
        "cargo", "run", "--bin", "uniffi-bindgen",
        "generate",
        "--library", soPath.absolutePath,
        "--language", "kotlin",
        "--out-dir", bindingsOutDir.absolutePath
    )
    inputs.file(soPath)
    outputs.dir(bindingsOutDir)
}

tasks.named("preBuild") {
    dependsOn(generateUniFFIBindings)
}

dependencies {
    implementation(libs.jna)
    implementation(libs.coroutines.android)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
}
```

### settings.gradle.kts
```kotlin
// android/settings.gradle.kts
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "pktap"
include(":app")
include(":rust-bridge")
```

---

## Runtime State Inventory

Not applicable — this is a greenfield phase creating new files. No rename/refactor/migration involved.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| UDL files (`.udl`) for UniFFI interface definitions | Proc-macro API (`#[uniffi::export]`, `uniffi::setup_scaffolding!()`) | UniFFI 0.22+ | No separate spec file; Rust source is the single source of truth |
| `uniffi-bindgen generate --udl <file>` | `uniffi-bindgen generate --library <path-to.so>` | UniFFI 0.24+ | `--library` reads proc-macro metadata from compiled artifact; more reliable |
| kapt (Kotlin Annotation Processing Tool) | KSP (Kotlin Symbol Processing) | 2022-2023, mandatory for Kotlin 2.0+ | KSP is ~2x faster and is the future; kapt is in deprecation trajectory |
| Separate `compose-compiler` extension in build.gradle | `org.jetbrains.kotlin.plugin.compose` plugin | Kotlin 2.0 (May 2024) | Compose compiler ships with Kotlin; same version number; no separate extension needed |
| NDK r25 (previous Rust minimum) | NDK r26 or r27 | Rust 1.68 (2023) | r25 was minimum; r26 is current LTS-adjacent; r27 is current LTS |

**Deprecated/outdated:**
- UDL-based UniFFI workflow: still works but is considered legacy; proc-macro API is the current standard
- kapt: works but deprecated trajectory in Kotlin 2.x; KSP is the replacement for Room, Hilt
- NDK r25: minimum supported by Rust, but r26+ strongly preferred for stability

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | UniFFI 0.31 maps `Vec<u8>` to `List<UByte>` in Kotlin (not `ByteArray`) | Architecture Patterns, Pitfall 4 | PktapBridge.kt type conversions would be wrong; compilation error (caught early) |
| A2 | `pktap_core` crate name produces `libpktap_core.so` (underscore, not hyphen) | Architecture Patterns (Pitfall 3) | Gradle task would reference wrong filename; build would fail (caught immediately) |
| A3 | NDK r27 is sufficient for Rust 1.93 cross-compilation without r28's 16 KB page-size changes | Standard Stack | Possible link failure; upgrade to r28 if needed |
| A4 | KSP 2.0.21-1.0.28 is the correct pairing for Kotlin 2.0.21 | Standard Stack | Incompatible annotation processing; symbol resolution errors |
| A5 | Instrumented test on x86_64 emulator is the correct test target (not local JUnit) for FFI-02 | Architecture Patterns, Pitfall 1 | FFI test would never actually load the .so; always pass trivially |

---

## Open Questions

1. **Debug vs. release profile for `buildRustLibrary`**
   - What we know: `--debug` is faster to compile; `--release` is required for production size/performance
   - What's unclear: Whether `--debug` is acceptable for Phase 3 (proof-of-pipeline) or if `--release` should be established from the start
   - Recommendation: Use `--debug` during Phase 3 for faster iteration; switch to `--release` in Phase 6 or when performance testing begins

2. **Package name for the Android project**
   - What we know: Claude's discretion per CONTEXT.md
   - What's unclear: `com.pktap.app` vs `dev.pktap.app` vs other
   - Recommendation: Use `com.pktap.app` for `:app` namespace, `com.pktap.bridge` for `:rust-bridge` — simple and consistent

3. **AGP version: 8.7.x vs later**
   - What we know: AGP 8.5 is minimum for Kotlin 2.0; 8.7.x is current stable-ish; 8.13.0 is latest (Sep 2025)
   - What's unclear: Whether 8.7.3 is the best starting point given it's stable and widely tested, or if 8.13 should be used
   - Recommendation: Use 8.7.3 (current in ecosystem examples, well-tested) rather than 8.13 which is very recent

4. **JNA artifact variant**
   - What we know: JNA 5.17.0 exists; `@aar` classifier is required for Android
   - What's unclear: Whether `net.java.dev.jna:jna:5.17.0@aar` or `5.15.0@aar` is preferred
   - Recommendation: Use 5.17.0@aar (current, confirmed AAR support)

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | cargo-ndk build | ✓ | 1.93.0 | — |
| cargo-ndk | D-03 exec task | ✗ | — | Install: `cargo install cargo-ndk --version "~3.5"` |
| aarch64-linux-android target | arm64-v8a .so | ✗ | — | Install: `rustup target add aarch64-linux-android` |
| x86_64-linux-android target | x86_64 .so | ✗ | — | Install: `rustup target add x86_64-linux-android` |
| Android SDK + NDK r27 | All Android build | ✗ | — | Install via Android Studio SDK Manager — NO fallback; blocks execution |
| ANDROID_HOME env var | cargo-ndk NDK discovery | ✗ | — | Set in shell profile before Gradle runs |
| Java (JDK) | Gradle | ✓ | OpenJDK 25.0.2 | — |

**Missing dependencies with no fallback:**
- Android SDK (with Build Tools + Platform API 35) — must be installed via Android Studio before this phase starts
- Android NDK r27 — must be installed via SDK Manager (`Tools > SDK Manager > SDK Tools > NDK (Side by side)`)
- `ANDROID_HOME` env var pointing to SDK location

**Missing dependencies with fallback (install before coding):**
- `cargo-ndk` 3.5.x — install with `cargo install cargo-ndk --version "~3.5"`
- `aarch64-linux-android` Rust target — install with `rustup target add aarch64-linux-android`
- `x86_64-linux-android` Rust target — install with `rustup target add x86_64-linux-android`

**Android Emulator note:** The FFI-02 instrumented test requires an x86_64 emulator with API 33+ running. This must be set up before the hello-world test can execute.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | JUnit 4 (instrumented via `androidx.test:runner`) |
| Config file | `android/rust-bridge/build.gradle.kts` — `testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"` |
| Quick run command | `./gradlew :rust-bridge:connectedAndroidTest` (requires emulator/device) |
| Full suite command | `./gradlew :rust-bridge:connectedAndroidTest :app:connectedAndroidTest` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FFI-01 | Rust functions accessible from Kotlin via generated bindings | instrumented (smoke) | `./gradlew :rust-bridge:connectedAndroidTest` | ❌ Wave 0 |
| FFI-02 | `pktapPing()` returns "pktap-ok" | instrumented (unit) | `./gradlew :rust-bridge:connectedAndroidTest --tests "*.PktapBridgeInstrumentedTest.pktapPingReturnsPktapOk"` | ❌ Wave 0 |
| FFI-03 | `./gradlew assembleDebug` completes end-to-end without manual steps | build verification | `./gradlew assembleDebug` | ❌ Wave 0 |

**Note:** All FFI tests are instrumented (androidTest) because JVM local tests cannot load Android-compiled `.so` files. There is no local unit test equivalent for FFI verification.

### Sampling Rate
- **Per task commit:** `./gradlew assembleDebug` (build smoke test — no emulator needed)
- **Per wave merge:** `./gradlew assembleDebug && ./gradlew :rust-bridge:connectedAndroidTest` (requires emulator)
- **Phase gate:** Full instrumented suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `android/rust-bridge/src/androidTest/java/com/pktap/bridge/PktapBridgeInstrumentedTest.kt` — covers FFI-01, FFI-02
- [ ] `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt` — covers FFI-03 (zeroing contract)
- [ ] Android project scaffold (`android/`, `settings.gradle.kts`, module `build.gradle.kts` files) — needed before any test can compile

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | partial | FFI functions already validate input lengths (implemented in Phase 1); `PktapBridge.kt` must not bypass these |
| V6 Cryptography | yes (memory) | `ByteArray.fill(0)` in `PktapBridge.kt` for any input containing secret material; Rust `zeroize` already handles internal state |

### Known Threat Patterns for FFI Bridge

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret key material retained in JVM heap after FFI call | Information Disclosure | `ByteArray.fill(0)` immediately after use in `PktapBridge.kt` (D-06) |
| Caller bypasses bridge and calls generated bindings directly | Elevation of Privilege | Code review convention + Kotlin `internal` visibility on generated binding package (if feasible) |
| Malformed inputs from Kotlin reaching Rust without length checks | Tampering | All three FFI functions in ffi.rs validate input lengths before crypto; `PktapBridge.kt` adds no extra validation but must not strip inputs before passing |

**Memory safety note:** JVM garbage collection means `ByteArray.fill(0)` is best-effort — the JIT may have already copied the bytes elsewhere. This is the standard trade-off in JVM-native secret handling and is consistent with Android's own `EncryptedSharedPreferences` approach. The Rust side using `zeroize` provides the stronger guarantee.

---

## Sources

### Primary (HIGH confidence)
- [uniffi 0.31.0 on docs.rs](https://docs.rs/crate/uniffi/latest) — confirmed as current version
- [UniFFI Gradle Integration Guide (official)](https://mozilla.github.io/uniffi-rs/latest/kotlin/gradle.html) — JNA requirement, Exec task pattern
- [uniffi-rs GitHub gradle.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/kotlin/gradle.md) — canonical Gradle task template
- [AGP & Kotlin compatibility](https://developer.android.com/build/kotlin-support) — AGP 8.5 minimum for Kotlin 2.0
- `pktap-core/Cargo.toml` — confirmed uniffi 0.31.0, crate-type cdylib+staticlib
- `pktap-core/src/ffi.rs` — confirmed `#[uniffi::export]` on three functions
- `pktap-core/src/lib.rs` — confirmed `uniffi::setup_scaffolding!()`
- `uniffi-bindgen/Cargo.toml` — confirmed uniffi 0.31.0 with `cli` feature

### Secondary (MEDIUM confidence)
- [uniffi-starter Android integration (DeepWiki)](https://deepwiki.com/ianthetechie/uniffi-starter/5-android-integration) — multi-module structure, `--library` flag in Gradle exec task, JNA 5.15.0
- [cargo-ndk 3.5.5 on crates.io](https://crates.io/crates/cargo-ndk/3.5.5) — confirmed 3.5.x series; latest is 4.1.2
- [NDK version comparison: r27 LTS vs r28](https://medium.com/@musaddiq625/understanding-android-ndk-versions-r27-r28-and-beyond-c93141d4ebd1) — r27 is LTS, r28 adds 16 KB page defaults
- [JNA Maven Central](https://mvnrepository.com/artifact/net.java.dev.jna/jna) — 5.17.0 current (Mar 2025)
- [Kotlin 2.0 compose-compiler plugin](https://developer.android.com/develop/ui/compose/compiler) — plugin ships with Kotlin 2.0
- [AGP 8.13.0 release notes](https://developer.android.com/build/releases/agp-8-13-0-release-notes) — confirmed latest AGP

### Tertiary (LOW confidence — verify before use)
- [UniFFI Vec<u8> → List<UByte> type mapping] — from training knowledge, not verified against 0.31 generated output; verify by inspecting generated .kt after first bindgen run
- [Robolectric native library support limitation](https://github.com/robolectric/robolectric/issues/6123) — community reports, not official docs; confirmed by multiple issue threads

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — uniffi 0.31.0 confirmed in Cargo.toml; cargo-ndk version in project spec; JNA and AGP verified against official sources
- Architecture: HIGH — exec task pattern confirmed against official uniffi docs and reference project; multi-module structure confirmed from decisions
- Pitfalls: MEDIUM — JVM/.so incompatibility confirmed via robolectric issues; type mapping is ASSUMED (A1)

**Research date:** 2026-04-05
**Valid until:** 2026-05-05 (AGP and toolchain versions move fast; re-verify NDK and AGP versions if more than 30 days pass)
