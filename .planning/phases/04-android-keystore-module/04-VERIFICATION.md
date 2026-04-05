---
phase: 04-android-keystore-module
verified: 2026-04-05T20:00:00Z
status: human_needed
score: 9/9 must-haves verified (automated)
human_verification:
  - test: "First-launch mnemonic display on emulator"
    expected: "App shows exactly 12 numbered BIP-39 words, Continue button disabled until checkbox checked, navigates to main screen after acknowledgment showing truncated pubkey hex"
    why_human: "Compose UI rendering and touch interaction cannot be verified programmatically without an instrumented Compose test"
  - test: "Persistence after force-stop and relaunch"
    expected: "Relaunching after force-stop (post-acknowledgment) goes directly to main screen; same pubkey hex as before"
    why_human: "Requires live app lifecycle — SharedPreferences persistence across process death"
  - test: "Interrupted setup re-shows mnemonic (D-08)"
    expected: "Force-stopping before checking the checkbox, then relaunching, shows the mnemonic screen again"
    why_human: "Requires live app state manipulation"
  - test: "FLAG_SECURE blocks screenshots on mnemonic screen"
    expected: "Screenshot attempt on the mnemonic screen produces a black image"
    why_human: "System-level screenshot prevention cannot be tested programmatically"
  - test: "No mnemonic words in logcat"
    expected: "adb logcat | grep -i mnemonic shows operation names only, no word values"
    why_human: "Requires running app and logcat stream"
  - test: "Keystore TEE log entry on emulator"
    expected: "adb logcat | grep KeystoreManager shows 'Key generated via TEE' (not StrongBox on emulator)"
    why_human: "Requires live emulator run with logcat"
---

# Phase 4: Android Keystore Module Verification Report

**Phase Goal:** The app generates hardware-backed keys on first launch, encrypts the HKDF seed with a Keystore AES key stored in SharedPreferences, displays the BIP-39 mnemonic, and handles StrongBox/TEE fallback transparently across all supported device types
**Verified:** 2026-04-05T20:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | AES-256-GCM key generated in Android Keystore, non-extractable, StrongBox/TEE fallback | VERIFIED | `KeystoreManager.kt`: `KeyGenerator.getInstance(AES, AndroidKeyStore)`, API 28+ StrongBox attempt, `StrongBoxUnavailableException` catch with TEE fallback, API 26-27 guard |
| 2  | Random 32-byte seed encrypted with Keystore AES key and stored in SharedPreferences | VERIFIED | `SeedRepository.generateAndStoreSeed()`: `SecureRandom().nextBytes(32)`, `KeystoreManager.encrypt(seed, key)`, stores `base64(iv):base64(ciphertext)` in `pktap_secure_prefs` |
| 3  | Decrypting the stored seed returns the original 32 bytes | VERIFIED | `SeedRepository.decryptSeed()` fully wired through `KeystoreManager.decrypt()`; 9 instrumented tests pass including `testDecryptSeedMatchesGenerated` |
| 4  | `derive_public_key` FFI returns a 32-byte Ed25519 public key from a 32-byte seed | VERIFIED | `ffi.rs:derive_public_key` exports `SigningKey::from_bytes(&seed).verifying_key().to_bytes().to_vec()`; 4 Rust unit tests pass including dalek match test |
| 5  | `derive_mnemonic_from_seed` FFI returns exactly 12 BIP-39 words from a 32-byte seed | VERIFIED | `ffi.rs:derive_mnemonic_from_seed` → `mnemonic::mnemonic_from_entropy` using `bip39::Mnemonic::from_entropy(&seed[..16])`; `test_derive_mnemonic_from_seed_returns_12_words` passes |
| 6  | On emulator (no StrongBox), key generation falls back to TEE without error | VERIFIED | `KeystoreManager.generateOrGetKey()` catches `StrongBoxUnavailableException` and retries with `strongBox = false`; 9/9 instrumented tests pass on `pktap-test` AVD (emulator TEE path) |
| 7  | On first launch, app generates keys and shows 12-word BIP-39 mnemonic | VERIFIED (code path) | `MnemonicViewModel.init{}` calls `generateAndStoreSeed()` if no seed exists, then `PktapBridge.deriveMnemonicFromSeed()`; `_words` StateFlow populated; MnemonicScreen renders words grid | HUMAN NEEDED for visual confirmation |
| 8  | User must check checkbox before proceeding; mnemonic screen re-shown if setup interrupted | VERIFIED (code path) | Button `enabled = checked && words.isNotEmpty()`; `acknowledge()` calls `setMnemonicAcknowledged()`; `MainActivity` logic: no seed or `!isMnemonicAcknowledged()` → `MnemonicSetup` | HUMAN NEEDED for behavioral confirmation |
| 9  | Ed25519 public key derived once on app start and cached in AppViewModel | VERIFIED | `AppViewModel.init{}` on `Dispatchers.IO` decrypts seed, calls `PktapBridge.derivePublicKey(seed.copyOf())`, stores in `_publicKeyBytes` and `_publicKeyHex: StateFlow<String>` |

