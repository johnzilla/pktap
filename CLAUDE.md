<!-- GSD:project-start source:PROJECT.md -->
## Project

**PKTap**

A privacy-first, decentralized contact exchange app for Android. Users tap phones over NFC to swap Ed25519 public keys, then the app handles ECDH key agreement, XChaCha20-Poly1305 encryption, and publishes encrypted contact records to the Mainline DHT via Pkarr. No accounts, no cloud, no middleman. The recipient's app resolves the same deterministic DHT address, decrypts, and displays the shared contact fields.

**Core Value:** Two people tap phones and instantly see each other's chosen contact info — encrypted end-to-end, stored nowhere but their devices and a temporary DHT record that expires.

### Constraints

- **Platform**: Android-only for MVP — full NFC HCE support required (no iOS NFC write capability)
- **Record size**: 1000 bytes max per Pkarr record — text fields only, no photos
- **Key storage**: Master key must be non-extractable from Android Keystore (StrongBox/TEE)
- **No server**: Zero network requests to any PKTap-controlled server — DHT only
- **Crypto in Rust**: All cryptographic operations happen in Rust via UniFFI — no JVM crypto libraries for protocol operations
- **Memory safety**: All secret material zeroed after use (zeroize crate in Rust, explicit ByteArray zeroing in Kotlin post-FFI)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

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
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| androidx.hilt:hilt-android | 2.51.x [VERIFY] | Dependency injection | Use Hilt for ViewModel and service injection; avoids manual DI wiring for the Rust bridge singleton and Room DAOs | MEDIUM |
| hilt-compiler (kapt/ksp) | 2.51.x [VERIFY] | Hilt annotation processing | Use KSP variant if available in your Hilt version | MEDIUM |
| androidx.datastore:datastore-preferences | 1.1.x [VERIFY] | App settings (non-sensitive) — e.g., QR fallback enabled, public mode toggle | Prefer DataStore over SharedPreferences for new non-sensitive settings; EncryptedSharedPreferences remains for the seed | MEDIUM |
| kotlinx-datetime | 0.6.x [VERIFY] | TTL timestamps, "last verified" timestamps in KMP common code | KMP-compatible; avoids java.util.Date in common code | LOW |
| accompanist-permissions | 0.34.x [VERIFY] | Compose-friendly camera permission request flow | Google Accompanist; simplifies the boilerplate around `rememberPermissionState` | LOW |
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
## Target API Levels
| Setting | Value | Rationale |
|---------|-------|-----------|
| minSdk | 26 (Android 8.0) | Required for full NFC HCE stability, EncryptedSharedPreferences, and modern Keystore features (StrongBox available on API 28+, but TEE fallback covers 26+); covers ~95% of active Android devices |
| targetSdk | 35 [VERIFY] | Latest stable at time of writing; required for Play Store compliance |
| compileSdk | 35 [VERIFY] | Match targetSdk |
## Rust Workspace Layout
## Installation
# pktap-core/Cargo.toml
# uniffi-bindgen/Cargo.toml
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
## Sources
- Rust crates: https://crates.io/crates/[name]
- pkarr specifically: https://crates.io/crates/pkarr and https://github.com/Nuhvi/pkarr
- UniFFI: https://github.com/mozilla/uniffi-rs/releases
- Android dependencies: https://developer.android.com/jetpack/androidx/releases/
- Compose BOM mapping: https://developer.android.com/jetpack/compose/bom/bom-mapping
- ML Kit: https://developers.google.com/ml-kit/vision/barcode-scanning/android
- cargo-ndk: https://github.com/bbqsrc/cargo-ndk/releases
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, or `.github/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

## Skill routing

When the user's request matches an available skill, ALWAYS invoke it using the Skill
tool as your FIRST action. Do NOT answer directly, do NOT use other tools first.
The skill has specialized workflows that produce better results than ad-hoc answers.

Key routing rules:
- Product ideas, "is this worth building", brainstorming → invoke office-hours
- Bugs, errors, "why is this broken", 500 errors → invoke investigate
- Ship, deploy, push, create PR → invoke ship
- QA, test the site, find bugs → invoke qa
- Code review, check my diff → invoke review
- Update docs after shipping → invoke document-release
- Weekly retro → invoke retro
- Design system, brand → invoke design-consultation
- Visual audit, design polish → invoke design-review
- Architecture review → invoke plan-eng-review
- Save progress, checkpoint, resume → invoke checkpoint
- Code quality, health check → invoke health
