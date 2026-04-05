# Phase 2: Pkarr DHT Integration - Research

**Researched:** 2026-04-05
**Domain:** pkarr DHT publish/resolve in pure Rust
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Use pkarr 2.3.x (not 5.x+). Research confirmed 2.3.x is compatible with ed25519-dalek 2.x and curve25519-dalek 4.x from Phase 1. Pkarr 5.x pulls dalek 3.0.0-pre which conflicts with the Phase 1 crypto stack.
- **D-02:** Pkarr signs DHT records in Rust using the HKDF-derived key. This is transport-layer signing (BEP-44 requires it), separate from the contact payload signature handled by Android Keystore (Phase 1 D-03). The HKDF-derived key is acceptable for DHT record signing because it's a transport concern, not an identity assertion.
- **D-03:** In-memory queue (VecDeque) for pending publishes. Lost on process kill — Android layer (Phase 6) can persist to Room/SQLite if needed. Keeps Phase 2 scope minimal.
- **D-04:** Publish-attempt-based retry with exponential backoff (1s, 2s, 4s... capped at 60s). No separate connectivity detection — the publish attempt IS the connectivity check. On network error, enqueue and schedule retry.
- **D-05:** Phase 2 provides the republish mechanism and TTL tracking. Phase 6 (Android WorkManager) triggers republishing on schedule. Phase 2 does NOT run background tasks or spawn Tokio timers — it exposes `republish(record_key)` and `get_records_expiring_before(timestamp)`.
- **D-06:** BEP-44 sequence numbers use `SystemTime::now()` as u64 unix timestamp at publish time. If two publishes happen in the same second, increment by 1. No persistent counter needed.
- **D-07:** Use a local DHT bootstrap node in the test process via pkarr/mainline test helpers. No external network required. Tests are deterministic and fast. Publish and resolve against the local node.

### Claude's Discretion

- DhtClient module structure (single file vs split)
- Async runtime choice (tokio features needed for pkarr)
- Exact retry backoff implementation details
- Whether to expose TTL as a configurable parameter or hardcode 24h/7d defaults
- Error variant naming for DHT-specific failures (DhtPublishFailed, DhtResolveFailed, etc.)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DHT-01 | App publishes signed encrypted record to Mainline DHT at deterministic address `_pktap._share.<SHA-256(sort(A_pk, B_pk))>` | pkarr 2.3.0 `PkarrClient::publish()` + `Keypair::from_secret_key()` with HKDF-derived bytes; record name is the TXT subdomain within the signed packet; DHT address is the derived keypair's public key |
| DHT-02 | App resolves encrypted record from DHT by computing same deterministic address | `PkarrClient::resolve(&public_key)` returns `Option<SignedPacket>`; caller calls `signed_packet.resource_records("_pktap._share.<hash>")` to find the TXT record |
| DHT-03 | App publishes plaintext DNS TXT records for public mode at `_pktap._profile.<derived_key>` | Same `publish()` API; uses `public_profile_name()` from Phase 1 as the TXT record name; keypair derived from profile key |
| DHT-04 | App resolves public mode records from DHT given a public key | Same `resolve()` API with peer public key; calls `resource_records("_pktap._profile.<hex>")` |
| DHT-05 | Encrypted records have default TTL of 24 hours; public records have default TTL of 7 days | TTL is the `ttl` field of each `ResourceRecord` in the DNS packet; pass `86400` for private, `604800` for public |
| DHT-06 | App uses monotonically increasing BEP-44 sequence numbers (unix timestamp) for record versioning | `SignedPacket::from_packet()` auto-generates timestamp in microseconds via `system_time()`; no manual seq needed; pkarr returns `Error::NotMostRecent` when a newer packet already exists |
| DHT-07 | App validates record payload fits within ~858 usable bytes before publish | Analysis: 750-byte plaintext → 791-byte ciphertext → ~898-949 byte DNS packet; pkarr enforces hard 1000-byte limit with `Error::PacketTooLarge`; DhtClient pre-validates ciphertext size before building packet |
| DHT-08 | App queues DHT publish operations when offline and syncs when connectivity returns | `VecDeque<PendingPublish>` in DhtClient; on `Error::MainlineError` enqueue + schedule next retry; `flush_queue()` drains on next successful network attempt |
</phase_requirements>

---

## Summary

Phase 2 adds a `DhtClient` struct to `pktap-core` that wraps pkarr 2.3.0's `PkarrClient` and provides publish/resolve for both private (encrypted) and public (plaintext) contact records. The module is pure Rust, synchronous (no tokio dependency needed), and extends Phase 1's crypto core.

