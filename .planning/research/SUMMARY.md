# Project Research Summary

**Project:** PKTap
**Domain:** Decentralized encrypted NFC contact exchange (Android)
**Researched:** 2026-04-04
**Confidence:** MEDIUM-HIGH

## Executive Summary

PKTap is a privacy-first Android app that uses NFC tap to exchange encrypted contact information between two devices, with no servers, no accounts, and no persistent infrastructure beyond a public DHT. The recommended approach is a Rust crypto core (via UniFFI FFI bindings) handling all cryptographic operations in memory-safe, zeroizing code, with an Android Kotlin layer managing UI, NFC HCE, and hardware-backed key storage. The DHT rendezvous mechanism is the most novel architectural element: both parties independently derive the same deterministic record address from their exchanged public keys, publish their encrypted contact data there, and resolve each other's records — all without a coordinator. This makes the system genuinely serverless, but also means reliability depends on DHT reachability and a functioning NFC exchange.

The key technical risks are centered on the intersection of memory safety (JVM cannot zero secrets), hardware variance (NFC HCE behavior differs significantly across Android OEMs, especially Samsung and Xiaomi), and DHT constraints (1000-byte record limit that is tighter than it appears, UDP firewall blocking). The most dangerous architectural mistake is letting crypto operations happen inside the NFC APDU handler — the NFC field timeout (~300ms) means `processCommandApdu()` must return a raw public key immediately, with all ECDH, encryption, and DHT publish deferred to a post-tap coroutine. Get this wrong and the core user interaction silently fails on most real-world devices.

The recommended build order is strict bottom-up: Rust crypto core first (unit-tested in isolation), then Pkarr DHT integration, then UniFFI bindings and Android build plumbing, then Android Keystore module, then NFC HCE, then app integration and UI, and finally QR fallback and polish. Each layer is independently testable before the next is built on top. Skipping ahead — particularly trying to wire NFC before the Rust core is proven — invites compound debugging problems where crypto issues, FFI issues, and NFC issues are indistinguishable.

---

## Key Findings

### Recommended Stack

The stack is a two-language system: a Rust workspace (`pktap-core` library crate + `uniffi-bindgen` binary crate) handling all crypto and DHT operations, and an Android Kotlin app using Jetpack Compose, Room, CameraX, and Android NFC APIs. The two layers connect via UniFFI 0.28.x using the proc-macro API (not UDL files). All [VERIFY] annotations in STACK.md must be checked against crates.io and Maven Central before pinning — research was conducted from training data, not live registry queries.

The most critical dependency constraint is `curve25519-dalek = "4"` — both `ed25519-dalek 2.x` and `x25519-dalek 2.x` require this version, and if `pkarr` pulls an older version the workspace will fail to resolve. This must be pinned explicitly in the Cargo workspace root.

**Core technologies:**
- `pkarr 2.3.x`: Mainline DHT publish/resolve with DNS TXT encoding — the only maintained Rust implementation of the Pkarr protocol
- `ed25519-dalek 2.x` + `x25519-dalek 2.x` + `curve25519-dalek 4.x`: RustCrypto standard for identity signing and ECDH; share backend (must be version-compatible)
- `chacha20poly1305 0.10.x`: XChaCha20-Poly1305 AEAD — preferred over AES-GCM for its 192-bit nonce and lack of timing-attack surface on devices without AES hardware
- `uniffi 0.28.x`: Mozilla's FFI binding generator; proc-macro API eliminates most UDL boilerplate; generates both Kotlin and Swift from the same Rust
- `zeroize 1.7.x`: Mandatory for all secret material; `ZeroizeOnDrop` derive makes zeroing automatic
- `bip39 2.0.x`: BIP-39 mnemonic generation and recovery from seed bytes
- Jetpack Compose BOM 2024.09.xx + Material3: Declarative Android UI, all Compose library versions pinned via BOM
- Room 2.6.x + KSP: SQLite ORM with reactive Flow queries; KSP over kapt for faster incremental builds
- ML Kit Barcode Scanning 17.3.x: On-device QR decode integrated with CameraX — faster and more accurate than ZXing
- `androidx.security:security-crypto 1.1.0-alpha06`: EncryptedSharedPreferences backed by Android Keystore AES-256-GCM for HKDF seed storage
- `cargo-ndk 3.5.x`: Cross-compiles Rust to Android targets (arm64-v8a, armeabi-v7a, x86_64)
- Kotlin 2.0.x + AGP 8.5.x + Gradle 8.8+: Current stable Android build toolchain; KMP skeleton in `:core` module from day one

