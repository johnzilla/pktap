# Domain Pitfalls

**Domain:** Decentralized encrypted contact exchange (Pkarr + NFC HCE + Rust/UniFFI + Android)
**Researched:** 2026-04-04
**Confidence:** MEDIUM-HIGH — core claims are grounded in UniFFI source behavior, Android HCE specs, and established
cryptographic literature. DHT/Pkarr-specific claims are MEDIUM confidence (smaller community, less documented).

---

## Critical Pitfalls

Mistakes that cause rewrites or major security issues.

---

### Pitfall 1: UniFFI copies every ByteArray — secrets survive zeroing in Rust but linger in JVM heap

**What goes wrong:**
UniFFI's generated Kotlin bindings always copy byte buffers when crossing the FFI boundary. When Rust returns
a `Vec<u8>` containing a derived key or plaintext, UniFFI serializes it into a JVM `ByteArray`. The Rust side
can call `zeroize()` on the original `Vec<u8>`, but the JVM copy is already on the heap and will not be zeroed
until GC collects it — and GC does not zero memory. In a long-running process, those bytes can sit in heap
dumps, crash reports, or be accessible via memory inspection for an unbounded time.

**Why it happens:**
The JNI specification requires that Java byte arrays are independent objects managed by the JVM. UniFFI
generates `fun fromByteBuffer(buf: ByteBuffer): ByteArray` wrappers that allocate a new `ByteArray` and
copy into it. There is no in-place handoff. This is fundamental to how JNI works, not a UniFFI bug.

**Consequences:**
- Derived ECDH session keys, decrypted contact plaintext, and BIP-39 seed material exist in JVM heap
  after the FFI call returns, even if Rust has zeroed its own copy.
- Heap dumps included in crash reports (Firebase Crashlytics, etc.) may contain key material.
- The `zeroize` crate's cross-FFI story is documented as "we zero our side; JVM is your problem."

**Prevention:**
- Design the Rust API so secrets never cross the FFI boundary in raw form. Instead of returning a derived key,
  pass the plaintext into Rust, encrypt inside Rust, and return only ciphertext. The key never becomes a JVM
  `ByteArray` at all.
- Where a secret must cross (e.g., displaying a BIP-39 mnemonic), explicitly zero the Kotlin `ByteArray`
  immediately after use: `secret.fill(0); secret = null`. This is best-effort given GC, but reduces the window.
- Disable Firebase Crashlytics or configure it to not collect heap dumps for the crypto-touching screens.
- Never return raw key material from UniFFI functions. Architect Rust functions as: "take inputs, return
  ciphertext or a success/error status."

**Warning signs:**
- Any UniFFI function whose return type is `ByteArray` and whose name contains "key", "secret", "seed", or
  "plaintext" — flag for architecture review.
- Kotlin callers that store a returned `ByteArray` in a field or ViewModel rather than a local variable.

**Phase mapping:** Address in Phase 1 (Rust crypto core design). Getting the API boundary wrong here requires
rearchitecting the entire UniFFI surface later.

---

### Pitfall 2: Ed25519 → X25519 key conversion is point-arithmetic, not a hash — and it is one-way with caveats

**What goes wrong:**
The conversion from an Ed25519 signing key to an X25519 Diffie-Hellman key uses the fact that both curves
use the same underlying field (Curve25519). The Ed25519 *private* key conversion applies SHA-512 to the seed,
takes the lower 32 bytes, clamps them, and treats the result as the X25519 scalar. The *public* key conversion
maps the Edwards point to the Montgomery curve via the birational equivalence formula
`u = (1 + y) / (1 - y)`. This is deterministic and correct, BUT:

- The conversion is only defined for the **compressed** Ed25519 public key format (the 32-byte y-coordinate
  with sign bit). If you pass the 64-byte uncompressed form or an in-memory point representation, the formula
  gives garbage.
- The formula `(1 + y) / (1 - y)` has a singularity at `y = 1` (the identity point / point at infinity). For
  well-formed Ed25519 keys this never occurs in practice, but passing a malformed or all-zero public key from
  a rogue NFC peer can trigger a divide-by-zero or produce the all-zero X25519 public key, which then produces
  an all-zero shared secret regardless of your private key.
