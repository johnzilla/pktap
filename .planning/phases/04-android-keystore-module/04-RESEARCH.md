# Phase 4: Android Keystore Module - Research

**Researched:** 2026-04-05
**Domain:** Android Keystore, AES-256-GCM, EncryptedSharedPreferences replacement, BIP-39 mnemonic (Rust), Compose Navigation first-launch flow
**Confidence:** HIGH (Keystore patterns), MEDIUM (EncryptedSharedPreferences deprecation path), HIGH (bip39 Rust API)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Ed25519 master keypair derived from HKDF seed in Rust (not stored in Keystore directly — Android Keystore doesn't support Ed25519). Generate random 32-byte seed, encrypt with Keystore AES-256-GCM key, store in EncryptedSharedPreferences. When needed, decrypt and pass to Rust which derives Ed25519 via ed25519-dalek. Keystore protects the seed at rest.
- **D-02:** Try StrongBox first, silent TEE fallback. Attempt `KeyGenParameterSpec.Builder().setIsStrongBoxBacked(true)`. Catch `StrongBoxUnavailableException`, retry without the flag. User never sees the difference. Log which path was used for diagnostics.
- **D-03:** 12-word mnemonic (128 bits entropy). Standard for mobile wallets, easier to write down. Sufficient security for a contact exchange app.
- **D-04:** No word re-entry verification. Display words with a checkbox: "I have written down these words". Simple acknowledgment only — the mnemonic is a backup, not a password.
- **D-05:** Decrypt seed from EncryptedSharedPreferences into a ByteArray, pass to PktapBridge, zero in finally block. Seed exists in Kotlin memory only for the duration of the FFI call. No caching of the decrypted seed.
- **D-06:** Derive Ed25519 pubkey once on app start, cache in memory (singleton/ViewModel). Pubkey is not secret — no need to re-derive for every NFC tap or DHT address computation. Zero the seed immediately after derivation.
- **D-07:** Detect first launch by checking EncryptedSharedPreferences for the encrypted seed. If present → returning user → main screen. If absent → first launch → key generation + mnemonic flow. No separate "has_launched" flag.
- **D-08:** Store a `mnemonic_acknowledged` boolean alongside the seed. On next launch, if seed exists but flag is false, show the mnemonic screen again. User must acknowledge before reaching the main screen. Prevents losing the backup opportunity if process is killed mid-setup.

### Claude's Discretion
- Exact EncryptedSharedPreferences key names
- ViewModel vs singleton for pubkey cache
- Mnemonic screen visual layout and typography
- Whether to use Hilt for dependency injection in this phase
- BIP-39 mnemonic generation: Rust (bip39 crate) vs Kotlin library
- Navigation architecture (Compose Navigation setup)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| KEY-01 | App generates Ed25519 master keypair in Android Keystore (StrongBox/TEE), non-extractable | D-01 design: Ed25519 derived in Rust from HKDF seed; Keystore holds AES-256-GCM key protecting that seed — not Ed25519 directly (Keystore doesn't support Ed25519). Requirement is satisfied by the AES key being non-extractable and the seed never leaving hardware protection. |
| KEY-02 | App generates AES-256-GCM key in Keystore for local data encryption | Direct `KeyGenerator("AndroidKeyStore")` with `KeyGenParameterSpec` — well-documented platform API, no library needed. |
| KEY-03 | App generates random 32-byte HKDF seed, encrypts with Keystore AES key, stores in EncryptedSharedPreferences | `EncryptedSharedPreferences` deprecated June 2025. Must use direct Keystore AES-256-GCM encryption + `SharedPreferences` or DataStore for storage. Research covers both the encryption pattern and migration risk. |
| KEY-04 | App displays BIP-39 mnemonic at first launch so user can back up their seed | `bip39` crate 2.2.2 provides `Mnemonic::from_entropy(&[u8])` — needs a new `derive_mnemonic_from_seed` FFI export in `ffi.rs`. Compose screen pattern documented. |
| KEY-05 | App falls back to TEE (non-StrongBox) Keystore on devices without StrongBox hardware | `StrongBoxUnavailableException` catch-and-retry pattern fully documented — verified via Android platform docs. |
</phase_requirements>

---

## Summary

Phase 4 is the security foundation of the entire PKTap app. It generates two Keystore keys: an AES-256-GCM key (for seed encryption) and conceptually satisfies KEY-01 via the HKDF seed path (Ed25519 is derived in Rust from that seed — the Keystore protects the seed at rest). All four components have well-understood patterns: Keystore key generation, AES-256-GCM encrypt/decrypt, BIP-39 mnemonic display, and Compose Navigation first-launch flow.

The most significant research finding is that `androidx.security:security-crypto` (EncryptedSharedPreferences) was **deprecated as of June 4, 2025** (version 1.1.0-beta01) — fully stable at 1.1.0 in July 2025. The CLAUDE.md technology stack recommends 1.1.0-alpha06, which predates awareness of this deprecation. Since D-01 and D-07 explicitly reference EncryptedSharedPreferences by name, the planner must choose between using the deprecated library (still functional, but carries long-term risk) or implementing the equivalent pattern directly using the Android Keystore API + plain `SharedPreferences`. Research shows the direct-Keystore pattern is straightforward and the recommended migration path. This is a decision point flagged in the Assumptions Log.

The second key finding is that `bip39` crate (2.2.2 — the latest, up from 2.0.x referenced in CLAUDE.md) is NOT yet in `pktap-core/Cargo.toml`. Phase 4 requires adding it and exporting a new FFI function `derive_mnemonic_from_seed(seed_bytes: Vec<u8>) -> Result<String, PktapError>`.

**Primary recommendation:** Add `bip39` to Cargo.toml, add `derive_mnemonic_from_seed` FFI export, implement AES-256-GCM + plain SharedPreferences for seed storage (replacing the deprecated EncryptedSharedPreferences), and use Compose Navigation with `@Serializable` route objects for the first-launch flow.

---

## Project Constraints (from CLAUDE.md)

- **Platform**: Android-only, minSdk 26
- **Crypto in Rust**: All cryptographic operations happen in Rust via UniFFI — no JVM crypto libraries for protocol operations
- **Memory safety**: All secret material zeroed after use (zeroize in Rust, explicit ByteArray.fill(0) in Kotlin post-FFI)
- **Key storage**: Master key must be non-extractable from Android Keystore (StrongBox/TEE)
- **No server**: Zero network requests to any PKTap-controlled server — DHT only
- **Build environment**: `jvmToolchain(21)` — Java 21, NOT system Java 25 (would break Kotlin compiler)
- **AGP 8.7.3, Kotlin 2.0.21**: Established in Phase 3 — do not upgrade
- **Multi-module**: `:app` depends on `:rust-bridge` — Keystore code lives in `:app`, FFI additions live in `:rust-bridge`

---

## Standard Stack

### Core (New Dependencies for Phase 4)

| Library | Version | Purpose | Why Standard | Source |
|---------|---------|---------|--------------|--------|
| bip39 (Rust crate) | 2.2.2 | BIP-39 mnemonic generation from 16 bytes entropy (12 words) | Not yet in Cargo.toml — must be added. `Mnemonic::from_entropy(&[u8])` is the canonical API; 128-bit entropy = 12 words | [VERIFIED: docs.rs/bip39/2.2.2] |
| Android Keystore (platform) | API 26+ | AES-256-GCM key generation, non-extractable, StrongBox/TEE | Platform API — no library needed. `KeyGenerator.getInstance("AndroidKeyStore")` with `KeyGenParameterSpec` | [VERIFIED: developer.android.com/privacy-and-security/keystore] |
| androidx.navigation:navigation-compose | 2.8.x (latest stable is 2.9.7 [VERIFY]) | NavHost + type-safe routes for first-launch flow | Required for multi-screen onboarding flow; type-safe routes via `@Serializable` classes landed in 2.8 | [CITED: developer.android.com/guide/navigation/design/type-safety] |
| androidx.lifecycle:lifecycle-viewmodel-compose | 2.8.x (latest stable is 2.8.7+ [VERIFY]) | ViewModel hoisting for pubkey cache in Compose | D-06 delegates pubkey caching to ViewModel/singleton — viewmodel-compose provides `viewModel()` and `collectAsStateWithLifecycle()` | [CITED: developer.android.com/jetpack/androidx/releases/lifecycle] |

### Dependencies Needing Version Catalog Additions

The following libraries are NOT currently in `android/gradle/libs.versions.toml` and must be added:

```
navigation-compose
lifecycle-viewmodel-compose
(security-crypto — see deprecation note below)
```

### EncryptedSharedPreferences Deprecation — Critical Finding

`androidx.security:security-crypto` was **deprecated June 4, 2025** (v1.1.0-beta01) and reached stable deprecated status at 1.1.0 on July 30, 2025. [VERIFIED: developer.android.com/jetpack/androidx/releases/security]

The CLAUDE.md stack table lists `1.1.0-alpha06` as the recommendation, written before this deprecation was known. Two paths exist:

**Option A — Still use EncryptedSharedPreferences (deprecated):** Add `1.1.0` (the final stable release). It remains fully functional on all supported Android versions. Risk: no future security patches from Google, possible Play Store warnings in future.

**Option B — Direct Keystore + SharedPreferences (recommended):**
1. Generate AES-256-GCM key in Keystore (already needed for KEY-02)
2. Use that same key to encrypt the 32-byte seed with `Cipher("AES/GCM/NoPadding")`
3. Store `iv_hex + ":" + ciphertext_hex` in a plain `SharedPreferences` file (the value is encrypted — the file itself need not be)

Option B aligns with the official deprecation migration path ("use platform APIs and Android Keystore directly") and eliminates a deprecated dependency. Option B is recommended here, but the planner should confirm with the developer since D-01 explicitly names EncryptedSharedPreferences. [ASSUMED: that D-01's reference to EncryptedSharedPreferences was written before the deprecation was known, and Option B satisfies the same security intent]

### Supporting Libraries Already in Version Catalog

| Library | Version (current catalog) | Phase 4 Role |
|---------|--------------------------|--------------|
| coroutines-android | 1.8.1 | All Keystore + FFI operations run on `Dispatchers.IO` |
| compose-material3 | via BOM 2024.09.03 | Mnemonic screen UI |
| compose-ui | via BOM | Screen layout |
| androidx-core-ktx | 1.15.0 | `SecureRandom`, coroutine utils |

### Cargo.toml Additions

```toml
# pktap-core/Cargo.toml — add to [dependencies]
bip39 = "2.2.2"
```

---

## Architecture Patterns

### Recommended Project Structure (Phase 4 additions)

```
android/
├── app/src/main/java/com/pktap/app/
│   ├── keystore/
│   │   ├── KeystoreManager.kt        # AES key generation + seed encrypt/decrypt
│   │   └── SeedRepository.kt         # Seed read/write against SharedPreferences
│   ├── ui/
│   │   ├── onboarding/
│   │   │   ├── MnemonicScreen.kt     # 12-word display + checkbox acknowledgment
│   │   │   └── MnemonicViewModel.kt  # Holds mnemonic words, drives NavController
│   │   └── main/
│   │       └── MainScreen.kt         # Placeholder for Phase 6
│   ├── AppViewModel.kt               # Pubkey cache singleton (D-06)
│   └── MainActivity.kt               # NavHost root — owns the navigation graph
│
pktap-core/src/
├── ffi.rs                            # Add: derive_mnemonic_from_seed export
└── mnemonic.rs                       # New: bip39 wrapper logic (internal)
```

### Pattern 1: AES-256-GCM Keystore Key Generation with StrongBox/TEE Fallback (D-02, KEY-02, KEY-05)

**What:** Generate a non-extractable AES-256-GCM key in Android Keystore. Try StrongBox first; catch `StrongBoxUnavailableException` and retry without the flag (silent TEE fallback).

**When to use:** First launch only, called once before seed generation.

```kotlin
// Source: developer.android.com/privacy-and-security/keystore
// [VERIFIED]
fun generateOrGetKeystoreKey(alias: String): SecretKey {
    val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    keyStore.getKey(alias, null)?.let { return it as SecretKey }

    return try {
        createKey(alias, strongBox = true)
    } catch (e: StrongBoxUnavailableException) {
        Log.d("KeystoreManager", "StrongBox unavailable, falling back to TEE")
        createKey(alias, strongBox = false)
    }
}

private fun createKey(alias: String, strongBox: Boolean): SecretKey {
    val keyGenerator = KeyGenerator.getInstance(
        KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore"
    )
    val spec = KeyGenParameterSpec.Builder(
        alias,
        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
    )
        .setKeySize(256)
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .apply { if (strongBox) setIsStrongBoxBacked(true) }
        .build()
    keyGenerator.init(spec)
    return keyGenerator.generateKey()
}
```

**Key constraint:** `StrongBoxUnavailableException` is only available on API 28+. Since minSdk is 26, wrap the `setIsStrongBoxBacked(true)` call with `Build.VERSION.SDK_INT >= Build.VERSION_CODES.P` [VERIFIED: android.security.keystore.StrongBoxUnavailableException — API 28+].

### Pattern 2: Encrypt/Decrypt Seed with Keystore AES Key (D-01, KEY-03)

**What:** Generate random 32-byte seed, encrypt with the Keystore AES key using `Cipher("AES/GCM/NoPadding")`. Store `base64(iv) + ":" + base64(ciphertext)` in plain `SharedPreferences`.

**IV management:** AES-GCM requires a fresh random IV for each encryption. The Keystore Cipher generates it automatically on `ENCRYPT_MODE` — retrieve with `cipher.iv`. Must be stored alongside ciphertext for decryption.

```kotlin
// Source: developer.android.com/privacy-and-security/keystore [VERIFIED]
fun encryptSeed(seed: ByteArray, key: SecretKey): String {
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.ENCRYPT_MODE, key)
    val ciphertext = cipher.doFinal(seed)
    val iv = cipher.iv  // 12 bytes for GCM
    // Store as "iv_b64:ciphertext_b64"
    return "${Base64.encodeToString(iv, Base64.NO_WRAP)}:${Base64.encodeToString(ciphertext, Base64.NO_WRAP)}"
}

fun decryptSeed(stored: String, key: SecretKey): ByteArray {
    val parts = stored.split(":")
    val iv = Base64.decode(parts[0], Base64.NO_WRAP)
    val ciphertext = Base64.decode(parts[1], Base64.NO_WRAP)
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, iv))
    return cipher.doFinal(ciphertext)
}
```

**Zeroing:** Caller must `seed.fill(0)` immediately after `encryptSeed()` returns. The `ByteArray` returned from `decryptSeed()` must be zeroed in a `finally` block after it is passed to PktapBridge.

### Pattern 3: New FFI Export — `derive_mnemonic_from_seed` (KEY-04)

**What:** The current `ffi.rs` has no function that takes a seed and returns a mnemonic string. Phase 4 requires a new Rust FFI export.

**Signature:**
```rust
// pktap-core/src/ffi.rs — new export
// Source: docs.rs/bip39/2.2.2/bip39/struct.Mnemonic.html [VERIFIED]
#[uniffi::export]
pub fn derive_mnemonic_from_seed(seed_bytes: Vec<u8>) -> Result<String, PktapError> {
    if seed_bytes.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    // Use first 16 bytes as entropy (128 bits → 12 words)
    let entropy = &seed_bytes[..16];
    let mnemonic = bip39::Mnemonic::from_entropy(entropy)
        .map_err(|_| PktapError::SerializationFailed)?;
    Ok(mnemonic.words().collect::<Vec<_>>().join(" "))
}
```

**NOTE:** The seed ByteArray must be zeroed after calling this function (same D-05 pattern as `ecdhAndEncrypt`). Wrap in `PktapBridge.kt` with a `finally { seedBytes.fill(0) }` block.

**NOTE:** `Mnemonic::from_entropy` expects entropy of 16 bytes for 12 words (128 bits of entropy + 4-bit checksum = 132 bits → 12 words). The HKDF seed is 32 bytes; use only the first 16 bytes as entropy. The remaining bytes are not used for mnemonic derivation. [VERIFIED: docs.rs/bip39/2.2.2]

### Pattern 4: `derive_public_key` FFI Export (D-06)

**What:** D-06 requires deriving the Ed25519 public key once on app start and caching it. The current `ffi.rs` has no function that accepts a 32-byte seed and returns a 32-byte Ed25519 public key. This new export is needed.

**Signature:**
```rust
// pktap-core/src/ffi.rs — new export
#[uniffi::export]
pub fn derive_public_key(seed_bytes: Vec<u8>) -> Result<Vec<u8>, PktapError> {
    if seed_bytes.len() != 32 {
        return Err(PktapError::InvalidKey);
    }
    let seed: [u8; 32] = seed_bytes.as_slice().try_into()
        .map_err(|_| PktapError::InvalidKey)?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let pubkey = signing_key.verifying_key().to_bytes().to_vec();
    // seed array will be zeroed by the zeroize pattern — ensure seed is dropped
    Ok(pubkey)
}
```

**Seed zeroing:** The `seed` array must be zeroed before return. Use `seed.zeroize()` explicitly (as done in `ecdh_and_encrypt`). Wrap in `PktapBridge.kt` with `finally { seedBytes.fill(0) }`.

**Caching:** The returned 32-byte `ByteArray` is public — no zeroing needed. Cache in `AppViewModel` (or a Kotlin singleton). Phase 5 (NFC) will read from this cache.

### Pattern 5: Compose Navigation First-Launch Flow (D-07, D-08)

**What:** MainActivity hosts a `NavHost` with two destinations: `MnemonicSetup` (first launch) and `Main` (returning user). The start destination is determined by checking SharedPreferences at startup.

```kotlin
// Source: developer.android.com/develop/ui/compose/navigation [CITED]
// Navigation 2.8 type-safe routes
@Serializable object MnemonicSetup
@Serializable object Main

@Composable
fun AppNavigation(startDestination: Any) {
    val navController = rememberNavController()
    NavHost(navController = navController, startDestination = startDestination) {
        composable<MnemonicSetup> {
            MnemonicScreen(onAcknowledged = {
                navController.navigate(Main) {
                    popUpTo<MnemonicSetup> { inclusive = true }
                }
            })
        }
        composable<Main> {
            MainScreen()
        }
    }
}
```

**Start destination logic (D-07 / D-08):**
- Seed absent → `startDestination = MnemonicSetup` (generate + display mnemonic)
- Seed present, `mnemonic_acknowledged = false` → `startDestination = MnemonicSetup` (show mnemonic again — recovery from interrupted setup)
- Seed present, `mnemonic_acknowledged = true` → `startDestination = Main`

Compute this determination off-main-thread before `setContent {}` (using a splash/loading state or `runBlocking` on `Dispatchers.IO`) so the UI never flashes the wrong screen. [ASSUMED: that a brief loading state or early determination in `onCreate` via `Dispatchers.IO` is the correct pattern — verify against Navigation 2.8 documentation]

### Pattern 6: SharedPreferences Key Names

As this is Claude's Discretion, recommended names:

| Preference Key | Value | Notes |
|----------------|-------|-------|
| `PREF_SEED_ENCRYPTED` | `"iv_b64:ciphertext_b64"` | Encrypted 32-byte HKDF seed |
| `PREF_MNEMONIC_ACKNOWLEDGED` | boolean | D-08 interrupted-setup recovery |
| Keystore alias | `"pktap_aes_key"` | AES-256-GCM encryption key |

File name: `"pktap_secure_prefs"` (internal, mode `Context.MODE_PRIVATE`).

### Anti-Patterns to Avoid

- **Using `Cipher.ENCRYPT_MODE` without re-keying for each encrypt:** GCM is nonce-misuse-resistant within a single key but reusing the same IV with the same key breaks confidentiality completely. The Keystore Cipher auto-generates a fresh IV on each `ENCRYPT_MODE` init — always retrieve and store `cipher.iv` after init, never provide your own IV.
- **Calling Keystore operations on the main thread:** Keystore crypto (especially StrongBox) can take hundreds of milliseconds. Always wrap in `withContext(Dispatchers.IO)`. [VERIFIED: Android documentation]
- **Storing the decrypted seed in a class field:** D-05 is explicit — seed must exist only in local scope and be zeroed in a `finally` block. Never assign to `var seed` on a ViewModel.
- **Caching the decrypted seed:** D-06 caches only the pubkey (not secret), not the seed. The seed is decrypted fresh for each crypto operation.
- **Not handling KEY-01 vs KEY-02 key alias separation:** Two separate Keystore keys: one for AES seed encryption (KEY-02 / `pktap_aes_key`). The Ed25519 "keypair" (KEY-01) is implicitly satisfied by protecting the seed that derives it. Using one key alias for both purposes conflates them.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| BIP-39 mnemonic word list | Custom word array + checksum | `bip39` crate `Mnemonic::from_entropy()` | BIP-39 checksum and word encoding is finicky — off-by-one in entropy/checksum produces wrong words; the crate is tested against BIP-39 test vectors |
| IV storage format | Custom binary format | Concatenate `iv_b64 + ":" + ciphertext_b64` stored as `String` in SharedPreferences | Standard, easy to read back, survives app upgrades |
| Keystore key existence check | Manual error handling | `KeyStore.getKey(alias, null) != null` idiom | The platform handles key existence cleanly; don't wrap in a try-catch to detect absence |
| StrongBox detection | `PackageManager.hasSystemFeature(FEATURE_STRONGBOX_KEYSTORE)` check before attempting | Catch `StrongBoxUnavailableException` after attempting | `FEATURE_STRONGBOX_KEYSTORE` reflects hardware presence but not whether the current operation is supported; exception-based fallback is the documented pattern |
| Mnemonic word display grid | Custom 3x4 grid with manual word indices | Compose `LazyVerticalGrid` or simple `FlowRow` with index-labeled items | No crypto logic involved — use standard Compose layout |

---

## Common Pitfalls

### Pitfall 1: StrongBox API Level Guard Missing
**What goes wrong:** `setIsStrongBoxBacked(true)` causes a `NoSuchMethodError` on API 26-27 (pre-Pie).
**Why it happens:** `StrongBoxUnavailableException` and `setIsStrongBoxBacked()` were added in API 28. The emulator runs API 35, so this won't manifest in dev but will crash on minSdk 26 devices.
**How to avoid:** Guard the `setIsStrongBoxBacked(true)` call with `Build.VERSION.SDK_INT >= Build.VERSION_CODES.P` (API 28).
**Warning signs:** `NoSuchMethodError` or `VerifyError` on Android 8.x devices.

### Pitfall 2: GCM IV Reuse
**What goes wrong:** Reusing the same IV with the same AES-GCM key breaks ciphertext confidentiality entirely — two ciphertexts XOR to reveal the XOR of plaintexts.
**Why it happens:** Developer manually provides a fixed IV (e.g., `ByteArray(12)`) instead of letting the Keystore Cipher generate one.
**How to avoid:** Always use `cipher.init(Cipher.ENCRYPT_MODE, key)` without an IV parameter — the Keystore generates a random one. Retrieve it with `cipher.iv` after init.
**Warning signs:** Decryption produces garbage on subsequent encrypts, or `InvalidAlgorithmParameterException` when providing an IV.

### Pitfall 3: Seed Leaked into ViewModel / Class Field
**What goes wrong:** Decrypted seed stored as a class-level property survives across multiple FFI calls and GC cycles without zeroing.
**Why it happens:** Developer caches seed to avoid repeated Keystore decrypt overhead.
**How to avoid:** Enforce D-05 — seed is a local `val` in a suspend function, passed to PktapBridge, zeroed in `finally`. A code review checklist item: "no `seed` property on any class".
**Warning signs:** `ByteArray` named `seed` or `hkdfSeed` appearing as a ViewModel field.

### Pitfall 4: `bip39` Entropy Size Mismatch
**What goes wrong:** Passing 32 bytes to `Mnemonic::from_entropy()` generates a 24-word mnemonic (256 bits entropy), not the desired 12-word mnemonic.
**Why it happens:** D-03 specifies 12 words = 128 bits entropy = 16 bytes. The HKDF seed is 32 bytes total.
**How to avoid:** Pass only `seed_bytes[..16]` (first 16 bytes) as entropy to `Mnemonic::from_entropy()`. The remaining 16 bytes of the seed are not involved in mnemonic derivation. [VERIFIED: docs.rs/bip39/2.2.2 — "entropy must be a multiple of 32 bits and 128–256 bits in length"]
**Warning signs:** `mnemonic.word_count()` returns 24 instead of 12.

### Pitfall 5: Keystore Key Lost After Factory Reset
**What goes wrong:** The AES Keystore key is destroyed on factory reset. Attempting to decrypt the seed (still in SharedPreferences) after restore fails with `KeyPermanentlyInvalidatedException`.
**Why it happens:** Keystore keys are hardware-bound. This is by design — the encrypted seed becomes unrecoverable.
**How to avoid:** This is the expected security model, not a bug. The app must handle this gracefully: if decryption fails with `KeyPermanentlyInvalidatedException`, treat as first launch (clear prefs, regenerate). Document this behavior in the UX. [ASSUMED: that key invalidation on factory reset is expected behavior here; verify no other invalidation path (screen lock change) is relevant for pktap's use case]

### Pitfall 6: `mnemonic_acknowledged` Flag Not Cleared on Key Regeneration
**What goes wrong:** If the app ever regenerates keys (e.g., factory reset path above), the old `mnemonic_acknowledged = true` from SharedPreferences persists, skipping the mnemonic display for the new key.
**Why it happens:** SharedPreferences is wiped on factory reset along with the Keystore key (both are per-user), so this is actually safe in the reset case. However, if a developer test path regenerates keys without clearing prefs, the flag persists.
**How to avoid:** Whenever a new seed is generated, always reset `mnemonic_acknowledged = false` as part of the same atomic SharedPreferences write transaction.

### Pitfall 7: EncryptedSharedPreferences Deprecation (KEY-03)
**What goes wrong:** Using `EncryptedSharedPreferences` 1.1.0 adds a deprecated dependency that will receive no security updates.
**Why it happens:** CLAUDE.md recommends `security-crypto 1.1.0-alpha06` but this was written before the deprecation (June 2025).
**How to avoid:** Use the direct Keystore + plain SharedPreferences pattern (Pattern 2 above). The encryption is identical in security — just without the EncryptedSharedPreferences wrapper.

---

## Code Examples

### BIP-39 Mnemonic from Entropy (Rust, verified)

```rust
// Source: docs.rs/bip39/2.2.2/bip39/struct.Mnemonic.html [VERIFIED]
use bip39::Mnemonic;

// 16 bytes of entropy → 12 mnemonic words
let entropy: &[u8] = &seed_bytes[..16];  // first 16 bytes of 32-byte seed
let mnemonic = Mnemonic::from_entropy(entropy).expect("valid entropy length");
let phrase: String = mnemonic.words().collect::<Vec<_>>().join(" ");
// phrase is "word1 word2 ... word12"
```

### AES-256-GCM Keystore Encrypt/Decrypt (Kotlin, verified)

```kotlin
// Source: developer.android.com/privacy-and-security/keystore [VERIFIED]
// Encryption
val cipher = Cipher.getInstance("AES/GCM/NoPadding")
cipher.init(Cipher.ENCRYPT_MODE, keystoreKey)
val ciphertext = cipher.doFinal(plaintext)
val iv = cipher.iv  // Always 12 bytes for AES-GCM

// Decryption
val decipher = Cipher.getInstance("AES/GCM/NoPadding")
decipher.init(Cipher.DECRYPT_MODE, keystoreKey, GCMParameterSpec(128, iv))
val plaintext = decipher.doFinal(ciphertext)
```

### StrongBox/TEE Fallback (Kotlin, verified pattern)

```kotlin
// Source: developer.android.com/privacy-and-security/keystore [VERIFIED]
// StrongBoxUnavailableException is API 28+ — guard with version check
fun generateKey(alias: String): SecretKey {
    return try {
        buildKeyGenSpec(alias, strongBox = Build.VERSION.SDK_INT >= 28)
            .let { spec -> KeyGenerator.getInstance(KEY_ALGORITHM_AES, "AndroidKeyStore")
                .also { it.init(spec) }
                .generateKey()
            }
    } catch (e: StrongBoxUnavailableException) {
        // Silent fallback — TEE is still hardware-backed
        buildKeyGenSpec(alias, strongBox = false)
            .let { spec -> KeyGenerator.getInstance(KEY_ALGORITHM_AES, "AndroidKeyStore")
                .also { it.init(spec) }
                .generateKey()
            }
    }
}
```

### Navigation 2.8 Type-Safe Routes (Kotlin, verified pattern)

```kotlin
// Source: developer.android.com/guide/navigation/design/type-safety [CITED]
@Serializable object MnemonicSetup
@Serializable object Main

NavHost(navController, startDestination = if (hasSeed && acknowledged) Main else MnemonicSetup) {
    composable<MnemonicSetup> { MnemonicScreen(onAcknowledged = {
        navController.navigate(Main) { popUpTo<MnemonicSetup> { inclusive = true } }
    }) }
    composable<Main> { MainScreen() }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| EncryptedSharedPreferences | Direct Keystore AES-256-GCM + plain SharedPreferences | June 2025 (deprecated) | EncryptedSharedPreferences still works but receives no updates; direct Keystore is the migration path |
| bip39 crate 2.0.x (CLAUDE.md reference) | bip39 2.2.2 (current) | API unchanged; 2.2.2 is current as of April 2026 | Same `Mnemonic::from_entropy` API — version bump is safe |
| Navigation string-based routes | `@Serializable` type-safe routes (Navigation 2.8+) | Navigation 2.8.0 (August 2024) | Avoids string route typos; required by CLAUDE.md preference |
| UDL files for UniFFI | Proc-macro `#[uniffi::export]` | UniFFI 0.28+ | Already established in Phase 3 — new FFI exports follow same pattern |

**Deprecated/outdated:**
- `EncryptedSharedPreferences`: Deprecated June 2025 — do not use in new code
- `kapt` for Room annotation processing: Already established as KSP in project conventions

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Android Emulator (AVD: pktap-test) | Instrumented Keystore tests | ✓ | API 35, x86_64, Google APIs | — |
| Java 21 (`jvmToolchain(21)`) | Kotlin compiler, Gradle | ✓ (system has Java 25, but jvmToolchain(21) downloads JDK 21) | 21 via toolchain | — |
| `cargo`/`cargo-ndk` | Rust FFI additions (bip39) | [ASSUMED: available from Phase 3] | — | — |
| UniFFI bindgen | Kotlin binding regeneration | [ASSUMED: available from Phase 3] | — | — |
| StrongBox hardware | KEY-05 happy path | ✗ on emulator (TEE path exercised instead) | — | TEE fallback — emulator exercises the fallback path |

**Note on emulator StrongBox:** The `pktap-test` AVD is API 35 / x86_64 / Google APIs. Android emulators do not support StrongBox hardware. Tests will exercise only the TEE fallback path. Physical device testing of the StrongBox path must happen separately (ideally on a Pixel 6+ or similar). [VERIFIED: Android emulator does not support StrongBox — it will throw StrongBoxUnavailableException, which is exactly the fallback path to test]

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | AndroidX Test (instrumented) + JUnit4 — established in Phase 3 |
| Config file | Instrumentation runner: `androidx.test.runner.AndroidJUnitRunner` (in `rust-bridge/build.gradle.kts` — app module needs same) |
| Quick run command | `./gradlew :app:connectedDebugAndroidTest` (requires emulator running) |
| Full suite command | `./gradlew :app:connectedDebugAndroidTest :rust-bridge:connectedDebugAndroidTest` |
| Emulator launch | `~/Android/Sdk/emulator/emulator -avd pktap-test -no-audio -no-window -gpu angle_indirect &` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| KEY-01 | Ed25519 keypair deterministically derived from seed, public key is 32 bytes | Instrumented (needs Keystore) | `./gradlew :app:connectedDebugAndroidTest` | ❌ Wave 0 |
| KEY-02 | AES-256-GCM key generated in Keystore, non-extractable, persists across restarts | Instrumented (needs Keystore) | `./gradlew :app:connectedDebugAndroidTest` | ❌ Wave 0 |
| KEY-03 | Encrypted seed survives app restart (decrypt returns same 32 bytes) | Instrumented (needs Keystore + SharedPreferences) | `./gradlew :app:connectedDebugAndroidTest` | ❌ Wave 0 |
| KEY-04 | `derive_mnemonic_from_seed` returns 12-word string from 32-byte seed | Rust unit test | `cargo test -p pktap-core test_derive_mnemonic` | ❌ Wave 0 |
| KEY-04 | MnemonicScreen displays 12 words, cannot be bypassed without checkbox | Compose UI test (manual review acceptable) | `./gradlew :app:connectedDebugAndroidTest` | ❌ Wave 0 |
| KEY-05 | Key generation succeeds on emulator (TEE fallback path) without crashing | Instrumented | `./gradlew :app:connectedDebugAndroidTest` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p pktap-core` (Rust unit tests, no emulator needed, fast)
- **Per wave merge:** `./gradlew :app:connectedDebugAndroidTest` (requires running emulator)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `android/app/src/androidTest/java/com/pktap/app/keystore/KeystoreManagerTest.kt` — covers KEY-02, KEY-03, KEY-05
- [ ] `android/app/src/androidTest/java/com/pktap/app/keystore/SeedRepositoryTest.kt` — covers KEY-03 (round-trip survive restart)
- [ ] `android/app/src/androidTest/java/com/pktap/app/keystore/AppViewModelTest.kt` — covers KEY-01 (pubkey derivation + caching)
- [ ] `pktap-core/src/ffi.rs` — test for `derive_mnemonic_from_seed` (12 words, deterministic, correct first 16 bytes)
- [ ] Add `testInstrumentationRunner` to `:app` module `build.gradle.kts` (currently only in `:rust-bridge`)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No user auth in this phase |
| V3 Session Management | No | No session state |
| V4 Access Control | No | Single-user app |
| V5 Input Validation | Yes | Rust FFI validates `seed_bytes.len() == 32`; Kotlin validates Keystore decrypt output before use |
| V6 Cryptography | Yes | AES-256-GCM (Keystore), Ed25519 derivation (Rust/dalek), BIP-39 (bip39 crate) — no hand-rolled crypto |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Seed extracted from memory after decryption | Information Disclosure | Zero ByteArray immediately in `finally` block (D-05, D-06 patterns) |
| Seed logged (e.g., crash reporter captures stack trace) | Information Disclosure | Never log seed bytes; use opaque length-only log messages |
| IV reuse with AES-GCM | Tampering | Let Keystore Cipher auto-generate IV on `ENCRYPT_MODE`; never provide a fixed IV |
| Mnemonic displayed in screenshot / notification | Information Disclosure | Set `WindowManager.LayoutParams.FLAG_SECURE` on the mnemonic screen's window |
| StrongBox downgrade attack | Tampering / Elevation of Privilege | StrongBox/TEE selection is transparent to users; D-02 silent fallback is appropriate since both provide non-extractable keys |
| Keystore key invalidated silently | Denial of Service | Handle `KeyPermanentlyInvalidatedException` on decrypt; treat as first launch |

**FLAG_SECURE note:** The mnemonic screen must prevent screenshots and recent apps thumbnails from capturing the 12 words. Call `window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)` in `MnemonicScreen`'s `DisposableEffect` or in `MainActivity` before setting content on that screen. [ASSUMED: that FLAG_SECURE is the correct API — verify against Compose documentation for the correct lifecycle point]

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | D-01's reference to EncryptedSharedPreferences was written before the June 2025 deprecation; Option B (direct Keystore + SharedPreferences) satisfies the same security intent | EncryptedSharedPreferences Deprecation | Developer may prefer to continue using deprecated library; plan should present choice clearly |
| A2 | The HKDF "seed" in D-01 is the 32-byte value produced by Android's `SecureRandom`, NOT `ed25519_dalek::SigningKey::to_scalar_bytes()` | Pattern 3 (bip39 FFI export) | If seed = scalar bytes, the bip39 mnemonic must roundtrip back to the same scalar — test round-trip recovery explicitly |
| A3 | `cargo` and `cargo-ndk` are available from Phase 3 completion | Environment Availability | If not available, adding bip39 dependency requires re-establishing Rust build |
| A4 | Navigation 2.9.x (latest) is backward-compatible with 2.8 patterns; using 2.8.x from CLAUDE.md is still recommended | Standard Stack | If 2.9 has breaking changes, version in catalog may need update |
| A5 | `FLAG_SECURE` on the mnemonic screen is compatible with Compose window management and does not cause black screen on all API levels | Security Domain | Some OEM launchers have known FLAG_SECURE rendering issues; verify on emulator |
| A6 | `KeyPermanentlyInvalidatedException` is the only exception path for decryption failure; not also `UnrecoverableKeyException` | Pitfall 5 | Missing an exception type means a crash instead of graceful first-launch recovery |

---

## Open Questions (RESOLVED)

1. **EncryptedSharedPreferences vs. direct Keystore pattern**
   - What we know: security-crypto is deprecated as of June 2025; D-01 explicitly names EncryptedSharedPreferences
   - What's unclear: Whether the developer wants to keep the deprecated dependency or migrate to the direct pattern
   - Recommendation: Planner should present both options with a clear recommendation for Option B (direct), since D-01's intent (seed protected by Keystore AES) is fully satisfied by Option B
   - **RESOLVED:** Direct Keystore + SharedPreferences (Option B). User approved the change; CONTEXT.md D-01 updated accordingly.

2. **Seed definition (random bytes vs. Ed25519 scalar bytes)**
   - What we know: D-01 says "generate random 32-byte seed"; ffi.rs tests use `signing_key.to_scalar_bytes()` as the seed
   - What's unclear: Whether the "seed" stored in SharedPreferences is truly random (`SecureRandom.nextBytes(32)`) or is an Ed25519 signing key scalar
   - Recommendation: Use `SecureRandom.nextBytes(32)` — a purely random seed passed to Rust, which derives the Ed25519 signing key from it via `SigningKey::from_bytes(&seed)`. This matches D-01 and the FFI round-trip test pattern.
   - **RESOLVED:** `SecureRandom.nextBytes(32)` — per Plan 01 `SeedRepository` implementation.

3. **Hilt DI — include or defer?**
   - What we know: CLAUDE.md lists Hilt as a supporting library; it's Claude's Discretion for Phase 4
   - What's unclear: Whether adding Hilt in Phase 4 is premature (adds compile-time annotation processing complexity) or worth it for consistency with Phase 6
   - Recommendation: Defer Hilt to Phase 6 (App Integration). Phase 4 has few injection points: `KeystoreManager` as a plain object + `AppViewModel` accessed via `viewModel()`. Manual wiring is simpler and Hilt adds KSP configuration overhead.
   - **RESOLVED:** Deferred to Phase 6 — Plan 02 uses a manual factory pattern instead.

---

## Sources

### Primary (HIGH confidence)
- [Android Keystore system](https://developer.android.com/privacy-and-security/keystore) — AES key generation, Cipher patterns, StrongBox fallback
- [Security releases](https://developer.android.com/jetpack/androidx/releases/security) — EncryptedSharedPreferences deprecation confirmed June 2025
- [bip39 2.2.2 docs.rs](https://docs.rs/bip39/2.2.2/bip39/struct.Mnemonic.html) — `from_entropy`, `words()`, `to_entropy()` API signatures
- [Type-safe navigation](https://developer.android.com/guide/navigation/design/type-safety) — `@Serializable` routes, `composable<T>` pattern

### Secondary (MEDIUM confidence)
- [EncryptedSharedPreferences deprecation — ProAndroidDev](https://www.spght.dev/articles/28-05-2024/jetsec-deprecation) — Ed Holloway-George article on migration path
- [droidcon migration guide 2026](https://www.droidcon.com/2025/12/16/goodbye-encryptedsharedpreferences-a-2026-migration-guide/) — Community migration examples
- [StrongBox/TEE — Medium/Comviva](https://medium.com/@dfs.techblog/safeguarding-cryptographic-keys-implementing-tee-and-strongbox-in-android-applications-7894c800e43e) — Architecture confirmation

### Tertiary (LOW confidence)
- WebSearch results on Navigation onboarding patterns — multiple community sources agree on NavHost conditional start destination pattern

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Keystore API is platform-stable; bip39 2.2.2 verified on docs.rs; Navigation/Lifecycle versions need verification against current catalog
- Architecture: HIGH — Keystore + AES-GCM patterns are well-established; bip39 from_entropy API verified
- Pitfalls: HIGH — StrongBox API level guard, IV reuse, seed zeroing are all well-documented failure modes
- EncryptedSharedPreferences deprecation: HIGH — verified directly on official AndroidX releases page

**Research date:** 2026-04-05
**Valid until:** 2026-07-05 (stable APIs; 90 days)
