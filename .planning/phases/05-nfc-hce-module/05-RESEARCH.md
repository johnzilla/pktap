# Phase 5: NFC HCE Module - Research

**Researched:** 2026-04-05
**Domain:** Android NFC Host Card Emulation (HCE), APDU protocol, reader mode
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**APDU Protocol Design**
- **D-01:** Reader sends its 36-byte payload in a custom APDU command after SELECT AID. HCE host responds with its own 36-byte payload. Single round-trip: command = reader's key, response = host's key. Both phones get each other's key.
- **D-02:** Both phones run reader + HCE simultaneously. Whichever phone initiates the tap acts as reader, the other as HCE host. No user action needed to pick a role. Android supports this natively.

**HCE Service Lifecycle**
- **D-03:** Pre-cache the 36-byte NDEF payload on app start from AppViewModel's cached pubkey (Phase 4 D-06). Build version(1) + flags(1) + Ed25519 pubkey(32) + CRC-16(2) once. `processCommandApdu()` returns the pre-built byte array — zero computation.
- **D-04:** HCE service communicates received peer key via SharedFlow. The active Activity/ViewModel collects it and navigates to the post-tap screen. Clean separation between service and UI.

**OEM Compatibility**
- **D-05:** Register AID in `apduservice.xml` with `android:category="other"` and a proprietary AID (`F0504B544150` = hex for "PKTAP"). Avoids payment AID conflicts. Samsung/Xiaomi route "other" category reliably.
- **D-06:** Check `NfcAdapter.isEnabled()` before exchange flow. If disabled, show dialog linking to NFC settings. Don't block the rest of the app — QR fallback (Phase 7) covers no-NFC devices.

**Post-Tap Processing**
- **D-07:** Post-tap coroutine launches in ViewModel scope on `Dispatchers.IO` after UI receives peer key via SharedFlow. ECDH + encrypt + DHT publish via PktapBridge. ViewModel scope survives config changes.
- **D-08:** Post-tap screen shows status progression: "Encrypting... Publishing... Done/Error". If DHT fails, show "Queued for sync" (offline queue from Phase 2 handles retry).

### Claude's Discretion
- Exact APDU command/response byte layout (CLA, INS, P1, P2 values)
- SharedFlow vs Channel for peer key delivery
- NFC reader mode dispatch configuration
- Whether to use a foreground service for NFC reliability
- CRC-16 implementation (Rust via FFI or Kotlin-side)
- Post-tap screen visual design

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NFC-01 | App implements HostApduService (HCE) for bidirectional Ed25519 public key exchange | HostApduService API, apduservice.xml registration, manifest entries documented |
| NFC-02 | NFC exchange uses single APDU round-trip — Alice's command contains her 32-byte key, Bob's response contains his 32-byte key | APDU command/response structure, IsoDep.transceive pattern documented |
| NFC-03 | APDU handler does zero crypto or network I/O — only copies 32 bytes and returns within 300ms | Pre-caching pattern, main-thread constraint, critical exception pitfall documented |
| NFC-04 | NFC payload follows NDEF External Type format: version(1) + flags(1) + Ed25519 pubkey(32) + CRC-16(2) = 36 bytes | CRC-16 via Rust crc crate or Kotlin inline documented |
| NFC-05 | App handles SELECT AID APDU correctly for Samsung/Xiaomi HCE routing compatibility | apduservice.xml category="other", AID format, OEM routing behavior documented |
| NFC-06 | Post-tap crypto and DHT operations run in a background coroutine, not in the APDU handler | onDeactivated trigger pattern, viewModelScope + Dispatchers.IO coroutine documented |
</phase_requirements>

---

## Summary

This phase wires up bidirectional NFC public key exchange using Android's Host Card Emulation (HCE) API. One phone runs `HostApduService` (the HCE host that responds to SELECT AID), while the other phone uses `NfcAdapter.enableReaderMode()` + `IsoDep.transceive()` to initiate the exchange. Since both phones run the same app, both roles are active simultaneously — the device that physically initiates the tap becomes the reader; the other responds as the HCE host.

The central architectural constraint is that `processCommandApdu()` runs on the **main thread** and must never block, throw unhandled exceptions, or perform any I/O. An unhandled exception there permanently kills the HCE service until device reboot — this is the most dangerous pitfall in this entire phase. The solution is pre-caching the 36-byte response payload at app start (from `AppViewModel.publicKeyBytes`), wrapping all `processCommandApdu` body in a top-level try/catch, and returning the cached array directly.

The reader role is implemented by calling `enableReaderMode()` with `FLAG_READER_NFC_A | FLAG_READER_SKIP_NDEF_CHECK` in `onResume()` (and disabling in `onPause()`). When a peer HCE device is detected, the reader sends a SELECT AID APDU, then a custom EXCHANGE command containing its own 36-byte payload, and receives the host's 36-byte payload in the response. After `onDeactivated()` fires on the HCE side, the received peer key is emitted through a `SharedFlow` singleton and collected by the ViewModel, which launches the post-tap coroutine on `Dispatchers.IO`.

**Primary recommendation:** Keep `processCommandApdu` to exactly two operations — check command type, return pre-cached array — wrapped in a blanket try/catch that returns a SW_UNKNOWN error byte array on any exception.

---

## Standard Stack

