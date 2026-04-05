# Architecture Patterns

**Project:** PKTap — Decentralized Encrypted Contact Exchange
**Researched:** 2026-04-04
**Confidence:** HIGH for UniFFI/Rust and NFC HCE patterns; MEDIUM for Pkarr-specific DHT record construction (library is relatively new); HIGH for Android Keystore integration

---

## Recommended Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Android Application                       │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Jetpack Compose UI Layer                 │  │
│  │  ProfileSetupScreen │ TapScreen │ ContactListScreen  │  │
│  │  QRScreen           │ MnemonicScreen                 │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                           │ ViewModel calls                 │
│  ┌────────────────────────▼──────────────────────────────┐  │
│  │              ViewModel / AppCoordinator               │  │
│  │  (Android ViewModel, coroutines, StateFlow)           │  │
│  └───────┬──────────────────┬───────────────────────────┘  │
│          │                  │                               │
│  ┌───────▼──────┐  ┌────────▼────────┐                     │
│  │ NFC Module   │  │ Keystore Module │                     │
│  │              │  │                 │                     │
│  │ HCE Service  │  │ KeystoreWrapper │                     │
│  │ Reader mode  │  │ (AES-256-GCM)   │                     │
│  │ NDEF codec   │  │ (Ed25519 sign)  │                     │
│  └───────┬──────┘  └────────┬────────┘                     │
│          │ raw bytes        │ sealed bytes                  │
│  ┌───────▼──────────────────▼────────────────────────────┐  │
│  │              PktapCore (Kotlin FFI Bridge)             │  │
│  │  Thin wrapper: ByteArray marshaling, error mapping,   │  │
│  │  memory zeroing after each FFI call                   │  │
│  └───────────────────────┬───────────────────────────────┘  │
│                           │ UniFFI generated bindings        │
└───────────────────────────┼─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                     Rust Core (pktap-core)                  │
│                                                             │
│  ┌─────────────────┐  ┌─────────────────┐                  │
│  │  CryptoOps      │  │   RecordBuilder │                  │
│  │                 │  │                 │                  │
│  │ key_convert()   │  │ build_share()   │                  │
│  │ ecdh_agree()    │  │ build_profile() │                  │
│  │ kdf_derive()    │  │ parse_record()  │                  │
│  │ aead_encrypt()  │  │ verify_record() │                  │
│  │ aead_decrypt()  │  └────────┬────────┘                  │
│  │ ed25519_sign()  │           │                           │
│  │ ed25519_verify()│  ┌────────▼────────┐                  │
│  └─────────────────┘  │  DhtClient      │                  │
│                        │  (pkarr crate)  │                  │
│  ┌─────────────────┐  │                 │                  │
│  │  KeyManager     │  │ publish()       │                  │
│  │                 │  │ resolve()       │                  │
│  │ derive_x25519() │  └─────────────────┘                  │
│  │ hkdf_expand()   │                                       │
│  │ zeroize on drop │                                       │
│  └─────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                      Local Storage                          │
│                                                             │
│  SQLite (Room)                  EncryptedSharedPreferences  │
│  - contacts table               - encrypted HKDF seed       │
│  - sensitive cols: AES-enc      - app preferences           │
│  - last_verified timestamp                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Boundaries

### What Talks to What

