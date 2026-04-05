# Technology Stack

**Project:** PKTap
**Researched:** 2026-04-04
**Confidence note:** Web search was unavailable. All version numbers come from training data (cutoff August 2025). Every version marked [VERIFY] must be checked against crates.io or Maven Central before pinning in Cargo.toml / build.gradle.

---

## Recommended Stack

### Rust Core (via UniFFI)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| pkarr | 2.3.x [VERIFY] | Mainline DHT publish/resolve, DNS record signing | Only maintained Rust library for Pkarr protocol; wraps mainline (BEP44) DHT with Ed25519 signing and DNS TXT encoding natively | MEDIUM |
| ed25519-dalek | 2.1.x [VERIFY] | Ed25519 keypair generation, signing, verification | RustCrypto standard; 2.x API is the current stable series; works seamlessly with x25519-dalek via curve25519-dalek shared backend | HIGH |
| x25519-dalek | 2.0.x [VERIFY] | X25519 ECDH key agreement after Ed25519->X25519 conversion | RustCrypto standard; shares curve25519-dalek 4.x backend with ed25519-dalek 2.x — critical for version compatibility | HIGH |
| curve25519-dalek | 4.1.x [VERIFY] | Shared backend; must match versions used by ed25519-dalek and x25519-dalek | Pin this explicitly to prevent dependency resolution pulling incompatible versions | HIGH |
| chacha20poly1305 | 0.10.x [VERIFY] | XChaCha20-Poly1305 AEAD encryption | RustCrypto standard; `XChaCha20Poly1305` type ships in this crate; 96-bit nonce variant avoids nonce-reuse risk | HIGH |
| hkdf | 0.12.x [VERIFY] | HKDF key derivation from ECDH shared secret | RustCrypto standard; use with SHA-256 to derive the 32-byte encryption key from the X25519 output | HIGH |
| sha2 | 0.10.x [VERIFY] | SHA-256 for deterministic DHT address `SHA-256(sort(A_pk, B_pk))` and HKDF | RustCrypto standard | HIGH |
| zeroize | 1.7.x [VERIFY] | Memory zeroing of all secret material (keys, shared secrets, plaintext) after use | The `Zeroize` and `ZeroizeOnDrop` derives make it automatic; pin to >=1.5 for the derive macro | HIGH |
| bip39 | 2.0.x [VERIFY] | BIP-39 mnemonic generation and recovery from seed bytes | `bip39` crate (not `tiny-bip39`) is most maintained; supports 12/24 word lists and entropy->mnemonic->entropy roundtrip | MEDIUM |
| uniffi | 0.28.x [VERIFY] | Kotlin/Swift bindings generation from Rust | Mozilla's official solution; 0.28 introduced `uniffi::export` proc-macro API which eliminates most UDL boilerplate — use proc-macro style, not UDL files | HIGH |
| rand | 0.8.x [VERIFY] | Cryptographically secure random bytes (nonce generation, keypair entropy) | Standard; use `rand::rngs::OsRng` — never `thread_rng()` for key material | HIGH |
| crc | 3.x [VERIFY] | CRC-16 for NDEF payload integrity in the 36-byte NFC packet | Pure Rust, no_std compatible; simpler than rolling own | MEDIUM |

**Critical version constraint:** `ed25519-dalek 2.x` and `x25519-dalek 2.x` both require `curve25519-dalek 4.x`. If pkarr pulls an older curve25519-dalek, dependency resolution will break. Pin `curve25519-dalek = "4"` in workspace `[dependencies]` and verify pkarr's own constraint before starting.