**Critical architectural clarification (verified from pkarr source):** In pkarr, the DHT address is ALWAYS the signer's Ed25519 public key — there are no arbitrary DHT keys. The `_pktap._share.<hash>` in the requirements is the DNS TXT record **name within** the signed packet, not a standalone DHT key. This means the DhtClient uses the HKDF-derived bytes as an Ed25519 seed (via `pkarr::Keypair::from_secret_key(&[u8; 32])`) to create a signing keypair. Because ECDH is symmetric, both Alice and Bob derive the identical keypair from their shared secret — both can publish and resolve from the same DHT address. The `_pktap._share.<hash>` is then the subdomain name within the packet that carries the ciphertext.

**Sequence number management:** `SignedPacket::from_packet()` auto-generates a microsecond-resolution unix timestamp (via `SystemTime::now()`). D-06's "unix timestamp" guidance is satisfied internally by pkarr. DhtClient does not manage seq numbers manually — pkarr returns `Error::NotMostRecent` when a newer packet already exists at that address.

**No tokio needed:** `PkarrClient` (sync) uses `flume` channels internally and runs its own background thread. Zero tokio dependency unless you use the `async` feature (which Phase 2 does not need).

**Primary recommendation:** Use `pkarr 2.3.0` with `features = ["dht"]` (default). Use `pkarr::mainline::Testnet::new(N)` and `DhtSettings { bootstrap: Some(testnet.bootstrap), .. Default::default() }` for all integration tests.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pkarr | 2.3.0 | Mainline DHT publish/resolve via `PkarrClient` | Only maintained Rust pkarr implementation; wraps BEP-44 DHT with Ed25519 signing; 2.3.x is the last version compatible with ed25519-dalek 2.x |

### Dependencies Pulled in by pkarr 2.3.0

| Library | Version | Notes |
|---------|---------|-------|
| mainline | 2.0.1 | BEP-44 DHT implementation; provides `Testnet` struct for tests |
| simple-dns (pkarr re-exports as `dns`) | 0.9.1 | DNS packet construction; `CharacterString::new(&[u8])` for binary TXT records |
| ed25519-dalek | ^2.1.1 | Compatible with Phase 1's 2.2.0 — no version conflict |
| flume | 0.11.1 | Internal channel for sync client actor loop — not a dev concern |

**No additional dependencies needed for Phase 2.** pkarr 2.3.0 transitively provides all required types.

### Workspace Dependencies (existing, no change)

| Library | Version | Pinned In |
|---------|---------|-----------|
| curve25519-dalek | 4.1.3 | Cargo.toml workspace root |
| zeroize | 1.8.2 | Cargo.toml workspace root |

**Version verification (performed 2026-04-05):**
- pkarr 2.3.0: published 2025-01-09 — CONFIRMED exists on crates.io [VERIFIED: crates.io API]
- pkarr 2.3.0 requires ed25519-dalek ^2.1.1 — Phase 1 uses 2.2.0 — NO CONFLICT [VERIFIED: crates.io dependencies API]
- mainline 2.0.1 requires ed25519-dalek ^2.1.0 — NO CONFLICT [VERIFIED: crates.io dependencies API]
- Neither pkarr 2.3.0 nor mainline 2.0.1 directly pins curve25519-dalek — workspace pin 4.1.3 takes precedence [VERIFIED: source inspection]

**Installation (add to `pktap-core/Cargo.toml`):**
```toml
pkarr = { version = "2.3.0", features = ["dht"] }
```

`dht` feature enables `PkarrClient` and pulls `mainline`. The `rand` feature (also default) is not needed since Phase 2 creates keypairs from existing seed bytes, not randomly. Explicitly disable `rand` to reduce dependencies if desired: `features = ["dht"], default-features = false`.

For integration tests, `mainline::Testnet` is available via pkarr's re-export without adding a separate `mainline` dev-dependency.

---

## Architecture Patterns

### DHT Address Model (Critical)

```
  HKDF-derived 32 bytes
         |
         v
  pkarr::Keypair::from_secret_key(&[u8;32])
         |
         v
  keypair.public_key()  <-- This IS the DHT address
         |
  SignedPacket::from_packet(&keypair, &dns_packet)
         |
  dns_packet contains ResourceRecord {
      name: "_pktap._share.<hash>",   <-- record lookup key within packet
      ttl: 86400,
      rdata: RData::TXT(binary_ciphertext),
  }
         |
  PkarrClient::publish(&signed_packet)  -- stores at keypair.public_key() on DHT
  PkarrClient::resolve(&public_key)     -- fetches packet at that address
      .resource_records("_pktap._share.<hash>")  -- finds the TXT record
```

**Why this works for PKTap:** ECDH is symmetric — `shared_secret(A_priv, B_pub) == shared_secret(B_priv, A_pub)` — so both peers independently derive the same HKDF bytes → same `Keypair` → same DHT address. Both can publish (with latest timestamp winning via BEP-44) and resolve from the same address.