| Component | Responsibility | Communicates With | Does NOT touch |
|-----------|---------------|-------------------|----------------|
| Compose UI | Render state, user events | ViewModel only | Rust, Keystore, NFC directly |
| ViewModel / AppCoordinator | Orchestrate flows, hold UI state | UI, NFC Module, Keystore Module, PktapCore FFI Bridge | Rust internals |
| NFC Module (HCE + Reader) | Byte exchange over NFC, NDEF framing | ViewModel (callback), PktapCore (for payload construction) | Crypto, DHT |
| Keystore Module | Generate/use hardware-backed keys, sign with Ed25519, AES encrypt/decrypt for storage | ViewModel, PktapCore FFI Bridge (for sealed seed handoff) | DHT, NFC |
| PktapCore FFI Bridge | Marshal types across UniFFI boundary, zero memory after FFI calls, map Rust errors to Kotlin sealed classes | Rust Core via UniFFI, ViewModel | Android APIs |
| Rust CryptoOps | Ed25519, X25519 conversion, ECDH, HKDF, XChaCha20-Poly1305, zeroize | RecordBuilder, DhtClient, KeyManager | Android APIs |
| Rust RecordBuilder | Construct/parse DNS TXT records for Pkarr | CryptoOps, DhtClient | Android APIs |
| Rust DhtClient (pkarr) | Mainline DHT publish and resolve | RecordBuilder | Android APIs, Keystore |
| SQLite (Room) | Persist contacts; AES-encrypt sensitive columns at rest | ViewModel (via Repository) | Rust directly |

---

## 1. Rust/UniFFI Boundary Design

### Guiding Principle

The UniFFI boundary is a serialization boundary, not a function call boundary. Every crossing pays a marshaling cost and every byte returned to Kotlin is a potential leak unless explicitly zeroed. Design the surface to be wide in capability but narrow in call frequency.

### Recommended FFI Interface Surface

```
// pktap.udl (UniFFI Definition Language)

namespace pktap {};

// Opaque handle — keeps key material in Rust heap, never crosses boundary
interface KeyManager {
    constructor(bytes seed);  // seed: 32-byte HKDF root, zeroized after use
    bytes public_key();       // returns Ed25519 pubkey (32 bytes, non-secret)
    void drop();              // explicit zeroize trigger (also called on GC)
};

// One-shot crypto operations — no state held in Kotlin
[Throws=PktapError]
bytes ecdh_and_encrypt(
    bytes our_x25519_privkey,   // 32 bytes, zeroized after
    bytes their_ed25519_pubkey, // 32 bytes
    bytes plaintext             // contact payload
);

[Throws=PktapError]
bytes ecdh_and_decrypt(
    bytes our_x25519_privkey,
    bytes their_ed25519_pubkey,
    bytes ciphertext
);

[Throws=PktapError]
bytes build_share_record(
    bytes signing_key,          // Ed25519 secret key (64 bytes), zeroized after
    bytes ciphertext,
    string record_name          // "_pktap._share.<hash>"
);

[Throws=PktapError]
ShareRecord parse_share_record(bytes dns_wire_format);

[Throws=PktapError]
void publish_record(string name, bytes signed_dns_packet);

[Throws=PktapError]
sequence<bytes> resolve_record(string name);

dictionary ShareRecord {
    bytes signer_pubkey;
    bytes ciphertext;
    u64 timestamp;
    boolean signature_valid;
};

[Error]
enum PktapError {
    "InvalidKey",
    "DecryptionFailed",
    "SignatureInvalid",
    "RecordTooLarge",
    "DhtPublishFailed",
    "DhtResolveFailed",
    "InvalidPayload"
};
```

### Key Boundary Rules

1. **Never pass Ed25519 private key material across the boundary if avoidable.** Prefer opaque `KeyManager` handles. The exception is when Keystore holds the signing key — in that case Kotlin owns signing and only the signature crosses into Rust for record construction.

2. **Use `bytes` (ByteArray) not `string` for all crypto material.** Strings in JVM are interned and not zeroizable.

3. **Kotlin side must zero ByteArrays immediately after FFI return** — UniFFI does not zero return buffers. Pattern:
   ```kotlin
   val result = pktapCore.ecdhAndEncrypt(privkey, theirPubkey, payload)
   privkey.fill(0)  // zero immediately
   ```

4. **Error types are enums, not strings.** Map `PktapError` to Kotlin sealed classes at the bridge layer before they reach ViewModels.

5. **Async Rust via callback interface.** DHT publish/resolve are async. UniFFI supports callback interfaces for async completion. Use a `CompletionCallback` interface rather than blocking the calling thread.