### Android Dependencies

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Jetpack Compose BOM | 2024.09.xx [VERIFY] | Declarative UI — all Compose library versions via BOM | Google-maintained BOM; single version pin keeps Compose libraries compatible with each other | HIGH |
| compose.material3 | via BOM | Material 3 UI components | Material You is the current Android design system; M3 has stable API since 2023 | HIGH |
| androidx.activity:activity-compose | 1.9.x [VERIFY] | Compose integration with Activity lifecycle | Required for `setContent {}` entry point | HIGH |
| androidx.lifecycle:lifecycle-viewmodel-compose | 2.8.x [VERIFY] | ViewModel integration with Compose | Hoist state out of composables; avoid recomposition-triggered side effects | HIGH |
| androidx.navigation:navigation-compose | 2.8.x [VERIFY] | Type-safe navigation between screens | Type-safe nav (Kotlin Serialization routes) landed in 2.8; avoids string route typos | HIGH |
| androidx.room:room-runtime + room-ktx | 2.6.x [VERIFY] | SQLite ORM for local contact storage | Room is the standard Android SQLite layer; supports Flow for reactive queries; coroutine-friendly | HIGH |
| androidx.room:room-compiler (kapt/ksp) | 2.6.x [VERIFY] | Room annotation processing | Use KSP over kapt — faster, incremental, Kotlin-first | HIGH |
| androidx.security:security-crypto | 1.1.0-alpha06 [VERIFY] | EncryptedSharedPreferences for storing encrypted HKDF seed | Wraps Android Keystore AES-256-GCM; the ONLY AndroidX library for Keystore-backed SharedPreferences; alpha status is fine — it's been stable in practice since 2021 | MEDIUM |
| androidx.camera:camera-camera2 + camera-lifecycle + camera-view | 1.3.x [VERIFY] | CameraX for QR code scanner viewfinder | CameraX is the modern camera API; stable since 1.0; use camera-view for PreviewView composable integration | HIGH |
| com.google.mlkit:barcode-scanning | 17.3.x [VERIFY] | QR code decode from CameraX frames | ML Kit Barcode Scanning; on-device, no network, no Google account required; preferred over ZXing for speed and accuracy | HIGH |
| androidx.core:core-ktx | 1.13.x [VERIFY] | Kotlin extensions for NFC, system APIs | Standard Kotlin-friendly wrappers; NFC Adapter access, PendingIntent helpers | HIGH |
| org.jetbrains.kotlinx:kotlinx-coroutines-android | 1.8.x [VERIFY] | Coroutines on Android main thread + IO dispatcher | Standard; use `Dispatchers.IO` for all Rust FFI calls (they may block) | HIGH |
| org.jetbrains.kotlinx:kotlinx-serialization-json | 1.7.x [VERIFY] | JSON serialization for contact record wire format | Kotlin-native; no reflection; works with KMP for future iOS; use with Navigation 2.8 type-safe routes | HIGH |

**NFC HCE:** Uses only Android platform classes — no library dependency needed. `HostApduService` (extends `android.nfc.cardemulation.HostApduService`), `NfcAdapter`, `IsoDep` are all in the Android SDK. Minimum API level 19 for HCE (in practice target API 26+ for all other dependencies).

**ZXing vs ML Kit:** Do NOT use `com.journeyapps:zxing-android-embedded`. It pulls a large Java dependency, has a non-Compose UI, and ML Kit's barcode scanner is faster with better low-light performance. ZXing was the 2015 answer; ML Kit is the 2024 answer.

**SQLCipher: explicitly excluded.** Full-database encryption is unnecessary overhead when sensitive columns are individually encrypted via Keystore-managed AES-256-GCM. SQLCipher also adds a significant .so size (~3 MB) and its Android bindings are maintained by a third party.