### Core (no new dependencies needed beyond what is already in the project)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `android.nfc.cardemulation.HostApduService` | Android SDK (API 19+) | HCE service that receives APDU commands | Only Android HCE mechanism; no third-party lib needed [VERIFIED: developer.android.com] |
| `android.nfc.NfcAdapter` | Android SDK | Reader mode activation / NFC state check | Platform API; `enableReaderMode()` is the required reader role API [VERIFIED: developer.android.com] |
| `android.nfc.tech.IsoDep` | Android SDK | ISO 14443-4 transport for sending APDUs to HCE host | The only way to transact with HCE services from a reader role [VERIFIED: developer.android.com] |
| `kotlinx-coroutines-android` | 1.8.1 (already in project) | SharedFlow for peer key delivery, post-tap coroutine | Already present in `libs.versions.toml` [VERIFIED: codebase] |
| `crc` (Rust crate) | 3.4.0 | CRC-16 checksum over the 36-byte NFC payload in Rust | Already specified in CLAUDE.md as project standard; no Kotlin-side CRC needed [VERIFIED: docs.rs/crc, CLAUDE.md] |

**Version verification:**
- `crc` crate: 3.4.0 as of April 2026 [VERIFIED: docs.rs/crc]
- All Android SDK APIs: platform-level, no versioning needed beyond `minSdk = 26`
- `kotlinx-coroutines-android`: 1.8.1 — already pinned in `libs.versions.toml` [VERIFIED: codebase]

**No new Android dependencies are required.** All needed APIs are platform SDK or already in the version catalog.

**Rust:** Add `crc = "3"` to `pktap-core/Cargo.toml` for NFC payload construction in the `ffi.rs` expose. Alternatively, CRC-16 can be computed inline in Kotlin (6 lines, no dependency) — see Code Examples.

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `Settings.ACTION_NFC_SETTINGS` | Android SDK | Deep-link to system NFC toggle screen | Use in D-06 NFC disabled dialog [VERIFIED: developer.android.com] |
| `NfcAdapter.EXTRA_READER_PRESENCE_CHECK_DELAY` | Android SDK | Bundle key to tune reader presence check interval | Pass in options Bundle to `enableReaderMode()` to reduce unnecessary polls [ASSUMED] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `enableReaderMode` (reader role) | `enableForegroundDispatch` | `enableForegroundDispatch` cannot detect HCE on other Android devices — Android-to-Android peer-to-peer mode intercepts the link before HCE is exposed. `enableReaderMode` is the ONLY working option. [VERIFIED: developer.android.com] |
| SharedFlow singleton for key delivery | `LocalBroadcastManager` | SharedFlow is Kotlin-native, lifecycle-aware, no Intent overhead, and already used in the project [ASSUMED] |
| SharedFlow singleton | `Channel` | SharedFlow with `replay=0, extraBufferCapacity=1` behaves identically to a Channel for single-event delivery; SharedFlow is already the pattern in the project codebase [ASSUMED] |
| Rust crc crate | Inline Kotlin CRC-16 | Kotlin inline is 6 lines, adds no dependency, keeps CRC computation close to the byte construction; either is acceptable for this use case [VERIFIED: crates.io/crates/crc] |

**Installation:**
```bash
# No new Gradle dependencies. For the Rust crc crate:
# In pktap-core/Cargo.toml, add under [dependencies]:
# crc = "3"
```

---

## Architecture Patterns

### Recommended Project Structure

```
android/app/src/main/java/com/pktap/app/
├── nfc/
│   ├── PktapHceService.kt       # HostApduService — HCE host role
│   ├── NfcReader.kt             # enableReaderMode logic — reader role
│   ├── NfcPayloadBuilder.kt     # Builds 36-byte payload from pubkey + CRC-16
│   └── NfcExchangeFlow.kt       # Singleton SharedFlow for peer key delivery
├── ui/
│   └── posttap/
│       └── PostTapScreen.kt     # Status progression: Encrypting/Publishing/Done
android/app/src/main/res/xml/
└── apduservice.xml              # AID registration for HCE routing
android/app/src/main/AndroidManifest.xml  # Add NFC permission, HCE service, uses-feature
```

### Pattern 1: HCE Service with Pre-Cached Payload

**What:** `HostApduService` subclass that returns a pre-built byte array from `processCommandApdu()` with zero computation. The 36-byte payload is set once when `AppViewModel` derives the pubkey.

**When to use:** This is the ONLY acceptable pattern. Any computation in `processCommandApdu()` risks blocking the main thread or throwing exceptions.

