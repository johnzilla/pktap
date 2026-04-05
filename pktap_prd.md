# PKTap — Product Requirements Document

## One-Liner

A privacy-first, decentralized contact exchange app powered by Pkarr and the Mainline DHT. Tap phones, swap public keys, resolve encrypted contact profiles. No accounts, no cloud, no middleman.

## Problem

Exchanging contact info in person is still awkward. You either spell out a phone number, airdrop a vCard snapshot that goes stale, or sign up for a SaaS digital business card platform that owns your data. None of these options respect user privacy, stay current, or work without a centralized service.

Existing NFC/QR contact apps that use decentralized infrastructure (like Pkarr) still publish contact data in plaintext on the DHT — anyone with your public key can read your records. That's fine for a conference badge, but not for sharing your personal cell number with someone you just met.

## Solution

PKTap turns Ed25519 public keys into self-sovereign contact identities with two sharing modes:

- **Public mode** — plaintext DNS records on the DHT for broad sharing (conference card, professional profile). Anyone with your key can resolve your info.
- **Encrypted mode** — per-recipient encrypted records using X25519 ECDH. Only the intended recipient can decrypt your contact fields. The DHT acts as a dead drop, and the record naturally expires via TTL.

Users exchange 32-byte public keys over NFC tap or QR scan, then the app handles key agreement, encryption, publication, and resolution transparently.

## Core Principles

- **No accounts, no cloud** — the DHT is the infrastructure
- **Encrypted by default** — one-on-one exchanges use per-recipient encryption; public mode is an explicit opt-in
- **Selective disclosure** — share only the fields you choose, per recipient
- **Live contacts** — resolve on demand, never stale snapshots
- **Natural expiry** — TTL-governed records fade from the DHT automatically, managing bloat and enabling revocation
- **Hardware-backed keys** — master keypair lives in platform keystore, never exported
- **Cross-platform** — NFC (Android), QR (universal), deep links (fallback)

## User Stories

### First Launch
1. User opens PKTap for the first time
2. App generates an Ed25519 master keypair inside Android Keystore (StrongBox/TEE) — private key is non-extractable
3. App derives a signing subkey for the default profile context, encrypts it with a Keystore-managed AES key, and stores it in EncryptedSharedPreferences
4. User creates their first profile: name + one or more contact fields
5. User sees a confirmation with their shareable QR code and public key
6. No DHT publish happens yet — records are only published when the user shares (encrypted mode) or explicitly opts into public mode

### Sharing a Contact — Encrypted Mode (Default)

#### Android → Android (NFC)
1. User A selects fields to share and initiates a tap
2. NFC (HCE) performs a bidirectional exchange: both phones swap 32-byte Ed25519 public keys
3. User A's app performs X25519-ECDH (Alice private + Bob public → shared secret)
4. App encrypts selected contact fields with XChaCha20-Poly1305 using the shared secret
5. App signs the encrypted payload with User A's Ed25519 key
6. App publishes to DHT at deterministic address: `_share.<sorted_hash(A_pk, B_pk)>`
7. User B's app computes the same address, retrieves the record, verifies signature, derives the same shared secret, decrypts
8. User B sees a contact preview with only the fields User A chose to share
9. User B saves locally; the DHT record expires via TTL

#### Any → Any (QR)
1. User A displays QR encoding `pktap://pk/<base32-pubkey>?mode=enc`
2. User B scans, app extracts User A's key
3. User B's app sends their own key back via a short-lived DHT record at `_handshake.<hash(A_pk)>` (encrypted to A's key)
4. Both parties now have each other's keys → encrypted exchange proceeds as above
5. Fallback: if User B doesn't have PKTap, QR resolves to web page explaining the app (no contact data exposed)

### Sharing a Contact — Public Mode (Opt-In)
1. User explicitly enables "Public Profile" for a context
2. App signs plaintext DNS TXT records and publishes to DHT under the context's derived key
3. Anyone with the public key can resolve and read the contact info
4. Useful for: conference badges, business cards, link-in-bio, NFC stickers

### Updating Contact Info
1. User edits a field in their profile
2. For public mode: app re-signs and republishes plaintext records
3. For encrypted mode: existing per-recipient records are stale (recipients will see cached version). User can optionally re-encrypt and republish for specific recipients.
4. New exchanges always use current data

