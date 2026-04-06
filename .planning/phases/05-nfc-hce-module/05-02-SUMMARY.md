---
phase: 05-nfc-hce-module
plan: "02"
subsystem: nfc
tags: [nfc, reader-mode, hce, viewmodel, compose, navigation, android, kotlin]

# Dependency graph
requires:
  - phase: 05-nfc-hce-module
    plan: "01"
    provides: NfcPayloadBuilder, PktapHceService.cachedPayload, NfcExchangeFlow.peerKeyFlow, AID F0504B544150

provides:
  - NfcReader: enableReaderMode + IsoDep.transceive SELECT AID + EXCHANGE
  - NfcViewModel: PostTapState sealed class, peerKeyFlow collector, post-tap coroutine on Dispatchers.IO
  - PostTapScreen: Encrypting/Publishing/Done/Error/Queued status UI
  - PostTap navigation route with auto-navigate on peer key receipt
  - AppViewModel.cachedPayload wiring via NfcPayloadBuilder.buildNfcPayload (D-03)
  - MainActivity NFC reader lifecycle: enableReaderMode/onResume, disableReaderMode/onPause
  - MainScreen NFC-disabled Card with Settings.ACTION_NFC_SETTINGS action (D-06)

affects:
  - 06-dht-publish-resolve (inherits publish lambda obligation; NfcViewModel.publish is a no-op stub)

# Tech tracking
tech-stack:
  added:
    - kotlinx-coroutines-test (StandardTestDispatcher pattern for ViewModel coroutine testing)
  patterns:
    - ViewModel with injectable suspend lambdas: avoids Android context and Rust FFI in JVM tests
    - advanceUntilIdle() before tryEmit pattern: required with StandardTestDispatcher + replay=0 SharedFlow to ensure collector subscribes before emit
    - ioDispatcher injection: enables StandardTestDispatcher to control Dispatchers.IO-equivalent coroutines in tests
    - NfcAdapter.enableReaderMode (not enableForegroundDispatch — Pitfall 2 avoidance)
    - FLAG_READER_SKIP_NDEF_CHECK: prevents NDEF interference with IsoDep exchange (Pitfall 5)

key-files:
  created:
    - android/app/src/main/java/com/pktap/app/nfc/NfcReader.kt
    - android/app/src/main/java/com/pktap/app/nfc/NfcViewModel.kt
    - android/app/src/main/java/com/pktap/app/ui/posttap/PostTapScreen.kt
    - android/app/src/test/java/com/pktap/app/nfc/NfcViewModelTest.kt
  modified:
    - android/app/src/main/java/com/pktap/app/AppViewModel.kt
    - android/app/src/main/java/com/pktap/app/MainActivity.kt
    - android/app/src/main/java/com/pktap/app/navigation/AppNavigation.kt
    - android/app/src/main/java/com/pktap/app/ui/main/MainScreen.kt
    - android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt
    - android/gradle/libs.versions.toml
    - android/app/build.gradle.kts

key-decisions:
  - "ioDispatcher injected into NfcViewModel constructor (defaulting to Dispatchers.IO) to enable StandardTestDispatcher control in JVM tests — avoids real IO threads that advanceUntilIdle() cannot drain"
  - "advanceUntilIdle() before tryEmit() required in tests: StandardTestDispatcher does not run coroutines eagerly, so init collector must be scheduled and run before the SharedFlow emit"
  - "EXCHANGE_CLA and EXCHANGE_INS promoted to PktapHceService companion object so NfcReader can reference the protocol constants without duplicating magic bytes"
  - "publish lambda is an explicit no-op stub with comment: Phase 6 will wire DhtClient here — not silently absent"

# Metrics
duration: ~20min
completed: 2026-04-06
---

# Phase 5 Plan 02: NFC Reader Mode + Post-Tap Processing Summary

**NFC reader role (enableReaderMode + IsoDep.transceive), post-tap ViewModel coroutine on Dispatchers.IO, PostTapScreen status UI, and MainScreen NFC-disabled card — full bidirectional exchange wired, pending physical device verification**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-04-06T02:10:00Z
- **Completed:** 2026-04-06T02:30:00Z
- **Tasks:** 2 (of 3 — Task 3 is a physical device checkpoint)
- **Files modified:** 11

## Accomplishments

- NfcReader implements reader role: `enableReaderMode` with `FLAG_READER_NFC_A | FLAG_READER_SKIP_NDEF_CHECK`, SELECT AID F0504B544150, EXCHANGE APDU transceive, peer payload extraction and CRC validation
- NfcViewModel: `PostTapState` sealed class (Idle/Encrypting/Publishing/Done/Queued/Error), collects from `NfcExchangeFlow.peerKeyFlow`, runs post-tap crypto on injectable `ioDispatcher` (Dispatchers.IO in prod)
- AppViewModel now caches 36-byte NFC payload via `NfcPayloadBuilder.buildNfcPayload` after pubkey derivation (D-03)
- PostTapScreen shows all state machine states with CircularProgressIndicator, icons, and action buttons
- PostTap navigation route added with `LaunchedEffect` auto-navigation on peer key receipt
- MainActivity manages reader mode lifecycle: `enableReaderMode()` in `onResume`, `disableReaderMode()` in `onPause`
- MainScreen shows conditional `Card` when NFC is available but disabled; action launches `Settings.ACTION_NFC_SETTINGS`
- 9 JVM unit tests in NfcViewModelTest — all passing; covers state transitions, seed zeroing, error paths, invalid payload rejection

## Task Commits

1. **Task 1: NfcReader + NfcViewModel + AppViewModel payload cache (TDD)** — `4c712d0`
2. **Task 2: PostTapScreen + PostTap route + MainActivity lifecycle** — `a695d7d`

