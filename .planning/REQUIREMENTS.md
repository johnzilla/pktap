# Requirements: PKTap

**Defined:** 2026-04-04
**Core Value:** Two people tap phones and instantly see each other's chosen contact info — encrypted end-to-end, stored nowhere but their devices and a temporary DHT record that expires.

## v1 Requirements

### Key Management

- [ ] **KEY-01**: App generates Ed25519 master keypair in Android Keystore (StrongBox/TEE), non-extractable
- [ ] **KEY-02**: App generates AES-256-GCM key in Keystore for local data encryption
- [ ] **KEY-03**: App generates random 32-byte HKDF seed, encrypts with Keystore AES key, stores in EncryptedSharedPreferences
- [ ] **KEY-04**: App displays BIP-39 mnemonic at first launch so user can back up their seed
- [ ] **KEY-05**: App falls back to TEE (non-StrongBox) Keystore on devices without StrongBox hardware
- [ ] **KEY-06**: All secret material (seed, derived keys, shared secrets) is zeroed from memory after use — Rust via zeroize crate, Kotlin via explicit ByteArray zeroing post-FFI

### Crypto Core (Rust)

- [ ] **CRYPTO-01**: Rust module performs Ed25519 to X25519 key conversion with input validation (reject malformed keys)
- [ ] **CRYPTO-02**: Rust module performs X25519 ECDH key agreement and derives encryption key via HKDF with domain separator "pktap-v1"
- [ ] **CRYPTO-03**: Rust module encrypts contact field payload with XChaCha20-Poly1305 (random 24-byte nonce, AEAD)
- [ ] **CRYPTO-04**: Rust module signs encrypted payload with Ed25519 key
- [ ] **CRYPTO-05**: Rust module decrypts and verifies signature on received encrypted records
- [ ] **CRYPTO-06**: Rust module constructs DNS TXT records for both encrypted and public mode with `_pktap.` namespace prefix
- [ ] **CRYPTO-07**: All crypto operations are composed inside Rust (e.g., ecdhAndEncrypt as one FFI call) — secret material never crosses FFI boundary

### DHT Integration

- [ ] **DHT-01**: App publishes signed encrypted record to Mainline DHT at deterministic address `_pktap._share.<SHA-256(sort(A_pk, B_pk))>`
- [ ] **DHT-02**: App resolves encrypted record from DHT by computing same deterministic address
- [ ] **DHT-03**: App publishes plaintext DNS TXT records for public mode at `_pktap._profile.<derived_key>`
- [ ] **DHT-04**: App resolves public mode records from DHT given a public key
- [ ] **DHT-05**: Encrypted records have default TTL of 24 hours; public records have default TTL of 7 days
- [ ] **DHT-06**: App uses monotonically increasing BEP-44 sequence numbers (unix timestamp) for record versioning
- [ ] **DHT-07**: App validates record payload fits within ~858 usable bytes (1000 byte BEP-44 limit minus wire format + AEAD overhead) before publish
- [ ] **DHT-08**: App queues DHT publish operations when offline and syncs when connectivity returns

### NFC Exchange

- [ ] **NFC-01**: App implements HostApduService (HCE) for bidirectional Ed25519 public key exchange
- [ ] **NFC-02**: NFC exchange uses single APDU round-trip — Alice's command contains her 32-byte key, Bob's response contains his 32-byte key
- [ ] **NFC-03**: APDU handler does zero crypto or network I/O — only copies 32 bytes and returns within 300ms
- [ ] **NFC-04**: NFC payload follows NDEF External Type format: version (1 byte) + flags (1 byte) + Ed25519 pubkey (32 bytes) + CRC-16 (2 bytes) = 36 bytes
- [ ] **NFC-05**: App handles SELECT AID APDU correctly for Samsung/Xiaomi HCE routing compatibility
- [ ] **NFC-06**: Post-tap crypto and DHT operations run in a background coroutine, not in the APDU handler

### QR Fallback

- [ ] **QR-01**: App displays QR code encoding `pktap://pk/<base32-pubkey>?mode=enc`
- [ ] **QR-02**: App scans QR codes via CameraX + ML Kit and extracts peer public key
- [ ] **QR-03**: Scanner publishes encrypted handshake to `_pktap._handshake.<SHA-256(displayer_PK)>` with Bob's key encrypted to Alice's key
- [ ] **QR-04**: Displayer polls `_pktap._handshake.<SHA-256(own_PK)>` every 2 seconds for up to 5 minutes
- [ ] **QR-05**: After handshake completes, both parties proceed to encrypted exchange (same as NFC post-tap)
- [ ] **QR-06**: Non-PKTap QR scan resolves to web page explaining the app (no contact data exposed)

### Profile & Contacts

- [ ] **PROF-01**: User can create a profile with display name and one or more contact fields (email, phone, social handles, URL)
- [ ] **PROF-02**: User can edit their profile fields at any time
- [ ] **PROF-03**: User selects which fields to share before each exchange
- [ ] **PROF-04**: User sees a contact preview after receiving a decrypted exchange with save/reject options
- [ ] **PROF-05**: User can save received contacts to local SQLite storage (sensitive columns AES-encrypted via Keystore)
- [ ] **PROF-06**: User can view a contact list showing saved contacts with "last verified" timestamps
- [ ] **PROF-07**: User can manually refresh a saved contact (re-resolve from DHT)
- [ ] **PROF-08**: Contact list shows "expires in Xh" label for TTL-governed records and warns before expiry

### Public Mode