### Android Keystore and Rust Private Keys: The Hybrid Problem

The Android Keystore holds the Ed25519 master key as non-extractable. This creates a seam: Rust cannot use a Keystore-backed key directly because the key never leaves the Keystore. Resolution:

- **Keystore path (signing):** Kotlin calls Keystore to sign → raw signature bytes passed to Rust `RecordBuilder.attach_signature(record, sig_bytes)`. Rust validates format only, does not re-sign.
- **Rust path (ECDH):** The ECDH private key is derived via HKDF from a seed stored in EncryptedSharedPreferences. The seed is AES-decrypted by Keystore into a Kotlin ByteArray, passed to Rust `KeyManager`, then immediately zeroed in Kotlin. Rust holds the derived X25519 key in a zeroizing struct for the duration of the operation.

This means two key lineages:
1. **Keystore-backed Ed25519** — for signing records, non-extractable, survives device reboot
2. **HKDF-derived X25519** — for ECDH key agreement, reconstructed from seed per session, zeroed when done

---

## 2. NFC HCE Architecture

### HostApduService Lifecycle

HCE is initiated by the Android NFC subsystem when the device is tapped as a card. The `HostApduService` runs in the background and is invoked by the system — it does not require the app to be in the foreground (but the screen must be unlocked for security).

```
Device B (HCE, card emulator)              Device A (reader/writer)
────────────────────────────               ─────────────────────────
HostApduService.onCreate()
HostApduService.onStartCommand()

                                           NfcAdapter.enableForegroundDispatch()
                                           NfcAdapter.enableReaderMode() ← preferred

[NFC field detected by Device A]

HostApduService.processCommandApdu()  ←── SELECT AID command
  → return: response APDU (SW 9000)

HostApduService.processCommandApdu()  ←── GET DATA command
  → return: our Ed25519 pubkey (36 bytes)
               [version byte] [flags byte] [32-byte pubkey] [2-byte CRC-16]

                                           Activity.onNewIntent() / ReaderCallback
                                             receives 36-byte payload
                                             extracts 32-byte pubkey

                                           Device A sends ITS pubkey
HostApduService.processCommandApdu()  ←── PUT DATA command with Device A pubkey
  → return: SW 9000 (ACK)
  → notify ViewModel via BroadcastReceiver or
    bounded service callback

HostApduService.onDeactivated()       ←── field removed
```

### AID Registration

Register a proprietary AID in `res/xml/apduservice.xml`:
```xml
<host-apdu-service xmlns:android="..."
    android:description="@string/service_desc"
    android:requireDeviceScreenOn="true">
    <aid-group android:description="@string/aid_desc"
               android:category="other">
        <aid-filter android:name="F0504B544150"/>
        <!-- F0 = proprietary, 504B544150 = "PKTAP" in hex -->
    </aid-group>
</host-apdu-service>
```

### Bidirectional Protocol

HCE only makes Device B a card emulator — it cannot simultaneously read Device A. Bidirectionality requires a protocol-layer solution:

**Option A: Sequential exchange (recommended for MVP)**
Both phones take turns: one is always the reader, one is always the card. A pre-agreed protocol determines roles:
- The phone that initiates tap (reader mode active, no HCE response) is "Alice"
- The phone that responds (HCE active) is "Bob"
- After Alice reads Bob's key via HCE, Bob switches to reader mode to read Alice's key
- Coordination: Bob's `processCommandApdu` receives Alice's key in the PUT DATA command; Alice receives Bob's key in the GET DATA response. Single tap handles both directions.

**The practical MVP approach:** Embed both keys in a single exchange:
1. Alice (reader) sends SELECT AID + her pubkey in a single command APDU
2. Bob (HCE) returns his pubkey in the response APDU
3. One round-trip, one tap, both keys exchanged

APDU command format:
```
Command: CLA=00 INS=CA P1=00 P2=00 Lc=36 [36-byte Alice pubkey] Le=36
Response: [36-byte Bob pubkey] SW1=90 SW2=00
```