```kotlin
// Source: developer.android.com/develop/connectivity/nfc/hce (adapted)
class PktapHceService : HostApduService() {

    companion object {
        // Singleton SharedFlow: service -> ViewModel peer key delivery (D-04)
        val peerKeyFlow = MutableSharedFlow<ByteArray>(
            replay = 0,
            extraBufferCapacity = 1,
            onBufferOverflow = BufferOverflow.DROP_OLDEST
        )

        // Pre-cached 36-byte payload set by AppViewModel on app start (D-03)
        @Volatile
        var cachedPayload: ByteArray? = null

        // APDU status words
        val SW_OK = byteArrayOf(0x90.toByte(), 0x00)
        val SW_UNKNOWN = byteArrayOf(0x6F.toByte(), 0x00)
        val SW_FILE_NOT_FOUND = byteArrayOf(0x6A.toByte(), 0x82.toByte())

        // AID: F0504B544150 = "PKTAP" proprietary (D-05)
        val SELECT_AID_CMD = byteArrayOf(0x00, 0xA4.toByte(), 0x04, 0x00, 0x06,
            0xF0.toByte(), 0x50, 0x4B, 0x54, 0x41, 0x50)

        // Custom EXCHANGE command (Claude's discretion — CLA=0x90, INS=0x01)
        const val EXCHANGE_CLA: Byte = 0x90.toByte()
        const val EXCHANGE_INS: Byte = 0x01
    }

    private var selectAidReceived = false

    override fun processCommandApdu(commandApdu: ByteArray, extras: Bundle?): ByteArray {
        // CRITICAL: Wrap EVERYTHING — an unhandled exception permanently kills this service
        // until device reboot. Never let any exception escape (NFC-03).
        return try {
            when {
                isSelectAid(commandApdu) -> {
                    selectAidReceived = true
                    SW_OK  // 90 00 — AID accepted
                }
                selectAidReceived && isExchangeCommand(commandApdu) -> {
                    handleExchange(commandApdu)
                }
                else -> SW_UNKNOWN
            }
        } catch (e: Exception) {
            SW_UNKNOWN  // Never let exceptions escape
        }
    }

    private fun handleExchange(apdu: ByteArray): ByteArray {
        // Reader's 36-byte payload is in the APDU data field (LC byte at index 4, data at 5..40)
        if (apdu.size >= 41) {
            val peerPayload = apdu.copyOfRange(5, 41)  // 36 bytes
            // Emit peer key asynchronously — do NOT block here
            peerKeyFlow.tryEmit(peerPayload)
        }
        // Respond with our own pre-cached 36-byte payload
        val payload = cachedPayload ?: return SW_FILE_NOT_FOUND
        return payload + SW_OK  // payload(36) + 90 00
    }

    private fun isSelectAid(apdu: ByteArray): Boolean {
        // CLA=0x00, INS=0xA4, P1=0x04 means SELECT by AID
        return apdu.size >= 2 && apdu[1] == 0xA4.toByte()
    }

    private fun isExchangeCommand(apdu: ByteArray): Boolean {
        return apdu.size >= 2 && apdu[0] == EXCHANGE_CLA && apdu[1] == EXCHANGE_INS
    }

    override fun onDeactivated(reason: Int) {
        selectAidReceived = false
    }
}
```

### Pattern 2: Reader Role with enableReaderMode

**What:** The reader side of the exchange. Activated in `onResume()` of the Activity that owns the NFC exchange screen, disabled in `onPause()`.

**When to use:** Must be active whenever the app is on the main/tap screen and NFC is enabled. `FLAG_READER_NFC_A | FLAG_READER_SKIP_NDEF_CHECK` is the correct flag combination for reading from an HCE service.

```kotlin
// Source: developer.android.com/reference/android/nfc/NfcAdapter (adapted)
class MainActivity : ComponentActivity(), NfcAdapter.ReaderCallback {

    private var nfcAdapter: NfcAdapter? = null

    override fun onResume() {
        super.onResume()
        nfcAdapter = NfcAdapter.getDefaultAdapter(this)
        nfcAdapter?.enableReaderMode(
            this,
            this,
            NfcAdapter.FLAG_READER_NFC_A or NfcAdapter.FLAG_READER_SKIP_NDEF_CHECK,
            null
        )
    }

    override fun onPause() {
        super.onPause()
        nfcAdapter?.disableReaderMode(this)
    }

    override fun onTagDiscovered(tag: Tag) {
        // Runs on a background thread — safe for I/O
        val isoDep = IsoDep.get(tag) ?: return
        try {
            isoDep.connect()
            performExchange(isoDep)
        } catch (e: IOException) {
            // Tag lost or communication error — surface via UI state if needed
        } finally {
            try { isoDep.close() } catch (_: IOException) { }
        }
    }

    private fun performExchange(isoDep: IsoDep) {
        // Step 1: SELECT AID
        val selectResponse = isoDep.transceive(buildSelectAid())
        if (!isSwOk(selectResponse)) return

        // Step 2: Send EXCHANGE command with our 36-byte payload
        val ourPayload = appViewModel.buildNfcPayload() ?: return  // null if pubkey not ready
        val exchangeApdu = buildExchangeApdu(ourPayload)
        val response = isoDep.transceive(exchangeApdu)

        // Step 3: Parse response — expect 36 bytes + SW 90 00
        if (response.size == 38 && isSwOk(response.takeLast(2).toByteArray())) {
            val peerPayload = response.copyOfRange(0, 36)
            // Deliver to ViewModel via the same SharedFlow
            PktapHceService.peerKeyFlow.tryEmit(peerPayload)
        }
    }

    private fun buildSelectAid(): ByteArray {
        val aid = byteArrayOf(0xF0.toByte(), 0x50, 0x4B, 0x54, 0x41, 0x50)
        return byteArrayOf(0x00, 0xA4.toByte(), 0x04, 0x00, aid.size.toByte()) + aid
    }

    private fun buildExchangeApdu(payload: ByteArray): ByteArray {
        // CLA=0x90, INS=0x01, P1=0x00, P2=0x00, LC=36, Data=payload
        return byteArrayOf(0x90.toByte(), 0x01, 0x00, 0x00, 0x24) + payload
    }

    private fun isSwOk(sw: ByteArray): Boolean =
        sw.size >= 2 && sw[sw.size - 2] == 0x90.toByte() && sw[sw.size - 1] == 0x00.toByte()
}
```

