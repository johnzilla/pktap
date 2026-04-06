---
phase: 05-nfc-hce-module
verified: 2026-04-05T00:00:00Z
status: human_needed
score: 6/8 must-haves verified
gaps: []
deferred:
  - truth: "Two physical devices (including at least one non-Pixel) successfully exchange 36-byte public key payloads via NFC tap — both apps receive the peer's key"
    addressed_in: "Phase 6"
    evidence: "Phase 6 success criteria: 'After a two-device NFC tap, both devices show a contact preview with the other person's chosen fields within 10 seconds'"
human_verification:
  - test: "Two-device NFC tap exchange"
    expected: "Both devices receive and display the peer's 36-byte Ed25519 pubkey payload after tapping phones; peerPubKeyHex appears in NfcViewModel on both sides"
    why_human: "Requires two physical Android devices with NFC hardware; emulator NFC has known quirks per research notes"
  - test: "Samsung One UI AID routing — no manual configuration"
    expected: "After tapping a Samsung Galaxy device running PKTap against any other NFC device running PKTap, the HCE service is invoked without requiring any manual AID allowlisting or user approval dialog on the Samsung side"
    why_human: "Requires a physical Samsung device; AID category=other routing behavior is OEM-specific and cannot be verified from code alone (NFC-05)"
---

# Phase 5: NFC HCE Module Verification Report

**Phase Goal:** Two phones running the app can exchange their 32-byte Ed25519 public keys via NFC tap using a single APDU round-trip — the APDU handler returns within 300ms, crypto runs in a post-tap coroutine, and the flow works on Samsung One UI and Xiaomi MIUI in addition to Pixel
**Verified:** 2026-04-05
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | A 32-byte Ed25519 pubkey produces a valid 36-byte NFC payload with correct version, flags, and CRC-16 | VERIFIED | `NfcPayloadBuilder.kt` implements full wire format: `buf[0]=VERSION(0x01)`, `buf[1]=FLAGS(0x00)`, `pubKeyBytes.copyInto(buf, 2)`, CRC-16/CCITT-FALSE appended at [34..35]. CCITT check value 0x29B1 tested in `NfcPayloadBuilderTest.kt`. |
| 2 | The HCE service responds to SELECT AID with SW 90 00, and to EXCHANGE command with its cached 36-byte payload + SW 90 00 | VERIFIED | `PktapApduProtocol.handleApdu` returns `SW_OK` on `CLA=0x00, INS=0xA4`; returns `cachedPayload + SW_OK` on `CLA=0x90, INS=0x01` after SELECT AID. Verified by `PktapHceServiceTest.kt` tests. |
| 3 | Malformed APDUs return SW_UNKNOWN without any exception escaping processCommandApdu | VERIFIED | Blanket `try/catch(e: Exception)` in `PktapHceService.processCommandApdu` returns `SW_UNKNOWN` on any exception. `PktapApduProtocol.handleApdu` returns `SW_UNKNOWN` on apdu size < 4. Tests: empty, 1-byte, truncated EXCHANGE all verified. |
| 4 | The HCE service emits received peer payload via SharedFlow | VERIFIED | `processCommandApdu` calls `result.peerPayload?.let { NfcExchangeFlow.peerKeyFlow.tryEmit(it) }`. Test `EXCHANGE emits received peer payload` confirms the peerPayload is set in `ApduResult` and emitted. |
| 5 | AID F0504B544150 is registered in apduservice.xml with category=other | VERIFIED | `android/app/src/main/res/xml/apduservice.xml` contains `<aid-filter android:name="F0504B544150" />` inside `<aid-group android:category="other">`. |
| 6 | Post-tap crypto runs in a ViewModel coroutine on Dispatchers.IO, not in processCommandApdu | VERIFIED | `NfcViewModel.launchPostTapCrypto` launches `viewModelScope.launch(ioDispatcher)` with default `ioDispatcher = Dispatchers.IO`. `processCommandApdu` only calls `tryEmit` — zero crypto. `NfcViewModelTest` verifies the full state machine runs on injected test dispatcher. |
| 7 | Two physical devices (including at least one non-Pixel) successfully exchange keys via NFC tap | DEFERRED | Cannot verify without two physical NFC devices. Addressed in Phase 6 (end-to-end tap flow). |
| 8 | SELECT AID routing works on Samsung without manual AID configuration (Samsung/Xiaomi OEM compatibility) | NEEDS HUMAN | AID registered with `category=other` in `apduservice.xml` per research recommendation for Samsung routing. Actual OEM routing behavior requires physical Samsung device to verify. |