- [ ] **PUB-01**: User can explicitly opt a profile into "Public Mode" (not the default)
- [ ] **PUB-02**: Public mode publishes plaintext DNS TXT records signed with context-derived key to DHT
- [ ] **PUB-03**: Anyone with the user's public key can resolve and read public mode contact info
- [ ] **PUB-04**: Public mode records auto-republish on schedule to stay alive (7-day TTL)

### Error Handling & UX

- [ ] **UX-01**: App shows clear NFC error messages with guidance ("Hold phones back-to-back", retry prompts)
- [ ] **UX-02**: App shows "pending sync" state when DHT publish is queued offline
- [ ] **UX-03**: First-tap explainer screen shows what was exchanged and who can see what
- [ ] **UX-04**: App shows padlock icon for encrypted mode, globe icon for public mode to visually distinguish sharing modes

### UniFFI Bridge

- [ ] **FFI-01**: Rust crypto core exposed to Kotlin via UniFFI proc-macro API (no UDL files)
- [ ] **FFI-02**: UniFFI bindings verified with a hello-world FFI call before building crypto operations on top
- [ ] **FFI-03**: Build pipeline uses cargo-ndk + custom Gradle exec task for UniFFI bindgen

## v2 Requirements

### Multi-Platform

- **PLAT-01**: iOS app with SwiftUI + Keychain/Secure Enclave via KMP shared module
- **PLAT-02**: Web fallback resolver for public profiles

### Advanced Identity

- **ID-01**: Multi-context profiles (Professional, Personal, Minimal) with HKDF key derivation per context
- **ID-02**: Forward secrecy via ephemeral X25519 keys in NFC payload (68-byte total)
- **ID-03**: Key restore from BIP-39 mnemonic on new device

### Enhanced Features

- **ENH-01**: Background auto-republish for public mode records
- **ENH-02**: Background contact re-resolution with staleness indicators
- **ENH-03**: Re-encrypt and republish for existing recipients on profile update
- **ENH-04**: Export contacts as vCard
- **ENH-05**: NFC tag programming (write public key to sticker)
- **ENH-06**: Contact expiry visual treatment with countdown and warning

## Out of Scope

| Feature | Reason |
|---------|--------|
| Profile photos | Pkarr 1000-byte record limit makes embedded images impractical |
| Cloud sync / backup | Destroys the "no server" guarantee; creates contact graph honeypot |
| Analytics / telemetry | Contradicts privacy promise; even aggregate metrics erode trust |
| Social graph / followers | Turns app into network product requiring servers; misaligns with decentralization |
| Push notifications (FCM) | Requires Google infrastructure, reveals IP, breaks no-cloud model |
| Auto-save received contacts | Consent is load-bearing for trust; silent save is a dark pattern |
| Contact import from phone | Increases attack surface and permission creep |
| Web dashboard / CRM | Competing with Popl/Linq on their turf; wrong audience |
| Identity verification (phone/email) | Breaks pseudonymity; the public key IS the identity |
| OAuth / social login | No accounts by design |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| KEY-01 | Phase 4 | Pending |
| KEY-02 | Phase 4 | Pending |
| KEY-03 | Phase 4 | Pending |
| KEY-04 | Phase 4 | Pending |
| KEY-05 | Phase 4 | Pending |
| KEY-06 | Phase 1 | Pending |
| CRYPTO-01 | Phase 1 | Pending |
| CRYPTO-02 | Phase 1 | Pending |
| CRYPTO-03 | Phase 1 | Pending |
| CRYPTO-04 | Phase 1 | Pending |
| CRYPTO-05 | Phase 1 | Pending |
| CRYPTO-06 | Phase 1 | Pending |
| CRYPTO-07 | Phase 1 | Pending |
| DHT-01 | Phase 2 | Pending |
| DHT-02 | Phase 2 | Pending |
| DHT-03 | Phase 2 | Pending |
| DHT-04 | Phase 2 | Pending |
| DHT-05 | Phase 2 | Pending |
| DHT-06 | Phase 2 | Pending |
| DHT-07 | Phase 2 | Pending |
| DHT-08 | Phase 2 | Pending |
| NFC-01 | Phase 5 | Pending |
| NFC-02 | Phase 5 | Pending |
| NFC-03 | Phase 5 | Pending |
| NFC-04 | Phase 5 | Pending |
| NFC-05 | Phase 5 | Pending |
| NFC-06 | Phase 5 | Pending |
| QR-01 | Phase 7 | Pending |
| QR-02 | Phase 7 | Pending |
| QR-03 | Phase 7 | Pending |
| QR-04 | Phase 7 | Pending |
| QR-05 | Phase 7 | Pending |
| QR-06 | Phase 7 | Pending |
| PROF-01 | Phase 6 | Pending |
| PROF-02 | Phase 6 | Pending |
| PROF-03 | Phase 6 | Pending |
| PROF-04 | Phase 6 | Pending |
| PROF-05 | Phase 6 | Pending |
| PROF-06 | Phase 6 | Pending |
| PROF-07 | Phase 6 | Pending |
| PROF-08 | Phase 6 | Pending |
| PUB-01 | Phase 7 | Pending |
| PUB-02 | Phase 7 | Pending |
| PUB-03 | Phase 7 | Pending |
| PUB-04 | Phase 7 | Pending |
| UX-01 | Phase 6 | Pending |
| UX-02 | Phase 6 | Pending |
| UX-03 | Phase 6 | Pending |
| UX-04 | Phase 6 | Pending |
| FFI-01 | Phase 3 | Pending |
| FFI-02 | Phase 3 | Pending |
| FFI-03 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 52 total
- Mapped to phases: 52
- Unmapped: 0

---
*Requirements defined: 2026-04-04*
*Last updated: 2026-04-04 after roadmap creation — all 52 requirements mapped*
