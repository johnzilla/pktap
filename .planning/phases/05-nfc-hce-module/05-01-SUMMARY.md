---
phase: 05-nfc-hce-module
plan: "01"
subsystem: nfc
tags: [nfc, hce, apdu, crc16, android, kotlin, junit]

# Dependency graph
requires:
  - phase: 04-keystore-seed
    provides: AppViewModel.publicKeyBytes — Ed25519 pubkey cached for NFC payload construction
provides:
  - NfcPayloadBuilder: 36-byte NFC wire format with CRC-16/CCITT-FALSE
  - PktapHceService: HostApduService with SELECT AID + EXCHANGE APDU handling, zero-compute processCommandApdu
  - PktapApduProtocol: pure testable APDU logic object
  - NfcExchangeFlow: SharedFlow singleton for peer key delivery from HCE service to ViewModel
  - AID F0504B544150 registered in apduservice.xml with category=other
  - JVM unit test infrastructure (JUnit 4, src/test/ directory)
affects:
  - 05-02-nfc-reader-mode (wires reader side: IsoDep.transceive, PktapHceService.cachedPayload assignment)
  - 06-dht-publish-resolve (receives peer key via NfcExchangeFlow.peerKeyFlow)

# Tech tracking
tech-stack:
  added:
    - junit 4.13.2 (JVM unit test dependency)
  patterns:
    - TDD red-green for Android HCE logic: extract pure protocol object (PktapApduProtocol) to avoid HostApduService context in tests
    - Blanket try/catch wrapping entire processCommandApdu body (HCE permanent-death pitfall mitigation)
    - Pre-cached payload pattern: build ByteArray once on app start, return directly in processCommandApdu

key-files:
  created:
    - android/app/src/main/java/com/pktap/app/nfc/NfcPayloadBuilder.kt
    - android/app/src/main/java/com/pktap/app/nfc/NfcExchangeFlow.kt
    - android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt
    - android/app/src/main/res/xml/apduservice.xml
    - android/app/src/main/res/values/strings.xml
    - android/app/src/test/java/com/pktap/app/nfc/NfcPayloadBuilderTest.kt
    - android/app/src/test/java/com/pktap/app/nfc/PktapHceServiceTest.kt
  modified:
    - android/gradle/libs.versions.toml
    - android/app/build.gradle.kts
    - android/app/src/main/AndroidManifest.xml

key-decisions:
  - "CRC-16 implemented inline in Kotlin (6 lines, no dependency) rather than via Rust FFI — trivial computation, no FFI overhead justified"
  - "PktapApduProtocol extracted as pure object for JVM testability — avoids HostApduService Android context requirement in unit tests"
  - "android:required=false on NFC features — ensures app installs on NFC-less devices (QR fallback Phase 7)"

patterns-established:
  - "HCE testability pattern: extract APDU logic into pure PktapApduProtocol object, delegate from HostApduService; tests exercise protocol directly"
  - "Blanket try/catch in processCommandApdu: permanent HCE death prevention; every unhandled exception returns SW_UNKNOWN"
  - "NFC wire format: version(1) + flags(1) + pubkey(32) + CRC-16(2) = 36 bytes; validated by crc16Ccitt with CCITT-FALSE parameters"

requirements-completed: [NFC-01, NFC-02, NFC-03, NFC-04, NFC-05]

# Metrics
duration: 8min
completed: 2026-04-06
---

# Phase 5 Plan 01: NFC HCE Module Foundation Summary

**HCE HostApduService with 36-byte CRC-validated payload builder, AID F0504B544150 registration, and SharedFlow peer key delivery — zero FFI calls in processCommandApdu**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-04-06T01:51:00Z
- **Completed:** 2026-04-06T01:59:33Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- NfcPayloadBuilder produces valid 36-byte NFC payloads (version=0x01, flags=0x00, 32-byte pubkey, CRC-16/CCITT-FALSE); CCITT check value 0x29B1 verified
- PktapHceService with pre-cached payload, blanket try/catch on processCommandApdu, and zero FFI/crypto/IO per NFC-03
- NfcExchangeFlow SharedFlow singleton (replay=0, extraBufferCapacity=1, DROP_OLDEST) delivers received peer keys from HCE service to ViewModel
- AID F0504B544150 registered in apduservice.xml with category="other"; NFC permission and BIND_NFC_SERVICE in manifest
- JVM unit test infrastructure established: 20 tests across two test classes, all passing

## Task Commits

Each task was committed atomically:

1. **Task 1: JVM test infrastructure + NfcPayloadBuilder (TDD)** - `138b4f1` (feat)
2. **Task 2: NfcExchangeFlow, PktapHceService, AID registration, manifest** - `7e0c430` (feat)