### Resolving Stale Contacts
1. App periodically re-resolves saved contacts in background
2. Encrypted records: if TTL expired and sender hasn't republished, contact shows last cached version with "last verified" timestamp
3. Public records: if unreachable, contact is flagged as "unverified"
4. User can keep, archive, or delete unverified contacts

## Exchange Protocol

### Encrypted Mode (Default)

```
PHASE 1: KEY EXCHANGE (NFC tap or QR handshake)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Alice ──[Alice_PK (32 bytes)]──► Bob
  Alice ◄──[Bob_PK (32 bytes)]──── Bob

  Both parties now hold each other's Ed25519 public keys.

PHASE 2: ENCRYPTED PUBLISH (both sides, independently)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  1. Convert Ed25519 keys → X25519 keys
  2. X25519-ECDH: own_private + their_public → shared_secret
  3. KDF(shared_secret, "pktap-v1") → encryption_key
  4. Select fields to share
  5. Serialize fields → plaintext blob
  6. XChaCha20-Poly1305(encryption_key, nonce, plaintext) → ciphertext
  7. Ed25519-sign(own_key, ciphertext) → signature
  8. Record address = _share.<SHA-256(sort(Alice_PK, Bob_PK))>
  9. Publish signed ciphertext to DHT at record address
  10. Set TTL (default: 24h, configurable)

PHASE 3: RETRIEVAL (both sides, independently)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  1. Compute record address (same deterministic hash)
  2. Query DHT for record
  3. Verify Ed25519 signature against sender's public key
  4. Derive same shared_secret via ECDH
  5. Decrypt ciphertext → contact fields
  6. Display preview → user confirms → save locally
  7. Record expires from DHT via TTL (no cleanup needed)
```

### Public Mode (Opt-In)

```
PUBLISH:
  1. Sign plaintext DNS TXT records with context-derived Ed25519 key
  2. Publish to DHT under the derived public key
  3. Auto-republish on schedule to keep records alive

RESOLVE:
  1. Any holder of the public key queries DHT
  2. Verify signature → display contact fields
```

### QR Handshake (When NFC Unavailable)

```
  Alice shows QR: pktap://pk/<base32-Alice_PK>?mode=enc
  Bob scans, extracts Alice_PK
  Bob publishes encrypted handshake to: _handshake.<SHA-256(Alice_PK)>
    → payload: Bob_PK encrypted to Alice_PK (X25519 one-way)
    → TTL: 5 minutes
  Alice polls _handshake.<SHA-256(Alice_PK)>, decrypts, gets Bob_PK
  Both proceed to Phase 2 (encrypted publish)
```

## Pkarr Record Schema

### Public Mode Records
Published under the context-derived public key:

```
_profile.TXT  → v=pktap1;n=John Doe;ctx=professional
_email.TXT    → work=john@company.com
_phone.TXT    → cell=+15551234567
_social.TXT   → signal=john.42;nostr=npub1abc...
_url.TXT      → site=https://johndoe.dev
_meta.TXT     → ttl=86400;updated=1743811200
```

### Encrypted Mode Records
Published at deterministic shared address:

```
_share.<SHA-256(sort(A_pk, B_pk))>.TXT →
  v=pktap1;enc=xchacha20poly1305;
  nonce=<24-byte-nonce-base64>;
  data=<ciphertext-base64>;
  sig=<ed25519-signature-base64>
```

### Record Constraints
- All records signed by Ed25519 key (context-derived for public, identity for encrypted)
- Total payload must be ≤1000 bytes (Pkarr limit)
- Encrypted mode: ciphertext + nonce + sig + overhead ≈ ~600 bytes available for contact fields (plenty for text fields, no photos)
- Fields are optional — user decides what to include
- `_meta.TXT` includes TTL hint and last-updated Unix timestamp
- TTL governs natural expiry: encrypted records default to 24h, public records default to 7 days

## Threat Model

### What PKTap Protects