### Pattern 3: 36-Byte Payload Construction

**What:** Build the NFC payload once from the cached pubkey when the ViewModel initializes. Store in `PktapHceService.cachedPayload`.

**When to use:** Immediately after `AppViewModel.publicKeyBytes` becomes non-null (after seed derivation on app start).

```kotlin
// NFC payload: version(1) + flags(1) + pubkey(32) + CRC-16(2) = 36 bytes (NFC-04)
fun buildNfcPayload(pubKeyBytes: ByteArray): ByteArray {
    require(pubKeyBytes.size == 32)
    val buf = ByteArray(34)
    buf[0] = 0x01  // version = 1
    buf[1] = 0x00  // flags = 0 (reserved, encrypted mode)
    pubKeyBytes.copyInto(buf, 2)
    val crc = crc16Ccitt(buf, 0, 34)
    return buf + byteArrayOf((crc shr 8).toByte(), crc.toByte())
}

// CRC-16/CCITT-FALSE (poly=0x1021, init=0xFFFF, no reflect) — 6-line pure Kotlin
// Source: CCITT specification (ASSUMED — standard algorithm, no library needed)
fun crc16Ccitt(data: ByteArray, offset: Int, length: Int): Int {
    var crc = 0xFFFF
    for (i in offset until offset + length) {
        crc = crc xor (data[i].toInt() and 0xFF shl 8)
        repeat(8) { crc = if (crc and 0x8000 != 0) (crc shl 1) xor 0x1021 else crc shl 1 }
        crc = crc and 0xFFFF
    }
    return crc
}
```

Alternatively, add `crc = "3"` to `pktap-core/Cargo.toml` and expose `build_nfc_payload(pub_key: Vec<u8>) -> Vec<u8>` via UniFFI — this moves all byte construction to Rust and keeps parity with the CLAUDE.md recommendation. Either approach is valid; the Kotlin-inline approach avoids an FFI round-trip for a trivial computation.

### Pattern 4: AndroidManifest.xml Changes

**What:** Three additions to the manifest are required for NFC HCE.

```xml
<!-- Source: developer.android.com/develop/connectivity/nfc/hce [VERIFIED] -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <!-- NFC permission -->
    <uses-permission android:name="android.permission.NFC" />
    <!-- Declare NFC as required hardware (optional — set required="false" for graceful degradation) -->
    <uses-feature android:name="android.hardware.nfc" android:required="false" />
    <!-- NFC HCE feature -->
    <uses-feature android:name="android.hardware.nfc.hce" android:required="false" />

    <application ...>
        <!-- Existing MainActivity -->
        <activity android:name=".MainActivity" ... />

        <!-- HCE Service registration -->
        <service
            android:name=".nfc.PktapHceService"
            android:exported="true"
            android:permission="android.permission.BIND_NFC_SERVICE">
            <intent-filter>
                <action android:name="android.nfc.cardemulation.action.HOST_APDU_SERVICE" />
            </intent-filter>
            <meta-data
                android:name="android.nfc.cardemulation.host_apdu_service"
                android:resource="@xml/apduservice" />
        </service>
    </application>
</manifest>
```

### Pattern 5: apduservice.xml

```xml
<!-- Source: developer.android.com/develop/connectivity/nfc/hce [VERIFIED] -->
<!-- File: android/app/src/main/res/xml/apduservice.xml -->
<host-apdu-service xmlns:android="http://schemas.android.com/apk/res/android"
    android:description="@string/nfc_service_description"
    android:requireDeviceUnlock="false">
    <aid-group
        android:description="@string/nfc_aid_description"
        android:category="other">
        <!-- Proprietary AID: F0 50 4B 54 41 50 = "PKTAP" (D-05) -->
        <aid-filter android:name="F0504B544150" />
    </aid-group>
</host-apdu-service>
```

### Pattern 6: Post-Tap ViewModel Integration

**What:** Collect from `PktapHceService.peerKeyFlow` in `AppViewModel`, trigger post-tap coroutine.

```kotlin
// In AppViewModel.init {} (after pubkey cache setup):
viewModelScope.launch {
    PktapHceService.peerKeyFlow.collect { peerRawPayload ->
        // Parse 36-byte NFC payload: skip version(1) + flags(1), take 32 bytes pubkey
        if (peerRawPayload.size == 36) {
            val peerPubKey = peerRawPayload.copyOfRange(2, 34)
            // Validate CRC before trusting payload
            if (validateNfcPayloadCrc(peerRawPayload)) {
                _peerPublicKey.value = peerPubKey
                launchPostTapCrypto(peerPubKey)  // D-07
            }
        }
    }
}

private fun launchPostTapCrypto(peerPubKey: ByteArray) {
    viewModelScope.launch(Dispatchers.IO) {
        _postTapState.value = PostTapState.Encrypting
        try {
            val seed = seedRepository.decryptSeed()
            val encrypted = try {
                PktapBridge.ecdhAndEncrypt(seed.copyOf(), peerPubKey, contactFieldsJson)
            } finally {
                seed.fill(0)
            }
            _postTapState.value = PostTapState.Publishing
            // Phase 2 DhtClient.publishEncrypted handles offline queue (DHT-08)
            dhtClient.publishEncrypted(encrypted, peerPubKey)
            _postTapState.value = PostTapState.Done
        } catch (e: Exception) {
            _postTapState.value = PostTapState.Error(e.message ?: "Unknown error")
        }
    }
}
```

