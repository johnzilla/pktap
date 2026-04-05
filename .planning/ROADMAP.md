# Roadmap: PKTap

## Overview

PKTap is built bottom-up, with each layer independently testable before the next is added on top. The Rust crypto core is established first because every API decision there cascades into the FFI surface, the Android build pipeline, and ultimately the user flows. Phases 1-3 lay the invisible foundation (Rust, DHT, bindings). Phases 4-5 add the Android-side security infrastructure (Keystore, NFC). Phase 6 wires everything into the full tap-to-contact flow with UI. Phase 7 adds QR fallback and public mode to complete all v1 requirements.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Rust Crypto Core** - All cryptographic primitives in Rust, unit-tested in isolation, with zeroize memory safety
- [ ] **Phase 2: Pkarr DHT Integration** - DHT publish/resolve in pure Rust with record construction, size validation, and offline queuing
- [ ] **Phase 3: UniFFI Bridge + Android Build** - Working .aar artifact, generated Kotlin bindings, and proven FFI hello-world call
- [ ] **Phase 4: Android Keystore Module** - Hardware-backed key generation, HKDF seed management, StrongBox/TEE fallback, and BIP-39 mnemonic display
- [ ] **Phase 5: NFC HCE Module** - Bidirectional public key exchange via HostApduService, single-APDU protocol, OEM-compatible AID handling
- [ ] **Phase 6: App Integration + Core UI** - Full tap-to-contact flow wired end-to-end: tap -> ECDH -> encrypt -> DHT publish -> resolve -> decrypt -> display -> save
- [ ] **Phase 7: QR Fallback + Public Mode** - QR code display/scan with async DHT handshake, public mode opt-in, and TTL expiry UI

## Phase Details

### Phase 1: Rust Crypto Core
**Goal**: The Rust pktap-core library is fully tested and its API surface is finalized — key conversion, ECDH, encryption, signing, decryption, and verification all work correctly with zeroize memory safety
**Depends on**: Nothing (first phase)
**Requirements**: CRYPTO-01, CRYPTO-02, CRYPTO-03, CRYPTO-04, CRYPTO-05, CRYPTO-06, CRYPTO-07, KEY-06
**Success Criteria** (what must be TRUE):
  1. `cargo test` passes with 100% of crypto paths covered — Ed25519/X25519 conversion, ECDH+HKDF, XChaCha20-Poly1305 encrypt/decrypt, Ed25519 sign/verify
  2. A malformed peer public key input is rejected with a typed error, not a panic or all-zero shared secret
  3. All secret material (shared secrets, derived keys) is wrapped in `ZeroizeOnDrop` types and drops correctly after each test
  4. The composite FFI-facing functions (`ecdhAndEncrypt`, `decryptAndVerify`) exist as single entry points — no raw secret material exposed as intermediate return values
  5. `curve25519-dalek` version resolves without conflict across `ed25519-dalek`, `x25519-dalek`, and `pkarr` workspace members
**Plans:** 3 plans

Plans:
- [x] 01-01-PLAN.md — Workspace setup, error types, key conversion, ECDH+HKDF key derivation
- [x] 01-02-PLAN.md — XChaCha20-Poly1305 cipher, Ed25519 signing, DNS record name derivation
- [x] 01-03-PLAN.md — Composite FFI functions (ecdh_and_encrypt, decrypt_and_verify) and pipeline integration test

### Phase 2: Pkarr DHT Integration
**Goal**: The DhtClient Rust module can publish a signed encrypted record to Mainline DHT and resolve it back — the deterministic address derivation, size budget enforcement, offline queuing, and TTL handling all work before any Android code touches them
**Depends on**: Phase 1
**Requirements**: DHT-01, DHT-02, DHT-03, DHT-04, DHT-05, DHT-06, DHT-07, DHT-08
**Success Criteria** (what must be TRUE):
  1. An integration test publishes a signed record and resolves it back using the deterministic `_pktap._share.<SHA-256(sort(A_pk, B_pk))>` address
  2. A record exceeding the ~858 usable byte budget is rejected before publish with a descriptive error
  3. BEP-44 sequence numbers are monotonically increasing unix timestamps — a second publish with an older seq is rejected
  4. Offline queuing test: publish is enqueued when DHT is unreachable and completes after connectivity is restored
**Plans**: TBD

### Phase 3: UniFFI Bridge + Android Build
**Goal**: The Rust pktap-core builds as an .aar, Kotlin bindings are generated and importable, and a hello-world FFI call proves the pipeline before any real crypto is wired through it
**Depends on**: Phase 1
**Requirements**: FFI-01, FFI-02, FFI-03
**Success Criteria** (what must be TRUE):
  1. `./gradlew assembleDebug` succeeds with the Rust .aar bundled — no manual steps required
  2. A Kotlin Android unit test calls a Rust function via UniFFI bindings and gets back the expected result
  3. ByteArray secrets returned from FFI calls are zeroed (`.fill(0)`) immediately after use in the bridge layer — verifiable by code review and test