### Recommended Module Structure

```
pktap-core/src/
├── dht.rs              # DhtClient struct + PendingPublish queue
├── dht/                # (alternative: split if dht.rs exceeds ~400 lines)
│   ├── client.rs       # DhtClient, PkarrClient wrapper
│   ├── packet.rs       # DNS packet construction helpers
│   └── queue.rs        # PendingPublish, VecDeque, retry logic
├── record.rs           # EXISTING: shared_record_name(), public_profile_name()
├── error.rs            # EXTEND: add DHT error variants
└── ffi.rs              # EXTEND Phase 3: DHT-aware FFI composites
```

Single `dht.rs` is recommended for Phase 2 scope — split only if needed.

### Pattern 1: Building a Signed Packet with Binary TXT Data

Binary ciphertext cannot be stored in a DNS TXT string directly. Use `CharacterString::new(&[u8])` to wrap raw bytes. DNS TXT allows multiple `CharacterString` entries per record; each is limited to 255 bytes.

```rust
// Source: pkarr-2.3.0/src/client.rs tests + simple-dns-0.9.3/src/dns/character_string.rs
use pkarr::{dns, Keypair, PkarrClient, SignedPacket};
use pkarr::dns::rdata::{RData, TXT};
use pkarr::dns::{CharacterString, CLASS, Name, Packet, ResourceRecord};

fn build_signed_packet(
    hkdf_seed: &[u8; 32],
    record_name: &str,
    ciphertext: &[u8],
    ttl_secs: u32,
) -> Result<SignedPacket, pkarr::Error> {
    // Derive signing keypair from HKDF shared secret
    let keypair = Keypair::from_secret_key(hkdf_seed);

    // Build DNS packet with binary TXT rdata
    let mut packet = Packet::new_reply(0);
    let mut txt = TXT::new();

    // Split ciphertext into 255-byte CharacterString chunks
    for chunk in ciphertext.chunks(255) {
        // CharacterString::new accepts raw &[u8] — binary-safe
        let cs = CharacterString::new(chunk)
            .map_err(|_| pkarr::Error::DnsError(/* ... */))?;
        txt.add_char_string(cs);
    }

    packet.answers.push(ResourceRecord::new(
        Name::new(record_name).map_err(|_| pkarr::Error::DnsError(/* ... */))?,
        CLASS::IN,
        ttl_secs,
        RData::TXT(txt),
    ));

    SignedPacket::from_packet(&keypair, &packet)
}
```

**Note on record name:** pkarr normalizes names by appending the signer's z-base-32 public key as TLD. Pass `"_pktap._share.<hash>"` (without the TLD) — pkarr appends it automatically. When reading back, call `resource_records("_pktap._share.<hash>")` which also normalizes the lookup.

### Pattern 2: Publish/Resolve with Local Testnet (Integration Tests)

```rust
// Source: pkarr-2.3.0/src/client.rs::tests::publish_resolve (VERIFIED from source)
use mainline::{dht::DhtSettings, Testnet};  // mainline re-exported via pkarr
use pkarr::{Keypair, PkarrClient, Settings, SignedPacket};

#[test]
fn test_publish_resolve_round_trip() {
    let testnet = Testnet::new(10);  // 10 local DHT nodes, no real network

    let client_a = PkarrClient::builder()
        .dht_settings(DhtSettings {
            bootstrap: Some(testnet.bootstrap.clone()),
            request_timeout: None,
            server: None,
            port: None,
        })
        .build()
        .unwrap();

    let client_b = PkarrClient::builder()
        .dht_settings(DhtSettings {
            bootstrap: Some(testnet.bootstrap),
            request_timeout: None,
            server: None,
            port: None,
        })
        .build()
        .unwrap();

    // Build and publish
    let signed_packet = /* ... */;
    let keypair = /* ... */;
    client_a.publish(&signed_packet).expect("publish");

    // Resolve from different client
    let resolved = client_b.resolve(&keypair.public_key())
        .expect("resolve ok")
        .expect("packet present");

    assert_eq!(resolved.as_bytes(), signed_packet.as_bytes());
}
```

### Pattern 3: DhtClient Offline Queue Design