- The `sign` bit of the Ed25519 y-coordinate is discarded in X25519, so two different Ed25519 keys can map to
  the same X25519 public key (they differ by a cofactor). This is expected behavior, not a bug, but it means
  you cannot verify an X25519 public key by round-tripping it back to Ed25519.

**Why it happens:**
Developers who know "Ed25519 and X25519 are related" often write the conversion themselves or cargo-add a
utility crate without checking whether it handles the compressed-key input requirement and the identity-point
case.

**Consequences:**
- All-zero shared secret from a low-order or identity Ed25519 public key received via NFC → trivially
  predictable derived key → attacker who intercepts the NFC exchange can decrypt all contact records.
- Subtle miscomputation for a subset of valid keys if the wrong byte representation is fed to the formula.

**Prevention:**
- Use `ed25519-dalek`'s built-in `to_montgomery()` method (added in v2.0) which handles the compressed-point
  input correctly. Do not implement the formula manually.
- After receiving a peer's Ed25519 public key over NFC, validate it: deserialize with
  `VerifyingKey::from_bytes()` which rejects low-order points and the identity. Only then convert to
  Montgomery form. If deserialization fails, abort the exchange and show an error — never silently continue.
- Add a test: `assert_ne!(shared_secret, [0u8; 32])` for any ECDH operation in the test suite.

**Warning signs:**
- Manual implementation of the birational formula in Rust rather than using the library method.
- Passing raw bytes from NFC payload directly to ECDH without `VerifyingKey::from_bytes()` validation first.

**Phase mapping:** Phase 1 (Rust crypto core). The validation step belongs in the ECDH function signature itself.

---

### Pitfall 3: NFC HCE Android 10+ requires SELECT AID before any APDU — missing this causes silent failure on many devices

**What goes wrong:**
Android HCE routes incoming NFC APDUs to the registered `HostApduService` based on Application Identifier (AID)
selection. The reader (the other phone acting as an NFC reader, via `IsoDep`) must send a SELECT AID APDU first.
If the reader-side code skips straight to sending data APDUs, some Android devices (particularly Samsung and
Xiaomi on Android 10+) silently route the APDUs to a different service or return 6F00 (internal error) without
ever calling `processCommandApdu()` on your service.

Additionally, Android 12+ introduced a change where HCE services declared with `android:requireDeviceUnlock="true"`
in the manifest will not receive APDUs at all when the screen is off or the device is locked. Many demos omit
this attribute and then wonder why tapping fails in a locked state.

**Why it happens:**
Most HCE tutorials demonstrate tag reading (phone → static NFC tag), not phone-to-phone bidirectional exchange.
The reader-side code in those tutorials is minimal. Developers copy the reader side without implementing proper
ISO 7816-4 APDU framing.

**Consequences:**
- `processCommandApdu()` is never called on one or both devices → exchange silently fails with no error log.
- Works on development Pixel, fails on Samsung Galaxy (most common test failure pattern for HCE apps).

**Prevention:**
- Implement a full ISO 7816-4 command flow: SELECT AID → custom INS byte data commands → response APDUs with
  status words (9000 for success, 6700 for wrong length, 6A80 for incorrect data).
- Register the AID in the `apduservice.xml` with `android:requireDeviceUnlock="false"` so exchanges work
  while the screen is on but before unlock (common use case: tap at a trade show).
- The reader side (the non-HCE phone) must use `IsoDep.connect()` and then `transceive(SELECT_AID_BYTES)`
  as the very first APDU.
- Test explicitly on Samsung (One UI) and Xiaomi (MIUI) — both have HCE routing bugs that Pixel does not.

**Warning signs:**
- HCE tutorial code that jumps straight to `processCommandApdu()` data handling without a SELECT AID handler.
- `apduservice.xml` missing the AID registration entry.
- Only tested on a Pixel emulator or Pixel device.

**Phase mapping:** Phase 2 (NFC exchange). Build the full APDU state machine from the first commit; retrofitting
it later breaks the protocol framing.

---

