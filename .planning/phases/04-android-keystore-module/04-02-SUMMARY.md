---
phase: 04-android-keystore-module
plan: "02"
subsystem: ui-onboarding
tags: [android, compose, navigation, bip39, keystore, viewmodel, flag-secure]
dependency_graph:
  requires:
    - 04-01 (KeystoreManager, SeedRepository, deriveMnemonicFromSeed, derivePublicKey FFI)
  provides:
    - First-launch navigation graph (MnemonicSetup → Main)
    - MnemonicScreen with FLAG_SECURE screenshot prevention and checkbox acknowledgment
    - AppViewModel with cached Ed25519 public key (for NFC/DHT in Phase 5+)
  affects:
    - 05-xx (NFC phase reads publicKeyBytes from AppViewModel cache)
tech_stack:
  added:
    - androidx.navigation:navigation-compose 2.8.5 (type-safe Compose NavHost)
    - androidx.lifecycle:lifecycle-viewmodel-compose 2.8.7 (viewModel() in composables)
    - org.jetbrains.kotlinx:kotlinx-serialization-json 1.7.3 (Serializable route objects)
    - kotlin.plugin.serialization (Gradle plugin for @Serializable)
  patterns:
    - Type-safe Navigation 2.8 routes using @Serializable objects
    - ViewModel companion object factory (viewModelFactory { initializer {} }) — deferred Hilt
    - DisposableEffect for FLAG_SECURE lifecycle management on mnemonic screen
    - Seed decrypted in IO coroutine, local copy zeroed in finally (T-04-13, D-06)
    - Start destination determined synchronously in onCreate before setContent (D-07, D-08)
key_files:
  created:
    - android/app/src/main/java/com/pktap/app/navigation/AppNavigation.kt
    - android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicScreen.kt
    - android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicViewModel.kt
    - android/app/src/main/java/com/pktap/app/ui/main/MainScreen.kt
    - android/app/src/main/java/com/pktap/app/AppViewModel.kt
  modified:
    - android/gradle/libs.versions.toml (added navigation, viewmodel-compose, serialization deps)
    - android/app/build.gradle.kts (added kotlin.serialization plugin + 3 deps)
    - android/app/src/main/java/com/pktap/app/MainActivity.kt (rewired to AppNavigation)
decisions:
  - "Deferred Hilt DI — used companion object viewModelFactory pattern instead; avoids Hilt setup overhead for Phase 4 MVP"
  - "Start destination determined synchronously in onCreate (SharedPreferences boolean reads are fast) — no splash screen or loading state needed"
  - "MnemonicViewModel passes seed.copyOf() to PktapBridge.deriveMnemonicFromSeed — PktapBridge zeros its copy, original zeroed in finally; double-zero is safe and belt-and-suspenders"
  - "MainScreen is a placeholder — will be replaced/extended in Phase 6 NFC exchange UI"
metrics:
  duration: "~25 minutes (Task 1 only — checkpoint at Task 2)"
  completed_date: "2026-04-05"
  tasks_completed: 1
  tasks_total: 2
  files_created: 5
  files_modified: 3
---

# Phase 04 Plan 02: First-Launch UI Flow Summary (Partial — Task 2 Pending)

**One-liner:** Compose Navigation 2.8 type-safe graph wires MnemonicSetup→Main; MnemonicScreen shows 12-word BIP-39 phrase with FLAG_SECURE and checkbox gate; AppViewModel caches Ed25519 pubkey derived via Rust FFI.

**Status: Task 1 complete, awaiting Task 2 (emulator verification checkpoint).**

## What Was Built

### Task 1: Version catalog + deps + Navigation + ViewModels + Screens + MainActivity wiring

**Version catalog + build config:**
- Added `navigationCompose = "2.8.5"`, `lifecycleViewmodelCompose = "2.8.7"`, `kotlinxSerializationJson = "1.7.3"` to `libs.versions.toml`
- Added corresponding library entries and `kotlin-serialization` plugin entry
- Updated `app/build.gradle.kts`: added `kotlin.serialization` plugin + 3 implementation deps