### Build Tooling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| cargo-ndk | 3.5.x [VERIFY] | Cross-compile Rust to Android targets (arm64-v8a, armeabi-v7a, x86_64) | Standard tool for this workflow; wraps NDK toolchains; integrates with Gradle exec tasks | HIGH |
| Android NDK | r26 / r27 [VERIFY] | C/C++ and Rust native toolchain for Android | NDK r26 is the current LTS-adjacent stable; r27 exists but r26 is more widely tested with Rust cross-compilation as of mid-2025 | MEDIUM |
| Android Gradle Plugin (AGP) | 8.5.x [VERIFY] | Android build system | AGP 8.x is required for Kotlin 2.x compatibility; use the stable channel | HIGH |
| Kotlin | 2.0.x [VERIFY] | Primary Android development language | Kotlin 2.0 introduced the new K2 compiler; stable as of May 2024; required for compose-compiler plugin in 2.x | HIGH |
| Kotlin Gradle Plugin (KGP) | 2.0.x [VERIFY] | Kotlin compilation in Gradle | Must match Kotlin stdlib version | HIGH |
| KSP (Kotlin Symbol Processing) | 2.0.x-1.0.2x [VERIFY] | Annotation processing for Room | Replaces kapt; faster incremental builds; version must match Kotlin version (first segment) | HIGH |
| Gradle | 8.8+ [VERIFY] | Build orchestration | AGP 8.5 requires Gradle 8.7+; use the Gradle wrapper | HIGH |
| uniffi-bindgen (CLI) | 0.28.x [VERIFY] — match Rust crate | Generates Kotlin source files from Rust crate | Run as a Cargo build step; generated .kt files are checked into source or generated at sync time | HIGH |
| uniffi Gradle plugin (unofficial) | — | Automate uniffi-bindgen invocation from Gradle | No official Gradle plugin exists as of mid-2025; use `exec {}` task in build.gradle.kts calling `cargo run --bin uniffi-bindgen generate` | MEDIUM |

**KMP (Kotlin Multiplatform):** The shared module skeleton should be set up from day 1 even though iOS is out of scope for MVP. Use `kotlin("multiplatform")` plugin on the `:core` module. Place all non-Android business logic (contact record model, DHT address derivation logic if done in Kotlin, serialization) in `commonMain`. The Rust FFI wrapper goes in `androidMain`. This avoids a painful module restructure in v0.2.

**UniFFI Gradle integration pattern:** There is no blessed official Gradle plugin. The standard approach is:

```kotlin
// In the :rust-bridge module's build.gradle.kts
val buildRustLibs = tasks.register<Exec>("buildRustLibs") {
    workingDir = file("../rust-core")
    commandLine("cargo", "ndk",
        "-t", "arm64-v8a", "-t", "armeabi-v7a", "-t", "x86_64",
        "-o", "../android-app/src/main/jniLibs",
        "build", "--release")
}

val generateKotlinBindings = tasks.register<Exec>("generateKotlinBindings") {
    dependsOn(buildRustLibs)
    workingDir = file("../rust-core")
    commandLine("cargo", "run", "--bin", "uniffi-bindgen",
        "generate", "--library", "target/debug/libpktap_core.so",
        "--language", "kotlin",
        "--out-dir", "../android-app/src/main/java/com/pktap/generated")
}
```

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| androidx.hilt:hilt-android | 2.51.x [VERIFY] | Dependency injection | Use Hilt for ViewModel and service injection; avoids manual DI wiring for the Rust bridge singleton and Room DAOs | MEDIUM |
| hilt-compiler (kapt/ksp) | 2.51.x [VERIFY] | Hilt annotation processing | Use KSP variant if available in your Hilt version | MEDIUM |
| androidx.datastore:datastore-preferences | 1.1.x [VERIFY] | App settings (non-sensitive) — e.g., QR fallback enabled, public mode toggle | Prefer DataStore over SharedPreferences for new non-sensitive settings; EncryptedSharedPreferences remains for the seed | MEDIUM |
| kotlinx-datetime | 0.6.x [VERIFY] | TTL timestamps, "last verified" timestamps in KMP common code | KMP-compatible; avoids java.util.Date in common code | LOW |
| accompanist-permissions | 0.34.x [VERIFY] | Compose-friendly camera permission request flow | Google Accompanist; simplifies the boilerplate around `rememberPermissionState` | LOW |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| DHT client | pkarr (Rust) | mainline crate directly | pkarr wraps mainline with the Pkarr DNS-over-DHT semantics already implemented; reinventing on top of mainline would duplicate pkarr's record format, signing, and SALT derivation |
| AEAD cipher | XChaCha20-Poly1305 | AES-256-GCM | XChaCha20 has a 192-bit nonce (vs 96-bit for AES-GCM) — eliminates nonce collision risk with random nonces even at high volume; ChaCha20 has no timing-attack surface on platforms without AES hardware acceleration |
| Bindings generator | UniFFI | JNI by hand | Manual JNI is error-prone, not type-safe, and doubles the effort for future iOS; UniFFI's proc-macro API generates both Kotlin and Swift bindings from the same Rust source |
| Barcode scanning | ML Kit | ZXing android-embedded | ML Kit is faster, on-device, no network dependency, integrates with CameraX analysis pipeline cleanly |
| QR code generation | zxing-core (Java) | qrcode (Rust via FFI) | Display-only QR generation is simpler from Kotlin; `com.google.zxing:core` (not the Android-embedded wrapper) is a lightweight Java-only dep acceptable here |
| Android DI | Hilt | Koin | Hilt has first-class ViewModel and WorkManager support; Koin is fine but Hilt's Compose integration is more mature |
| Room annotation processing | KSP | kapt | KSP is Kotlin-first, significantly faster, incremental; kapt is deprecated trajectory |
| Full-DB encryption | Keystore column encryption | SQLCipher | SQLCipher adds ~3 MB .so, third-party maintenance burden, and unnecessary given column-level encryption covers all sensitive fields |
| NFC exchange format | 36-byte NDEF custom record | Full NDEF text record | Custom binary format fits the 32-byte Ed25519 pubkey + 4 bytes (version + flags + CRC-16) in a single NDEF record; text encoding adds overhead that matters at NFC read speeds |