### Pitfall 4: NFC HCE bidirectional exchange timeout is ~300ms per APDU round-trip — Rust crypto must complete in time

**What goes wrong:**
NFC HCE APDU exchanges have a field timeout enforced by the reader device's NFC controller. On most Android
devices this is configurable but defaults to ~300ms per command-response pair. If the HCE service (your
`processCommandApdu()`) takes longer than this to return a response byte array, the NFC field drops and the
exchange fails with a `TagLostException` on the reader side.

The problematic sequence: the reader sends the peer's public key to the HCE service, which then calls into
Rust via UniFFI to do ECDH + KDF + AEAD encrypt + DHT publish, and then tries to return the encrypted record
in the APDU response. The DHT publish alone can take seconds.

**Why it happens:**
Developers conflate "the exchange is fast" (NFC tap latency ~100ms) with "the entire protocol must complete
in one APDU." The NFC field only stays active while both phones are in proximity; once the response is too slow
the field dies.

**Consequences:**
- Exchange fails at the most critical moment — the tap itself. Users retry repeatedly without success.
- DHT publish during the NFC window is especially dangerous because it requires network I/O.

**Prevention:**
- Split the exchange into two phases: (1) NFC phase — swap public keys only (32 bytes each), immediately
  respond with 9000. (2) Post-NFC async phase — ECDH, encrypt, DHT publish, then poll for peer's record.
  This is exactly the architecture already in PROJECT.md but it must be enforced as a constraint in code.
- The APDU response for the key exchange must return in <200ms to leave headroom. Benchmark the UniFFI call
  overhead alone (typically 1-5ms on modern Android) and the 32-byte payload serialization.
- `processCommandApdu()` should do NO crypto and NO network. Extract the 32-byte public key, store it,
  return `[0x90, 0x00]`. All subsequent work happens in a coroutine after the NFC field drops.

**Warning signs:**
- Any Rust FFI call inside `processCommandApdu()` that does more than byte copying.
- Any network call (DHT publish, DNS resolution) before the APDU response is returned.
- `TagLostException` appearing in logs during integration testing.

**Phase mapping:** Phase 2 (NFC exchange). The coroutine handoff architecture must be established before any
crypto is wired up to the NFC path.

---

### Pitfall 5: Android Keystore StrongBox is unavailable on most mid-range and older devices — must gracefully fall back to TEE

**What goes wrong:**
`KeyGenParameterSpec.Builder.setIsStrongBoxBacked(true)` throws `StrongBoxUnavailableException` on any device
without a dedicated Secure Element chip (most non-flagship phones, all Android emulators, many Samsung A-series).
Apps that require StrongBox and do not catch this exception crash at first launch for a large fraction of users.

A secondary issue: StrongBox-backed keys have tighter algorithm constraints. On some devices, StrongBox does
not support `EC` keys with curve `secp256r1` — but that is irrelevant here since Ed25519 keys are not
generated in the Keystore at all. The Keystore is only used for AES-256-GCM (to encrypt the HKDF seed) and
the StrongBox availability check is for that AES key. However, StrongBox on some Android 10 devices does not
support AES-256-GCM with `KeyProperties.BLOCK_MODE_GCM`; it only supports AES-128. This is documented in
Android 12 release notes.

**Why it happens:**
Testing on a Pixel 6+ or Samsung Galaxy S21+ gives a false sense of StrongBox availability. Those are the
devices where it always works.

**Consequences:**
- App crashes at first launch on a large fraction of target devices.
- If the fallback path is not tested, it introduces a second untested code path that fails silently.

**Prevention:**
- Always wrap `setIsStrongBoxBacked(true)` in a try/catch for `StrongBoxUnavailableException` and retry
  with `setIsStrongBoxBacked(false)` (TEE-backed). Log which mode was used.
- Expose `isStrongBoxBacked()` in the UI settings (optional but useful for privacy-conscious users).
- Test the TEE fallback path explicitly with an emulator — emulators never have StrongBox.
- For the AES key, use `AES/GCM/NoPadding` with 256-bit key size, but test on real mid-range hardware
  (Moto G series, Samsung A series) since StrongBox on those devices may silently downgrade key operations.