```rust
// Source: design pattern — [ASSUMED] based on D-03, D-04, D-05 decisions
use std::collections::VecDeque;
use std::time::{Duration, Instant};

struct PendingPublish {
    signed_packet: SignedPacket,
    next_attempt: Instant,
    attempt_count: u32,
}

struct DhtClient {
    inner: PkarrClient,
    queue: VecDeque<PendingPublish>,
}

impl DhtClient {
    fn publish(&mut self, signed_packet: SignedPacket) -> Result<(), PktapError> {
        // Try immediately; on network error, enqueue
        match self.inner.publish(&signed_packet) {
            Ok(()) => Ok(()),
            Err(pkarr::Error::NotMostRecent) => Err(PktapError::DhtOutdatedRecord),
            Err(_) => {
                // Network error — enqueue for retry
                self.queue.push_back(PendingPublish {
                    signed_packet,
                    next_attempt: Instant::now() + Duration::from_secs(1),
                    attempt_count: 0,
                });
                Ok(())  // Or: Err(PktapError::DhtPublishQueued) to signal offline
            }
        }
    }

    /// Drain the offline queue. Called by Phase 6 WorkManager on connectivity restore.
    fn flush_queue(&mut self) {
        let now = Instant::now();
        let mut next = VecDeque::new();
        while let Some(item) = self.queue.pop_front() {
            if item.next_attempt > now {
                next.push_back(item);
                continue;
            }
            match self.inner.publish(&item.signed_packet) {
                Ok(()) => {}  // succeeded, discard
                Err(_) => {
                    let delay = Duration::from_secs(
                        (1u64 << item.attempt_count).min(60)
                    );
                    next.push_back(PendingPublish {
                        next_attempt: now + delay,
                        attempt_count: item.attempt_count + 1,
                        ..item
                    });
                }
            }
        }
        self.queue = next;
    }
}
```

### Pattern 4: Resolving and Extracting TXT Binary Data

```rust
// Source: pkarr-2.3.0 + simple-dns-0.9.3 API [VERIFIED: source inspection]
fn resolve_ciphertext(
    client: &PkarrClient,
    signer_pubkey: &pkarr::PublicKey,
    record_name: &str,
) -> Result<Option<Vec<u8>>, PktapError> {
    let Some(packet) = client.resolve(signer_pubkey)
        .map_err(|_| PktapError::DhtResolveFailed)?
    else {
        return Ok(None);
    };

    // resource_records() normalizes the lookup name to the packet's origin
    let Some(rr) = packet.resource_records(record_name).next() else {
        return Ok(None);
    };

    // Extract binary bytes from TXT CharacterString chunks
    if let pkarr::dns::rdata::RData::TXT(txt) = &rr.rdata {
        let mut bytes = Vec::new();
        for cs in &txt.strings {  // TXT.strings field is Vec<CharacterString>
            bytes.extend_from_slice(&cs.data);
        }
        Ok(Some(bytes))
    } else {
        Err(PktapError::DhtResolveFailed)
    }
}
```

**Note:** `TXT.strings` is a `pub(crate)` field in simple-dns 0.9.3. The public accessor is through iteration. Verify access pattern compiles — may need `use simple_dns::rdata::TXT` for the struct's iterator.

### Anti-Patterns to Avoid

- **Creating a Tokio runtime for sync PkarrClient:** The default `PkarrClient` has no tokio dependency. Do not add `tokio` to `pktap-core` for Phase 2. The `async` pkarr feature adds `PkarrClientAsync` which wraps with tokio — Phase 2 does not need it.
- **Controlling sequence numbers manually:** `SignedPacket::from_packet()` auto-generates microsecond timestamps. Do not implement a custom seq counter — pkarr handles BEP-44 monotonicity. The `Error::NotMostRecent` variant signals a stale publish.
- **Publishing at an arbitrary DHT key:** Pkarr always publishes at the signing keypair's Ed25519 public key. There is no way to publish at a custom address. The `_pktap._share.<hash>` is a DNS record name *within* the packet, not the DHT address.
- **Storing >255 bytes in a single CharacterString:** DNS TXT CharacterString max is 255 bytes (`MAX_CHARACTER_STRING_LENGTH = 255` in simple-dns). Split ciphertext into 255-byte chunks and call `add_char_string()` for each.
- **Using hex encoding for `public_profile_name` DNS label:** Phase 1's `public_profile_name()` comment notes switching to z-base-32 encoding for pkarr in Phase 2. Pkarr normalizes record names using the signer's z-base-32 public key as TLD. The hex encoding in Phase 1 was a placeholder — Phase 2 should update to z-base-32 for the profile record name subdomain, OR keep as-is since the subdomain is just a lookup key within the packet.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| BEP-44 signing + encoding | Custom DHT signing | `SignedPacket::from_packet()` | BEP-44 bencoding of `seq` + `v` is complex; pkarr handles all of it correctly |
| DHT node bootstrapping | Custom bootstrap logic | `Testnet::new(N)` for tests, default settings for prod | Real bootstrap is unreliable; testnet is reproducible and fast |
| DNS packet wire format | Custom DNS serializer | `simple_dns` via `pkarr::dns` re-export | DNS wire format has edge cases in compression and label encoding |
| Microsecond timestamp | Manual `SystemTime` read | pkarr does it inside `from_packet()` | Monotonicity via `ntimestamp` pattern is built in |
| z-base-32 encoding | Custom z32 encoder | `keypair.public_key().to_z32()` / `z32::encode()` | pkarr re-exports `z32` crate; already available |