---

## Target API Levels

| Setting | Value | Rationale |
|---------|-------|-----------|
| minSdk | 26 (Android 8.0) | Required for full NFC HCE stability, EncryptedSharedPreferences, and modern Keystore features (StrongBox available on API 28+, but TEE fallback covers 26+); covers ~95% of active Android devices |
| targetSdk | 35 [VERIFY] | Latest stable at time of writing; required for Play Store compliance |
| compileSdk | 35 [VERIFY] | Match targetSdk |

---

## Rust Workspace Layout

```
pktap/
  Cargo.toml              # workspace root
  pktap-core/             # lib crate — all crypto, DHT, record construction
    Cargo.toml
    src/
      lib.rs              # uniffi::export entry points
      crypto.rs           # ECDH, AEAD, key conversion
      record.rs           # Pkarr record construction, signing
      dht.rs              # pkarr publish/resolve
      bip39.rs            # mnemonic generation/recovery
  uniffi-bindgen/         # binary crate — runs uniffi-bindgen CLI
    Cargo.toml
    src/main.rs           # single-file: uniffi::uniffi_bindgen_main()
```

The `uniffi-bindgen` binary crate is necessary because `uniffi-bindgen` must be built from the same UniFFI version as the `uniffi` crate used in `pktap-core`. Running the system-installed `uniffi-bindgen` binary against a different version causes silent binding mismatches.

---

## Installation

```toml
# pktap-core/Cargo.toml
[package]
name = "pktap-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
pkarr = "2"                          # [VERIFY version]
ed25519-dalek = { version = "2", features = ["rand_core"] }
x25519-dalek = { version = "2", features = ["static_secrets"] }
curve25519-dalek = "4"               # explicit pin to control version
chacha20poly1305 = "0.10"
hkdf = "0.12"
sha2 = "0.10"
zeroize = { version = "1", features = ["derive"] }
bip39 = "2"                          # [VERIFY crate name — may be "bip39" or "tiny-bip39"]
rand = "0.8"
crc = "3"
uniffi = { version = "0.28", features = ["tokio"] }  # [VERIFY version]

[build-dependencies]
uniffi = { version = "0.28", features = ["build"] }
```

