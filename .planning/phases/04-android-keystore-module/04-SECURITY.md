---
phase: 04-android-keystore-module
auditor: gsd-secure-phase
asvs_level: 1
block_on: high
audited_date: 2026-04-05
threats_total: 13
threats_closed: 13
threats_open: 0
result: SECURED
---

# Phase 04 Security Audit

**Phase:** 04 — Android Keystore Module
**Plans audited:** 04-01 (Keystore Infrastructure + Rust FFI), 04-02 (First-Launch UI)
**Threats Closed:** 13/13
**ASVS Level:** 1

---

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-04-01 | Information Disclosure | mitigate | CLOSED | `PktapBridge.kt:52,85,125,145` — `seedBytes.fill(0)` in finally blocks for all four seed-accepting wrappers; `AppViewModel.kt:49`, `MnemonicViewModel.kt:49` — `seed.fill(0)` in finally |
| T-04-02 | Information Disclosure | mitigate | CLOSED | `KeystoreManager.kt:42,46,52` logs operation names only ("Key generated via StrongBox/TEE"); `SeedRepository.kt:68` logs "Seed generated and stored"; no ByteArray or secret values in any log call |
| T-04-03 | Tampering | mitigate | CLOSED | `KeystoreManager.kt:90` — `cipher.init(Cipher.ENCRYPT_MODE, key)` with no IV argument; Keystore auto-generates a fresh 12-byte GCM IV on every encrypt call |
| T-04-04 | Denial of Service | mitigate | CLOSED | `SeedRepository.kt:87-98` — `KeyPermanentlyInvalidatedException` caught in both `generateOrGetKey` and `decrypt` paths; prefs cleared via `prefs.edit().clear().apply()`, `SeedInvalidatedException` thrown to signal re-setup |
| T-04-05 | Elevation of Privilege | accept | CLOSED | See accepted risks log below |
| T-04-06 | Information Disclosure | mitigate | CLOSED | `SeedRepository.kt` — no seed field on class; `generateAndStoreSeed` and `decryptSeed` return local `ByteArray` only; `AppViewModel.kt:26-35` comment confirms "Seed is NEVER assigned to a ViewModel field" |
| T-04-07 | Tampering | mitigate | CLOSED | `KeystoreManager.kt:39` — outer guard `Build.VERSION.SDK_INT >= Build.VERSION_CODES.P`; `KeystoreManager.kt:71` — inner guard in `createKey` before calling `setIsStrongBoxBacked`; API 26-27 path skips StrongBox entirely |
| T-04-08 | Information Disclosure | mitigate | CLOSED | `MnemonicScreen.kt:53-58` — `DisposableEffect(Unit)` calls `window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)` on enter and `window?.clearFlags(...)` on dispose |
| T-04-09 | Information Disclosure | mitigate | CLOSED | `MnemonicViewModel.kt:51` — logs "Mnemonic derived" only; no word list values appear in any log call in `MnemonicViewModel.kt`, `MnemonicScreen.kt`, or `PktapBridge.kt` |
| T-04-10 | Information Disclosure | accept | CLOSED | See accepted risks log below |
| T-04-11 | Tampering | accept | CLOSED | See accepted risks log below |
| T-04-12 | Spoofing | accept | CLOSED | See accepted risks log below |
| T-04-13 | Information Disclosure | mitigate | CLOSED | `MnemonicViewModel.kt:38-49` — seed confined to IO coroutine local scope; `seed.copyOf()` passed to `PktapBridge.deriveMnemonicFromSeed` (bridge zeros its copy); original `seed.fill(0)` in finally; seed never assigned to any ViewModel property |

---

## Accepted Risks Log

| Threat ID | Category | Risk Statement | Rationale | Owner |
|-----------|----------|----------------|-----------|-------|
| T-04-05 | Elevation of Privilege | StrongBox unavailable on some devices; key falls back to TEE (Trusted Execution Environment) | Both StrongBox and TEE provide non-extractable key storage. TEE is the minimum acceptable security bar for this threat model. The fallback is logged (`"Key generated via TEE"`) and is intentional per D-02. No user-visible degradation occurs. | Architecture (D-02) |
| T-04-10 | Information Disclosure | BIP-39 mnemonic words remain in `MnemonicViewModel._words: StateFlow<List<String>>` until the ViewModel is cleared by the Android lifecycle | The 12 words are the user's own backup phrase — they are by design shown to the user and already exist in the user's memory. StateFlow is GC-eligible after ViewModel cleared. Clearing the StateFlow on navigation is possible but provides no meaningful security improvement given the words are displayed plaintext on screen. | Architecture |
| T-04-11 | Tampering | `mnemonic_acknowledged` flag stored in `MODE_PRIVATE` SharedPreferences can be set by a rooted device without the user viewing words | SharedPreferences is app-private on non-rooted devices. A rooted user bypassing this flag is only degrading their own backup security — no other user's security is affected. This is a self-harm scenario, not an attack on others. | Architecture |
| T-04-12 | Spoofing | A rooted device could inject a fake seed into SharedPreferences | SharedPreferences is `MODE_PRIVATE`. The Keystore AES key used to encrypt the seed is non-extractable regardless; an attacker replacing the encrypted blob would produce a decryption failure or a different identity. Rooted device risk is accepted across all Android local storage. | Architecture |

---

## Unregistered Threat Flags

None. Both 04-01-SUMMARY.md and 04-02-SUMMARY.md reported no new threat flags outside the planned threat register.

---

## Files Audited

| File | Role |
|------|------|
| `pktap-core/src/ffi.rs` | Rust FFI exports — `derive_mnemonic_from_seed`, `derive_public_key`; seed zeroize calls |
| `pktap-core/src/mnemonic.rs` | BIP-39 mnemonic generation from entropy |
| `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt` | Kotlin FFI bridge — ByteArray zeroing in finally blocks |
| `android/app/src/main/java/com/pktap/app/keystore/KeystoreManager.kt` | AES-256-GCM Keystore key management — IV auto-gen, StrongBox/TEE fallback, API guard |
| `android/app/src/main/java/com/pktap/app/keystore/SeedRepository.kt` | Seed lifecycle — no class-level secret field, KeyPermanentlyInvalidatedException handling |
| `android/app/src/main/java/com/pktap/app/AppViewModel.kt` | Public key caching — seed zeroed in finally, seed never stored as field |
| `android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicViewModel.kt` | Mnemonic derivation — seed local to coroutine, no word values logged |
| `android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicScreen.kt` | FLAG_SECURE lifecycle management via DisposableEffect |