This is the recommended approach — single command+response APDU pair, no state machine needed.

### Reader Mode vs. Foreground Dispatch

Use `NfcAdapter.enableReaderMode()` (API 19+) rather than `enableForegroundDispatch()`:
- Reader mode suppresses peer-to-peer (Android Beam) and avoids accidental Beam triggering
- Reader mode callback runs on a dedicated thread, not the main thread
- Reader mode gives lower latency because Beam negotiation is skipped
- Call `disableReaderMode()` in `onPause()` — never leave it active in the background

### HCE Service Communication with ViewModel

`HostApduService` runs in a separate process context from the Activity. Use a `Messenger`-backed bound service or a `LocalBroadcastManager` broadcast to deliver the received pubkey to the Activity/ViewModel after the tap:

```kotlin
// In HostApduService
val intent = Intent(ACTION_PUBKEY_RECEIVED).apply {
    putExtra(EXTRA_PUBKEY, receivedPubkey)  // 32 bytes
}
LocalBroadcastManager.getInstance(this).sendBroadcast(intent)
```

The ViewModel registers/unregisters this receiver in `onStart()`/`onStop()`.

---

## 3. Android Keystore Integration with Rust Crypto

### What Lives Where

| Material | Location | Why |
|----------|----------|-----|
| Ed25519 master keypair | Android Keystore (StrongBox/TEE) | Non-extractable, hardware-backed |
| AES-256-GCM storage key | Android Keystore | Non-extractable, encrypts local data |
| HKDF seed (32 bytes) | EncryptedSharedPreferences | Encrypted by Keystore AES key; decrypted per session |
| X25519 ECDH private key | Rust heap (zeroizing) | Derived from HKDF seed for duration of tap operation only |
| XChaCha20-Poly1305 session key | Rust heap (zeroizing) | Output of ECDH+KDF, used and zeroed immediately |

### Key Generation Sequence (First Launch)

```
1. Generate 32-byte HKDF seed (SecureRandom in Kotlin — OS CSPRNG)
2. Generate Ed25519 keypair in Android Keystore
   - KeyPairGenerator.getInstance("EC", "AndroidKeyStore")
   - Note: Android Keystore uses "EC" with CURVE_25519 (API 33+) or
     workaround: generate in Rust, import as raw bytes (API 28+ for IMPORT_WRAPPED_KEY)
3. Generate AES-256-GCM key in Android Keystore for local storage
4. Encrypt HKDF seed with Keystore AES key → store in EncryptedSharedPreferences
5. Show BIP-39 mnemonic derived from HKDF seed (display only, do not persist words)
6. Zero seed bytes in memory after storage

IMPORTANT: Android Keystore EC support for Curve25519 (Ed25519) requires API 33+.
For API < 33, the recommended workaround is:
- Generate Ed25519 keypair in Rust (non-extractable-by-convention, not by hardware)
- Seal the private key with the Keystore AES key before storage
- Accept the weaker guarantee for API 31-32 devices
- For StrongBox: only available on API 33+ for Curve25519 anyway
```

### Signing Flow (Keystore Ed25519 → Rust Record)

```kotlin
// Kotlin side
val signature: ByteArray = keystoreWrapper.sign(
    keyAlias = "pktap_identity",
    data = recordPayload  // bytes to sign, produced by Rust RecordBuilder
)

// Hand off to Rust for record finalization
val signedRecord = pktapCore.attachSignature(unsignedRecord, signature)
// signedRecord is a complete DNS wire-format packet ready for DHT
```

### ECDH Flow (Rust with Keystore-unsealed seed)

```kotlin
// Kotlin side: unseal seed for duration of tap
val seed: ByteArray = keystoreWrapper.decrypt(encryptedSeed)
try {
    val ciphertext = pktapCore.ecdhAndEncrypt(
        seed = seed,
        theirPubkey = receivedPubkey,
        plaintext = contactPayload
    )
    // use ciphertext
} finally {
    seed.fill(0)  // mandatory: zero immediately after FFI call
}
```

