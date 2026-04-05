# Phase 4: Android Keystore Module - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-05
**Phase:** 04-android-keystore-module
**Areas discussed:** Key generation & storage flow, BIP-39 mnemonic UX, Seed lifecycle & FFI handoff, First-launch flow architecture

---

## Key Generation & Storage Flow

| Option | Description | Selected |
|--------|-------------|----------|
| Derive Ed25519 from HKDF seed in Rust | Seed encrypted in EncryptedSharedPreferences, passed to Rust for derivation | ✓ |
| Keystore EC + separate Ed25519 | Two key types, P-256 not used by protocol | |
| You decide | | |

**User's choice:** Derive from HKDF seed in Rust

| Option | Description | Selected |
|--------|-------------|----------|
| Try StrongBox first, silent TEE fallback | Catch StrongBoxUnavailableException, retry without | ✓ |
| Detect StrongBox upfront | Check PackageManager before generating | |
| You decide | | |

**User's choice:** Try StrongBox, silent TEE fallback

---

## BIP-39 Mnemonic UX

| Option | Description | Selected |
|--------|-------------|----------|
| 12 words | 128-bit entropy, standard for mobile | ✓ |
| 24 words | 256-bit entropy, matches seed exactly | |
| User chooses | Adds UI complexity | |

**User's choice:** 12 words

| Option | Description | Selected |
|--------|-------------|----------|
| No verification, checkbox acknowledge | Simple, low friction | ✓ |
| Verify 3 random words | Proves written down, more friction | |
| You decide | | |

**User's choice:** Checkbox acknowledge only

---

## Seed Lifecycle & FFI Handoff

| Option | Description | Selected |
|--------|-------------|----------|
| Decrypt → PktapBridge → zero | Seed in Kotlin only for FFI call duration | ✓ |
| Keep in ViewModel | Fewer decrypts but longer exposure | |
| You decide | | |

**User's choice:** Decrypt, pass, zero immediately

| Option | Description | Selected |
|--------|-------------|----------|
| Derive once, cache in memory | Pubkey not secret, cache in singleton | ✓ |
| Persist in SharedPreferences | Available without Rust call | |
| Re-derive every time | Most secure but wasteful | |

**User's choice:** Derive once, cache in memory

---

## First-Launch Flow Architecture

| Option | Description | Selected |
|--------|-------------|----------|
| Check EncryptedSharedPreferences for seed | Single source of truth | ✓ |
| Separate boolean in DataStore | More explicit but second state | |
| You decide | | |

**User's choice:** Check for seed existence

| Option | Description | Selected |
|--------|-------------|----------|
| Require mnemonic_acknowledged flag | Show mnemonic again if interrupted | ✓ |
| Regenerate on incomplete setup | Simpler but destructive | |
| You decide | | |

**User's choice:** Mnemonic ack flag — resume from mnemonic screen if interrupted

---

## Claude's Discretion

- EncryptedSharedPreferences key names
- ViewModel vs singleton for pubkey cache
- Mnemonic screen layout
- DI framework choice
- BIP-39 generation location (Rust vs Kotlin)
- Navigation architecture

## Deferred Ideas

None.