**Explicitly excluded:** SQLCipher (unnecessary overhead when column-level Keystore encryption is used), ZXing Android Embedded (superseded by ML Kit), JVM crypto for any secret operations, cloud sync, analytics, centralized relay servers.

### Expected Features

**Must have (table stakes):**
- NFC tap initiates bidirectional key exchange — the entire product premise
- Contact preview screen before saving — every NFC/QR card app provides this; absence feels broken
- Explicit save/reject decision — auto-save is a dark pattern for this audience
- Contact list (received contacts) — users need to find people after tapping
- Profile setup with field selection — user must configure own card before sharing
- QR code fallback — NFC fails on cases and some devices; QR is a universal secondary gesture
- Clear NFC error messages — silent failures are the top NFC UX complaint
- Offline-capable with "pending sync" state — DHT is not always reachable
- "Expires in Xh" label and manual refresh — TTL expiry without UI notice feels like data loss (research recommends pulling this into MVP despite PROJECT.md listing it as v0.2)

**Should have (differentiators):**
- End-to-end encryption with no server in the path — categorically absent from all mainstream NFC card apps
- No account required — zero signup friction
- BIP-39 seed backup — self-sovereign key recovery, normalized UX in the DID/Nostr space
- Deterministic DHT address — serverless rendezvous, invisible to users but meaningful architecture
- Encrypted-by-default with public as opt-in — opposite of Popl/Linq default
- Android Keystore / StrongBox key binding — hardware-backed, non-extractable identity
- Memory zeroing of all secrets — defense-in-depth, valued by security-conscious audience

**Defer to v0.2+:**
- Multi-context profiles (work vs personal HKDF derivation)
- Forward secrecy via ephemeral keys
- Background auto-republish for public mode
- Export as vCard
- NFC tag programming (sticker write)
- Key verification / "last verified" timestamp (LOW complexity, but not blocking MVP)

**Anti-features (never build):** cloud sync, analytics/telemetry, social graph, profile photos, centralized relay, FCM push notifications, auto-save received contacts, contact import from address book, web dashboard, mandatory identity linking.

### Architecture Approach

The architecture is a strict three-tier system with clearly enforced component boundaries: Compose UI talks only to ViewModels, ViewModels orchestrate the NFC Module, Keystore Module, and PktapCore FFI Bridge, and the FFI Bridge is the only component that crosses into the Rust layer. The Rust layer is itself divided into CryptoOps (all cryptographic primitives), RecordBuilder (Pkarr DNS TXT record construction and parsing), DhtClient (pkarr crate wrapper), and KeyManager (key derivation and lifecycle). Two separate key lineages exist: a Keystore-backed non-extractable Ed25519 key for signing records, and an HKDF-derived X25519 key reconstructed from seed per session for ECDH. The FFI surface is intentionally narrow — composite operations like `ecdhAndEncrypt` are single FFI calls rather than exposing individual primitives, minimizing the time secret material exists as JVM heap objects.

**Major components:**
1. **Rust pktap-core** — all crypto, DHT, record construction; never touches Android APIs; fully testable with `cargo test`
2. **PktapCore FFI Bridge (Kotlin)** — thin UniFFI wrapper; ByteArray marshaling, error mapping to sealed classes, mandatory zeroing after each FFI call
3. **NFC Module** — `HostApduService` (HCE) + reader mode; exchanges public keys only (36 bytes); no crypto in APDU handler
4. **Keystore Module** — generates/uses hardware-backed Ed25519 signing key and AES-256-GCM seed-encryption key; handles StrongBox/TEE fallback
5. **ViewModel / AppCoordinator** — orchestrates the post-tap coroutine: unseal seed, ECDH+encrypt via Rust, DHT publish, DHT resolve, decrypt, display
6. **Room + SQLite** — contacts persistence with Keystore-managed AES column encryption for sensitive fields
7. **Compose UI** — renders state from ViewModels; ProfileSetupScreen, TapScreen, ContactListScreen, QRScreen, MnemonicScreen

### Critical Pitfalls

1. **UniFFI copies every ByteArray — secrets linger in JVM heap** — Design Rust API so secrets never cross the FFI boundary in raw form. `ecdhAndEncrypt` takes plaintext and returns ciphertext; the derived key never becomes a JVM `ByteArray`. Where raw material must cross (BIP-39 mnemonic display), call `.fill(0)` immediately after use. Disable heap dump collection in crash reporters for crypto screens.