**Score:** 6/8 truths verified (2 require hardware or human verification)

### Deferred Items

Items not yet met but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Two physical devices exchange 36-byte public key payloads via NFC tap | Phase 6 | Phase 6 success criteria: "After a two-device NFC tap, both devices show a contact preview with the other person's chosen fields within 10 seconds" |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `android/app/src/main/java/com/pktap/app/nfc/NfcPayloadBuilder.kt` | 36-byte NFC payload construction with CRC-16 | VERIFIED | 80 lines; exports `buildNfcPayload`, `validateNfcPayload`, `extractPubKey`, `crc16Ccitt` |
| `android/app/src/main/java/com/pktap/app/nfc/PktapHceService.kt` | HCE HostApduService with pre-cached payload | VERIFIED | 149 lines; `PktapHceService` + `PktapApduProtocol` + `ApduResult`; zero FFI in `processCommandApdu` confirmed |
| `android/app/src/main/java/com/pktap/app/nfc/NfcExchangeFlow.kt` | Singleton SharedFlow for peer key delivery | VERIFIED | 21 lines; `MutableSharedFlow(replay=0, extraBufferCapacity=1, DROP_OLDEST)` |
| `android/app/src/main/res/xml/apduservice.xml` | AID registration for NFC routing | VERIFIED | Contains `F0504B544150` with `category="other"` |
| `android/app/src/main/java/com/pktap/app/nfc/NfcReader.kt` | Reader role: enableReaderMode, SELECT AID, EXCHANGE transceive | VERIFIED | 118 lines; `enableReaderMode` with `FLAG_READER_NFC_A | FLAG_READER_SKIP_NDEF_CHECK`; performs SELECT AID then EXCHANGE via `IsoDep.transceive`; emits to `NfcExchangeFlow.peerKeyFlow` |
| `android/app/src/main/java/com/pktap/app/nfc/NfcViewModel.kt` | NFC ViewModel with peer key collection and post-tap coroutine | VERIFIED | 111 lines; `PostTapState` sealed class; collects `peerKeyFlow`; `launchPostTapCrypto` on `ioDispatcher`; inject-friendly for JVM tests |
| `android/app/src/main/java/com/pktap/app/ui/posttap/PostTapScreen.kt` | Post-tap status UI | VERIFIED | 142 lines; renders all 6 states (Idle/Encrypting/Publishing/Done/Queued/Error); not a stub |
| `android/app/src/test/java/com/pktap/app/nfc/NfcPayloadBuilderTest.kt` | JVM unit tests for payload construction and CRC | VERIFIED | 12 tests covering all payload builder behaviors including CCITT check value 0x29B1 |
| `android/app/src/test/java/com/pktap/app/nfc/PktapHceServiceTest.kt` | JVM unit tests for APDU handling | VERIFIED | 10 tests covering SELECT AID, EXCHANGE, null payload, malformed APDUs, NFC-03 code-level assertion |
| `android/app/src/test/java/com/pktap/app/nfc/NfcViewModelTest.kt` | JVM unit tests for post-tap coroutine and state transitions | VERIFIED | 9 tests covering state machine, seed zeroing, error paths, invalid payload rejection |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `NfcExchangeFlow.peerKeyFlow` | `NfcViewModel` | `viewModelScope.launch` collects SharedFlow | WIRED | Line 61 in `NfcViewModel.kt`: `NfcExchangeFlow.peerKeyFlow.collect { peerRawPayload ->` |
| `NfcViewModel.launchPostTapCrypto` | `PktapBridge.ecdhAndEncrypt` | `Dispatchers.IO` coroutine | WIRED | `ioDispatcher = Dispatchers.IO` default; `ecdhEncrypt` lambda defaults to `PktapBridge.ecdhAndEncrypt`; confirmed by injection pattern in tests |
| `NfcReader.onTagDiscovered` | `IsoDep.transceive` | SELECT AID then EXCHANGE command | WIRED | `performExchange` calls `isoDep.transceive(buildSelectAid())` then `isoDep.transceive(exchangeApdu)` |
| `AppNavigation` | `PostTapScreen` | Compose Navigation route | WIRED | `composable<PostTap> { PostTapScreen(...) }` at line 72 of `AppNavigation.kt`; `LaunchedEffect(peerHex)` auto-navigates on peer key receipt |
| `NfcReader.isNfcEnabled` | `MainScreen` NFC disabled Card | boolean state passed from `MainActivity` | WIRED | `MainActivity` passes `isNfcEnabled = nfcReader.isNfcEnabled()` to `AppNavigation`; `MainScreen` shows `Card` when `isNfcAvailable && !isNfcEnabled` |
| `MainScreen` NFC disabled Card action | `Settings.ACTION_NFC_SETTINGS` | Intent launched from card button | WIRED | `context.startActivity(Intent(Settings.ACTION_NFC_SETTINGS))` at line 80 of `MainScreen.kt` |
| `AppViewModel` pubkey derivation | `PktapHceService.cachedPayload` | `NfcPayloadBuilder.buildNfcPayload` call | WIRED | Line 56 of `AppViewModel.kt`: `PktapHceService.cachedPayload = NfcPayloadBuilder.buildNfcPayload(pubKey)` |
| `PktapHceService.processCommandApdu` | `NfcExchangeFlow.peerKeyFlow` | `tryEmit` on valid EXCHANGE command | WIRED | Line 44 of `PktapHceService.kt`: `result.peerPayload?.let { NfcExchangeFlow.peerKeyFlow.tryEmit(it) }` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `PostTapScreen.kt` | `state` (PostTapState) | `NfcViewModel.postTapState` StateFlow, driven by `launchPostTapCrypto` | Yes — state transitions driven by real crypto flow; stub is in `publish` lambda only (documented, Phase 6 obligation) | FLOWING |
| `PostTapScreen.kt` | `peerHex` (String?) | `NfcViewModel.peerPubKeyHex` StateFlow, populated from `NfcExchangeFlow.peerKeyFlow` | Yes — derives from real peer payload over NFC | FLOWING |
| `MainScreen.kt` | `publicKeyHex` (String) | `AppViewModel.publicKeyHex` StateFlow, populated by `PktapBridge.derivePublicKey` in IO coroutine | Yes — derived from real seed via Rust FFI | FLOWING |
| `NfcViewModel.kt` | `publish` lambda | Explicit no-op stub with comment: "Phase 6 will wire DhtClient here" | No — intentional stub, documented in SUMMARY | STATIC (intentional, documented, deferred to Phase 6) |

