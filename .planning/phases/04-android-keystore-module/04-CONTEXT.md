# Phase 4: Android Keystore Module - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

First-launch key generation and seed management on Android. Generates AES-256-GCM key in Keystore (with StrongBox/TEE fallback), creates a random HKDF seed, encrypts it with the Keystore AES key via EncryptedSharedPreferences, derives the Ed25519 keypair from the seed in Rust, displays a BIP-39 mnemonic for backup, and handles the complete first-launch flow. No NFC, no contact exchange, no DHT — just key infrastructure and the mnemonic screen.

</domain>

<decisions>
## Implementation Decisions

### Key Generation & Storage
- **D-01:** Ed25519 master keypair derived from HKDF seed in Rust (not stored in Keystore directly — Android Keystore doesn't support Ed25519). Generate random 32-byte seed, encrypt with Keystore AES-256-GCM key, store in EncryptedSharedPreferences. When needed, decrypt and pass to Rust which derives Ed25519 via ed25519-dalek. Keystore protects the seed at rest.
- **D-02:** Try StrongBox first, silent TEE fallback. Attempt `KeyGenParameterSpec.Builder().setIsStrongBoxBacked(true)`. Catch `StrongBoxUnavailableException`, retry without the flag. User never sees the difference. Log which path was used for diagnostics.

### BIP-39 Mnemonic UX
- **D-03:** 12-word mnemonic (128 bits entropy). Standard for mobile wallets, easier to write down. Sufficient security for a contact exchange app.
- **D-04:** No word re-entry verification. Display words with a checkbox: "I have written down these words". Simple acknowledgment only — the mnemonic is a backup, not a password.

### Seed Lifecycle & FFI Handoff
- **D-05:** Decrypt seed from EncryptedSharedPreferences into a ByteArray, pass to PktapBridge, zero in finally block. Seed exists in Kotlin memory only for the duration of the FFI call. No caching of the decrypted seed.
- **D-06:** Derive Ed25519 pubkey once on app start, cache in memory (singleton/ViewModel). Pubkey is not secret — no need to re-derive for every NFC tap or DHT address computation. Zero the seed immediately after derivation.

### First-Launch Flow
- **D-07:** Detect first launch by checking EncryptedSharedPreferences for the encrypted seed. If present → returning user → main screen. If absent → first launch → key generation + mnemonic flow. No separate "has_launched" flag.
- **D-08:** Store a `mnemonic_acknowledged` boolean alongside the seed. On next launch, if seed exists but flag is false, show the mnemonic screen again. User must acknowledge before reaching the main screen. Prevents losing the backup opportunity if process is killed mid-setup.

### Claude's Discretion
- Exact EncryptedSharedPreferences key names
- ViewModel vs singleton for pubkey cache
- Mnemonic screen visual layout and typography
- Whether to use Hilt for dependency injection in this phase
- BIP-39 mnemonic generation: Rust (bip39 crate) vs Kotlin library
- Navigation architecture (Compose Navigation setup)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Specifications
- `.planning/PROJECT.md` — Core constraints (non-extractable Keystore keys, BIP-39 mnemonic at first launch)
- `.planning/REQUIREMENTS.md` §Key Management — KEY-01 through KEY-05 acceptance criteria
- `CLAUDE.md` §Technology Stack — EncryptedSharedPreferences 1.1.0-alpha06, security-crypto, bip39 2.0.x

### Prior Phase Code (reuse these)
- `android/rust-bridge/src/main/java/com/pktap/bridge/PktapBridge.kt` — FFI wrapper with ByteArray zeroing (Phase 3 D-06). Extend with seed-to-pubkey derivation function.
- `pktap-core/src/ffi.rs` — `ecdh_and_encrypt`, `decrypt_and_verify` accept seed bytes. May need a new `derive_public_key(seed)` export.
- `pktap-core/src/keys.rs` — `seed_to_x25519_scalar`, `ed25519_pub_to_x25519_pub` — key derivation primitives
- `android/rust-bridge/build.gradle.kts` — Build pipeline (cargo-ndk + UniFFI bindgen)

### Prior Phase Decisions
- `.planning/phases/01-rust-crypto-core/01-CONTEXT.md` — D-02 (Rust accepts raw key bytes from Kotlin), D-03 (split signing model)
- `.planning/phases/03-uniffi-bridge-android-build/03-CONTEXT.md` — D-06 (PktapBridge.kt ByteArray zeroing)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `PktapBridge.kt` — Already wraps FFI calls with ByteArray zeroing. Extend with key derivation wrapper.
- `uniffi/pktap_core/pktap_core.kt` — Generated UniFFI bindings. Any new Rust FFI export (`derive_public_key`) will appear here after bindgen.
- `android/app/src/main/java/com/pktap/app/MainActivity.kt` — Minimal Compose activity. Will host the navigation graph for first-launch flow.

### Established Patterns
- PktapBridge wrapper pattern for FFI zeroing
- Multi-module: `:app` depends on `:rust-bridge`
- jvmToolchain(21), AGP 8.7.3, Kotlin 2.0.21

### Integration Points
- Phase 5 (NFC) will read the cached pubkey from the singleton/ViewModel
- Phase 6 (App Integration) will use the seed-decrypt-then-FFI pattern established here for every crypto operation
- The first-launch navigation flow established here becomes the app's entry point architecture

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

*Phase: 04-android-keystore-module*
*Context gathered: 2026-04-05*