**Score:** 9/9 truths verified (automated code-path verification); 6 behaviors require human confirmation on a live device

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `pktap-core/src/mnemonic.rs` | BIP-39 mnemonic from 16-byte entropy | VERIFIED | `mnemonic_from_entropy(&[u8; 32])` uses `bip39::Mnemonic::from_entropy(&seed[..16])`, 43 lines, substantive |
| `pktap-core/src/ffi.rs` | `derive_public_key` and `derive_mnemonic_from_seed` exports | VERIFIED | Both `#[uniffi::export]` functions present with length validation and `seed.zeroize()` before return |
| `android/app/.../keystore/KeystoreManager.kt` | AES key gen with StrongBox/TEE fallback, encrypt/decrypt | VERIFIED | 112 lines, `StrongBoxUnavailableException`, `Build.VERSION_CODES.P` guard, `AES/GCM/NoPadding`, `SEED_KEY_ALIAS` |
| `android/app/.../keystore/SeedRepository.kt` | Seed gen, persistence, retrieval, mnemonic_acknowledged flag | VERIFIED | 110 lines, `SharedPreferences`, `SecureRandom`, `mnemonic_acknowledged`, `KeyPermanentlyInvalidatedException` handling |
| `android/rust-bridge/.../PktapBridge.kt` | Bridge wrappers with ByteArray zeroing | VERIFIED | `deriveMnemonicFromSeed` and `derivePublicKey` wrappers present; `seedBytes.fill(0)` in 4 `finally` blocks |
| `android/app/.../navigation/AppNavigation.kt` | NavHost with MnemonicSetup and Main destinations | VERIFIED | `NavHost`, `composable<MnemonicSetup>`, `composable<Main>`, `@Serializable` routes, `popUpTo<MnemonicSetup> { inclusive = true }` |
| `android/app/.../onboarding/MnemonicScreen.kt` | 12-word display with checkbox and FLAG_SECURE | VERIFIED | `FLAG_SECURE` via `DisposableEffect`, `Checkbox`, words grid, Continue button with `enabled = checked && words.isNotEmpty()` |
| `android/app/.../onboarding/MnemonicViewModel.kt` | Mnemonic generation logic, acknowledgment state | VERIFIED | `PktapBridge.deriveMnemonicFromSeed(seed.copyOf())`, `seed.fill(0)` in finally, `acknowledge()` calls `setMnemonicAcknowledged()` |
| `android/app/.../AppViewModel.kt` | Cached Ed25519 public key | VERIFIED | `PktapBridge.derivePublicKey(seed.copyOf())`, `_publicKeyBytes` cached, `_publicKeyHex: StateFlow<String>` |
| `android/app/.../ui/main/MainScreen.kt` | Placeholder showing pubkey hex | VERIFIED | 78 lines, gets `AppViewModel` via factory, collects `publicKeyHex`, renders truncated hex with monospace font |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `KeystoreManager.kt` | Android Keystore | `KeyGenerator.getInstance(AES, AndroidKeyStore)` | WIRED | Line 59: `KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")` |
| `SeedRepository.kt` | `KeystoreManager.kt` | `keystoreManager.encrypt` | WIRED | Lines 62-63: `KeystoreManager.generateOrGetKey(...)`, `KeystoreManager.encrypt(seed, key)` |
| `PktapBridge.kt` | `pktap-core/src/ffi.rs` | UniFFI generated bindings | WIRED | Imports `ffiDerivePublicKey` and `ffiDeriveMnemonicFromSeed` from `uniffi.pktap_core`; both wrap the Rust exports |
| `MnemonicViewModel.kt` | `PktapBridge.kt` | `PktapBridge.deriveMnemonicFromSeed()` | WIRED | Line 46: `PktapBridge.deriveMnemonicFromSeed(seed.copyOf())` |
| `AppViewModel.kt` | `PktapBridge.kt` | `PktapBridge.derivePublicKey()` | WIRED | Line 47: `PktapBridge.derivePublicKey(seed.copyOf())` |
| `MainActivity.kt` | `AppNavigation.kt` | `setContent { AppNavigation(...) }` | WIRED | Line 37: `AppNavigation(startDestination = startDestination)` |
| `MainActivity.kt` | `SeedRepository.kt` | SeedRepository for start destination | WIRED | Lines 26-32: `SeedRepository(applicationContext)`, `hasSeed()`, `isMnemonicAcknowledged()` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `MnemonicScreen.kt` | `words: List<String>` | `MnemonicViewModel._words` ← `PktapBridge.deriveMnemonicFromSeed()` ← `SeedRepository.generateAndStoreSeed()` / `decryptSeed()` | Yes — `SecureRandom` seed → Rust BIP-39 via bip39 2.2.2 | FLOWING |
| `MainScreen.kt` | `publicKeyHex: String` | `AppViewModel._publicKeyHex` ← `PktapBridge.derivePublicKey()` ← `SeedRepository.decryptSeed()` | Yes — Keystore-decrypted seed → Rust ed25519_dalek `verifying_key()` | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED for all automated checks requiring a running Android emulator (live Keystore, live UI). The 76 Rust unit tests and 9 instrumented Android tests (per execution context) constitute the automated verification baseline. The human verification items below cover the remaining behavioral checks.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| KEY-01 | 04-01, 04-02 | App generates Ed25519 master keypair in Android Keystore (StrongBox/TEE), non-extractable | SATISFIED (via D-01 deviation) | Android Keystore does not support Ed25519 natively. Per D-01 (documented in 04-RESEARCH.md, user-approved): AES-256-GCM key in Keystore protects the 32-byte seed; Ed25519 derived on-demand in Rust via `SigningKey::from_bytes(&seed)`. The seed's non-extractability is enforced by the Keystore AES key. AppViewModel caches the derived public key. |
| KEY-02 | 04-01 | App generates AES-256-GCM key in Keystore for local data encryption | SATISFIED | `KeystoreManager.generateOrGetKey()` generates AES-256-GCM 256-bit key in `AndroidKeyStore` provider with `PURPOSE_ENCRYPT or PURPOSE_DECRYPT` |
| KEY-03 | 04-01 | App generates random 32-byte HKDF seed, encrypts with Keystore AES key, stores in SharedPreferences | SATISFIED (deviation documented) | `SeedRepository.generateAndStoreSeed()` uses `SecureRandom`, encrypted with Keystore AES key via `KeystoreManager.encrypt()`, stored in plain `SharedPreferences`. The requirement text named `EncryptedSharedPreferences` but the implementation uses direct Keystore + plain SharedPreferences — the user-approved deviation per 04-RESEARCH.md finding that `security-crypto` was deprecated June 2025. Security intent is equivalent. |
| KEY-04 | 04-01 (prep), 04-02 | App displays BIP-39 mnemonic at first launch so user can back up their seed | SATISFIED | `MnemonicScreen` shows 12-word grid from `MnemonicViewModel`, requires checkbox acknowledgment before `Continue` is enabled; FLAG_SECURE prevents screenshot |
| KEY-05 | 04-01, 04-02 | App falls back to TEE (non-StrongBox) Keystore on devices without StrongBox hardware | SATISFIED | `KeystoreManager.generateOrGetKey()` catches `StrongBoxUnavailableException` and retries without StrongBox; API 26-27 guard skips StrongBox entirely; 9 instrumented tests pass on emulator (TEE path) |