## Files Created/Modified

- `android/app/src/main/java/com/pktap/app/nfc/NfcReader.kt` — reader role with enableReaderMode + IsoDep exchange
- `android/app/src/main/java/com/pktap/app/nfc/NfcViewModel.kt` — PostTapState sealed class, post-tap coroutine, peerKeyFlow collector
- `android/app/src/main/java/com/pktap/app/ui/posttap/PostTapScreen.kt` — status progression UI
- `android/app/src/test/java/com/pktap/app/nfc/NfcViewModelTest.kt` — 9 JVM tests for state transitions
- `android/app/src/main/java/com/pktap/app/AppViewModel.kt` — added NfcPayloadBuilder.buildNfcPayload call (D-03)
- `android/app/src/main/java/com/pktap/app/MainActivity.kt` — NfcReader lifecycle management
- `android/app/src/main/java/com/pktap/app/navigation/AppNavigation.kt` — PostTap route, isNfcAvailable/isNfcEnabled params, LaunchedEffect auto-nav
- `android/app/src/main/java/com/pktap/app/ui/main/MainScreen.kt` — NFC-disabled Card with Settings link
- `android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt` — EXCHANGE_CLA/EXCHANGE_INS promoted to companion
- `android/gradle/libs.versions.toml` — added coroutines-test library
- `android/app/build.gradle.kts` — added testImplementation(libs.coroutines.test)

## Decisions Made

- **ioDispatcher injection:** `NfcViewModel` accepts `ioDispatcher: CoroutineDispatcher = Dispatchers.IO`. Tests pass `testDispatcher` so `advanceUntilIdle()` drains all coroutines including the IO work. Production uses real `Dispatchers.IO`.
- **advanceUntilIdle() before tryEmit():** With `StandardTestDispatcher` (non-eager) and `replay=0` SharedFlow, the init collector coroutine must be started before emitting. An initial `advanceUntilIdle()` schedules and runs the collector launch before the emit. This pattern is now documented in tech-stack.
- **EXCHANGE_CLA/EXCHANGE_INS in PktapHceService companion:** The constants were `private` on `PktapApduProtocol`. Promoted to `PktapHceService.companion` so `NfcReader` can reference them without duplicating the magic byte values.
- **publish lambda as explicit no-op stub:** The stub includes a KDoc comment: "Phase 6 will wire DhtClient here; DhtClient not yet exposed via UniFFI." The state machine still transitions through Publishing → Done with the stub, so the UI flow is fully testable in Phase 5.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] EXCHANGE_CLA/EXCHANGE_INS missing from PktapHceService companion**
- **Found during:** Task 1 compilation
- **Issue:** Plan spec referenced `PktapHceService.EXCHANGE_CLA` and `PktapHceService.EXCHANGE_INS`, but these were `private const` on `PktapApduProtocol` (not accessible from `NfcReader`)
- **Fix:** Added `EXCHANGE_CLA` and `EXCHANGE_INS` as `const` members of `PktapHceService.companion`
- **Files modified:** `android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt`
- **Commit:** included in `4c712d0`

**2. [Rule 1 - Bug] NfcViewModel test coroutine dispatcher isolation**
- **Found during:** Task 1 TDD GREEN phase — 8 tests failing with state staying Idle
- **Issue:** `StandardTestDispatcher` does not run coroutines eagerly. The ViewModel's `init` collector (on `viewModelScope`) had not yet subscribed when `tryEmit` was called. Additionally, `launchPostTapCrypto` used real `Dispatchers.IO`, which `advanceUntilIdle()` cannot drain.
- **Fix:** (a) Added `ioDispatcher: CoroutineDispatcher = Dispatchers.IO` parameter to `NfcViewModel`; (b) Tests pass `testDispatcher` as `ioDispatcher`; (c) All tests call `advanceUntilIdle()` before `tryEmit()` to let the init collector start
- **Files modified:** `NfcViewModel.kt`, `NfcViewModelTest.kt`
- **Commit:** `4c712d0`

## Known Stubs

- **`NfcViewModel.publish` lambda** — explicitly a no-op stub for Phase 5: `// Phase 6 will wire DhtClient here; DhtClient not yet exposed via UniFFI.` The state machine transitions Publishing → Done with the stub. Phase 6 inherits the wiring obligation. This stub is intentional and documented; it does not prevent the plan's Phase 5 goal (bidirectional key exchange + status UI) from being achieved.

## Threat Flags

No new threat surface beyond what is documented in the plan's threat model. All T-05-06 through T-05-10 mitigations are implemented:
- T-05-06 (Tampering): NfcReader validates response size == 38 and SW == 90 00; NfcPayloadBuilder.validateNfcPayload checks CRC before trusting
- T-05-08 (Information Disclosure): seed decrypted in IO coroutine, passed as `copyOf()` to PktapBridge, original zeroed in `finally`; seed never stored as ViewModel field

## Self-Check

- [x] `android/app/src/main/java/com/pktap/app/nfc/NfcReader.kt` — exists
- [x] `android/app/src/main/java/com/pktap/app/nfc/NfcViewModel.kt` — exists
- [x] `android/app/src/main/java/com/pktap/app/ui/posttap/PostTapScreen.kt` — exists
- [x] `android/app/src/test/java/com/pktap/app/nfc/NfcViewModelTest.kt` — exists
- [x] Commit `4c712d0` — Task 1
- [x] Commit `a695d7d` — Task 2

---
*Phase: 05-nfc-hce-module*
*Completed (Tasks 1-2): 2026-04-06*
*Task 3 (physical device checkpoint): awaiting human verification*
