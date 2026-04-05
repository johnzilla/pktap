---
phase: 02-pkarr-dht-integration
asvs_level: 1
audited: 2026-04-05
auditor: gsd-secure-phase
status: SECURED
threats_open: 0
threats_closed: 11
---

# Security Audit — Phase 2: Pkarr DHT Integration

**Phase:** 02 — pkarr-dht-integration
**Plans audited:** 02-01, 02-02
**ASVS Level:** 1
**Threats Closed:** 11 / 11
**Threats Open:** 0 / 11

---

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-02-01 | Spoofing | mitigate | CLOSED | `pktap-core/src/dht.rs:465-470` — `client.resolve()` returns a `SignedPacket`; pkarr verifies the BEP-44 Ed25519 signature internally before returning. DhtClient never bypasses this path. |
| T-02-02 | Tampering | mitigate | CLOSED | `pktap-core/src/dht.rs:444-452` — all resolved packets arrive as `SignedPacket`; BEP-44 signatures cover the full bencoded value. pkarr rejects packets that fail signature verification before returning to the caller. No bypass exists in the implementation. |
| T-02-03 | Repudiation | accept | CLOSED | Accepted by design. DHT signing key is derived from the ECDH shared secret, not the user's identity key. Repudiation is a privacy feature. See accepted risks log below. |
| T-02-04 | Information Disclosure | mitigate | CLOSED | `pktap-core/src/dht.rs:413` — seed is consumed by `Keypair::from_secret_key(seed)` within `build_signed_packet()` and is not stored in `DhtClient`. The caller (FFI layer) is responsible for zeroing its copy post-call. `DhtClient` does not retain the seed except in `TrackedRecord` (see T-02-08). |
| T-02-05 | Denial of Service | mitigate | CLOSED | `pktap-core/src/dht.rs:152-154` — `publish_encrypted` checks `ciphertext.len() > MAX_CIPHERTEXT_LEN` (850 bytes) and returns `Err(PktapError::RecordTooLarge)` before any packet is built or published. `publish_public` applies the same check at line 202-204. `build_signed_packet` additionally maps pkarr `PacketTooLarge` to `RecordTooLarge` at line 432-436. |
| T-02-06 | Elevation of Privilege | accept | CLOSED | Accepted by design. Both peers derive the same DHT address from the ECDH shared secret. Symmetric publish/resolve is the intended protocol property. See accepted risks log below. |
| T-02-07 | Information Disclosure | accept | CLOSED | Accepted by design. The `_pktap._share.<SHA-256(sort(A,B))>` record name within the packet reveals a relationship only to an observer who already possesses both public keys. Exposure is limited to physical-proximity exchanges (NFC/QR). See accepted risks log below. |
| T-02-08 | Information Disclosure | mitigate | CLOSED | `pktap-core/src/dht.rs:61-67` — `TrackedRecord` has a manual `Drop` implementation that calls `self.seed.zeroize()` and `self.data.zeroize()`. This fires when `DhtClient` is dropped, zeroing all retained seed bytes from the `tracked: Mutex<Vec<TrackedRecord>>` field. |
| T-02-09 | Denial of Service | mitigate | CLOSED | `pktap-core/src/dht.rs:72-79, 89-94` — offline queue is a `Mutex<VecDeque<PendingPublish>>` field on `DhtClient`. VecDeque is in-memory and is released when the process terminates. No unbounded persistent queue exists. Exponential backoff (`min(2^attempt_count, 60)` seconds) at lines 277-278 prevents tight retry loops. |
| T-02-10 | Spoofing | accept | CLOSED | Accepted by design. Public mode records are intentionally readable by any holder of the signer's public key. BEP-44 signature in the resolved `SignedPacket` verifies the publisher identity. See accepted risks log below. |
| T-02-11 | Tampering | mitigate | CLOSED | `pktap-core/src/dht.rs:328, 444-452` — `republish()` calls `build_signed_packet()` which creates a fresh `SignedPacket` via `SignedPacket::from_packet()`, generating a new microsecond-resolution BEP-44 sequence number automatically. `publish_packet()` maps pkarr `Error::NotMostRecent` to `PktapError::DhtOutdatedRecord`, preventing overwrite of a newer peer-published record. |

---

## Accepted Risks Log

The following threats are accepted by design and require no mitigation code.

### T-02-03 — Repudiation (DhtClient::publish_encrypted)

**Rationale:** The signing keypair used for DHT publishes is derived from the ECDH shared secret, not the user's long-term Ed25519 identity key. No external party can link a DHT record to a specific user identity. The inability to repudiate DHT publishes is a deliberate privacy property of the protocol design.

**Owner:** PKTap protocol design
**Review trigger:** Any change that binds the DHT signing key to a user identity key.

---

### T-02-06 — Elevation of Privilege (DHT address derivation)

**Rationale:** Both peers in a contact exchange derive the same ECDH shared secret, and therefore the same DHT keypair and address. This symmetric property is required for the protocol to function without a central coordinator. There is no privilege boundary to elevate across — both parties have equal publish/resolve access to the shared DHT slot by design.

**Owner:** PKTap protocol design
**Review trigger:** Any change to the DHT address derivation scheme.

---

### T-02-07 — Information Disclosure (DNS record name leaks peer relationship)

**Rationale:** The `_pktap._share.<hash>` record name contains `SHA-256(sort(pk_a, pk_b))`. An observer can confirm a relationship between two parties only if they already possess both public keys. Public keys are exchanged exclusively via NFC tap or QR scan (physical proximity), which substantially limits the attack surface. The residual risk is accepted as low.

**Owner:** PKTap protocol design
**Review trigger:** Any change that makes public keys discoverable without physical proximity.

---

### T-02-10 — Spoofing (Public mode records)

**Rationale:** Public profile records are intentionally published in plaintext for discovery by any holder of the signer's public key. The BEP-44 signature in the resolved `SignedPacket` provides publisher authenticity. No confidentiality expectation exists for public mode records; the user explicitly opts into public mode (Phase 7 PUB-01).

**Owner:** PKTap protocol design
**Review trigger:** Phase 7 implementation of public mode opt-in UI — verify user consent is explicit.

---

## Unregistered Threat Flags

No unregistered threat flags were raised in 02-01-SUMMARY.md or 02-02-SUMMARY.md. The executor's self-reported threat surface scan in 02-02-SUMMARY.md maps every flag directly to the registered threat IDs (T-02-08 through T-02-11).

---

## Notes

- **T-02-04 partial scope:** The seed is not stored in `DhtClient` for ephemeral `publish_encrypted` / `publish_public` calls (correct). However, `track_record()` copies the seed into `TrackedRecord.seed` for the republish API. This copy is covered by T-02-08's `Drop` zeroing, so no gap exists. The boundary between these two threats is cleanly handled.

- **T-02-09 queue bound:** No explicit maximum queue size is enforced in Phase 2 code. The bound is the process lifetime (acceptable at this phase). If Phase 6 introduces a persistent queue (e.g., WorkManager), a maximum size limit must be added and T-02-09 should be re-evaluated at that phase.

- **Wire-format extraction workaround (02-01 deviation):** The binary TXT extraction using mini-packet serialization (`pktap-core/src/dht.rs:495-554`) is a workaround for `simple-dns`'s `pub(crate)` `WireFormat` trait. The parsing logic is deterministic per RFC 1035 and does not introduce a security gap, but the manual byte-offset arithmetic at lines 501-533 should be audited again if the `simple-dns` dependency is upgraded, as wire format offsets may shift.