**Requirement KEY-06** (memory zeroing — assigned to Phase 1 per REQUIREMENTS.md traceability) is not scoped to Phase 4 but is implemented here as well: `seed.zeroize()` in all 4 Rust FFI functions that handle seed bytes, plus `seedBytes.fill(0)` in 4 `finally` blocks in `PktapBridge.kt`, plus `seed.fill(0)` in `MnemonicViewModel` and `AppViewModel` finally blocks.

**Orphaned requirement check:** REQUIREMENTS.md maps KEY-01 through KEY-05 to Phase 4. Both plans claim KEY-01, KEY-02, KEY-03, KEY-05 (04-01) and KEY-01, KEY-04, KEY-05 (04-02). All five Phase 4 requirements are claimed and verified. No orphaned requirements.

### Anti-Patterns Found

No anti-patterns detected across all 9 phase 4 files. No TODO/FIXME/PLACEHOLDER comments, no empty implementations, no return null stubs, no hardcoded empty data in rendering paths. The post-merge duplicate companion object compile error was fixed in commit `b6ab2e1` before execution context was recorded.

### Human Verification Required

Six items require a live emulator session. These replicate the checkpoint from 04-02-PLAN.md Task 2 (which was noted as pending in the SUMMARY). All automated code-path checks pass; the outstanding items are behavioral confirmation only.