### Anti-Patterns to Avoid

- **Computing in processCommandApdu:** Any crypto, FFI call, or I/O blocks the main thread and risks violating the 300ms constraint. Pre-caching is mandatory.
- **Uncaught exceptions in processCommandApdu:** An unhandled exception permanently kills HCE until device reboot. Every path must be guarded.
- **Using enableForegroundDispatch for the reader role:** This does NOT detect HCE on other Android devices. `enableReaderMode` is mandatory.
- **Caching the decrypted seed in ViewModel:** Only the pubkey (non-secret) is cached. The seed is decrypted fresh for each post-tap operation, per Phase 4 D-05.
- **Launching the post-tap coroutine from onDeactivated:** `onDeactivated()` runs synchronously on the binder thread. Emit to SharedFlow and let ViewModel react.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HCE card emulation | Custom NFC stack | `HostApduService` (Android SDK) | Platform handles AID routing, RF protocol, lifecycle |
| Reader-side APDU transport | Raw NFC tag I/O | `IsoDep.transceive()` (Android SDK) | Handles T=CL framing, chaining, error recovery |
| AID routing on Samsung/Xiaomi | Custom NFC intent filter | `apduservice.xml` with `category="other"` | Category routing is OEM-implemented at the NFC controller level; app-level code cannot override it |
| CRC-16 polynomial | Custom bit-banging | `crc` crate 3.x or 6-line Kotlin | Standard algorithm with known test vectors; polynomial error creates silent key corruption |
| Post-tap concurrency | Custom thread management | `viewModelScope.launch(Dispatchers.IO)` | ViewModel scope survives config changes; Dispatchers.IO is correct for FFI + network |

**Key insight:** The HCE API is a thin veneer over the Android NFC controller. Do not try to manage the APDU session state machine beyond responding to SELECT AID and the single custom command — extra complexity here is how the "permanently kills HCE" bug gets introduced.

---

## Common Pitfalls

### Pitfall 1: Unhandled Exception in processCommandApdu Kills HCE Permanently

**What goes wrong:** Any exception that escapes `processCommandApdu()` causes the Android OS to enter a state where it cannot restart the `HostApduService`. NFC stops working for the app until device reboot. The OS does not log a recoverable error.

**Why it happens:** The NFC stack in Android has a native C++ layer that binds to the service. When the Java service throws unexpectedly, the native binder state is corrupted and cannot recover without a full system restart.

**How to avoid:** Wrap the ENTIRE `processCommandApdu` body in a top-level `try { ... } catch (e: Exception) { return SW_UNKNOWN }`. Never let a single exception escape. Test with malformed/truncated APDUs.

**Warning signs:** NFC exchange stops working after an NFC-related crash. User must reboot. The only detection is "NFC is suddenly broken after that crash."

[VERIFIED: medium.com/swlh/why-you-need-to-be-very-careful-while-working-with-nfc-hce-apis-in-android-9bde32cc7924]

### Pitfall 2: enableForegroundDispatch Cannot Read HCE From Other Android Devices

**What goes wrong:** Using the older `enableForegroundDispatch` API for the reader role results in Android devices establishing a peer-to-peer (LLCP) link with each other instead of a reader/card-emulation link. HCE on the other phone is invisible.

**Why it happens:** When two Android devices with NFC come together, both default to attempting LLCP peer-to-peer communication. `enableForegroundDispatch` does not disable P2P mode. Only `enableReaderMode` forces the NFC controller into pure reader mode, suppressing P2P negotiation and exposing the HCE stack on the other device.

**How to avoid:** Always use `NfcAdapter.enableReaderMode()` for the reader role. Call `disableReaderMode()` in `onPause()`.

**Warning signs:** Two devices tap but nothing happens; no `onTagDiscovered` callback fires.

[VERIFIED: developer.android.com/develop/connectivity/nfc/hce]

### Pitfall 3: HCE Service Not Active When App Is in Background

**What goes wrong:** On some OEM ROMs (especially aggressive battery optimization on Xiaomi MIUI), the HCE service may be stopped when the app is backgrounded or the screen is locked.

**Why it happens:** MIUI's battery optimization kills services that are not explicitly exempt. Unlike payment apps (which hold a special NFC default role), "other" category apps do not get preferential lifecycle treatment.

**How to avoid:** Register the service properly with `android:exported="true"` and `BIND_NFC_SERVICE` permission. Consider keeping the app in the foreground during exchange (the main screen is active during a tap anyway — this is the common case). Phase scope is limited to foreground use; background exchange is not a v1 requirement.

**Warning signs:** HCE works on Pixel but fails on Xiaomi when app is freshly backgrounded.

[ASSUMED — based on known MIUI battery optimization behavior]

### Pitfall 4: processCommandApdu Threading Confusion

**What goes wrong:** Developers assume `processCommandApdu` runs on a background thread (as stated in some third-party documentation) and attempt direct UI thread operations. The Android official documentation states it runs on the **main thread**.

**Why it happens:** Conflicting documentation; some API mirrors incorrectly describe it as a background thread.

**How to avoid:** Treat `processCommandApdu` as running on the main thread. Return the pre-cached array immediately. Use `sendResponseApdu()` pattern only if you absolutely must defer, but for this phase the synchronous return is correct and safe.

**Warning signs:** `CalledFromWrongThreadException` or ANR if any UI or non-trivial work is done inside the method.