## Files Created/Modified

- `android/app/src/main/java/com/pktap/app/nfc/NfcPayloadBuilder.kt` - 36-byte wire format builder with inline CRC-16/CCITT-FALSE
- `android/app/src/main/java/com/pktap/app/nfc/NfcExchangeFlow.kt` - MutableSharedFlow singleton for peer key delivery (D-04)
- `android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt` - HostApduService + PktapApduProtocol + ApduResult; zero FFI in processCommandApdu
- `android/app/src/main/res/xml/apduservice.xml` - AID F0504B544150, category=other (D-05, NFC-05)
- `android/app/src/main/res/values/strings.xml` - NFC service description strings
- `android/app/src/main/AndroidManifest.xml` - NFC permission, HCE feature flags, PktapHceService registration
- `android/app/src/test/java/com/pktap/app/nfc/NfcPayloadBuilderTest.kt` - 10 tests: payload structure, CRC, validation, edge cases
- `android/app/src/test/java/com/pktap/app/nfc/PktapHceServiceTest.kt` - 10 tests: SELECT AID, EXCHANGE, null payload, malformed APDUs, NFC-03 guard
- `android/gradle/libs.versions.toml` - Added junit 4.13.2
- `android/app/build.gradle.kts` - Added testImplementation(libs.junit)

## Decisions Made

- **CRC-16 in Kotlin, not Rust FFI:** CRC-16/CCITT-FALSE is 6 lines of Kotlin; no FFI round-trip overhead justified for this trivial computation. The `crc` Rust crate remains available for future use.
- **PktapApduProtocol as pure object:** HostApduService requires Android context making it untestable in JVM unit tests. Extracting all APDU logic to a pure `PktapApduProtocol` object with `handleApdu(commandApdu, cachedPayload, selectAidReceived): ApduResult` enables full protocol testing without mocking.
- **android:required="false" for NFC features:** Ensures the app installs on NFC-less devices; QR fallback (Phase 7) covers those devices.

## Deviations from Plan

None — plan executed exactly as written. The TDD sequence (RED → GREEN) was followed for both tasks.

## Issues Encountered

The NFC-03 code-level assertion test (`PktapHceService processCommandApdu contains no PktapBridge references`) initially failed because:
1. The source file contained "PktapBridge" in a KDoc comment line — the test stripped only `import` lines, not comment lines.
2. Fix: Updated the test to also strip lines starting with `//`, `*`, and `/*` before checking for the string.
3. Additionally updated the comment in PktapHceService.kt itself to say "NO FFI bridge calls" instead of "NO calls to PktapBridge" to satisfy the stricter grep-based acceptance criterion.

## Known Stubs

None — no hardcoded placeholder values or unconnected data flows. `PktapHceService.cachedPayload` is intentionally nullable (`@Volatile var cachedPayload: ByteArray? = null`) — it is set by AppViewModel in Plan 02.

## Threat Flags

No new threat surface beyond what is documented in the plan's threat model. All T-05-01 through T-05-05 mitigations are implemented:
- T-05-01 (Tampering): APDU size validated >= 41 before extracting data; exactly 36 bytes copied by fixed offset
- T-05-02 (DoS): Blanket try/catch in processCommandApdu; SW_UNKNOWN returned on any exception
- T-05-03/04/05: Accepted per threat register

## Next Phase Readiness

Plan 02 (NFC reader mode) can now:
- Import and set `PktapHceService.cachedPayload` from AppViewModel.publicKeyBytes
- Import `NfcExchangeFlow.peerKeyFlow` to collect received peer keys in ViewModel
- Implement `NfcAdapter.enableReaderMode()` + `IsoDep.transceive()` sending the SELECT AID then EXCHANGE APDU

No blockers. Physical device testing (Samsung + Xiaomi) is still required per the Phase 5 blocker note in STATE.md — that is a hardware integration concern, not a code blocker.

## Self-Check

- [x] `android/app/src/main/java/com/pktap/app/nfc/NfcPayloadBuilder.kt` — exists
- [x] `android/app/src/main/java/com/pktap/app/nfc/NfcExchangeFlow.kt` — exists
- [x] `android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt` — exists
- [x] `android/app/src/main/res/xml/apduservice.xml` — exists
- [x] `android/app/src/main/res/values/strings.xml` — exists
- [x] `android/app/src/main/AndroidManifest.xml` — updated
- [x] Commit `138b4f1` — Task 1
- [x] Commit `7e0c430` — Task 2

---
*Phase: 05-nfc-hce-module*
*Completed: 2026-04-06*