**1. First-launch mnemonic display**

**Test:** Install the app fresh on the emulator, launch PKTap.
**Expected:** Mnemonic screen appears with exactly 12 numbered BIP-39 words in a 4x3 grid. Continue button is disabled. Checking the checkbox enables it. Tapping Continue navigates to the main screen showing "PKTap" and a truncated pubkey hex.
**Why human:** Compose rendering and touch interaction require a running app.

**2. Persistence after force-stop and relaunch**

**Test:** After completing the first-launch flow, force-stop the app (Settings > Apps > PKTap > Force Stop), then relaunch.
**Expected:** App goes directly to main screen (no mnemonic). The pubkey hex shown is identical to the previous session.
**Why human:** Requires live app lifecycle and SharedPreferences persistence across process death.

**3. Interrupted setup re-shows mnemonic (D-08)**

**Test:** Clear app data, launch PKTap, see the mnemonic screen, then force-stop WITHOUT tapping Continue. Relaunch.
**Expected:** Mnemonic screen appears again (not main screen) because `isMnemonicAcknowledged()` is false.
**Why human:** Requires live state manipulation.

**4. FLAG_SECURE screenshot prevention**

**Test:** While the mnemonic screen is visible, attempt to take a screenshot (power + volume-down or the Android screenshot shortcut).
**Expected:** Screenshot is blocked — the captured image is black.
**Why human:** System-level FLAG_SECURE behavior requires a live device/emulator session.

**5. No mnemonic words in logcat**

**Test:** `adb logcat | grep -i mnemonic` while going through the first-launch flow.
**Expected:** Only sees entries like "Mnemonic derived" or "Mnemonic acknowledged" — no BIP-39 word values in any log line.
**Why human:** Requires live logcat stream.

**6. TEE fallback logged on emulator**

**Test:** `adb logcat | grep KeystoreManager` during first launch on emulator.
**Expected:** Log line "Key generated via TEE" (not "StrongBox") — confirming the emulator exercises the TEE fallback path.
**Why human:** Requires live logcat.

### Gaps Summary

No automated gaps were found. All 9 must-have truths are verified at the code level (exists, substantive, wired, data flowing). The outstanding items are behavioral verifications requiring a live emulator session, which matches the pending Task 2 checkpoint from 04-02-PLAN.md. These are tracked as human verification items, not gaps.

The KEY-01 and KEY-03 deviations from the literal requirement text (Ed25519 not in Keystore directly; plain SharedPreferences instead of EncryptedSharedPreferences) are user-approved and documented in 04-RESEARCH.md. The security intent of both requirements is satisfied by the implemented approach.

---

_Verified: 2026-04-05T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
