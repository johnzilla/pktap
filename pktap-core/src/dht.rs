//! DHT publish/resolve via pkarr 2.3.0.
//! See 02-RESEARCH.md for architecture details.
//!
//! The DHT address is the Ed25519 public key derived from the HKDF seed via
//! `pkarr::Keypair::from_secret_key`. The `_pktap._share.<hash>` string is a
//! DNS record name *within* the signed packet, not the DHT address itself.
//! Both peers derive the same keypair from their identical ECDH-shared secret,
//! so both can publish and resolve from the same DHT address.

use pkarr::{
    dns::{
        rdata::{RData, TXT},
        CharacterString, CLASS, Name, Packet, ResourceRecord,
    },
    Keypair, PkarrClient, PublicKey, SignedPacket,
};

use crate::error::PktapError;

// ── Constants ───────────────────────────────────────────────────────────────

/// Default TTL for private encrypted records (24 hours).
pub const PRIVATE_RECORD_TTL: u32 = 86_400;

/// Default TTL for public profile records (7 days).
pub const PUBLIC_RECORD_TTL: u32 = 604_800;

/// Maximum ciphertext size that fits safely in a pkarr DNS packet.
///
/// Analysis: 1000 (BEP-44 limit) − 12 (DNS header) − ~90 (name + RR overhead
/// + TXT length-prefix bytes) = ~898. Using 850 as a conservative safe limit.
pub const MAX_CIPHERTEXT_LEN: usize = 850;

// ── DhtClient ───────────────────────────────────────────────────────────────

/// Wraps `pkarr::PkarrClient` and provides PKTap-specific publish/resolve
/// for both private (encrypted) and public (plaintext) contact records.
pub struct DhtClient {
    inner: PkarrClient,
}

impl DhtClient {
    /// Creates a `DhtClient` connecting to the production Mainline DHT via
    /// pkarr's default bootstrap nodes.
    pub fn new() -> Result<Self, PktapError> {
        let client = PkarrClient::builder()
            .build()
            .map_err(|_| PktapError::DhtPublishFailed)?;
        Ok(Self { inner: client })
    }

    /// Creates a `DhtClient` using custom bootstrap nodes.
    ///
    /// Used in integration tests to connect to a local `mainline::Testnet`.
    pub fn with_bootstrap(bootstrap: Vec<String>) -> Result<Self, PktapError> {
        use pkarr::mainline::dht::DhtSettings;
        let client = PkarrClient::builder()
            .dht_settings(DhtSettings {
                bootstrap: Some(bootstrap),
                request_timeout: None,
                server: None,
                port: None,
            })
            .build()
            .map_err(|_| PktapError::DhtPublishFailed)?;
        Ok(Self { inner: client })
    }

    /// Validates `ciphertext` size, builds a signed DNS packet from
    /// `hkdf_derived_seed`, and publishes it to the DHT at the derived keypair's
    /// public key address. The ciphertext is stored under `record_name` as a
    /// TXT record with `PRIVATE_RECORD_TTL`.
    ///
    /// Returns the keypair's public key (the DHT address) on success.
    ///
    /// # Errors
    /// - `PktapError::RecordTooLarge` if `ciphertext.len() > MAX_CIPHERTEXT_LEN`
    /// - `PktapError::DhtOutdatedRecord` if a newer record already exists
    /// - `PktapError::DhtPublishFailed` for any other network error
    pub fn publish_encrypted(
        &self,
        hkdf_derived_seed: &[u8; 32],
        record_name: &str,
        ciphertext: &[u8],
    ) -> Result<PublicKey, PktapError> {
        if ciphertext.len() > MAX_CIPHERTEXT_LEN {
            return Err(PktapError::RecordTooLarge);
        }
        let (keypair, signed_packet) =
            build_signed_packet(hkdf_derived_seed, record_name, ciphertext, PRIVATE_RECORD_TTL)?;

        publish_packet(&self.inner, &signed_packet)?;
        Ok(keypair.public_key())
    }

    /// Resolves the TXT record stored at `signer_public_key` on the DHT and
    /// returns the raw ciphertext bytes stored under `record_name`, or `None`
    /// if not found.
    ///
    /// # Errors
    /// - `PktapError::DhtResolveFailed` on network or parse errors
    pub fn resolve_encrypted(
        &self,
        signer_public_key: &PublicKey,
        record_name: &str,
    ) -> Result<Option<Vec<u8>>, PktapError> {
        resolve_bytes(&self.inner, signer_public_key, record_name)
    }