**Warning signs:**
- `KeyGenParameterSpec.Builder.setIsStrongBoxBacked(true)` without a surrounding try/catch.
- No log statement recording whether StrongBox or TEE was used at key generation time.

**Phase mapping:** Phase 1 (Android Keystore setup). First thing to implement; every later operation depends on it.

---

### Pitfall 6: Android Keystore key invalidation on biometric enrollment change silently breaks the app for existing users

**What goes wrong:**
When `setUserAuthenticationRequired(true)` is set on a Keystore key AND `setInvalidatedByBiometricEnrollment(true)`
is set (which is the default in API 24+), the key is permanently destroyed whenever the user adds or removes
a fingerprint or face scan. Attempts to use the key after this return `KeyPermanentlyInvalidatedException`.

For PKTap's architecture, the AES-GCM Keystore key encrypts the HKDF seed. If that key is invalidated, the
seed is unrecoverable from `EncryptedSharedPreferences`, and the user's entire identity (Ed25519 keypair,
all contact records) is lost unless they have their BIP-39 mnemonic.

**Why it happens:**
Developers set `setUserAuthenticationRequired(true)` to require biometrics before the key can be used (good
security practice), but don't realize the enrollment-change invalidation is opt-in-to-disable via
`setInvalidatedByBiometricEnrollment(false)`.

**Consequences:**
- User adds a new fingerprint (e.g., after wearing gloves for months) → next app launch, seed decryption
  throws `KeyPermanentlyInvalidatedException` → app appears broken with no obvious error message.
- If the BIP-39 mnemonic was never written down, the user loses all contact data permanently.

**Prevention:**
- Use `setInvalidatedByBiometricEnrollment(false)` for the AES key that protects the HKDF seed. The seed
  is recoverable via BIP-39 mnemonic anyway, so the biometric-invalidation "protection" adds no real security.
- Implement a `KeyPermanentlyInvalidatedException` handler that prompts the user to enter their BIP-39
  mnemonic to recover the seed, then re-generates the Keystore key.
- Make the mnemonic backup screen impossible to skip (not just a "later" button that users always click).
- Document this behavior explicitly in onboarding: "If you change your fingerprints, you will need your
  recovery phrase."

**Warning signs:**
- `setInvalidatedByBiometricEnrollment` not explicitly set (relying on default).
- No `KeyPermanentlyInvalidatedException` handler anywhere in the codebase.
- Mnemonic backup is skippable.

**Phase mapping:** Phase 1 (key generation) and Phase 3 (recovery flow). The key generation parameters must
be correct from Phase 1; the recovery flow is Phase 3.

---

### Pitfall 7: Pkarr record size limit is 1000 bytes including the signature and DNS wire format overhead — not 1000 bytes of payload

**What goes wrong:**
Pkarr publishes BEP-44 mutable items to Mainline DHT. The value field in a BEP-44 item is limited to 1000
bytes. Pkarr packs DNS TXT records in DNS wire format inside that value. The overhead is:
- Ed25519 signature: 64 bytes
- Sequence number (u64 big-endian): 8 bytes
- DNS wire format overhead: ~20-30 bytes per TXT record (name label length bytes + type + class + TTL + rdlength)

So for a single TXT record containing encrypted contact data, the actual usable payload is approximately
1000 - 64 - 8 - 30 = **898 bytes**. With XChaCha20-Poly1305 overhead (24-byte nonce + 16-byte tag = 40 bytes),
the usable plaintext is ~858 bytes. PROJECT.md notes ~600 bytes, which seems to include additional field
name overhead and base64/hex encoding of the ciphertext within the TXT value. Whichever estimate is correct,
hitting this limit silently truncates or errors — Pkarr will refuse to publish a record that exceeds 1000 bytes.

**Why it happens:**
Developers test with a small contact record (just a name) and everything works. They then add phone, email,
signal handle, website, and notes — and the publish silently fails or returns an error that the app ignores.

**Consequences:**
- Silent publish failure when contact fields are numerous — user thinks their contact was shared but it wasn't.
- If error handling is missing, the UI shows "shared successfully" while the record was never written to DHT.