**AppNavigation.kt:**
- `@Serializable object MnemonicSetup` and `@Serializable object Main` as type-safe routes
- `AppNavigation(startDestination: Any)` composable with `NavHost` — `composable<MnemonicSetup>` and `composable<Main>` entries
- Navigate from MnemonicSetup to Main with `popUpTo<MnemonicSetup> { inclusive = true }` (cleans back stack)

**MnemonicViewModel.kt:**
- Derives mnemonic in `init {}` on `Dispatchers.IO`: generates seed if absent, decrypts if present, calls `PktapBridge.deriveMnemonicFromSeed(seed.copyOf())`, zeros original in finally (T-04-13)
- `words: StateFlow<List<String>>` and `isLoading: StateFlow<Boolean>` for UI
- `acknowledge()` calls `seedRepository.setMnemonicAcknowledged()`
- Never logs word values (T-04-09)
- Companion `factory(context)` using `viewModelFactory { initializer {} }`

**MnemonicScreen.kt:**
- `DisposableEffect(Unit)` adds/removes `FLAG_SECURE` on activity window (T-04-08)
- Loading state: `CircularProgressIndicator` centered
- Words displayed in 4 rows × 3 columns using `Card` chips with 1-indexed labels
- `Checkbox` + label "I have written down these words" (D-04)
- "Continue" `Button` enabled only when checkbox checked AND words loaded

**AppViewModel.kt:**
- `init {}` on `Dispatchers.IO`: decrypts seed, calls `PktapBridge.derivePublicKey(seed.copyOf())`, zeros original in finally (D-06)
- Caches `_publicKeyBytes: ByteArray?` and `_publicKeyHex: StateFlow<String>` (hex)
- Public key is not secret — caching is safe per D-06
- Companion `factory(context)` pattern

**MainScreen.kt:**
- Placeholder screen: "PKTap" title, truncated pubkey hex (first 8 + last 8 chars), "Ready for NFC exchange"
- Gets `AppViewModel` via factory

**MainActivity.kt:**
- Rewired `onCreate`: creates `SeedRepository`, determines `startDestination` synchronously, calls `setContent { MaterialTheme { AppNavigation(startDestination) } }`
- Decision logic: no seed → `MnemonicSetup`; seed + not acknowledged → `MnemonicSetup` (D-08); else → `Main` (D-07)

**Verification:** `JAVA_HOME=/usr/lib/jvm/java-21-openjdk ANDROID_NDK_HOME=/home/john/Android/Sdk/ndk/27.2.12479018 ./gradlew :app:assembleDebug` — BUILD SUCCESSFUL.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

- `MainScreen.kt` is intentionally a placeholder. The pubkey hex IS wired (not stubbed — `AppViewModel` derives it from the real seed via Rust FFI). The "Ready for NFC exchange" subtitle is display-only placeholder text for Phase 6.

## Threat Flags

None. All threat mitigations from the plan's threat model were implemented:
- T-04-08: FLAG_SECURE via DisposableEffect in MnemonicScreen
- T-04-09: No word values logged in MnemonicViewModel
- T-04-13: Seed exists only in IO coroutine local scope, zeroed in finally

## Self-Check: PASSED

Files exist:
- android/app/src/main/java/com/pktap/app/navigation/AppNavigation.kt — FOUND
- android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicScreen.kt — FOUND
- android/app/src/main/java/com/pktap/app/ui/onboarding/MnemonicViewModel.kt — FOUND
- android/app/src/main/java/com/pktap/app/ui/main/MainScreen.kt — FOUND
- android/app/src/main/java/com/pktap/app/AppViewModel.kt — FOUND

Commits:
- eb53caf — feat(04-02): add first-launch flow — NavGraph, MnemonicScreen, AppViewModel