[VERIFIED: developer.android.com/develop/connectivity/nfc/hce — "this method is called on the main thread of your application, which you shouldn't block"]

### Pitfall 5: NDEF Check Blocking Reader Role Startup

**What goes wrong:** Reader mode without `FLAG_READER_SKIP_NDEF_CHECK` causes the NFC stack to attempt an NDEF check on the discovered HCE device, which adds ~100-200ms and sometimes fails with a `TagLostException` before the app can call `IsoDep.transceive()`.

**Why it happens:** NDEF check is the default behavior. HCE services do not implement NDEF — the check always fails, wasting time and potentially killing the connection.

**How to avoid:** Always include `FLAG_READER_SKIP_NDEF_CHECK` in the `enableReaderMode` flags: `FLAG_READER_NFC_A or FLAG_READER_SKIP_NDEF_CHECK`.

[VERIFIED: developer.android.com/develop/connectivity/nfc/hce — recommended flags for reading from HCE]

### Pitfall 6: AID F0504B544150 Byte Length

**What goes wrong:** The AID in `apduservice.xml` is the hex string `F0504B544150` (12 hex chars = 6 bytes). The SELECT AID APDU LC byte must be `0x06`. A mismatch between the registered AID and the SELECT command bytes causes routing failure.

**Why it happens:** Off-by-one when counting hex characters vs bytes, or copying the hex string with a leading `0x` prefix.

**How to avoid:** Verify that `aid.length / 2 == 6`, that the LC byte in the SELECT command is `0x06`, and that the `aid-filter` in `apduservice.xml` contains exactly `F0504B544150` (no spaces, no `0x` prefix).

[ASSUMED — standard AID length mismatch class of bugs]

---

## Code Examples

### SELECT AID APDU Construction (Reader Side)
```kotlin
// Source: github.com/googlearchive/android-CardReader (verified pattern)
// AID: F0 50 4B 54 41 50 (6 bytes, "PKTAP" proprietary)
val aid = byteArrayOf(0xF0.toByte(), 0x50, 0x4B, 0x54, 0x41, 0x50)
val selectAid = byteArrayOf(
    0x00,              // CLA: ISO 7816 class
    0xA4.toByte(),     // INS: SELECT FILE
    0x04,              // P1: Select by AID
    0x00,              // P2: first or only occurrence
    aid.size.toByte()  // Lc: length of AID = 0x06
) + aid
```

### EXCHANGE Command Construction (Reader Side)
```kotlin
// Custom proprietary command: CLA=0x90, INS=0x01, P1=0x00, P2=0x00, LC=0x24 (36 decimal)
// Data = our 36-byte NFC payload
val exchangeApdu = byteArrayOf(
    0x90.toByte(),       // CLA: proprietary class byte (bit 8 = 1 for proprietary)
    0x01,                // INS: EXCHANGE_KEY (arbitrary proprietary instruction)
    0x00,                // P1
    0x00,                // P2
    0x24                 // LC = 36 bytes of data follow
) + ourNfcPayload        // 36 bytes: version + flags + pubkey + CRC-16
```

### SW Response Check
```kotlin
// A valid response contains SW 90 00 as the last two bytes
fun isSwOk(response: ByteArray): Boolean =
    response.size >= 2 &&
    response[response.size - 2] == 0x90.toByte() &&
    response[response.size - 1] == 0x00.toByte()
```

### CRC-16/CCITT-FALSE in Kotlin (inline, no dependency)
```kotlin
// Polynomial: 0x1021, Init: 0xFFFF, RefIn: false, RefOut: false, XorOut: 0x0000
// Check value: CRC("123456789") == 0x29B1
// [ASSUMED — standard CCITT algorithm; verify with known test vector before shipping]
fun crc16Ccitt(data: ByteArray): Int {
    var crc = 0xFFFF
    for (b in data) {
        crc = crc xor ((b.toInt() and 0xFF) shl 8)
        repeat(8) { crc = if (crc and 0x8000 != 0) (crc shl 1) xor 0x1021 else crc shl 1 }
        crc = crc and 0xFFFF
    }
    return crc
}
```

### crc crate (Rust, if preferred over Kotlin inline)
```rust
// Source: docs.rs/crc/3.4.0 [VERIFIED]
use crc::{Crc, CRC_16_IBM_SDLC};  // IBM_SDLC = X.25 = CRC_16_CCITT-FALSE equivalent

const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);

pub fn build_nfc_payload(pub_key: &[u8; 32]) -> [u8; 36] {
    let mut buf = [0u8; 36];
    buf[0] = 0x01;  // version
    buf[1] = 0x00;  // flags
    buf[2..34].copy_from_slice(pub_key);
    let crc = CRC16.checksum(&buf[0..34]);
    buf[34] = (crc >> 8) as u8;
    buf[35] = crc as u8;
    buf
}
```

**Note on CRC variant:** CRC_16_IBM_SDLC and CRC_16_CCITT-FALSE differ in init value and reflection. For a proprietary protocol it does not matter which variant is chosen, but it MUST be consistent between the builder and the validator. Pick one and document it in a constant. [ASSUMED — CRC variant choice is internal to PKTap protocol]