2. **NFC HCE timeout (~300ms) — crypto in `processCommandApdu()` kills the tap** — The APDU handler must do nothing except return the pre-cached 36-byte public key payload and immediately return `[0x90, 0x00]`. All ECDH, encryption, and DHT publish happen in a coroutine after `onDeactivated()`. Any Rust FFI call or network access inside `processCommandApdu()` will cause `TagLostException` on real devices.

3. **Missing SELECT AID causes silent routing failure on Samsung/Xiaomi** — Implement full ISO 7816-4 APDU state machine from the first commit: SELECT AID handler → custom INS data commands → status word responses (9000/6700/6A80). The reader side must call `IsoDep.transceive(SELECT_AID_BYTES)` as the very first APDU. Test on Samsung One UI and Xiaomi MIUI early — Pixel tests give a false sense of compatibility.

4. **Android Keystore StrongBox unavailable on most mid-range devices** — Wrap `setIsStrongBoxBacked(true)` in try/catch for `StrongBoxUnavailableException` and retry with `false` (TEE fallback) from day one. Emulators never have StrongBox. The TEE fallback is the common case, not the exception.

5. **Biometric enrollment change permanently invalidates the Keystore AES key** — Use `setInvalidatedByBiometricEnrollment(false)` for the AES key protecting the HKDF seed. Implement `KeyPermanentlyInvalidatedException` handler that prompts BIP-39 mnemonic recovery. Make the mnemonic backup screen impossible to skip. Document this behavior in onboarding.

6. **Pkarr record size limit is 1000 bytes total including overhead** — After signature (64 bytes), sequence number (8 bytes), DNS wire format (~30 bytes), AEAD overhead (40 bytes), usable plaintext is ~858 bytes. Implement a byte-budget checker in Rust before serialization. Surface "contact record too large" to UI. Test with maximum-field records, not just "Alice" + phone number.

7. **Ed25519 → X25519 conversion with malformed peer key produces all-zero shared secret** — Use `ed25519-dalek`'s `to_montgomery()` method (not a manual formula). Validate every received peer public key with `VerifyingKey::from_bytes()` before ECDH. Assert `shared_secret != [0u8; 32]` in every test.

---

## Implications for Roadmap

Research strongly confirms the 7-phase build order suggested in ARCHITECTURE.md. Each phase is independently testable and each is a prerequisite for the next. The ordering is driven by dependency chains, not arbitrary priority.

### Phase 1: Rust Crypto Core Foundation
**Rationale:** Everything else depends on the crypto layer being correct and its API boundaries being right. Getting the UniFFI surface wrong (exposing raw secrets, wrong function composition) requires rearchitecting every later layer. This is the highest-leverage phase to get right first.
**Delivers:** `pktap-core` Rust library with 100% test coverage on crypto paths; key types, Ed25519/X25519 conversion, ECDH+HKDF, XChaCha20-Poly1305 encrypt/decrypt, all with zeroize wrappers
**Addresses:** Identity foundation (FEATURES.md MVP item 1)
**Avoids:** Pitfall 1 (secrets as JVM heap objects), Pitfall 2 (malformed key ECDH), Pitfall 9 (nonce reuse), Pitfall 10 (HKDF info collision), Pitfall 16 (zeroize + async clones)

### Phase 2: Pkarr DHT Integration (within Rust)
**Rationale:** DHT is the transport layer for contact records; must be proven in pure Rust before crossing the FFI boundary where debugging is harder. Record size budget must be validated here before any UI is built around it.
**Delivers:** `DhtClient` wrapping pkarr; `publishRecord()` + `resolveRecord()`; record name derivation (SHA-256 + base32); integration test: publish + resolve round-trip
**Addresses:** Encrypted exchange core (FEATURES.md MVP item 5)
**Avoids:** Pitfall 7 (record size limit), Pitfall 8 (DHT bootstrap unreachability), Pitfall 14 (sequence number staleness)

### Phase 3: UniFFI Bindings + Android Build
**Rationale:** The FFI boundary is where the two language environments meet. Establishing a working `.aar` artifact and the `PktapCore.kt` bridge early allows the Keystore and NFC modules to be built and tested in parallel.
**Delivers:** `pktap.udl` interface definition; generated Kotlin bindings; Android Gradle config for AAR packaging; `PktapCore.kt` bridge with error mapping and ByteArray zeroing; Android unit tests proving Rust calls work from JVM
**Avoids:** Pitfall 1 (secrets as JVM heap objects — the bridge enforces zeroing discipline), Pitfall 15 (UniFFI naming collisions)