```toml
# uniffi-bindgen/Cargo.toml
[package]
name = "uniffi-bindgen"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "uniffi-bindgen"
path = "src/main.rs"

[dependencies]
uniffi = { version = "0.28", features = ["cli"] }
```

```kotlin
// android-app/build.gradle.kts (partial)
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.ksp)
    alias(libs.plugins.hilt)
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.09.00") // [VERIFY]
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.activity:activity-compose:1.9.0")           // [VERIFY]
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.0") // [VERIFY]
    implementation("androidx.navigation:navigation-compose:2.8.0")        // [VERIFY]

    implementation("androidx.room:room-runtime:2.6.1")                    // [VERIFY]
    implementation("androidx.room:room-ktx:2.6.1")                        // [VERIFY]
    ksp("androidx.room:room-compiler:2.6.1")                              // [VERIFY]

    implementation("androidx.security:security-crypto:1.1.0-alpha06")     // [VERIFY]

    implementation("androidx.camera:camera-camera2:1.3.4")                // [VERIFY]
    implementation("androidx.camera:camera-lifecycle:1.3.4")              // [VERIFY]
    implementation("androidx.camera:camera-view:1.3.4")                   // [VERIFY]
    implementation("com.google.mlkit:barcode-scanning:17.3.0")            // [VERIFY]

    implementation("com.google.zxing:core:3.5.3")                         // [VERIFY] QR generation only
    implementation("com.google.dagger:hilt-android:2.51.1")               // [VERIFY]
    ksp("com.google.dagger:hilt-compiler:2.51.1")                         // [VERIFY]

    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1") // [VERIFY]
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.1") // [VERIFY]
}
```

---

## Testing Stack

| Tool | Purpose | Notes |
|------|---------|-------|
| JUnit 5 (via `junit-vintage-engine`) | Unit tests for Kotlin business logic | Standard; Room in-memory DB for DAO tests |
| Robolectric 4.12.x [VERIFY] | Run Android instrumented-style tests on JVM | Useful for testing NFC APDU command/response logic without a physical device |
| `#[cfg(test)]` Rust unit tests | Test all crypto primitives, record construction, DHT address derivation in isolation | Run with `cargo test` — no Android device needed for the Rust layer |
| Android Emulator (API 33+) | End-to-end NFC HCE testing | The emulator supports HCE simulation via `adb` since API 29; use `nfc-test-app` pattern: one emulator acts as HCE host, `IsoDep.get(tag)` simulates the reader |
| Two physical Android devices | Definitive NFC HCE integration test | Cannot be replaced — emulator NFC has quirks; plan for hardware testing in the NFC milestone |
| `pkarr` CLI or a local DHT node | DHT integration tests | pkarr ships a CLI tool usable for publishing/resolving records in tests; alternatively, run a local DHT bootstrap node |
| Espresso / Compose UI test | UI flow tests | Use `createComposeRule()` for Compose screen tests; focus on the tap-to-contact-display flow |

**NFC HCE testing note:** The hardest testing surface in this project. The emulator's NFC HCE support is incomplete for card emulation — `HostApduService` routing works but timing and connection teardown differ from physical hardware. Allocate time for physical device testing and consider writing a minimal NFC "reader" test app that exercises just the APDU exchange loop independently of the full UI.

---

## Sources

Version information sourced from training data (knowledge cutoff August 2025). All [VERIFY] items must be checked before pinning:

- Rust crates: https://crates.io/crates/[name]
- pkarr specifically: https://crates.io/crates/pkarr and https://github.com/Nuhvi/pkarr
- UniFFI: https://github.com/mozilla/uniffi-rs/releases
- Android dependencies: https://developer.android.com/jetpack/androidx/releases/
- Compose BOM mapping: https://developer.android.com/jetpack/compose/bom/bom-mapping
- ML Kit: https://developers.google.com/ml-kit/vision/barcode-scanning/android
- cargo-ndk: https://github.com/bbqsrc/cargo-ndk/releases