### Behavioral Spot-Checks

Step 7b: SKIPPED — app is Android-only with no runnable entry points on the host JVM. The test suite (`./gradlew :app:testDebugUnitTest`) passes per user-provided context (20+ JVM tests passing). No CLI or API surface to invoke without an Android device or emulator.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| NFC-01 | 05-01-PLAN.md, 05-02-PLAN.md | App implements HostApduService (HCE) for bidirectional Ed25519 public key exchange | SATISFIED | `PktapHceService` registered in manifest; `NfcReader` implements reader role; bidirectional exchange flow complete in code |
| NFC-02 | 05-01-PLAN.md, 05-02-PLAN.md | NFC exchange uses single APDU round-trip — Alice's command contains her 32-byte key, Bob's response contains his 32-byte key | SATISFIED | `NfcReader.performExchange`: SELECT AID + single EXCHANGE transceive; EXCHANGE APDU carries 36-byte payload, response is 36-byte payload + SW 90 00; verified by `PktapHceServiceTest` |
| NFC-03 | 05-01-PLAN.md | APDU handler does zero crypto or network I/O — only copies 32 bytes and returns within 300ms | SATISFIED (code); NEEDS HUMAN (timing on device) | No `PktapBridge` references in `PktapHceService.kt` (grep returns 0 matches); blanket try/catch; code-level test verifies absence; 300ms timing requires physical device profiling |
| NFC-04 | 05-01-PLAN.md | NFC payload: version (1) + flags (1) + Ed25519 pubkey (32) + CRC-16 (2) = 36 bytes | SATISFIED | `NfcPayloadBuilder.kt` implements exact format; all 12 `NfcPayloadBuilderTest` tests pass |
| NFC-05 | 05-01-PLAN.md, 05-02-PLAN.md | App handles SELECT AID APDU correctly for Samsung/Xiaomi HCE routing compatibility | PARTIALLY SATISFIED | AID `F0504B544150` registered with `category="other"` per Samsung/Xiaomi research recommendation; `PktapApduProtocol` handles SELECT AID correctly; actual Samsung/Xiaomi routing behavior requires physical device (human verification item) |
| NFC-06 | 05-02-PLAN.md | Post-tap crypto and DHT operations run in a background coroutine, not in the APDU handler | SATISFIED | `NfcViewModel.launchPostTapCrypto` uses `viewModelScope.launch(ioDispatcher)` with `Dispatchers.IO`; `processCommandApdu` only calls `tryEmit`; ViewModel tests verify coroutine dispatch |