---

## 4. Data Flow: Full Round-Trip

### NFC Tap → DHT Published (Alice's perspective)

```
1. PRE-TAP SETUP
   Alice opens TapScreen
   ViewModel calls pktapCore.prepareSharePayload(fields)
   → Rust serializes selected contact fields → returns ~400 byte plaintext blob
   ViewModel caches plaintext blob in memory (brief)

2. NFC TAP
   Alice's NfcAdapter in reader mode detects Bob's HCE
   Alice sends: [SELECT AID] + [Alice Ed25519 pubkey 32 bytes]
   Bob's HCE returns: [Bob Ed25519 pubkey 32 bytes]

   Alice now has: bob_ed25519_pk
   Bob now has:   alice_ed25519_pk (from APDU command data)
   Bob broadcasts pubkey to his ViewModel via LocalBroadcast

3. ECDH KEY AGREEMENT + ENCRYPTION (Alice)
   ViewModel unseals HKDF seed from EncryptedSharedPreferences
   pktapCore.ecdhAndEncrypt(seed, bob_pk, plaintext_blob)
   Inside Rust:
     alice_x25519_sk = hkdf_derive_x25519(seed)          // zeroizing
     bob_x25519_pk   = ed25519_to_x25519(bob_ed25519_pk) // cofactor conversion
     shared_secret   = x25519(alice_x25519_sk, bob_x25519_pk) // DH
     session_key     = hkdf(shared_secret, "pktap-share-v1", alice_pk || bob_pk)
     nonce           = random_24_bytes()
     ciphertext      = xchacha20poly1305_encrypt(session_key, nonce, plaintext)
     // all intermediates zeroized on drop
   Returns: nonce || ciphertext (AEAD)
   Kotlin zeros seed immediately

4. RECORD CONSTRUCTION
   record_name = "_pktap._share." + sha256(sort(alice_pk, bob_pk)).to_base32()
   pktapCore.buildShareRecord(signing_key_or_sig, record_name, ciphertext)
   → produces DNS TXT wire format packet for Pkarr

   SIGNING: Either
   a) Pass signing responsibility to Keystore (preferred, API 33+):
      - Rust builds unsigned record → Kotlin signs with Keystore → Rust attaches sig
   b) Rust signs with derived key (fallback, weaker security model):
      - Unseal Ed25519 privkey from storage → pass to Rust → Rust signs → Kotlin zeros privkey

5. DHT PUBLISH
   pktapCore.publishRecord(record_name, signed_record)
   → Pkarr BEP-44 mutable item put on Mainline DHT
   → Returns Future/callback when DHT ACK received
   ViewModel shows "Shared" state on ACK

   Bob does the same: simultaneously running steps 3-5 with Alice's pubkey
```

### DHT Resolve → Decrypt → Display (Alice resolving Bob's record)

```
6. DHT RESOLVE
   Alice knows bob_ed25519_pk (from NFC tap)
   record_name = "_pktap._share." + sha256(sort(alice_pk, bob_pk)).to_base32()
   NOTE: Same deterministic name — Alice resolves what Bob published

   pktapCore.resolveRecord(record_name)
   → Pkarr DHT lookup → returns signed DNS packet bytes

7. VERIFY + DECRYPT
   pktapCore.parseShareRecord(dns_packet)
   Inside Rust:
     verify Ed25519 signature (signer = bob_ed25519_pk)
     if invalid → error, stop
     extract ciphertext from TXT record
     derive session_key (same derivation as Bob used, symmetric)
     xchacha20poly1305_decrypt(session_key, nonce, ciphertext)
     → plaintext contact fields
   Returns: ContactFields struct

8. DISPLAY + PERSIST
   ViewModel receives ContactFields
   UI renders contact preview screen
   User accepts → ViewModel stores in SQLite
   Sensitive columns (name, phone, email) encrypted with Keystore AES key before insert
```

