package com.pktap.bridge

import uniffi.pktap_core.PktapException
import uniffi.pktap_core.`decryptAndVerify` as ffiDecryptAndVerify
import uniffi.pktap_core.`derivePublicKey` as ffiDerivePublicKey
import uniffi.pktap_core.`deriveMnemonicFromSeed` as ffiDeriveMnemonicFromSeed
import uniffi.pktap_core.`deriveSharedRecordName` as ffiDeriveSharedRecordName
import uniffi.pktap_core.`ecdhAndEncrypt` as ffiEcdhAndEncrypt
import uniffi.pktap_core.`pktapPing` as ffiPktapPing

/**
 * Bridge wrapper for UniFFI-generated Rust bindings.
 *
 * ALL callers MUST use this object — never import from uniffi.pktap_core directly.
 * This wrapper enforces D-06: ByteArray secrets are zeroed (fill(0)) immediately after FFI use.
 *
 * The generated bindings live in package `uniffi.pktap_core` and use `kotlin.ByteArray`
 * directly for `Vec<u8>` parameters (no List<UByte> conversion required).
 */
object PktapBridge {

    /**
     * Pipeline health check. Returns "pktap-ok" from Rust (FFI-02).
     */
    fun ping(): String = ffiPktapPing()

    /**
     * ECDH + encrypt contact fields. Zeros seedBytes after FFI call (D-06).
     *
     * @param seedBytes 32-byte HKDF seed — ZEROED after this call returns
     * @param peerEd25519Public 32-byte peer Ed25519 public key
     * @param contactFieldsJson JSON string of contact fields (max 750 bytes)
     * @return Encrypted opaque blob as ByteArray (version || nonce || ciphertext+tag)
     * @throws PktapException.InvalidKey if seed or peer key has wrong length
     * @throws PktapException.RecordTooLarge if contact JSON exceeds 750 bytes
     * @throws PktapException.SerializationFailed if encryption fails unexpectedly
     */
    @Throws(PktapException::class)
    fun ecdhAndEncrypt(
        seedBytes: ByteArray,
        peerEd25519Public: ByteArray,
        contactFieldsJson: String
    ): ByteArray {
        try {
            return ffiEcdhAndEncrypt(
                ourSeedBytes = seedBytes,
                peerEd25519Public = peerEd25519Public,
                contactFieldsJson = contactFieldsJson
            )
        } finally {
            // D-06: Zero seedBytes immediately — it contained secret material
            seedBytes.fill(0)
        }
    }

    /**
     * Verify signature + ECDH + decrypt record. Zeros seedBytes after FFI call (D-06).
     *
     * Signature is verified BEFORE key derivation (T-01-12 — Spoofing mitigation).
     * All internal errors map to PktapException.RecordInvalid to prevent oracle attacks (D-08).
     *
     * @param seedBytes 32-byte HKDF seed — ZEROED after this call returns
     * @param peerEd25519Public 32-byte Ed25519 public key of the record sender
     * @param peerEd25519Signature 64-byte Ed25519 signature produced by the peer over recordBytes
     * @param recordBytes Opaque D-06 byte blob to verify and decrypt
     * @return Decrypted contact fields JSON string on success
     * @throws PktapException.RecordInvalid if signature invalid, record tampered, wrong key, or decryption failed
     */
    @Throws(PktapException::class)
    fun decryptAndVerify(
        seedBytes: ByteArray,
        peerEd25519Public: ByteArray,
        peerEd25519Signature: ByteArray,
        recordBytes: ByteArray
    ): String {
        try {
            return ffiDecryptAndVerify(
                ourSeedBytes = seedBytes,
                peerEd25519Public = peerEd25519Public,
                peerEd25519Signature = peerEd25519Signature,
                recordBytes = recordBytes
            )
        } finally {
            // D-06: Zero seedBytes — secret material
            seedBytes.fill(0)
        }
    }

    /**
     * Derive the deterministic shared DHT record name. No secrets involved — no zeroing needed.
     *
     * The name is symmetric: deriveSharedRecordName(A, B) == deriveSharedRecordName(B, A).
     * Format: "_pktap._share.<hex-encoded SHA-256(sort(A, B))>".
     *
     * @param pubKeyA 32-byte Ed25519 public key of party A
     * @param pubKeyB 32-byte Ed25519 public key of party B
     * @return Deterministic DHT record name string
     * @throws PktapException.InvalidKey if either key is not exactly 32 bytes
     */
    @Throws(PktapException::class)
    fun deriveSharedRecordName(pubKeyA: ByteArray, pubKeyB: ByteArray): String {
        return ffiDeriveSharedRecordName(
            pubKeyA = pubKeyA,
            pubKeyB = pubKeyB
        )
    }

    /**
     * Derive a 12-word BIP-39 mnemonic from a 32-byte seed. Zeros seedBytes after FFI call (D-05).
     *
     * Uses first 16 bytes of seed as entropy (128-bit → 12 words). The returned mnemonic string
     * is NOT secret — it is the human-readable backup phrase shown to the user once at first launch.
     *
     * @param seedBytes 32-byte seed — ZEROED after this call returns
     * @return Space-separated 12-word BIP-39 mnemonic string
     * @throws PktapException.InvalidKey if seed is not exactly 32 bytes
     * @throws PktapException.SerializationFailed if mnemonic generation fails unexpectedly
     */
    @Throws(PktapException::class)
    fun deriveMnemonicFromSeed(seedBytes: ByteArray): String {
        try {
            return ffiDeriveMnemonicFromSeed(seedBytes = seedBytes)
        } finally {
            // D-05: Zero seedBytes immediately — it contained secret material
            seedBytes.fill(0)
        }
    }

    /**
     * Derive the Ed25519 public key from a 32-byte seed. Zeros seedBytes after FFI call (D-05).
     *
     * The returned 32-byte public key is NOT secret — cache it in AppViewModel (D-06).
     * Do NOT zero the returned ByteArray.
     *
     * @param seedBytes 32-byte seed — ZEROED after this call returns
     * @return 32-byte Ed25519 public key
     * @throws PktapException.InvalidKey if seed is not exactly 32 bytes
     */
    @Throws(PktapException::class)
    fun derivePublicKey(seedBytes: ByteArray): ByteArray {
        try {
            return ffiDerivePublicKey(seedBytes = seedBytes)
        } finally {
            // D-05: Zero seedBytes immediately — it contained secret material
            seedBytes.fill(0)
        }
    }
}