### Phase 4: Android Keystore Module
**Rationale:** Key management is the security foundation for all Android-side operations. StrongBox/TEE fallback logic must be correct before anything depends on it. BIP-39 mnemonic display and mandatory backup happen here.
**Delivers:** `KeystoreWrapper.kt` with Ed25519 signing key + AES-256-GCM seed encryption key generation; first-launch key generation flow; HKDF seed seal/unseal; BIP-39 mnemonic display with `FLAG_SECURE`
**Addresses:** Keypair generation + BIP-39 backup (FEATURES.md MVP items 1, 2)
**Avoids:** Pitfall 5 (StrongBox crash), Pitfall 6 (biometric enrollment invalidation), Pitfall 13 (mnemonic in Recent Apps screenshot)

### Phase 5: NFC HCE Module
**Rationale:** NFC HCE is the most hardware-dependent and OEM-variant component. It must be built and tested on physical devices (Samsung, Xiaomi, not just Pixel) before the full app integration, because NFC bugs are infrastructure bugs that can't be patched at the app layer.
**Delivers:** `HostApduService` with AID registration and full ISO 7816-4 APDU state machine; NFC reader mode with `enableReaderMode()`; single-APDU bidirectional key exchange protocol (Alice sends pubkey in command, Bob returns pubkey in response); `LocalBroadcast` delivery to ViewModel
**Addresses:** NFC tap exchange (FEATURES.md MVP item 4)
**Avoids:** Pitfall 3 (missing SELECT AID), Pitfall 4 (crypto in APDU handler)

### Phase 6: App Integration + Core UI
**Rationale:** This phase wires all previous layers into the full tap flow. All the hard infrastructure (Rust, DHT, FFI, Keystore, NFC) is already tested; this phase is about orchestration and UI correctness.
**Delivers:** Full data flow: tap → ECDH → encrypt → DHT publish; full resolve flow: DHT resolve → verify → decrypt → display; `ContactListScreen` + Room persistence; `ProfileSetupScreen` + field selection; `TapScreen` ViewModel; end-to-end integration test on two physical devices
**Addresses:** FEATURES.md MVP items 3, 4, 5, 6, 7; NFC error UX (item 9)
**Avoids:** Pitfall 12 (AES key cached in heap — Room integration must use Keystore reference inline)

### Phase 7: QR Fallback + Polish
**Rationale:** QR fallback depends on the NFC tap flow being stable (same ECDH/DHT logic, different key exchange mechanism). Polish and TTL expiry UI are last because they require the full flow to exist first.
**Delivers:** QR display (ZXing core for generation, ML Kit for scan); async handshake polling with exponential backoff; "Expires in Xh" label and manual refresh; public mode opt-in; UX polish, error states, loading indicators
**Addresses:** FEATURES.md MVP items 8, 9; TTL expiry UI (recommended for MVP by FEATURES.md)
**Avoids:** Pitfall 11 (QR polling battery drain)

### Phase Ordering Rationale

