# PKTap

Privacy-first, decentralized contact exchange for Android. Tap phones over NFC to swap encrypted contact info -- no accounts, no servers, no cloud.

## How it works

1. **Tap** -- Two phones exchange Ed25519 public keys over NFC (HCE)
2. **Encrypt** -- ECDH key agreement derives a shared secret; contact fields are encrypted with XChaCha20-Poly1305
3. **Publish** -- The encrypted record is published to the Mainline DHT via [Pkarr](https://github.com/Nuhvi/pkarr) at a deterministic address (`SHA-256(sort(A_pk, B_pk))`)
4. **Resolve** -- The other device resolves the same DHT address, decrypts, and displays the shared contact fields

No PKTap server is ever contacted. Records expire from the DHT naturally.

## Architecture

```
+------------------+       UniFFI       +------------------+
|   Android App    | <================> |    pktap-core     |
|   (Kotlin/       |    (Kotlin FFI)    |    (Rust)         |
|    Compose)       |                   |                   |
+------------------+                    +------------------+
| Jetpack Compose  |                    | Ed25519 keys     |
| NFC HCE Service  |                    | X25519 ECDH      |
| Android Keystore |                    | XChaCha20-Poly1305|
| Room DB          |                    | Pkarr/DHT client  |
| Navigation       |                    | BIP-39 mnemonic   |
+------------------+                    +------------------+
```

**Rust core** (`pktap-core/`) -- all cryptographic operations: key generation, ECDH, encryption/decryption, signing, DHT publish/resolve, mnemonic backup. Exposed to Kotlin via UniFFI.

**Android app** (`android/`) -- Jetpack Compose UI, NFC HCE service, Android Keystore integration, local storage. No crypto in the JVM layer.

## Constraints

- Android-only (iOS lacks NFC HCE write capability)
- 1000 bytes max per Pkarr record -- text fields only
- Master key is non-extractable from Android Keystore (StrongBox/TEE)
- All secret material zeroed after use (`zeroize` in Rust, explicit `ByteArray` zeroing in Kotlin)

## Building

### Prerequisites

- Rust toolchain with Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
- [cargo-ndk](https://github.com/nickel-org/cargo-ndk): `cargo install cargo-ndk`
- Android SDK (API 35) + NDK r26+
- JDK 21

### Rust core

```bash
cargo test -p pktap-core
```

### Android app

```bash
cd android
./gradlew assembleDebug
```

The Gradle build invokes `cargo-ndk` to cross-compile the Rust library and runs `uniffi-bindgen` to generate Kotlin bindings.

## Project status

**v1.0 milestone -- 5 of 7 phases complete**

| Phase | Status |
|-------|--------|
| 1. Rust Crypto Core | Done |
| 2. Pkarr DHT Integration | Done |
| 3. UniFFI Bridge + Android Build | Done |
| 4. Android Keystore Module | Done |
| 5. NFC HCE Module | Done |
| 6. App Integration + Core UI | Next |
| 7. QR Fallback + Public Mode | Planned |

## License

TBD