---

## 5. Anti-Patterns to Avoid

### Anti-Pattern 1: Crypto in Kotlin/JVM Layer
**What:** Using Bouncy Castle or JCA for ECDH or AEAD operations instead of Rust.
**Why bad:** JVM strings are interned and unzeroizable. Bouncy Castle's secret material handling is inconsistent. Defeats the architecture's memory safety guarantees.
**Instead:** All cryptographic operations in Rust, even if it means slightly more FFI overhead.

### Anti-Pattern 2: Fat UniFFI Surface
**What:** Exposing low-level Rust primitives (raw HKDF, raw X25519 scalar mult) as individual FFI calls, requiring Kotlin to orchestrate multi-step crypto flows.
**Why bad:** Each FFI call returns secret material to Kotlin heap. Multi-step flows in Kotlin mean secrets live longer. Logic errors in Kotlin can misorder steps.
**Instead:** Compose crypto operations inside Rust. `ecdhAndEncrypt` is one FFI call, not four.

### Anti-Pattern 3: HCE Timeout Ignorance
**What:** Assuming the HCE APDU exchange will always complete within the NFC field dwell time.
**Why bad:** HCE `processCommandApdu()` blocks the HCE dispatcher thread. Any work beyond ~300ms risks NFC field timeout. Users pull phones apart before exchange completes.
**Instead:** Prepare all crypto material before the tap (step 1 above). The APDU handler returns immediately with cached material. DHT publish happens after field deactivation.

### Anti-Pattern 4: Storing Decrypted Contacts in SQLite Without Column Encryption
**What:** Using Room with plaintext for all columns, relying on Android full-disk encryption alone.
**Why bad:** Full-disk encryption is at-rest only — running app or backup extraction can expose contacts.
**Instead:** AES-GCM encrypt name/phone/email/etc columns with Keystore-managed key before Room insertion.

### Anti-Pattern 5: HCE and Reader Mode Simultaneously on Same Device
**What:** Activating `enableReaderMode()` while the app's HCE service is registered.
**Why bad:** A device cannot be both a card (HCE) and a reader simultaneously at the RF layer. The reader mode suppresses all card emulation including HCE on many devices.
**Instead:** On the "Alice" (initiator/reader) device, HCE is handled by Android's NFC stack passively — Alice's role is to actively read. The HCE service on Alice responds IF Alice's phone happens to be read by another device, but Alice is the active reader in the tap.

### Anti-Pattern 6: BIP-39 Mnemonic Written to Logcat or Analytics
**What:** Debug logging of seed or mnemonic words during development.
**Why bad:** Logcat is readable by all apps with `READ_LOGS` permission on rooted devices. Analytics pipelines are not designed for secrets.
**Instead:** Mnemonic display is UI-only, in-memory string, never logged.

---

## 6. Scalability Considerations

| Concern | MVP (1-100 users) | v0.2 (10K users) | v1.0 (1M+ users) |
|---------|-------------------|------------------|------------------|
| DHT record contention | Not a concern — each record is unique per key pair | Not a concern | Not a concern — DHT scales horizontally |
| Bootstrap nodes | Pkarr default bootstrap nodes sufficient | Same | Consider running own bootstrap node for reliability |
| NFC reliability | Single APDU round-trip is fast | Same | Same |
| SQLite contacts | File-based, no concern at any scale | Add indices on pubkey columns | Add indices, consider FTS for search |
| HKDF seed unseal latency | <10ms, synchronous | Same | Same |
| DHT publish latency | 500ms-3s depending on network | Same | Same — async, doesn't block UX |

---

## 7. Suggested Build Order

Dependencies determine order. Each layer must be testable before the next layer is built on top of it.