**Prevention:**
- Implement a byte-budget checker in Rust before attempting to serialize: compute the serialized size of the
  contact record and return an error (not a panic) if it exceeds 900 bytes (leaving 100 bytes headroom for
  DNS wire format). Surface this to the UI as "Contact record too large — remove some fields."
- Enforce a maximum field length per field (e.g., 100 bytes for display name, 64 for email) in the UI layer
  before the record is assembled in Rust.
- Write a test that publishes a maximum-size record and verifies the DHT node accepts it, then write a test
  that attempts to exceed the limit and verifies the error path.

**Warning signs:**
- No byte-budget validation before serialization.
- Pkarr `publish()` return value or `Result` is not checked.
- Test records only contain "Alice" and "+1234567890."

**Phase mapping:** Phase 2 (record construction and DHT publish). Must be addressed before any end-to-end
testing with real contact data.

---

## Moderate Pitfalls

---

### Pitfall 8: DHT bootstrap nodes may be unreachable on corporate/restricted networks — no fallback means silent failure

**What goes wrong:**
Pkarr uses Mainline DHT bootstrap nodes (the same ones BitTorrent uses: `router.bittorrent.com:6881`,
`dht.transmissionbt.com:6881`, etc.) over UDP. Corporate firewalls, some mobile carrier NATs, and restrictive
home routers frequently block outbound UDP on non-standard ports. The DHT publish/resolve will time out with
no informative error.

**Why it happens:**
Developers test on home WiFi where DHT works fine. Enterprise environments and some cellular networks block
DHT traffic.

**Prevention:**
- Surface DHT connectivity status explicitly in the UI ("Checking network connectivity...") and show a clear
  error when bootstrap nodes are unreachable.
- Implement a timeout of ~10 seconds for DHT operations and display "Could not reach contact network —
  check your connection" rather than spinning forever.
- Consider whether to offer an HTTP/HTTPS fallback relay for enterprise users (this would require a PKTap
  server, which is out of scope for MVP, but plan for it).

**Warning signs:**
- DHT `publish()` or `resolve()` calls with no timeout or with an unbounded wait.
- No UI indication that a DHT operation is in progress.

**Phase mapping:** Phase 2 (DHT integration). Test on a mobile hotspot, not just home WiFi.

---

### Pitfall 9: Nonce reuse with XChaCha20-Poly1305 is catastrophically bad — random nonces are correct here but require entropy

**What goes wrong:**
XChaCha20-Poly1305 with a reused nonce (same nonce + same key) leaks the keystream XOR of the two
plaintexts. This is the "two-time pad" attack and completely breaks confidentiality. XChaCha20's 192-bit
nonce is specifically designed to be safe for random generation (the probability of collision given 2^32
messages under the same key is astronomically small), but only if the RNG is cryptographically secure.

On Android, `OsSecureRandom` (or `java.security.SecureRandom` with the Android provider) seeded from
`/dev/urandom` is the correct source. In Rust, `rand::rngs::OsRng` is correct. The pitfall is using
`thread_rng()` (which seeds from `OsRng` but caches the state — fine in practice but conceptually impure
for crypto) or, worse, using a test-mode seed in production by accident.

A distinct pitfall: if the HKDF-derived encryption key is derived deterministically from the same inputs
for every message (e.g., `HKDF(shared_secret, "pktap-enc", "")` with no per-message variation), then
every contact record exchange for the same pair of keys uses the same key, and the nonce must never repeat.
This is fine with random nonces, but only if the key-per-message architecture is understood by all developers
on the team.

**Prevention:**
- Use `OsRng` directly in Rust for nonce generation: `let nonce = XNonce::from(OsRng.gen::<[u8; 24]>())`.
  Never reuse nonces. Never derive nonces deterministically from message content.
- Add a comment in the crypto code explaining why random nonces are safe here (192-bit nonce space).
- CI lint: grep for `thread_rng` in crypto paths and flag it for review.

