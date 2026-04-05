# PKTap

## What This Is

A privacy-first, decentralized contact exchange app for Android. Users tap phones over NFC to swap Ed25519 public keys, then the app handles ECDH key agreement, XChaCha20-Poly1305 encryption, and publishes encrypted contact records to the Mainline DHT via Pkarr. No accounts, no cloud, no middleman. The recipient's app resolves the same deterministic DHT address, decrypts, and displays the shared contact fields.

## Core Value

Two people tap phones and instantly see each other's chosen contact info — encrypted end-to-end, stored nowhere but their devices and a temporary DHT record that expires.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Generate Ed25519 master keypair in Android Keystore (StrongBox/TEE), non-extractable
- [ ] Generate AES-256-GCM key in Keystore for local data encryption
- [ ] Generate random HKDF seed, encrypt with Keystore AES key, store in EncryptedSharedPreferences
- [ ] Show BIP-39 mnemonic at first launch for seed backup/recovery
- [ ] Create a default contact profile with name and selectable contact fields
- [ ] Bidirectional NFC key exchange via HCE — both phones swap 32-byte Ed25519 public keys (36-byte NDEF payload with version + flags + CRC-16)
- [ ] Ed25519 to X25519 conversion, ECDH key agreement, KDF to derive encryption key (all in Rust)
- [ ] XChaCha20-Poly1305 encryption of selected contact fields with Ed25519 signature (Rust)
- [ ] Publish signed encrypted record to DHT at deterministic address `_pktap._share.<SHA-256(sort(A_pk, B_pk))>`
- [ ] Resolve encrypted record from DHT, verify signature, decrypt, display contact preview
- [ ] Full bidirectional round-trip: tap -> both phones show each other's selected contact fields
- [ ] Save resolved contacts locally in SQLite (sensitive columns AES-encrypted via Keystore)
- [ ] TTL-based record expiry (default 24h for encrypted, 7 days for public)
- [ ] Field selection UI before each share
- [ ] Basic contact list with "last verified" timestamps and manual refresh
- [ ] Public mode as opt-in alternative — plaintext DNS TXT records on DHT under `_pktap.` prefixed names
- [ ] QR code display and scan as NFC fallback — `pktap://pk/<base32-pubkey>?mode=enc`
- [ ] QR async handshake: Bob publishes encrypted key to `_pktap._handshake.<SHA-256(Alice_PK)>`, Alice polls every 2s for up to 5 minutes
- [ ] Memory zeroing (zeroize crate) on all secret material in Rust; zero ByteArrays after FFI calls in Kotlin

### Out of Scope

- iOS app (SwiftUI + Keychain/Secure Enclave) — v0.2+, KMP architecture supports it but not building now
- Multi-context profiles with HKDF key derivation — v0.2+, MVP uses single default profile
- Forward secrecy via ephemeral X25519 keys — v0.2+, adds 32 bytes to NFC payload
- Background auto-republish for public mode — v0.2+
- Background contact re-resolution — v0.2+
- Web fallback resolver for public profiles — v0.2+
- Contact expiry/staleness visual indicators — v0.2+
- Export contacts as vCard — v0.2+
- NFC tag programming (write key to sticker) — v0.2+
- Re-encrypt and republish for existing recipients on profile update — v0.2+
- Profile photos — Pkarr 1000-byte limit makes this impractical

## Context

- **Pkarr** is the DHT client library (Rust) — handles DNS record signing and Mainline DHT publish/resolve
- **UniFFI** generates Kotlin bindings from Rust — this is how the Rust crypto core is exposed to the Android app
- The Rust layer owns all cryptographic operations: key conversion, ECDH, AEAD encryption, record construction, DHT interaction, and memory zeroing
- Kotlin/Android layer is a thin shell: UI (Jetpack Compose), NFC HCE service, Keystore access, and FFI calls
- KMP shared module exists in the architecture for future iOS support but MVP is Android-only
- NFC HCE (Host Card Emulation) enables phone-to-phone bidirectional exchange without requiring an NFC tag
- The 1000-byte Pkarr record limit leaves ~600 bytes for contact fields after encryption overhead (~400 bytes for nonce + tag + sig + encoding)
- All record names use `_pktap.` prefix to avoid namespace collisions with other Pkarr-based apps

## Constraints

- **Platform**: Android-only for MVP — full NFC HCE support required (no iOS NFC write capability)
- **Record size**: 1000 bytes max per Pkarr record — text fields only, no photos
- **Key storage**: Master key must be non-extractable from Android Keystore (StrongBox/TEE)
- **No server**: Zero network requests to any PKTap-controlled server — DHT only
- **Crypto in Rust**: All cryptographic operations happen in Rust via UniFFI — no JVM crypto libraries for protocol operations
- **Memory safety**: All secret material zeroed after use (zeroize crate in Rust, explicit ByteArray zeroing in Kotlin post-FFI)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust-heavy architecture | All crypto, DHT, record construction, key derivation in Rust. Kotlin is thin UI/NFC shell. Maximizes memory safety and code reuse for future iOS. | -- Pending |
| NFC priority: reliability > polish > speed | A tap that fails is worse than one that takes 4 seconds or looks plain. Target every Android phone with NFC. | -- Pending |
| BIP-39 mnemonic for seed backup | Show 12/24 words at first launch. Standard crypto UX. Tension with non-extractable design acknowledged — seed copy exists during display. | -- Pending |
| `_pktap.` namespace prefix | All Pkarr record names prefixed to avoid collision with other DHT apps. `_pktap._share.`, `_pktap._profile.`, `_pktap._handshake.` | -- Pending |
| QR handshake: 5 min / 2 sec polling | Alice polls `_pktap._handshake.<hash>` every 2 seconds for up to 5 minutes after showing QR. Balances patience with UX. | -- Pending |
| NFC-first, QR as fallback | The demo moment is the tap. QR exists for devices without NFC or cross-platform future. | -- Pending |
| Encrypted by default | Public plaintext on DHT is bad default for personal contact info. Users opt into public, not out of private. | -- Pending |
| Standard SQLite, not SQLCipher | Sensitive columns encrypted via Keystore-managed AES key. Full-DB encryption unnecessary overhead. | -- Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-04 after initialization*