```
Phase 1: Rust Core Foundation
  pktap-core crate:
  ├── Key types + zeroize wrappers
  ├── Ed25519 → X25519 conversion
  ├── ECDH + HKDF key derivation
  ├── XChaCha20-Poly1305 encrypt/decrypt
  ├── DNS TXT record serialization (for Pkarr)
  └── Unit tests for all crypto ops
  
  Deliverable: Rust library with 100% test coverage on crypto paths

Phase 2: Pkarr DHT Integration (within Rust)
  pktap-core adds:
  ├── DhtClient wrapping pkarr crate
  ├── publishRecord() + resolveRecord()
  ├── Record name derivation (SHA-256, base32)
  └── Integration test: publish + resolve round-trip
  
  Deliverable: End-to-end DHT round-trip in pure Rust test

Phase 3: UniFFI Bindings + Android Build
  ├── Write pktap.udl interface definition
  ├── Generate Kotlin bindings with uniffi-bindgen
  ├── Android gradle config for AAR packaging
  ├── PktapCore.kt bridge (error mapping, memory zeroing)
  └── Android unit tests: call Rust from JVM, verify zeroing
  
  Deliverable: .aar artifact, Kotlin can call all Rust ops

Phase 4: Android Keystore Module
  ├── KeystoreWrapper.kt: generate/use Ed25519 + AES-256-GCM keys
  ├── First-launch key generation flow
  ├── HKDF seed seal/unseal
  ├── BIP-39 mnemonic display (ephemeral)
  └── Tests: key generation, sign/verify, seal/unseal
  
  Deliverable: Hardware-backed key management working end-to-end

Phase 5: NFC HCE Module
  ├── HostApduService: AID registration, APDU protocol
  ├── NFC reader mode: enableReaderMode(), ReaderCallback
  ├── Single-APDU bidirectional key exchange protocol
  ├── LocalBroadcast delivery to ViewModel
  └── Manual device testing (NFC cannot be unit tested on emulator)
  
  Deliverable: Two physical devices exchange pubkeys via tap

Phase 6: App Integration + UI
  ├── TapScreen ViewModel orchestrates phases 3-5
  ├── Full data flow: tap → ECDH → encrypt → DHT publish
  ├── Full resolve flow: DHT resolve → verify → decrypt → display
  ├── ContactListScreen + SQLite persistence
  ├── ProfileSetupScreen + field selection
  └── End-to-end integration test on two physical devices
  
  Deliverable: Complete working tap-to-exchange flow

Phase 7: QR Fallback + Polish
  ├── QR display/scan (ZXing or ML Kit)
  ├── Async handshake polling (_pktap._handshake.<hash>)
  ├── Public mode (opt-in plaintext DHT profile)
  ├── TTL expiry handling
  └── UX polish, error states, loading indicators
  
  Deliverable: Shippable MVP
```

**Critical dependency chain:**
```
Rust crypto ops → Pkarr DHT → UniFFI bindings → Keystore module
                                              ↘
                                               NFC module → App integration → QR fallback
```

The NFC module and Keystore module can be built in parallel once UniFFI bindings are working. QR fallback depends on the full tap flow being stable.

---

## Sources

- Android NFC HCE documentation: https://developer.android.com/guide/topics/connectivity/nfc/hce — HIGH confidence
- UniFFI documentation: https://mozilla.github.io/uniffi-rs/ — HIGH confidence (Mozilla project, actively maintained)
- Android Keystore system: https://developer.android.com/training/articles/keystore — HIGH confidence
- Pkarr crate: https://github.com/pubky/pkarr — MEDIUM confidence (relatively new library, API may evolve)
- BEP-44 (Mainline DHT mutable items): http://www.bittorrent.org/beps/bep_0044.html — HIGH confidence (stable spec)
- Curve25519 in Android Keystore (API 33+): https://developer.android.com/reference/android/security/keystore/KeyProperties#KEY_ALGORITHM_XDH — HIGH confidence
- XChaCha20-Poly1305 via RustCrypto: https://docs.rs/chacha20poly1305 — HIGH confidence
- zeroize crate: https://docs.rs/zeroize — HIGH confidence