**Warning signs:**
- Deterministic nonce derivation (e.g., a counter or hash of the ciphertext).
- `thread_rng()` in production crypto code.
- Missing nonce prepended to ciphertext in the stored record (means decryption cannot function and suggests
  nonce was hardcoded or not stored).

**Phase mapping:** Phase 1 (Rust crypto core). Establish the nonce generation pattern before any encryption
function is written.

---

### Pitfall 10: HKDF info string collisions silently derive the same key for different purposes

**What goes wrong:**
HKDF takes an `info` parameter to domain-separate derived keys. If two different derivation purposes use the
same info string (or if the info string is omitted), they derive the same key material from the same IKM.
In PKTap's case, HKDF might be called for: (1) deriving the contact record encryption key from the ECDH
shared secret, (2) deriving a handshake nonce for the QR async flow, (3) deriving a local storage key from
the HKDF seed. If any two of these use the same IKM + salt + info, they produce the same output.

**Prevention:**
- Define a constants module in Rust with named `info` strings for every HKDF invocation:
  ```rust
  const HKDF_INFO_CONTACT_ENC: &[u8] = b"pktap-v1-contact-enc";
  const HKDF_INFO_HANDSHAKE:    &[u8] = b"pktap-v1-qr-handshake";
  const HKDF_INFO_LOCAL_STORE:  &[u8] = b"pktap-v1-local-store";
  ```
- Never pass an empty `info` or a runtime-computed string to HKDF. All info values must be compile-time
  constants checked in code review.
- Add a test asserting that each info constant is unique (simple string equality check in the test suite).

**Warning signs:**
- `hkdf.expand(b"pktap", ...)` used for multiple purposes.
- HKDF info passed as a function parameter from Kotlin (suggests the caller controls domain separation,
  which is a footgun).

**Phase mapping:** Phase 1 (Rust crypto core). Establish all HKDF info constants before any key derivation
function is implemented.

---

### Pitfall 11: QR polling (2s / 5 min) creates a DHT amplification concern and drains battery

**What goes wrong:**
The QR async handshake polls `_pktap._handshake.<hash>` every 2 seconds for up to 5 minutes. That is 150
DHT `get` operations per exchange. Each Pkarr `resolve()` call may trigger multiple DHT routing table
queries under the hood. On mobile, this drains battery noticeably and may trigger Android's battery
optimization to kill the background service.

Additionally, if many PKTap users are scanning QR codes simultaneously, the same bootstrap nodes see a
significant query rate — though this is unlikely to be a practical problem at MVP scale.

**Prevention:**
- Use exponential backoff for polling: 1s, 2s, 4s, 8s, up to a max of 30s. This cuts DHT queries to ~15
  over 5 minutes instead of 150.
- Use Android `WorkManager` with a `PeriodicWorkRequest` (minimum 15 min interval) or foreground service
  notifications — not a raw coroutine timer — to avoid battery optimization killing the poll.
- Show a progress indicator with a visible countdown so users know the timeout has a bound.

**Warning signs:**
- `delay(2000)` in a loop for polling without backoff.
- No foreground notification for the polling service.

**Phase mapping:** Phase 3 (QR fallback flow).

---

### Pitfall 12: SQLite sensitive columns — Keystore AES key must be loaded per-operation, not cached in memory

**What goes wrong:**
The architecture uses column-level AES encryption for SQLite sensitive data via a Keystore-managed key.
A common implementation mistake is to decrypt the Keystore key once at startup and cache the raw AES key
bytes in a ViewModel or Application class field "for performance." This negates the Keystore's protection:
the key is now in JVM heap for the entire app lifetime, accessible in heap dumps.

**Prevention:**
- Never cache the AES key material outside of the Keystore abstraction. Each SQLite read that needs
  decryption should request the key from Keystore inline. The Keystore is fast enough (sub-millisecond
  for a cached TEE operation) that this is not a performance problem.
- Use `Cipher` directly with the Keystore `SecretKey` reference — the Keystore never returns raw key bytes
  via this API. The raw bytes never exist in JVM memory.

**Warning signs:**
- `secretKey.encoded` called anywhere (returns raw key bytes from a Keystore key, which should be null
  for non-extractable keys — if it returns non-null, the key was generated extractable by mistake).