**Key insight:** pkarr 2.3.0 is a complete, tested implementation of the full BEP-44 publish/resolve pipeline. The `DhtClient` in Phase 2 is a thin wrapper providing the PKTap-specific record construction (using Phase 1's `shared_record_name()` and `public_profile_name()`), the offline queue, and TTL tracking. Avoid reimplementing anything inside the pkarr/mainline stack.

---

## Common Pitfalls

### Pitfall 1: "Why does resolve return nothing?"
**What goes wrong:** `PkarrClient::resolve()` returns `Ok(None)` for a record that was just published. The testnet nodes haven't propagated yet.
**Why it happens:** DHT propagation is async internally. With `Testnet::new(10)`, the network needs a moment to propagate after `publish()`.
**How to avoid:** In integration tests, use the same client instance that published (it caches locally), OR add a brief `std::thread::sleep(Duration::from_millis(500))` between publish and resolve when using separate clients. The pkarr internal tests use a small sleep for this pattern.
**Warning signs:** `Ok(None)` immediately after `Ok(())` from publish on a separate client.

### Pitfall 2: CharacterString 255-byte limit
**What goes wrong:** `CharacterString::new(large_slice)` returns `Err(SimpleDnsError::InvalidCharacterString)` for payloads > 255 bytes.
**Why it happens:** DNS TXT CharacterStrings have a single-byte length prefix, limiting each to 255 bytes. PKTap ciphertexts at ~800 bytes exceed this.
**How to avoid:** Always chunk via `ciphertext.chunks(255)` and call `add_char_string()` for each chunk. The total TXT rdata can hold multiple CharacterStrings.
**Warning signs:** `Error::DnsError` from `SignedPacket::from_packet()` when passing large ciphertexts as a single string.

### Pitfall 3: pkarr::Error::PacketTooLarge at publish
**What goes wrong:** `from_packet()` returns `Error::PacketTooLarge` even though Phase 1 capped plaintext at 750 bytes.
**Why it happens:** The DNS packet overhead (header + name labels + TXT length prefixes) adds ~140-200 bytes. A 750-byte plaintext → 791-byte ciphertext → ~898-949 byte DNS packet fits comfortably. However, if TTL-related metadata or additional records are added carelessly, the budget shrinks.
**How to avoid:** Pre-validate ciphertext length before building the packet. Maximum safe ciphertext is ~800 bytes (with compressed name encoding: `1000 - 12(header) - 81(name compressed) - 10(RR overhead) - 4(TXT len bytes) = 893 bytes`). Phase 1's 750-byte plaintext limit produces 791-byte ciphertext — safe margin.
**Warning signs:** `Error::PacketTooLarge(N)` where N > 1000.

### Pitfall 4: DhtSettings struct literal requires all fields
**What goes wrong:** Rust gives `missing field` errors when constructing `DhtSettings { bootstrap: Some(...) }`.
**Why it happens:** `DhtSettings` has 4 fields (`bootstrap`, `server`, `port`, `request_timeout`) and does not derive `Default` in a way that allows partial struct update syntax easily in the stable API.
**How to avoid:** Use the struct literal with all fields: `DhtSettings { bootstrap: Some(testnet.bootstrap), server: None, port: None, request_timeout: None }`. OR use `mainline::dht::DhtSettings::default()` then set the `bootstrap` field.
**Warning signs:** Compile error "missing field `server` in initializer of `DhtSettings`".

### Pitfall 5: Record name normalization surprise
**What goes wrong:** `resource_records("_pktap._share.<hash>")` returns nothing even though publish succeeded.
**Why it happens:** pkarr normalizes all record names by appending the signer's z-base-32 public key as a trailing TLD label. The actual stored name is `_pktap._share.<hash>.<z32pubkey>`. The `resource_records()` method also normalizes the lookup, so consistent use of the method handles this transparently — BUT if you access `packet.packet().answers` directly, names will be fully qualified.
**How to avoid:** Always use `signed_packet.resource_records(name)` rather than iterating `packet.packet().answers` directly. The normalization is symmetric.
**Warning signs:** Correct packet bytes but no records found from direct answers iteration.

### Pitfall 6: `Error::NotMostRecent` on a legitimate re-publish
**What goes wrong:** A re-publish of a new record is rejected with `Error::NotMostRecent`.
**Why it happens:** pkarr caches signed packets internally. If the client still holds an older cached version AND the DHT has a newer version from a peer, pkarr rejects the publish to prevent clock-skew exploits.
**How to avoid:** `Error::NotMostRecent` is a signal that a newer packet exists — it should be treated as "resolve first, decide whether to re-publish" rather than a hard error. The `republish(record_key)` method exposed by DhtClient (D-05) should handle this gracefully.
**Warning signs:** `Err(NotMostRecent)` on what should be a fresh publish.

---

## Code Examples

### Complete publish/resolve integration test pattern (verified from pkarr source)

```rust
// Source: pkarr-2.3.0/src/client.rs tests [VERIFIED: source inspection]
#[cfg(test)]
mod dht_integration_tests {
    use mainline::{dht::DhtSettings, Testnet};
    use pkarr::{Keypair, PkarrClient, SignedPacket};
    use pkarr::dns::{self, CharacterString};
    use pkarr::dns::rdata::{RData, TXT};

    fn make_pkarr_client(bootstrap: Vec<String>) -> PkarrClient {
        PkarrClient::builder()
            .dht_settings(DhtSettings {
                bootstrap: Some(bootstrap),
                request_timeout: None,
                server: None,
                port: None,
            })
            .build()
            .unwrap()
    }

    #[test]
    fn dht_publish_resolve_encrypted_record() {
        let testnet = Testnet::new(10);

        let client_a = make_pkarr_client(testnet.bootstrap.clone());
        let client_b = make_pkarr_client(testnet.bootstrap);

        // Build keypair from HKDF-derived bytes (in production: from ECDH shared secret)
        let hkdf_seed = [0xABu8; 32];  // in prod: from ecdh_derive_key()
        let keypair = Keypair::from_secret_key(&hkdf_seed);

        // Build DNS packet with binary TXT
        let ciphertext = vec![0x42u8; 400];  // in prod: from ecdh_and_encrypt()
        let mut packet = dns::Packet::new_reply(0);
        let mut txt = TXT::new();
        for chunk in ciphertext.chunks(255) {
            txt.add_char_string(CharacterString::new(chunk).unwrap());
        }

        let record_name = "_pktap._share.deadbeef"; // in prod: shared_record_name(&a_pk, &b_pk)
        packet.answers.push(dns::ResourceRecord::new(
            dns::Name::new(record_name).unwrap(),
            dns::CLASS::IN,
            86400,  // 24h TTL
            RData::TXT(txt),
        ));

        let signed = SignedPacket::from_packet(&keypair, &packet).unwrap();

        client_a.publish(&signed).unwrap();

        let resolved = client_b
            .resolve(&keypair.public_key())
            .unwrap()
            .unwrap();

        // Verify record is present
        assert!(resolved.resource_records(record_name).next().is_some());
    }
}
```

### Extending PktapError with DHT variants

```rust
// Source: design based on Phase 1's error.rs [ASSUMED: variant names at discretion]
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PktapError {
    // --- Phase 1 variants (existing) ---
    #[error("Invalid key bytes")]
    InvalidKey,
    #[error("Record invalid or decryption failed")]
    RecordInvalid,
    #[error("Record payload too large")]
    RecordTooLarge,
    #[error("Serialization failed")]
    SerializationFailed,

    // --- Phase 2 additions ---
    #[error("DHT publish failed")]
    DhtPublishFailed,
    #[error("DHT resolve failed")]
    DhtResolveFailed,
    #[error("DHT record is outdated (a newer version exists)")]
    DhtOutdatedRecord,
    #[error("DHT publish queued (offline)")]
    DhtPublishQueued,
}
```

---

## Runtime State Inventory

This is a greenfield phase — no rename, refactor, or migration operations. Skipped per instructions.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| pkarr 1.x UDL-based API | pkarr 2.x+: same package, renamed types (`Client` → `PkarrClient`) | ~2024 | 2.x has cleaner builder API; decision D-01 locks to 2.3.x |
| Manual BEP-44 bencoded puts | `SignedPacket::from_packet()` handles all BEP-44 encoding | pkarr 1.0+ | Never hand-roll BEP-44 bencoding |
| String-only TXT records | `CharacterString::new(&[u8])` accepts binary | simple-dns 0.9.x | Binary ciphertext storage is fully supported |

**Deprecated/outdated:**
- `pkarr::Client` (renamed to `pkarr::PkarrClient` in 2.x) — use `PkarrClient`
- `pkarr` 5.x+ API: completely different (uses `pkarr_client` crate, different type names) — do NOT reference 5.x examples for Phase 2

---

## Byte Budget Analysis

The ~858 byte figure from DHT-07 is a conservative estimate. Verified analysis:

| Component | Bytes |
|-----------|-------|
| DNS packet header | 12 |
| Record name `_pktap._share.<64hex>` with z32 TLD (compressed) | ~81 |
| ResourceRecord type+class+ttl+rdlength | 10 |
| TXT CharacterString length bytes (4 chunks × 1 byte each) | 4 |
| Ciphertext payload (750-byte plaintext → 791 bytes) | 791 |
| **Total (750-byte plaintext)** | **898** |
| Hard limit enforced by pkarr | 1000 |
| Safety margin | 102 bytes |

Phase 1's 750-byte plaintext cap is appropriate. The DhtClient does NOT need to re-validate the plaintext — it pre-validates the **ciphertext blob** size: ciphertext ≤ 842 bytes (conservative) to ensure the DNS packet stays under 1000 bytes with uncompressed name encoding.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[cfg(test)]` modules |
| Config file | none (cargo test) |
| Quick run command | `cargo test --package pktap-core` |
| Full suite command | `cargo test --package pktap-core` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DHT-01 | Publish encrypted record at deterministic DHT address | integration | `cargo test --package pktap-core dht::tests::test_publish_shared_record` | ❌ Wave 0 |
| DHT-02 | Resolve encrypted record by same deterministic address | integration | `cargo test --package pktap-core dht::tests::test_resolve_shared_record` | ❌ Wave 0 |
| DHT-03 | Publish public mode TXT record | integration | `cargo test --package pktap-core dht::tests::test_publish_public_profile` | ❌ Wave 0 |
| DHT-04 | Resolve public mode record | integration | `cargo test --package pktap-core dht::tests::test_resolve_public_profile` | ❌ Wave 0 |
| DHT-05 | Records carry correct TTL values | unit | `cargo test --package pktap-core dht::tests::test_ttl_values` | ❌ Wave 0 |
| DHT-06 | BEP-44 seq: older publish rejected by pkarr | unit | `cargo test --package pktap-core dht::tests::test_outdated_publish_rejected` | ❌ Wave 0 |
| DHT-07 | Oversized ciphertext rejected before publish | unit | `cargo test --package pktap-core dht::tests::test_ciphertext_size_validation` | ❌ Wave 0 |
| DHT-08 | Offline publish queued, completes on reconnect | integration | `cargo test --package pktap-core dht::tests::test_offline_queue_drain` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --package pktap-core`
- **Per wave merge:** `cargo test --package pktap-core`
- **Phase gate:** All tests green + `cargo test --package pktap-core` before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `pktap-core/src/dht.rs` — new module covering all DHT-01 through DHT-08 tests
- [ ] `pktap-core/src/error.rs` — extend with DHT error variants before writing tests

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | DHT layer is transport; identity established by Ed25519 key in Phase 1 |
| V3 Session Management | no | Stateless DHT requests |
| V4 Access Control | no | Record read access is open by design (encrypted) |
| V5 Input Validation | yes | Ciphertext size validated before publish; DHT response bytes validated by pkarr |
| V6 Cryptography | yes | HKDF-derived key used as Ed25519 seed — never stored; pkarr handles BEP-44 signing |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Replay of old DHT record | Tampering | pkarr BEP-44 seq: `Error::NotMostRecent` rejects stale publishes |
| Record substitution at DHT | Spoofing | pkarr validates Ed25519 signature on every resolved packet |
| Oversized record exhaustion | DoS | Pre-validate ciphertext size in DhtClient before building DNS packet |
| HKDF seed leakage via DHT signing | Information Disclosure | HKDF bytes used as Ed25519 seed — `Keypair` wraps `SigningKey` which implements `ZeroizeOnDrop`; zero after use |
| DHT address linkability | Privacy | Deterministic DHT address from shared ECDH secret is only known to both parties; address is not guessable without both keys |

**Memory safety note:** `pkarr::Keypair` wraps `ed25519_dalek::SigningKey`. The `ed25519-dalek 2.x` `SigningKey` implements `ZeroizeOnDrop`. Phase 2's DhtClient should not retain the keypair past the publish/resolve call — drop it immediately after use to ensure the HKDF-derived key material is zeroed.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All Rust compilation | ✓ | rustc 1.93.0 (2026-01-19) | — |
| cargo | Build + test | ✓ | 1.93.0 | — |
| Internet access (for `cargo add pkarr`) | First-time dep download | ✓ | — | Use offline registry cache |

**Missing dependencies with no fallback:** None.

**Note:** Integration tests using `Testnet::new(N)` bind to `127.0.0.1` ephemeral ports — no external network required, no firewall concerns.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `TXT.strings` field is accessible for reading back binary data (used in resolve extraction pattern) | Code Examples | Low — workaround: use `resource_records()` which returns `&ResourceRecord`, then iterate `rdata` via enum match; the `TXT` struct may not expose `.strings` publicly; use `into_attributes()` or write a helper |
| A2 | Phase 1 D-03 HKDF-derived key is 32 bytes suitable as Ed25519 seed via `Keypair::from_secret_key(&[u8;32])` | Architecture Patterns | Medium — if HKDF output is used differently (e.g., as raw encryption key, not signing key), the DhtClient signing design changes; verify with Phase 1 `ecdh.rs` that HKDF output is 32 bytes |
| A3 | Error variant naming for DHT failures (DhtPublishFailed, etc.) is at Claude's discretion | Standard Stack | None — decisions explicitly defer naming to Claude |
| A4 | `DhtSettings` struct literal syntax requires all 4 fields — no partial struct update available | Common Pitfalls | Low — workaround is `..Default::default()` spread; verify compilation |

**Notes on A1:** The `TXT.strings` field in simple-dns 0.9.3 is declared `pub(crate)` (not `pub`). The iteration pattern shown in the code example may not compile. The safe alternative is to rely on `TXT::attributes()` for string data, OR to serialize the full `ResourceRecord` rdata bytes directly via `WireFormat::write_to()` and parse manually. This needs verification at implementation time.

---

## Open Questions

1. **TXT binary readback path**
   - What we know: `CharacterString::new(&[u8])` accepts binary for writing; simple-dns `TXT.strings` is `pub(crate)` visibility
   - What's unclear: The public API for iterating binary CharacterStrings back on the read side; `attributes()` is string-only
   - Recommendation: At Wave 0, write a quick test using `RData::TXT` with binary data and verify the readback compiles; fall back to raw `WireFormat` byte extraction if needed

2. **HKDF key as Ed25519 signing seed alignment**
   - What we know: D-02 says "HKDF-derived key signs DHT records"; pkarr needs a `Keypair` created from `SecretKey` = `[u8; 32]`; Phase 1's HKDF output is 32 bytes
   - What's unclear: In `ecdh.rs`, the HKDF output is the **encryption key** (passed to XChaCha20 cipher), not a separate signing key; using the same 32 bytes as both the encryption key AND the DHT signing key seed creates key reuse
   - Recommendation: Derive a separate HKDF output for DHT signing with a different `info` parameter (e.g., `"pktap-v1-dht-sign"` vs `"pktap-v1-enc"`). Verify with Phase 1's `ecdh.rs` implementation before writing DhtClient.

3. **record.rs `public_profile_name()` hex vs z-base-32**
   - What we know: Phase 1 comment says "Phase 2 will switch the encoding to z-base-32 when Pkarr integration is added"
   - What's unclear: The record name is a TXT record **subdomain** within the packet, not the DHT address itself; pkarr normalizes the TLD (public key) to z32, but the subdomain can be any valid DNS label
   - Recommendation: Keep hex encoding for the profile record subdomain name for now — it's just a lookup key; update only if there's a protocol-level reason to use z32

---

## Sources

### Primary (HIGH confidence)
- pkarr-2.3.0 source: `~/.cargo/registry/src/.../pkarr-2.3.0/` — `client.rs`, `keys.rs`, `signed_packet.rs`, `examples/publish.rs`
- mainline-2.0.1 source: `~/.cargo/registry/src/.../mainline-2.0.1/src/dht.rs` — `Testnet` struct, `DhtSettings` struct
- simple-dns-0.9.3 source: `~/.cargo/registry/src/.../simple-dns-0.9.3/src/dns/character_string.rs`, `rdata/txt.rs`
- crates.io API: `https://crates.io/api/v1/crates/pkarr/2.3.0/dependencies` — dependency versions verified

### Secondary (MEDIUM confidence)
- docs.rs/pkarr/2.3.0: PkarrClient, SignedPacket, Settings, PkarrClientBuilder method signatures
- docs.rs/mainline/latest: Testnet struct API and builder pattern

### Tertiary (LOW confidence)
- Architecture claim about "DHT address is always the signer's public key" — confirmed via docs.rs/pubky.github.io/pkarr and cross-verified against actual pkarr source (`resolve()` takes `&PublicKey`)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified against crates.io registry; dep tree inspected from source
- Architecture: HIGH — DHT address model verified directly from pkarr source (`client.rs` publish/resolve tests)
- Pitfalls: HIGH — CharacterString limit verified from source; packet size analysis calculated from source constants
- TXT binary readback: MEDIUM — write path verified; read path requires Wave 0 compilation test (A1 assumption)

**Research date:** 2026-04-05
**Valid until:** 2026-07-05 (pkarr 2.3.x is stable; unlikely to change in 90 days)