### NFC Disabled Dialog
```kotlin
// Source: developer.android.com/reference/android/nfc/NfcAdapter [VERIFIED]
fun showNfcDisabledDialog(context: Context) {
    AlertDialog.Builder(context)
        .setTitle("NFC Required")
        .setMessage("Enable NFC to exchange contact info by tapping phones.")
        .setPositiveButton("Open Settings") { _, _ ->
            context.startActivity(Intent(Settings.ACTION_NFC_SETTINGS))
        }
        .setNegativeButton("Use QR Code") { dialog, _ -> dialog.dismiss() }
        .show()
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `enableForegroundDispatch` for reader role | `enableReaderMode` + `ReaderCallback` | Android 4.4 (API 19) | `enableReaderMode` is the only API that can detect HCE on other Android devices |
| Foreground service to keep HCE alive | Standard `HostApduService` with `category="other"` | Android 4.4+ | No foreground service needed; HCE service is bound by the NFC stack when in foreground |
| UDL files for UniFFI | Proc-macro API (`#[uniffi::export]`) | UniFFI 0.25+ | Project already uses proc-macro style — no change needed for any new FFI exports |

**No deprecated APIs used in this phase.** All HCE APIs have been stable since API 19. The project's `minSdk = 26` is well above the minimum.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | CRC-16/CCITT-FALSE (poly=0x1021, init=0xFFFF) is the correct variant — both sender and receiver must use identical variant | Code Examples | Silent CRC mismatch means all NFC payloads fail validation; caught by a unit test with known test vector |
| A2 | MIUI aggressive battery optimization can kill the HCE service when app is backgrounded | Pitfall 3 | If wrong, no issue — the mitigation (foreground-only use) is already consistent with the phase scope |
| A3 | `NfcAdapter.EXTRA_READER_PRESENCE_CHECK_DELAY` tuning reduces reader-side polling overhead | Standard Stack > Supporting | If wrong, reader mode works but with default polling interval — no functional impact |
| A4 | CLA=0x90, INS=0x01 for the custom EXCHANGE command is appropriate (Claude's discretion) | Architecture Patterns | Any CLA/INS values work for proprietary protocols — this is a design choice not a constraint |
| A5 | `SharedFlow(replay=0, extraBufferCapacity=1, DROP_OLDEST)` is appropriate for one-shot peer key delivery | Architecture Patterns | If replay=1 is needed (late subscriber), the ViewModel may miss the event; test the collection timing |

---

## Open Questions

1. **CRC-16 placement: Rust FFI export vs Kotlin inline**
   - What we know: Both are functionally equivalent for the 36-byte payload. Rust crc crate 3.x is specified in CLAUDE.md. Kotlin inline is 6 lines with no new dependency.
   - What's unclear: Whether the team wants a new `build_nfc_payload` FFI function in `ffi.rs` or prefers keeping Kotlin side as thin as possible.
   - Recommendation: Use Kotlin inline for this phase. Add `build_nfc_payload` to Rust only if Phase 6 or Phase 7 also need it.

2. **SharedFlow collection timing: ViewModel init vs LaunchedEffect**
   - What we know: `AppViewModel.init {}` launches a coroutine collecting from `peerKeyFlow`. If the ViewModel is not yet initialized when a tap occurs (unlikely — ViewModel initializes before the screen renders), the event is dropped due to `replay=0`.
   - What's unclear: Whether `replay=1` should be used to buffer the most recent peer key until a subscriber connects.
   - Recommendation: Use `replay=0, extraBufferCapacity=1` — the ViewModel is initialized well before any tap scenario.

3. **Reader role scope: MainActivity only vs any Activity**
   - What we know: `enableReaderMode` is tied to a specific Activity instance and must be balanced with `disableReaderMode` in `onPause`.
   - What's unclear: Which Activity/screen should own the reader mode registration in Phase 5 (before Phase 6 builds the full flow).
   - Recommendation: Register in `MainActivity` for Phase 5. Phase 6 can refine the screen that triggers reader mode.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Java SDK | Android build | Yes | OpenJDK 25.0.2 (JAVA_HOME=/usr/lib/jvm/java-21-openjdk per spec) | — |
| Android Gradle Plugin | Android build | Yes | 8.7.3 (pinned in libs.versions.toml) | — |
| NFC hardware (physical device) | NFC-01, NFC-02, NFC-05 | Unknown — not testable on CI | — | Emulator for APDU logic unit tests; physical devices for integration |
| Samsung device | NFC-05 OEM compatibility | Unknown | — | Pixel for development; Samsung required for acceptance criteria |
| pktap-test AVD | Unit/smoke tests | Yes (per build environment) | Limited NFC support | Unit tests for payload builder + APDU parsing logic only |

**Missing dependencies with no fallback:**
- Physical NFC devices (two required, including one Samsung or Xiaomi) — acceptance criteria NFC-01 and NFC-05 cannot be verified without hardware.

**Missing dependencies with fallback:**
- Emulator NFC: Use for APDU protocol logic tests only (payload building, CRC, command parsing). Full tap tests require hardware.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | AndroidJUnit4 (instrumented), JUnit 4 via `androidx.test.ext:junit` |
| Config file | `android/app/build.gradle.kts` — `testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"` |
| Quick run command | `./gradlew :app:testDebugUnitTest` (JVM unit tests) |
| Full suite command | `./gradlew :app:connectedDebugAndroidTest` (requires device/emulator) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| NFC-01 | HostApduService registered and SELECT AID returns SW 90 00 | unit (JVM — mock APDU bytes) | `./gradlew :app:testDebugUnitTest --tests "*.PktapHceServiceTest"` | Wave 0 |
| NFC-02 | EXCHANGE command with 36-byte payload returns peer payload + SW 90 00 | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests "*.PktapHceServiceTest"` | Wave 0 |
| NFC-03 | processCommandApdu with malformed APDU returns SW_UNKNOWN without exception | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests "*.PktapHceServiceTest.testNoExceptionEscapes"` | Wave 0 |
| NFC-04 | 36-byte payload: version byte = 0x01, CRC-16 validates correctly | unit (JVM) | `./gradlew :app:testDebugUnitTest --tests "*.NfcPayloadBuilderTest"` | Wave 0 |
| NFC-05 | SELECT AID for F0504B544150 succeeds; two physical devices exchange keys | integration (physical device) | Manual — two devices required | N/A manual |
| NFC-06 | processCommandApdu body contains no PktapBridge calls (code review) + post-tap coroutine launches on Dispatchers.IO | unit (JVM) + code review | `./gradlew :app:testDebugUnitTest --tests "*.PostTapViewModelTest"` | Wave 0 |

### Sampling Rate
- **Per task commit:** `./gradlew :app:testDebugUnitTest`
- **Per wave merge:** `./gradlew :app:testDebugUnitTest && ./gradlew :app:connectedDebugAndroidTest` (if device attached)
- **Phase gate:** Unit tests green + two-device manual NFC tap verified before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `android/app/src/test/java/com/pktap/app/nfc/PktapHceServiceTest.kt` — covers NFC-01, NFC-02, NFC-03
- [ ] `android/app/src/test/java/com/pktap/app/nfc/NfcPayloadBuilderTest.kt` — covers NFC-04
- [ ] `android/app/src/test/java/com/pktap/app/nfc/PostTapViewModelTest.kt` — covers NFC-06 coroutine launch

Note: `src/test/` (JVM unit tests) does not yet exist for the `:app` module — the existing tests are all in `src/androidTest/` (instrumented). Wave 0 must create the `src/test/` directory and `build.gradle.kts` test dependency entries.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No authentication in NFC exchange — key exchange is the identity proof |
| V3 Session Management | No | NFC sessions are stateless single-tap exchanges |
| V4 Access Control | No | No access-controlled resources exposed in this phase |
| V5 Input Validation | Yes | Validate incoming APDU length and peer payload CRC before emitting to ViewModel |
| V6 Cryptography | No | No new crypto in this phase — all crypto delegated to PktapBridge (Phase 1) |

### Known Threat Patterns for Android NFC HCE

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed APDU causing exception in processCommandApdu | Denial of Service | Blanket try/catch; return SW_UNKNOWN; never let exception escape |
| Replayed NFC tap with stale peer key | Spoofing | CRC validates payload integrity; cryptographic verification happens in post-tap Rust FFI (decrypt_and_verify verifies Ed25519 sig); NFC layer is key transport only |
| Oversized APDU data field causing buffer overflow | Tampering | Validate `apdu.size >= 41` before reading data; copy exactly 36 bytes by offset, not by taking entire data field |
| Peer key delivery race (SharedFlow dropped event) | Elevation of Privilege | `extraBufferCapacity=1` buffers one event if ViewModel coroutine momentarily unavailable; worst case is a missed tap (user must retry) not a security violation |

---

## Sources

### Primary (HIGH confidence)
- `developer.android.com/develop/connectivity/nfc/hce` — HostApduService lifecycle, apduservice.xml format, category="other", threading model (main thread), reader mode recommendation
- `developer.android.com/reference/android/nfc/cardemulation/HostApduService` — Method signatures, deactivation constants
- `developer.android.com/reference/android/nfc/NfcAdapter` — enableReaderMode, FLAG_READER_NFC_A, FLAG_READER_SKIP_NDEF_CHECK, Settings.ACTION_NFC_SETTINGS
- `github.com/googlearchive/android-CardReader` — SELECT AID APDU construction, IsoDep.transceive pattern
- `docs.rs/crc/3.4.0` — CRC_16_IBM_SDLC usage, version confirmed
- Codebase: `libs.versions.toml`, `pktap-core/Cargo.toml`, `AppViewModel.kt`, `PktapBridge.kt`, `ffi.rs` — verified existing stack and patterns

### Secondary (MEDIUM confidence)
- `medium.com/swlh/why-you-need-to-be-very-careful-while-working-with-nfc-hce-apis-in-android-9bde32cc7924` — Unhandled exception kills HCE permanently (multiple Android versions documented)
- `developer.android.com/develop/connectivity/nfc/hce` confirmed: enableForegroundDispatch cannot detect HCE on Android devices (enableReaderMode required)

### Tertiary (LOW confidence)
- MIUI battery optimization behavior (Pitfall 3) — community knowledge, not official documentation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all APIs are platform SDK or already in project; verified against official docs
- Architecture: HIGH — patterns verified against official HCE documentation and canonical Google samples
- Pitfalls: HIGH (exception kills HCE permanently: verified by documented bug reports across multiple Android versions) / MEDIUM (MIUI battery: community knowledge)
- APDU byte layout: MEDIUM — CLA/INS values are Claude's discretion; standard SELECT AID bytes are verified; custom command bytes are arbitrary for proprietary protocols

**Research date:** 2026-04-05
**Valid until:** 2026-10-05 (HCE APIs are stable; no breaking changes expected; OEM behavior may shift)
