# Phase 5: NFC HCE Module - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Bidirectional NFC public key exchange via HostApduService (HCE). Two phones tap and swap 32-byte Ed25519 public keys using a single APDU round-trip. The APDU handler returns within 300ms with zero crypto — post-tap ECDH, encryption, and DHT publish run in a background coroutine. Must work on Samsung One UI and Xiaomi MIUI in addition to Pixel.

</domain>

<decisions>
## Implementation Decisions

### APDU Protocol Design
- **D-01:** Reader sends its 36-byte payload in a custom APDU command after SELECT AID. HCE host responds with its own 36-byte payload. Single round-trip: command = reader's key, response = host's key. Both phones get each other's key.
- **D-02:** Both phones run reader + HCE simultaneously. Whichever phone initiates the tap acts as reader, the other as HCE host. No user action needed to pick a role. Android supports this natively.

### HCE Service Lifecycle
- **D-03:** Pre-cache the 36-byte NDEF payload on app start from AppViewModel's cached pubkey (Phase 4 D-06). Build version(1) + flags(1) + Ed25519 pubkey(32) + CRC-16(2) once. `processCommandApdu()` returns the pre-built byte array — zero computation.
- **D-04:** HCE service communicates received peer key via SharedFlow. The active Activity/ViewModel collects it and navigates to the post-tap screen. Clean separation between service and UI.

### OEM Compatibility
- **D-05:** Register AID in `apduservice.xml` with `android:category="other"` and a proprietary AID (e.g., `F0504B544150` = hex for "PKTAP"). Avoids payment AID conflicts. Samsung/Xiaomi route "other" category reliably.
- **D-06:** Check `NfcAdapter.isEnabled()` before exchange flow. If disabled, show dialog linking to NFC settings. Don't block the rest of the app — QR fallback (Phase 7) covers no-NFC devices.

### Post-Tap Processing
- **D-07:** Post-tap coroutine launches in ViewModel scope on `Dispatchers.IO` after UI receives peer key via SharedFlow. ECDH + encrypt + DHT publish via PktapBridge. ViewModel scope survives config changes.
- **D-08:** Post-tap screen shows status progression: "Encrypting... Publishing... Done/Error". If DHT fails, show "Queued for sync" (offline queue from Phase 2 handles retry).

### Claude's Discretion
- Exact APDU command/response byte layout (CLA, INS, P1, P2 values)
- SharedFlow vs Channel for peer key delivery
- NFC reader mode dispatch configuration
- Whether to use a foreground service for NFC reliability
- CRC-16 implementation (Rust via FFI or Kotlin-side)
- Post-tap screen visual design

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Specifications
- `.planning/PROJECT.md` — Core constraints (NFC HCE bidirectional, 36-byte NDEF payload format)
- `.planning/REQUIREMENTS.md` §NFC Exchange — NFC-01 through NFC-06 acceptance criteria
- `CLAUDE.md` §Technology Stack — crc 3.x for CRC-16

### Prior Phase Code (reuse these)
- `android/app/src/main/java/com/pktap/app/AppViewModel.kt` — Cached pubkey bytes (D-06 from Phase 4)
- `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt` — FFI wrapper with zeroing
- `pktap-core/src/ffi.rs` — `ecdh_and_encrypt`, `decrypt_and_verify`, `derive_shared_record_name`
- `pktap-core/src/dht.rs` — `DhtClient::publish_encrypted` for post-tap DHT publish

### Prior Phase Decisions
- `.planning/phases/01-rust-crypto-core/01-CONTEXT.md` — D-03 (split signing model), D-04 (opaque byte blobs)
- `.planning/phases/04-android-keystore-module/04-CONTEXT.md` — D-05 (seed decrypt → bridge → zero), D-06 (pubkey cached in memory)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppViewModel.publicKeyBytes` — Cached Ed25519 pubkey, available for NFC payload construction
- `PktapBridge.ecdhAndEncrypt()` / `decryptAndVerify()` — Composite FFI calls for post-tap crypto
- `DhtClient::publish_encrypted()` — DHT publish with offline queue (Phase 2)
- `SeedRepository.decryptSeed()` — Seed access for ECDH in post-tap coroutine
- `AppNavigation.kt` — Compose Navigation with type-safe routes (extend with post-tap route)

### Established Patterns
- PktapBridge zeroing wrapper for all FFI calls with seed material
- ViewModel + StateFlow for reactive UI state
- Compose Navigation with sealed route classes
- `Dispatchers.IO` for all FFI and network operations

### Integration Points
- Phase 6 (App Integration) will wire NFC exchange into the full tap-to-contact flow with contact preview and save
- Phase 7 (QR Fallback) will use the same post-tap processing flow but triggered by QR scan instead of NFC

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches within the decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-nfc-hce-module*
*Context gathered: 2026-04-05*