| Threat | Encrypted Mode | Public Mode |
|--------|---------------|-------------|
| Passive DHT observer reads your contacts | ✅ Protected — ciphertext only | ❌ Not protected — plaintext by design |
| Third party correlates your contexts | ✅ Protected — derived keys are unlinkable | ✅ Protected — same |
| Forged/tampered contact records | ✅ Protected — Ed25519 signature verification | ✅ Protected — same |
| Centralized service logs who shared with whom | ✅ Protected — DHT has no central log | ✅ Protected — same |
| Contact data persists after relationship ends | ✅ Protected — TTL expiry, stop republishing | ⚠️ Partial — cached copies may persist on recipient devices |
| Recipient shares your data with others | ❌ Not protected — decrypted data can be copied | ❌ Not protected — same |

### What PKTap Does NOT Protect Against

- **DHT query observation** — DHT nodes along the resolution path see which key is being queried and the resolver's IP address. Use a VPN/Tor if this matters.
- **NFC eavesdropping** — the 32-byte public key exchange over NFC can theoretically be sniffed at close range (~10cm). The key itself is not secret (it's an identifier), but it reveals that an exchange occurred.
- **Metadata correlation** — an attacker who observes both the NFC exchange AND the DHT publish can correlate the timing to link physical proximity with a DHT record.
- **Recipient device compromise** — once decrypted and saved locally, contact data is subject to the security of the recipient's device.
- **Key compromise** — if a derived signing key is extracted from memory during a publish operation, the attacker can forge records for that context. The master key in Keystore/TEE mitigates this for the root identity.
- **No forward secrecy (MVP)** — encrypted mode uses identity keys for ECDH, not ephemeral keys. A future version could include ephemeral X25519 keys in the NFC payload (68 bytes total) for forward secrecy.

### Security Design Decisions

| Decision | Rationale |
|----------|-----------|
| Encrypted by default | Public plaintext on the DHT is a bad default for personal contact info. Users opt *into* public, not out of private. |
| TTL-based expiry | Natural record cleanup without explicit revocation infrastructure. Reduces DHT bloat and limits exposure window. |
| Deterministic record addresses | Both parties compute the same address independently — no coordination needed after key exchange. |
| Ed25519 → X25519 conversion | Reuse identity keypair for both signing and key agreement. Avoids managing separate keypairs. Standard conversion (RFC 7748). |
| XChaCha20-Poly1305 | 24-byte nonce eliminates collision risk for random nonce generation. AEAD provides integrity + confidentiality. |
| No profile photos | 1000-byte Pkarr limit makes embedded images impractical. Future: link to Pubky homeserver for rich profiles. |

## Context Profiles & Key Derivation

- Master keypair generated in Android Keystore (StrongBox/TEE), non-extractable
- A random seed is generated at first launch, encrypted with Keystore-managed AES-256-GCM key, and stored in EncryptedSharedPreferences
- Each context profile derives a child keypair: `HKDF(seed, "pktap-v1-professional")`, `HKDF(seed, "pktap-v1-personal")`, etc.
- Derived signing keys are encrypted with the Keystore-managed AES key and stored in EncryptedSharedPreferences
- Keys are decrypted into memory only during publish/sign operations, then zeroed
- For encrypted mode exchanges, the identity key (not context-derived) is used for ECDH — this is what gets exchanged during the tap
- Profiles are cryptographically unlinkable — recipients cannot correlate "professional" and "personal" identities unless the user discloses both
- Suggested default contexts: Professional, Personal, Minimal
- Users can create custom contexts

## Key Management & Storage

### Design Philosophy

Leverage platform keystores (hardware-backed where available) as the root of trust. No third-party encryption libraries for key storage. Sensitive key material touches memory only during signing/encryption, then is zeroed.

### Storage Map

| Data | Storage | Protection |
|------|---------|------------|
| Master Ed25519 keypair | Android Keystore (StrongBox/TEE) | Hardware-backed, non-extractable |
| AES-256-GCM encryption key | Android Keystore | Hardware-backed, used to encrypt seed, derived keys, and sensitive profile data |
| HKDF seed | EncryptedSharedPreferences | AES-256-GCM encrypted, Keystore-managed key |
| Derived context signing keys | EncryptedSharedPreferences | AES-256-GCM encrypted, Keystore-managed key |
| Own profiles (contact fields) | SQLite, sensitive columns encrypted | AES-256-GCM with Keystore-managed key |
| Saved contacts (others) | SQLite, plaintext | Decrypted data from resolved exchanges — local-only |
| Peer public keys | SQLite, plaintext | Public keys are not secrets |
| App preferences | EncryptedSharedPreferences | Keystore-backed |

### Key Lifecycle

```
┌─────────────────────────────────────────────────┐
│ FIRST LAUNCH                                     │
│                                                  │
│ 1. Generate Ed25519 master key in Keystore/TEE  │
│ 2. Generate AES-256-GCM key in Keystore         │
│ 3. Generate random HKDF seed (32 bytes)          │
│ 4. AES-encrypt seed → EncryptedSharedPrefs      │
│ 5. HKDF(seed, context) → derived signing key    │
│ 6. AES-encrypt derived key → EncryptedSharedPrefs│
│ 7. Zero seed + derived key from memory           │
├─────────────────────────────────────────────────┤
│ ENCRYPTED EXCHANGE                               │
│                                                  │
│ 1. NFC/QR key exchange → receive peer's Ed25519  │
│ 2. Convert own Ed25519 → X25519 (in Rust)        │
│ 3. Convert peer Ed25519 → X25519 (in Rust)       │
│ 4. ECDH → shared secret → KDF → encryption key  │
│ 5. Encrypt selected fields, sign, publish to DHT │
│ 6. Zero shared secret + encryption key from mem  │
├─────────────────────────────────────────────────┤
│ PUBLIC PUBLISH                                   │
│                                                  │
│ 1. Decrypt derived key from EncryptedSharedPrefs │
│ 2. Load into Rust/Pkarr via FFI                  │
│ 3. Sign DNS records, publish to DHT              │
│ 4. Zero key material in Rust + Kotlin memory     │
├─────────────────────────────────────────────────┤
│ DEVICE LOSS                                      │
│                                                  │
│ Master key is non-extractable → identity is gone │
│ HKDF seed is encrypted to Keystore → also gone   │
│ User must create new identity on new device      │
│ (See Open Questions: key backup)                 │
└─────────────────────────────────────────────────┘
```

### iOS Keychain (v0.2+)

| Android | iOS Equivalent |
|---------|---------------|
| Keystore (StrongBox/TEE) | Secure Enclave (`kSecAttrTokenIDSecureEnclave`) |
| EncryptedSharedPreferences | Keychain (`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`) |
| AES-256-GCM via Keystore | Keychain-managed symmetric key |

The KMP shared layer defines a `KeyManager` interface in Kotlin common code. Each platform provides a native implementation.

```kotlin
// shared/src/commonMain/kotlin/service/KeyManager.kt
expect class KeyManager {
    fun generateMasterKey()
    fun getIdentityPublicKey(): ByteArray
    fun deriveSigning(context: String): ByteArray  // caller must zero after use
    fun deriveX25519Private(): ByteArray            // caller must zero after use
    fun encryptLocal(data: ByteArray): ByteArray
    fun decryptLocal(data: ByteArray): ByteArray
}
```

## NFC Payload Format

```
┌──────────────────────────────────────────────────┐
│ NDEF Record (TNF=External, Type="pktap.io")      │
├──────────────────────────────────────────────────┤
│ Version        │ 1 byte  │ 0x01=public, 0x02=enc│
│ Flags          │ 1 byte  │ bit 0: mutual request│
│ Identity Key   │ 32 bytes│ Ed25519 pubkey       │
│ Checksum       │ 2 bytes │ CRC-16               │
├──────────────────────────────────────────────────┤
│ Total: 36 bytes                                  │
│                                                  │
│ Future (forward secrecy):                        │
│ Ephemeral Key  │ 32 bytes│ X25519 pubkey (opt)  │
│ Total: 68 bytes                                  │
└──────────────────────────────────────────────────┘
```

- Version 0x01: public mode — receiver resolves plaintext records from DHT
- Version 0x02: encrypted mode — triggers ECDH + encrypted dead drop flow
- Minimal payload for fast NFC transmission
- CRC-16 for integrity verification on noisy NFC reads
- External NDEF type allows non-PKTap NFC readers to recognize and route the payload

## Architecture

```
┌──────────────────────────────────────────────────┐
│                    PKTap App                      │
├────────────────────┬─────────────────────────────┤
│   Android UI       │       iOS UI (v0.2+)        │
│   Jetpack Compose  │       SwiftUI               │
├────────────────────┴─────────────────────────────┤
│        Shared KMP Module (Kotlin)                 │
│  - ContactProfile data classes                   │
│  - ProfileManager (CRUD, field selection)        │
│  - ExchangeService (orchestrates the protocol)   │
│  - TransportCodec (encode/decode NFC + QR)       │
│  - ResolutionService (resolve key → profile)     │
│  - RepublishScheduler (keep public records alive) │
│  - KeyManager (expect/actual, wraps keystore)    │
├──────────────────────────────────────────────────┤
│        Pkarr Core (Rust via UniFFI)               │
│  - Ed25519 signing (receives key bytes via FFI)  │
│  - Ed25519 → X25519 conversion                   │
│  - X25519 ECDH key agreement                     │
│  - XChaCha20-Poly1305 encrypt/decrypt            │
│  - DNS TXT record construction                   │
│  - Publish to Mainline DHT                       │
│  - Resolve public key → DNS records              │
│  - Memory zeroing (zeroize crate) on all secrets │
├──────────────────────────────────────────────────┤
│        Platform Layer (Native)                    │
│  Android: HostApduService (HCE), NFC Reader      │
│  iOS: Core NFC (read), QR gen/scan               │
│  Both: CameraX/AVFoundation QR, deep links       │
├──────────────────────────────────────────────────┤
│        Local Storage                              │
│  Android Keystore (StrongBox/TEE)                │
│  - Master Ed25519 keypair (non-extractable)      │
│  - AES-256-GCM key for local encryption          │
│  EncryptedSharedPreferences                      │
│  - HKDF seed (encrypted)                         │
│  - Derived signing keys (encrypted)              │
│  - App config                                    │
│  SQLite (standard, no SQLCipher)                 │
│  - profiles (sensitive cols AES-encrypted)        │
│  - contacts (plaintext, locally decrypted data)  │
│  - peer_keys (public keys, plaintext)            │
└──────────────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Shared logic | Kotlin Multiplatform (KMP) | Shared models, services, and business logic across platforms |
| Android UI | Jetpack Compose | Native Android UI, first-class NFC/HCE access |
| iOS UI (v0.2+) | SwiftUI | Native iOS UI, Core NFC access |
| Crypto + DHT | Rust (pkarr, x25519-dalek, chacha20poly1305, zeroize) via UniFFI | Production-ready Pkarr client, ECDH, AEAD encryption, memory safety |
| Key storage | Android Keystore / iOS Keychain | Hardware-backed, non-extractable master keys |
| Local DB | SQLite (standard) | No SQLCipher needed — sensitive columns encrypted via Keystore-managed AES key |
| NFC (Android) | HostApduService + NfcAdapter | Full HCE for phone-to-phone bidirectional exchange |
| NFC (iOS) | Core NFC (NFCNDEFReaderSession) | Read-only; QR is primary iOS exchange |
| QR | ZXing (Android) / AVFoundation (iOS) | Universal fallback, encodes pktap:// URI |
| Deep links | pktap://pk/<base32-pubkey>?mode=enc | App routing + web fallback |
| Web fallback | Static page (no contact data) | Explains app + download link; no data exposed for encrypted mode |

## MVP Scope (v0.1)

### In
- Single platform: **Android only** (full NFC + QR capability)
- Master keypair generation in Android Keystore (StrongBox/TEE)
- AES-256-GCM encryption key in Keystore for local data protection
- HKDF seed generation and encrypted storage
- Single default profile (no multi-context yet)
- **Encrypted mode as default**: ECDH key agreement, XChaCha20-Poly1305 encryption, signed encrypted records on DHT
- Share via QR code with handshake protocol
- Share via NFC (HCE bidirectional key exchange)
- Field selection UI before each share
- Resolve received encrypted records → decrypt → display contact → save locally
- TTL-based record expiry (default 24h for encrypted, 7 days for public)
- Manual refresh of saved contacts
- Basic contact list UI with "last verified" timestamps
- Public mode as opt-in alternative

### Out (v0.2+)
- iOS app (SwiftUI + Keychain/Secure Enclave)
- Multi-context profiles with HKDF key derivation
- Forward secrecy via ephemeral X25519 keys (68-byte NFC payload)
- Background auto-republish for public mode
- Background contact re-resolution
- Web fallback resolver for public profiles
- Contact expiry/staleness indicators with visual treatment
- Export contacts as vCard
- NFC tag programming (write your key to a sticker)
- Key backup / recovery (see Open Questions)
- Re-encrypt and republish for existing recipients on profile update

## Open Questions

1. **Key backup** — master key is non-extractable from Keystore. HKDF seed is encrypted to Keystore. Device loss = identity loss. Options: (a) encrypt seed to a user passphrase and export as QR/file, (b) BIP-39 mnemonic of the seed, (c) accept the tradeoff for MVP and add backup in v0.2. Note: any backup mechanism creates an extractable copy of the seed, which is an inherent tension with the non-extractable design.
2. **Forward secrecy** — MVP uses identity keys for ECDH. Adding ephemeral X25519 keys (32 extra bytes in NFC payload) would provide forward secrecy so that a future key compromise doesn't expose past exchanges. Worth the complexity for v0.1?
3. **Revocation** — TTL expiry handles most cases. Should there be an explicit "revoke" record type for immediate invalidation (e.g., publish a signed tombstone at the record address)?
4. **QR handshake latency** — the async QR flow requires Alice to poll `_handshake.<hash>` for Bob's key. How long to poll? 5 minutes max with 2-second intervals? UX for "waiting for response" state?
5. **Pkarr relay** — run our own for the web fallback, or point at a public relay? For encrypted mode the web fallback shows nothing useful anyway.
6. **Record size budget** — 1000 bytes total. Encrypted overhead (nonce + tag + sig + encoding) ≈ ~400 bytes. Leaves ~600 bytes for contact fields. Enough for text, not for photos. Future: Pubky homeserver link for rich profiles.
7. **Namespace collision** — should record names use a `_pktap.` prefix to avoid collisions with other Pkarr-based apps?
8. **Memory zeroing across FFI** — the `zeroize` crate handles Rust-side cleanup. Verify that UniFFI-generated Kotlin bindings don't retain copies of `ByteArray` arguments after the FFI call returns.
9. **Republish strategy for encrypted records** — if Alice wants Bob to always have current info, she'd need to re-encrypt and republish. Automated? On-demand? Only on next encounter?

## Success Metrics

- Time from first launch to first share: < 60 seconds
- NFC tap to encrypted contact preview: < 3 seconds (includes ECDH + DHT publish + resolve)
- QR scan to encrypted contact preview: < 5 seconds (includes handshake round-trip)
- Zero network requests to any server controlled by PKTap (DHT only)
- Zero PII transmitted unencrypted over DHT
- Zero sensitive key material persisted outside Keystore/EncryptedSharedPreferences
- Zero plaintext contact data on DHT unless user explicitly opted into public mode

## Repo Structure (Proposed)

```
pktap/
├── README.md
├── LICENSE                    # MIT
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # UniFFI interface
│       ├── keys.rs            # Ed25519, X25519 conversion, HKDF, zeroize
│       ├── crypto.rs          # ECDH, XChaCha20-Poly1305 encrypt/decrypt
│       ├── records.rs         # DNS TXT construction (public + encrypted)
│       └── dht.rs             # Pkarr publish/resolve
├── shared/                    # KMP shared module
│   └── src/commonMain/kotlin/
│       ├── model/             # ContactProfile, ContactField, PeerKey, etc.
│       ├── service/           # ProfileManager, ExchangeService, ResolutionService
│       ├── codec/             # NFC + QR payload encoding, record address computation
│       └── keymanager/        # expect KeyManager interface
├── android/
│   ├── app/
│   │   └── src/main/
│   │       ├── kotlin/
│   │       │   ├── ui/        # Compose screens (profile, share, contacts)
│   │       │   ├── nfc/       # HCE service + reader (bidirectional)
│   │       │   └── keymanager/# actual AndroidKeyManager (Keystore)
│   │       └── AndroidManifest.xml
│   └── build.gradle.kts
└── ios/                       # v0.2+
    └── PKTap/
        ├── UI/                # SwiftUI views
        ├── NFC/               # Core NFC wrapper
        └── KeyManager/        # IosKeyManager (Keychain + Secure Enclave)
```