**Plans**: TBD

### Phase 4: Android Keystore Module
**Goal**: The app generates hardware-backed keys on first launch, seals the HKDF seed in EncryptedSharedPreferences, displays the BIP-39 mnemonic, and handles StrongBox/TEE fallback transparently across all supported device types
**Depends on**: Phase 3
**Requirements**: KEY-01, KEY-02, KEY-03, KEY-04, KEY-05
**Success Criteria** (what must be TRUE):
  1. On first launch, the app generates an Ed25519 keypair and AES-256-GCM key in the Android Keystore — both non-extractable — without crashing on a device without StrongBox (emulator is acceptable for TEE path)
  2. The BIP-39 mnemonic screen displays 12/24 words and cannot be bypassed — it is shown at first launch and the words are never written to a log
  3. The HKDF seed survives an app restart — unseal with the Keystore AES key returns the same 32 bytes
  4. On a device without StrongBox, key generation falls back to TEE silently — no error is shown to the user
**Plans**: TBD
**UI hint**: yes

### Phase 5: NFC HCE Module
**Goal**: Two phones running the app can exchange their 32-byte Ed25519 public keys via NFC tap using a single APDU round-trip — the APDU handler returns within 300ms, crypto runs in a post-tap coroutine, and the flow works on Samsung One UI and Xiaomi MIUI in addition to Pixel
**Depends on**: Phase 4
**Requirements**: NFC-01, NFC-02, NFC-03, NFC-04, NFC-05, NFC-06
**Success Criteria** (what must be TRUE):
  1. Two physical devices (including at least one non-Pixel) successfully exchange 36-byte public key payloads via NFC tap — both apps receive the peer's key
  2. The `processCommandApdu()` method contains no crypto calls, no Rust FFI calls, and no network I/O — returns pre-cached payload within 300ms
  3. SELECT AID is handled correctly — NFC routing works on a Samsung device without requiring any manual AID configuration
  4. Post-tap operations (ECDH, encryption, DHT publish) run in a background coroutine that launches after `onDeactivated()`, not inside the APDU handler
**Plans**: TBD

### Phase 6: App Integration + Core UI
**Goal**: A user can complete the full PKTap flow end-to-end: set up a profile, tap phones with another PKTap user, see a decrypted contact preview, save the contact, and view it in a contact list — all encrypted, all without a server
**Depends on**: Phase 5
**Requirements**: PROF-01, PROF-02, PROF-03, PROF-04, PROF-05, PROF-06, PROF-07, PROF-08, UX-01, UX-02, UX-03, UX-04
**Success Criteria** (what must be TRUE):
  1. A user creates a profile with a name and at least one contact field, selects which fields to share, and the selection is preserved across app restarts
  2. After a two-device NFC tap, both devices show a contact preview with the other person's chosen fields within 10 seconds
  3. A user can save a received contact, find it in the contact list, see its TTL expiry label, and manually refresh it from DHT
  4. When the DHT publish is queued offline, the app shows a "pending sync" indicator and completes the publish when connectivity returns
  5. NFC errors show an actionable message ("Hold phones back-to-back") — no silent failures
**Plans**: TBD
**UI hint**: yes

### Phase 7: QR Fallback + Public Mode
**Goal**: Users without NFC or on different platforms can complete the same encrypted contact exchange via QR code, and users who want public discoverability can opt their profile into public mode with plaintext DHT records
**Depends on**: Phase 6
**Requirements**: QR-01, QR-02, QR-03, QR-04, QR-05, QR-06, PUB-01, PUB-02, PUB-03, PUB-04
**Success Criteria** (what must be TRUE):
  1. Alice displays a QR code, Bob scans it, Bob's key is published to `_pktap._handshake.<hash>`, Alice's app resolves it within 2 polling cycles, and both proceed to the standard encrypted exchange
  2. The QR scan of a non-PKTap code does not expose any contact data — it resolves to a web URL only
  3. A user who opts into Public Mode can have their contact info resolved by any app given their public key, without the two users having tapped first
  4. Public mode records auto-republish before the 7-day TTL expires — user never needs to manually re-publish to stay discoverable
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Rust Crypto Core | 0/3 | Planned | - |
| 2. Pkarr DHT Integration | 0/TBD | Not started | - |
| 3. UniFFI Bridge + Android Build | 0/TBD | Not started | - |
| 4. Android Keystore Module | 0/TBD | Not started | - |
| 5. NFC HCE Module | 0/TBD | Not started | - |
| 6. App Integration + Core UI | 0/TBD | Not started | - |
| 7. QR Fallback + Public Mode | 0/TBD | Not started | - |