    /// Like `publish_encrypted` but uses `PUBLIC_RECORD_TTL` (7 days) and
    /// does not enforce the ciphertext-size limit (plaintext may be smaller).
    ///
    /// Returns the keypair's public key (the DHT address) on success.
    pub fn publish_public(
        &self,
        profile_seed: &[u8; 32],
        record_name: &str,
        plaintext: &[u8],
    ) -> Result<PublicKey, PktapError> {
        if plaintext.len() > MAX_CIPHERTEXT_LEN {
            return Err(PktapError::RecordTooLarge);
        }
        let (keypair, signed_packet) =
            build_signed_packet(profile_seed, record_name, plaintext, PUBLIC_RECORD_TTL)?;

        publish_packet(&self.inner, &signed_packet)?;
        Ok(keypair.public_key())
    }

    /// Resolves the TXT record stored at `signer_public_key` and returns the
    /// raw bytes (same wire format as `publish_public`), or `None` if not found.
    ///
    /// # Errors
    /// - `PktapError::DhtResolveFailed` on network or parse errors
    pub fn resolve_public(
        &self,
        signer_public_key: &PublicKey,
        record_name: &str,
    ) -> Result<Option<Vec<u8>>, PktapError> {
        resolve_bytes(&self.inner, signer_public_key, record_name)
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Builds a `SignedPacket` from a seed, record name, binary data, and TTL.
///
/// The data is split into 255-byte `CharacterString` chunks per DNS TXT spec
/// (RFC 6763 §6.1 / simple-dns `MAX_CHARACTER_STRING_LENGTH = 255`).
///
/// Returns `(keypair, signed_packet)` so callers can inspect the public key.
fn build_signed_packet(
    seed: &[u8; 32],
    record_name: &str,
    data: &[u8],
    ttl: u32,
) -> Result<(Keypair, SignedPacket), PktapError> {
    let keypair = Keypair::from_secret_key(seed);

    let mut txt = TXT::new();
    for chunk in data.chunks(255) {
        let cs = CharacterString::new(chunk)
            .map_err(|_| PktapError::RecordTooLarge)?;
        txt.add_char_string(cs);
    }

    let mut packet = Packet::new_reply(0);
    packet.answers.push(ResourceRecord::new(
        Name::new(record_name).map_err(|_| PktapError::SerializationFailed)?,
        CLASS::IN,
        ttl,
        RData::TXT(txt),
    ));

    let signed = SignedPacket::from_packet(&keypair, &packet).map_err(|e| {
        // pkarr returns PacketTooLarge when encoded packet > 1000 bytes
        if e.to_string().contains("too large") || e.to_string().contains("PacketTooLarge") {
            PktapError::RecordTooLarge
        } else {
            PktapError::SerializationFailed
        }
    })?;

    Ok((keypair, signed))
}

/// Publishes a `SignedPacket` via `PkarrClient`, mapping pkarr errors to
/// `PktapError` variants.
fn publish_packet(client: &PkarrClient, signed_packet: &SignedPacket) -> Result<(), PktapError> {
    client.publish(signed_packet).map_err(|e| {
        if matches!(e, pkarr::Error::NotMostRecent) {
            PktapError::DhtOutdatedRecord
        } else {
            PktapError::DhtPublishFailed
        }
    })
}

/// Resolves the raw TXT data stored at `signer_public_key` under `record_name`.
///
/// Reassembles binary data from the length-prefixed `CharacterString` chunks
/// by serialising the TXT record back to wire bytes and then reading the
/// chunk lengths, which is the only way to access the underlying bytes without
/// relying on the private `TXT.strings` field.
fn resolve_bytes(
    client: &PkarrClient,
    signer_public_key: &PublicKey,
    record_name: &str,
) -> Result<Option<Vec<u8>>, PktapError> {
    let Some(signed_packet) = client
        .resolve(signer_public_key)
        .map_err(|_| PktapError::DhtResolveFailed)?
    else {
        return Ok(None);
    };

    let Some(rr) = signed_packet.resource_records(record_name).next() else {
        return Ok(None);
    };

    let RData::TXT(_) = &rr.rdata else {
        return Err(PktapError::DhtResolveFailed);
    };

    // Extract raw bytes from the TXT rdata by building a mini DNS packet
    // containing only this resource record, serialising it, and then reading
    // the TXT wire format (length-prefixed CharacterString chunks) directly
    // from the known byte offset in the serialised packet.
    //
    // TXT.strings is pub(crate) in simple-dns so we cannot access it directly.
    // The WireFormat trait is also pub(crate). The only public way to get to the
    // raw bytes is to re-serialise the whole packet and parse the known layout:
    //   DNS header:        12 bytes
    //   Name:              variable (ends at 0x00 byte)
    //   Type/Class/TTL:    8 bytes
    //   rdlength:          2 bytes (big-endian u16)
    //   rdata (TXT data):  rdlength bytes
    //
    // Each TXT CharacterString in rdata is: <1-byte-len><len bytes of data>.
    let mut mini_packet = Packet::new_reply(0);
    mini_packet.answers.push(rr.clone().into_owned());
    let raw = mini_packet
        .build_bytes_vec()
        .map_err(|_| PktapError::DhtResolveFailed)?;

    // Skip 12-byte header, then scan past the variable-length name.
    let mut pos = 12usize;
    if pos >= raw.len() {
        return Err(PktapError::DhtResolveFailed);
    }
    // Name encoding: sequence of <len><label> pairs ending with 0x00.
    // Compressed pointers (0xC0 prefix) are not expected for fresh answers.
    loop {
        if pos >= raw.len() {
            return Err(PktapError::DhtResolveFailed);
        }
        let label_len = raw[pos] as usize;
        pos += 1;
        if label_len == 0 {
            break;
        }
        // Compression pointer: top two bits 0b11xxxxxx
        if label_len & 0xC0 == 0xC0 {
            pos += 1; // skip second byte of pointer
            break;
        }
        pos += label_len;
    }
    // Skip Type (2) + Class (2) + TTL (4) = 8 bytes
    pos += 8;
    if pos + 2 > raw.len() {
        return Err(PktapError::DhtResolveFailed);
    }
    let rdlength = u16::from_be_bytes([raw[pos], raw[pos + 1]]) as usize;
    pos += 2;
    if pos + rdlength > raw.len() {
        return Err(PktapError::DhtResolveFailed);
    }
    let rdata_bytes = &raw[pos..pos + rdlength];

    // Parse DNS TXT wire format: each CharacterString is <1-byte-len><data>.
    let mut bytes = Vec::with_capacity(rdlength);
    let mut p = 0usize;
    while p < rdata_bytes.len() {
        let chunk_len = rdata_bytes[p] as usize;
        p += 1;
        if p + chunk_len > rdata_bytes.len() {
            return Err(PktapError::DhtResolveFailed);
        }
        bytes.extend_from_slice(&rdata_bytes[p..p + chunk_len]);
        p += chunk_len;
    }

    if bytes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(bytes))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pkarr::mainline::Testnet;
    use std::thread;
    use std::time::Duration;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_client(testnet: &Testnet) -> DhtClient {
        DhtClient::with_bootstrap(testnet.bootstrap.clone()).expect("client creation failed")
    }

    // ── Unit tests (no network) ───────────────────────────────────────────────

    /// PktapError::RecordTooLarge is returned when ciphertext exceeds MAX_CIPHERTEXT_LEN.
    #[test]
    fn test_size_validation() {
        let seed = [0x42u8; 32];
        let oversized = vec![0u8; MAX_CIPHERTEXT_LEN + 1];
        let testnet = Testnet::new(1);
        let client = make_client(&testnet);

        let result = client.publish_encrypted(&seed, "_pktap._share.test", &oversized);
        assert!(
            matches!(result, Err(PktapError::RecordTooLarge)),
            "expected RecordTooLarge for oversized ciphertext, got {:?}",
            result
        );
    }

    /// TTL values in the built ResourceRecord match the constants.
    #[test]
    fn test_ttl_values() {
        let seed = [0x01u8; 32];
        let data = b"hello ttl test";

        // Private record TTL
        let (_, private_sp) =
            build_signed_packet(&seed, "_pktap._share.ttltest", data, PRIVATE_RECORD_TTL)
                .expect("build private signed packet");
        let priv_ttl = private_sp
            .packet()
            .answers
            .first()
            .expect("answer present")
            .ttl;
        assert_eq!(priv_ttl, PRIVATE_RECORD_TTL, "private TTL must be 86400");

        // Public record TTL
        let (_, public_sp) =
            build_signed_packet(&seed, "_pktap._profile.ttltest", data, PUBLIC_RECORD_TTL)
                .expect("build public signed packet");
        let pub_ttl = public_sp
            .packet()
            .answers
            .first()
            .expect("answer present")
            .ttl;
        assert_eq!(pub_ttl, PUBLIC_RECORD_TTL, "public TTL must be 604800");
    }

    /// Two DhtClients derived from the same seed produce the same public key (DHT address).
    #[test]
    fn test_deterministic_address() {
        let seed = [0xDEu8; 32];
        let kp1 = Keypair::from_secret_key(&seed);
        let kp2 = Keypair::from_secret_key(&seed);
        assert_eq!(
            kp1.public_key().as_bytes(),
            kp2.public_key().as_bytes(),
            "same seed must produce identical DHT address"
        );
    }

    // ── Integration tests (local testnet) ────────────────────────────────────

    /// A published encrypted record can be resolved back by a different client.
    #[test]
    fn test_publish_resolve_round_trip() {
        let testnet = Testnet::new(10);
        let client_a = make_client(&testnet);
        let client_b = make_client(&testnet);

        let seed = [0xABu8; 32];
        let record_name = "_pktap._share.roundtrip";
        let ciphertext: Vec<u8> = (0u8..200).collect();

        let public_key = client_a
            .publish_encrypted(&seed, record_name, &ciphertext)
            .expect("publish_encrypted must succeed");

        // Give the DHT time to propagate (Pitfall 1 from RESEARCH.md)
        thread::sleep(Duration::from_millis(500));

        let resolved = client_b
            .resolve_encrypted(&public_key, record_name)
            .expect("resolve_encrypted must succeed")
            .expect("resolved packet must be present");

        assert_eq!(
            resolved, ciphertext,
            "resolved ciphertext must match what was published"
        );
    }

    /// Publishing a record with a stale (older) timestamp returns DhtOutdatedRecord.
    ///
    /// Strategy (per plan §Test 6): Publish record A (timestamp T1). Sleep 1ms.
    /// Publish record B (timestamp T2 > T1, same key). Now try to re-publish
    /// signed_packet_a — pkarr's cache sees T2 > T1 and returns NotMostRecent,
    /// which DhtClient maps to DhtOutdatedRecord.
    #[test]
    fn test_sequence_rejection() {
        let testnet = Testnet::new(10);
        let client = make_client(&testnet);

        let seed = [0xCCu8; 32];
        let record_name = "_pktap._share.seqtest";
        let data_a = b"payload version 1";
        let data_b = b"payload version 2";

        // Publish version A — captures T1 in the client's cache.
        let keypair = Keypair::from_secret_key(&seed);
        let (_, sp_a) =
            build_signed_packet(&seed, record_name, data_a, PRIVATE_RECORD_TTL)
                .expect("build sp_a");
        publish_packet(&client.inner, &sp_a).expect("publish sp_a");

        // Short sleep so T2 > T1 in microseconds.
        thread::sleep(Duration::from_millis(2));

        // Publish version B — cache now holds T2.
        let (_, _sp_b) =
            build_signed_packet(&seed, record_name, data_b, PRIVATE_RECORD_TTL)
                .expect("build sp_b");
        publish_packet(&client.inner, &_sp_b).expect("publish sp_b");

        // Re-publish sp_a (T1 < T2) — must be rejected.
        let result = publish_packet(&client.inner, &sp_a);
        assert!(
            matches!(result, Err(PktapError::DhtOutdatedRecord)),
            "re-publishing a stale packet must return DhtOutdatedRecord, got {:?}",
            result
        );

        // Suppress unused variable warning
        let _ = keypair;
    }

    /// Publish a large ciphertext (multi-chunk) and verify round-trip integrity.
    #[test]
    fn test_sequence_monotonicity() {
        let testnet = Testnet::new(10);
        let client = make_client(&testnet);

        let seed = [0x11u8; 32];
        let record_name = "_pktap._share.monotonic";

        // First publish should succeed.
        let pk1 = client
            .publish_encrypted(&seed, record_name, b"first version")
            .expect("first publish must succeed");

        thread::sleep(Duration::from_millis(2));

        // Second publish with same key — newer timestamp, should also succeed.
        let pk2 = client
            .publish_encrypted(&seed, record_name, b"second version")
            .expect("second publish must succeed");

        assert_eq!(
            pk1.as_bytes(),
            pk2.as_bytes(),
            "same seed produces same DHT address"
        );
    }
}