All 6 requirement IDs from both plan frontmatters are accounted for. No orphaned requirements found for Phase 5 in `REQUIREMENTS.md` (traceability table maps NFC-01 through NFC-06 to Phase 5 only).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `NfcViewModel.kt` | 47-49 | `publish` lambda defaults to no-op `{ _, _ -> }` | INFO | Intentional documented stub — "Phase 6 will wire DhtClient here; DhtClient not yet exposed via UniFFI." State machine still transitions Publishing → Done. No user-visible data lost. |

No other TODO/FIXME/placeholder comments found. No hardcoded empty data causing hollow rendering. No silent unimplemented handlers.

### Human Verification Required

#### 1. Two-Device NFC Tap Exchange

**Test:** Install the app on two Android devices with NFC. Ensure both have a seed set up (complete onboarding). Tap the phones back-to-back and observe.
**Expected:** Both devices navigate to PostTapScreen showing "Contact Exchange"; `peerPubKeyHex` is displayed on both sides as an 8-char prefix + ellipsis + 8-char suffix; state transitions through Encrypting → Publishing → Done on both devices.
**Why human:** Requires two physical Android devices with NFC hardware. Emulator NFC has documented quirks (per 05-RESEARCH.md) that make it an unreliable substitute.

#### 2. Samsung One UI AID Routing — No Manual Configuration

**Test:** Install the app on a Samsung Galaxy device (One UI). Bring a second NFC device close to trigger the HCE service.
**Expected:** The Samsung device routes the SELECT AID `F0504B544150` to PKTap's `PktapHceService` automatically — no user-facing dialog requiring manual AID allowlisting, no "Payment default" conflict, no NFC routing error.
**Why human:** Samsung One UI NFC routing for `category="other"` AIDs is OEM-specific behavior. The code registers the AID correctly per research recommendations, but actual invocation requires physical Samsung hardware to confirm routing works without manual configuration (NFC-05).

### Gaps Summary

No blocking gaps. All code artifacts exist, are substantive (not stubs), and are wired correctly. The `publish` lambda is an explicitly documented no-op stub inherited by Phase 6 — it is informational, not a blocker.

Two items remain that require human testing on physical hardware:
- Two-device NFC tap exchange (physical NFC exchange — deferred to Phase 6 end-to-end validation)
- Samsung One UI AID routing (OEM-specific NFC behavior — requires Samsung device)

The phase is code-complete. Human verification is the remaining gate.

---

_Verified: 2026-04-05_
_Verifier: Claude (gsd-verifier)_