- A field or singleton holding a `ByteArray` or `SecretKey.encoded` value.

**Phase mapping:** Phase 2 (SQLite/local storage).

---

## Minor Pitfalls

---

### Pitfall 13: BIP-39 mnemonic displayed in a regular TextView can appear in Recent Apps screenshots

**What goes wrong:**
Android's Recent Apps screen captures a screenshot of each activity. If the BIP-39 mnemonic is shown in a
normal `TextView`, it appears in the Recent Apps thumbnail. This is a privacy leak on shared devices.

**Prevention:**
- Set `window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)`
  on the Activity showing the mnemonic.
- Or use Jetpack Compose's `SecureWindow` (API 33+) equivalent or a dedicated composable that sets the
  FLAG_SECURE flag.

**Phase mapping:** Phase 1 (onboarding/backup screen).

---

### Pitfall 14: Pkarr sequence numbers must be monotonically increasing — republishing with a stale sequence number fails silently

**What goes wrong:**
BEP-44 mutable items require a `seq` field that must be greater than the current `seq` stored in DHT nodes
for that key. If you republish the same record with `seq = 0` or a previously used sequence number, DHT
nodes reject the update silently (they return success but don't actually update). The `resolve()` call
then returns the stale record.

**Why it happens:**
When the app is reinstalled or the Rust state is reset without persisting the sequence number, it restarts
at `seq = 0`, which is always less than any previously published value.

**Prevention:**
- Persist the last-used Pkarr sequence number in EncryptedSharedPreferences alongside the HKDF seed.
- On `publish()`, always use `seq = current_unix_timestamp_seconds` (u64). Timestamps are monotonically
  increasing as long as the device clock doesn't go backwards and the same key doesn't publish more than
  once per second. This is the pattern Pkarr recommends.
- On app reinstall/recovery, start `seq` from the current time, which will be greater than any previously
  published value (assuming records expire within 24 hours and the user reinstalls after that).

**Warning signs:**
- `seq = 0` hardcoded anywhere.
- `seq` not persisted across app restarts.

**Phase mapping:** Phase 2 (DHT publish integration).

---

### Pitfall 15: `cargo uniffi-bindgen generate` output is not idiomatic Kotlin — auto-generated names conflict with Kotlin reserved words

**What goes wrong:**
UniFFI generates Kotlin bindings whose names are derived from Rust function and type names. Rust naming
conventions (snake_case functions, PascalCase types) get mapped to Kotlin conventions, but there are
edge cases: Rust functions named `type`, `use`, `in`, `is`, `when`, or `object` (reserved Kotlin words)
will produce non-compilable Kotlin output. This is a build-time failure, not a runtime failure — easy
to catch early.

A subtler issue: UniFFI generates `suspend fun` wrappers for async Rust functions, but these require
the `kotlinx-coroutines-core` dependency at a specific version. If the version is wrong, the generated
coroutine glue fails to compile with cryptic error messages about missing extension functions.

**Prevention:**
- Avoid Rust function names that match Kotlin reserved words (use `r#type` in Rust if you must).
- Pin `kotlinx-coroutines-core` to a version explicitly tested with the UniFFI version in use.
- Add `./gradlew build` to CI from the very first commit that includes generated bindings.

**Phase mapping:** Phase 1 (UniFFI scaffolding).

---

### Pitfall 16: `zeroize` crate behavior with Rust's move semantics — values may be cloned before zeroing

**What goes wrong:**
If a `Zeroize`-implementing type (e.g., `zeroize::Zeroizing<Vec<u8>>`) is cloned before `drop()` is called,
the clone is not zeroed when the original is dropped. In async Rust code, `await` points can cause the
compiler to keep copies of values across tasks that outlive the original scope.

**Prevention:**
- Use `Zeroizing<>` wrappers for all secret material.
- Avoid `.clone()` on secret types; use `Arc<Zeroizing<>>` for shared ownership instead.
- Run `cargo-audit` and `cargo-geiger` in CI to detect unsafe memory operations in dependencies.
- Review all `async fn` in the crypto path for unintentional copies across `await` points.

**Phase mapping:** Phase 1 (Rust crypto core).

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| UniFFI API design | Secrets as return values become JVM heap objects (Pitfall 1) | Design functions to encrypt inside Rust, return ciphertext only |
| Keystore key generation | StrongBox unavailability crash (Pitfall 5) | Try/catch with TEE fallback, test on emulator |
| Keystore key generation | Biometric enrollment invalidation (Pitfall 6) | `setInvalidatedByBiometricEnrollment(false)`, recovery flow |
| ECDH key conversion | All-zero shared secret from malformed peer key (Pitfall 2) | `VerifyingKey::from_bytes()` validation before conversion |
| HKDF in Rust crypto core | Domain collision across derivation purposes (Pitfall 10) | Compile-time info constants, uniqueness test |
| AEAD encryption | Nonce reuse or wrong RNG (Pitfall 9) | `OsRng`, random 192-bit nonces, no deterministic nonces |
| NFC HCE wiring | Missing SELECT AID → silence on Samsung/Xiaomi (Pitfall 3) | Full ISO 7816-4 APDU state machine from first commit |
| NFC HCE wiring | Crypto in `processCommandApdu()` → timeout (Pitfall 4) | Key exchange only in HCE; all crypto in post-tap coroutine |
| DHT publish first integration | Record size limit exceeded (Pitfall 7) | Byte-budget check in Rust before serialization |
| DHT publish first integration | Stale sequence number on republish (Pitfall 14) | Use Unix timestamp as `seq`, persist across restarts |
| SQLite local storage | AES key cached in heap (Pitfall 12) | Use Keystore `SecretKey` reference inline, never `.encoded` |
| QR fallback flow | Battery drain + BGS kill from polling loop (Pitfall 11) | Exponential backoff + foreground service notification |
| Onboarding / mnemonic display | Screenshot in Recent Apps (Pitfall 13) | `FLAG_SECURE` on the mnemonic Activity |

---

## Sources

**Confidence notes:**
- UniFFI ByteArray copy behavior: HIGH — this is fundamental JNI behavior, documented in JNI spec and
  confirmed by UniFFI architecture (uniffi-rs GitHub source shows buffer copy in generated bindings).
- Ed25519→X25519 conversion pitfalls: HIGH — birational equivalence formula is mathematical fact;
  `ed25519-dalek` `to_montgomery()` API verified in crate docs (v2.x series).
- Android HCE SELECT AID requirement: HIGH — documented in Android HCE developer guide and ISO 7816-4.
  Samsung/Xiaomi routing behavior: MEDIUM — community-reported, not in official Android docs.
- Android HCE timeout (~300ms): MEDIUM — this is a practical observed value from the community; the
  actual value is hardware/driver-dependent and not specified in Android docs.
- StrongBox constraints: HIGH — documented in Android API reference for `KeyGenParameterSpec`.
- Biometric enrollment invalidation: HIGH — documented in `KeyPermanentlyInvalidatedException` Javadoc.
- Pkarr 1000-byte limit: HIGH — BEP-44 specifies 1000 bytes for value; overhead calculation is MEDIUM
  (computed from spec, not empirically measured).
- BEP-44 sequence number behavior: HIGH — specified in BEP-44.
- DHT firewall/UDP blocking: MEDIUM — observed behavior, not officially documented anywhere.
- HKDF domain separation: HIGH — HKDF RFC 5869 §3.2.
- XChaCha20-Poly1305 nonce safety: HIGH — cryptographic standard, IETF draft-irtf-cfrg-xchacha.
- `zeroize` crate async pitfalls: MEDIUM — documented in `zeroize` crate README and common Rust async
  security discussions; specific async behavior depends on compiler version.
- UniFFI Kotlin reserved word collisions: MEDIUM — known issue class for codegen tools; specific words
  are illustrative but the pattern is real.
- `FLAG_SECURE` for mnemonic screen: HIGH — Android WindowManager documentation.
- Pkarr `seq = unix_timestamp` pattern: MEDIUM — recommended in Pkarr project README/examples as of
  knowledge cutoff; verify against current Pkarr documentation before implementation.