- Rust before Android: The crypto primitives must be correct and their API surface finalized before any Kotlin code depends on them. Changing the UniFFI surface after Keystore and NFC modules are built is expensive.
- Keystore before NFC: The `HostApduService` pre-caches the public key payload before the tap. That payload requires the Keystore module to be operational (keypair exists, can derive public key). Parallel development is possible but the Keystore module must be complete before NFC integration testing.
- NFC before app integration: NFC is the riskiest hardware integration. Discovering OEM-specific routing bugs during full app integration adds debugging noise. Isolate and solve NFC first.
- QR last: QR fallback uses the same post-tap ECDH/DHT flow. Building it last avoids maintaining two parallel untested paths through the crypto layer.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (Pkarr DHT):** Pkarr is a relatively new library with an evolving API. Verify current API surface against the crates.io releases and the pubky/pkarr GitHub before writing the `DhtClient` wrapper. The 1000-byte record format overhead calculation in PITFALLS.md is MEDIUM confidence — measure empirically in Phase 2.
- **Phase 3 (UniFFI bindings):** No official Gradle plugin exists for UniFFI. The `exec {}` pattern shown in STACK.md is the community standard but may need adjustment based on the specific UniFFI version and project layout.
- **Phase 5 (NFC HCE):** OEM-specific HCE routing behavior (Samsung One UI, Xiaomi MIUI) is MEDIUM confidence — documented in community reports, not official Android docs. Acquire physical test devices before starting Phase 5.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Rust crypto):** RustCrypto crates (`ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`, `hkdf`, `zeroize`) are well-documented with stable APIs.
- **Phase 4 (Android Keystore):** Android Keystore system is thoroughly documented; StrongBox fallback pattern is well-established.
- **Phase 6 (App integration):** Jetpack Compose, Room, ViewModel/StateFlow patterns are mature with extensive official documentation.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | All version numbers from training data (cutoff Aug 2025); every version marked [VERIFY] must be checked against live registries before pinning. Core technology choices (UniFFI, RustCrypto, Compose, Room) are HIGH confidence. Pkarr specifically is MEDIUM — newer library. |
| Features | MEDIUM-HIGH | Anti-features and differentiators are HIGH (derived from stated architecture constraints). Table stakes are MEDIUM (inferred from competitive analysis without live verification). Feature dependency graph is HIGH. |
| Architecture | HIGH | UniFFI/Rust patterns, NFC HCE lifecycle, Android Keystore integration are all from well-documented official sources. Pkarr-specific record format details are MEDIUM. |
| Pitfalls | MEDIUM-HIGH | JNI copy behavior (HIGH), Android Keystore constraints (HIGH), BEP-44 spec claims (HIGH). NFC timeout values and Samsung/Xiaomi HCE routing behavior (MEDIUM — community-sourced). |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **All [VERIFY] version numbers:** Must be checked against crates.io and Maven Central before any dependency is pinned. No live registry queries were performed during research. Start Phase 1 with a `cargo update` pass and a Gradle dependency check.
- **Pkarr current API surface:** The `pkarr` crate API may have evolved since knowledge cutoff. Check the pubky/pkarr GitHub for current `publish()` and `resolve()` signatures and the recommended `seq` handling pattern before writing `DhtClient`.
- **Pkarr record overhead empirical measurement:** The 858-byte usable plaintext estimate in PITFALLS.md is computed from spec, not measured. Write a test that serializes a record and measures the actual wire size before setting field length limits in the UI.
- **Curve25519 in Android Keystore (API 33+):** ARCHITECTURE.md notes that native Ed25519 key generation in the Keystore requires API 33+. For API 26-32 devices, the fallback is generating Ed25519 in Rust and importing sealed. This code path needs explicit test coverage on an emulator at API 31-32.
- **DHT UDP reachability on cellular networks:** PITFALLS.md flags this as MEDIUM confidence. Test Pkarr publish/resolve over a mobile hotspot (not WiFi) as part of Phase 2 integration testing. This will determine whether a timeout-based fallback UX needs to be designed before Phase 6.

---

## Sources

### Primary (HIGH confidence)
- PROJECT.md (first-party constraints and decisions)
- Android NFC HCE documentation: https://developer.android.com/guide/topics/connectivity/nfc/hce
- Android Keystore system: https://developer.android.com/training/articles/keystore
- UniFFI documentation: https://mozilla.github.io/uniffi-rs/
- BEP-44 (Mainline DHT mutable items): http://www.bittorrent.org/beps/bep_0044.html
- RustCrypto crates documentation (ed25519-dalek, x25519-dalek, chacha20poly1305, zeroize, hkdf)
- HKDF RFC 5869: https://datatracker.ietf.org/doc/html/rfc5869
- Android Jetpack documentation: https://developer.android.com/jetpack/androidx/releases/

### Secondary (MEDIUM confidence)
- Pkarr crate: https://github.com/pubky/pkarr — newer library, API may have evolved
- Competitive analysis of Popl, Linq, HiHello, Dot, Blinq feature sets (training data through Aug 2025)
- Nostr contact/identity UX patterns (training data through Aug 2025)
- Samsung/Xiaomi HCE routing behavior — community-reported, not in official Android docs
- NFC field timeout ~300ms — practical community-observed value, not specified in Android docs

### Tertiary (LOW confidence)
- UniFFI Gradle integration pattern (`exec {}` approach) — community standard, no official plugin
- `pkarr` seq = unix_timestamp recommendation — from Pkarr project examples; verify against current docs

---
*Research completed: 2026-04-04*
*Ready for roadmap: yes*
